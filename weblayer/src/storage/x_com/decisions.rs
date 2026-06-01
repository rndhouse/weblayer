use super::super::{Result, StorageError};
use super::{
    clean_optional, normalize_text, now_unix_ms, stable_post_id, storage_key, Store, SITE_DIR,
};
use crate::{
    core::{ContentDecision, ContentItem, DecisionAction},
    storage::{RuleCatch, RuleCatchPage, RuleCatchQuery, RuleDecisionStats, StoredContentItem},
};
use rusqlite::params;
use std::collections::{BTreeMap, BTreeSet};

impl Store {
    pub(in crate::storage) fn record_decision_event(
        &mut self,
        item: &ContentItem,
        decision: &ContentDecision,
        source: &str,
    ) -> Result<bool> {
        if !should_record_decision(decision) {
            return Ok(false);
        }

        let post_id = stable_post_id(item);
        let normalized_text = normalize_text(&item.text);
        let Some(storage_key) = storage_key(item, post_id.as_deref(), &normalized_text) else {
            return Ok(false);
        };
        let matched_rule_ids_json = serde_json::to_string(&decision.matched_rule_ids)?;
        let reason = clean_optional(decision.reason.as_deref());
        let source = clean_optional(Some(source)).unwrap_or_else(|| "daemon".into());
        let confidence = decision.confidence.map(f64::from);

        self.connection.execute(
            "
            INSERT INTO content_decision_events (
                site,
                storage_key,
                post_id,
                created_at_unix_ms,
                client_id,
                action,
                matched_rule_ids_json,
                reason,
                confidence,
                source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ",
            params![
                SITE_DIR,
                storage_key,
                post_id,
                now_unix_ms(),
                decision.client_id.as_str(),
                decision_action_name(decision.action),
                matched_rule_ids_json,
                reason,
                confidence,
                source,
            ],
        )?;

        Ok(true)
    }

    pub(in crate::storage) fn rule_decision_stats(&self) -> Result<Vec<RuleDecisionStats>> {
        let mut statement = self.connection.prepare(
            "
            SELECT action, matched_rule_ids_json
            FROM content_decision_events
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut stats = BTreeMap::<String, RuleDecisionStatsBuilder>::new();

        for row in rows {
            let (action, matched_rule_ids_json) = row?;
            let matched_rule_ids = parse_matched_rule_ids(&matched_rule_ids_json)?;
            let unique_rule_ids = matched_rule_ids.into_iter().collect::<BTreeSet<_>>();

            for rule_id in unique_rule_ids {
                let entry = stats.entry(rule_id).or_default();
                entry.matched_count += 1;
                if action == "hide" {
                    entry.hide_count += 1;
                }
            }
        }

        Ok(stats
            .into_iter()
            .map(|(rule_id, counts)| RuleDecisionStats {
                rule_id,
                matched_count: counts.matched_count,
                hide_count: counts.hide_count,
            })
            .collect())
    }

    pub(in crate::storage) fn rule_catches(
        &self,
        rule_id: &str,
        query: RuleCatchQuery,
    ) -> Result<Option<RuleCatchPage>> {
        let Some(rule_id) = clean_optional(Some(rule_id)) else {
            return Ok(None);
        };
        if !self.rule_exists(&rule_id)? {
            return Ok(None);
        }

        let limit = query.limit.min(i64::MAX as usize);
        let offset = query.offset.min(i64::MAX as usize);
        let mut statement = self.connection.prepare(
            "
            SELECT
                events.id,
                events.created_at_unix_ms,
                events.action,
                events.matched_rule_ids_json,
                events.reason,
                events.confidence,
                events.source,
                events.storage_key,
                COALESCE(tweets.post_id, events.post_id),
                tweets.url,
                tweets.author_handle,
                COALESCE(tweets.text, ''),
                COALESCE(tweets.first_seen_at_unix_ms, events.created_at_unix_ms),
                COALESCE(tweets.last_seen_at_unix_ms, events.created_at_unix_ms),
                COALESCE(tweets.seen_count, 0),
                tweets.latest_captured_at
            FROM content_decision_events events
            LEFT JOIN tweets ON tweets.storage_key = events.storage_key
            WHERE events.site = ?1
                AND events.action = 'hide'
            ORDER BY events.created_at_unix_ms DESC, events.id DESC
            ",
        )?;
        let rows = statement.query_map(params![SITE_DIR], |row| {
            Ok(RuleCatchCandidate {
                matched_rule_ids_json: row.get(3)?,
                caught: RuleCatch {
                    event_id: row.get(0)?,
                    caught_at_unix_ms: row.get(1)?,
                    action: row.get(2)?,
                    reason: row.get(4)?,
                    confidence: row.get(5)?,
                    source: row.get(6)?,
                    content: StoredContentItem {
                        content_kind: "post".into(),
                        storage_key: row.get(7)?,
                        content_id: row.get(8)?,
                        url: row.get(9)?,
                        author: row.get(10)?,
                        text: row.get(11)?,
                        snippet: None,
                        first_seen_at_unix_ms: row.get(12)?,
                        last_seen_at_unix_ms: row.get(13)?,
                        seen_count: row.get(14)?,
                        latest_captured_at: row.get(15)?,
                    },
                },
            })
        })?;
        let mut total_matching = 0usize;
        let mut items = Vec::new();

        for row in rows {
            let candidate = row?;
            let matched_rule_ids = parse_matched_rule_ids(&candidate.matched_rule_ids_json)?;
            if !matched_rule_ids.iter().any(|id| id == &rule_id) {
                continue;
            }

            if total_matching >= offset && items.len() < limit {
                items.push(candidate.caught);
            }
            total_matching += 1;
        }

        Ok(Some(RuleCatchPage {
            rule_id,
            total_matching,
            limit,
            offset,
            items,
        }))
    }

    fn rule_exists(&self, rule_id: &str) -> Result<bool> {
        let exists = self.connection.query_row(
            "
            SELECT EXISTS(
                SELECT 1
                FROM content_rules
                WHERE site = ?1
                    AND id = ?2
            )
            ",
            params![SITE_DIR, rule_id],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(exists != 0)
    }
}

#[derive(Default)]
struct RuleDecisionStatsBuilder {
    matched_count: usize,
    hide_count: usize,
}

struct RuleCatchCandidate {
    matched_rule_ids_json: String,
    caught: RuleCatch,
}

fn should_record_decision(decision: &ContentDecision) -> bool {
    matches!(decision.action, DecisionAction::Hide) || !decision.matched_rule_ids.is_empty()
}

fn decision_action_name(action: DecisionAction) -> &'static str {
    match action {
        DecisionAction::Keep => "keep",
        DecisionAction::Hide => "hide",
        DecisionAction::Dim => "dim",
        DecisionAction::Label => "label",
        DecisionAction::Replace => "replace",
    }
}

fn parse_matched_rule_ids(value: &str) -> Result<Vec<String>> {
    serde_json::from_str(value).map_err(StorageError::from)
}
