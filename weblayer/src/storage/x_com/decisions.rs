use super::super::{Result, StorageError};
use super::{
    annotations::content_annotation_from_row, clean_optional,
    content::capture_diagnostics_from_payload_json, normalize_text, now_unix_ms, stable_post_id,
    storage_key, Store, SITE_DIR,
};
use crate::{
    core::{ContentDecision, ContentItem, DecisionAction},
    storage::{
        ContentAnnotationInput, ContentRule, RuleCatch, RuleCatchCorrection,
        RuleCatchCorrectionInput, RuleCatchPage, RuleCatchQuery, RuleDecisionStats,
        RuleUpdateInput, StoredContentItem,
    },
};
use rusqlite::{params, OptionalExtension};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const FALSE_POSITIVE_ANNOTATION_TYPE: &str = "ruleFalsePositive";
const MAX_FALSE_POSITIVE_REASON_CHARS: usize = 180;
const MAX_FALSE_POSITIVE_TEXT_CHARS: usize = 260;
const MAX_FALSE_POSITIVE_EXAMPLE_KEY_CHARS: usize = 120;
const MAX_RULE_NEGATIVE_EXAMPLES: usize = 20;

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

    pub(in crate::storage) fn latest_hide_decision(
        &self,
        item: &ContentItem,
    ) -> Result<Option<ContentDecision>> {
        let post_id = stable_post_id(item);
        let normalized_text = normalize_text(&item.text);
        let Some(storage_key) = storage_key(item, post_id.as_deref(), &normalized_text) else {
            return Ok(None);
        };

        self.connection
            .query_row(
                "
                SELECT
                    matched_rule_ids_json,
                    reason,
                    confidence
                FROM content_decision_events
                WHERE site = ?1
                    AND storage_key = ?2
                    AND action = 'hide'
                ORDER BY created_at_unix_ms DESC, id DESC
                LIMIT 1
                ",
                params![SITE_DIR, storage_key],
                |row| {
                    let matched_rule_ids_json: String = row.get(0)?;
                    let matched_rule_ids =
                        parse_matched_rule_ids(&matched_rule_ids_json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?;
                    let reason = row
                        .get::<_, Option<String>>(1)?
                        .unwrap_or_else(|| "Previously hidden by rule".into());
                    let confidence = row
                        .get::<_, Option<f64>>(2)?
                        .map(|value| value as f32)
                        .unwrap_or(1.0);
                    let label = if matched_rule_ids.is_empty() {
                        "WebLayer: hidden"
                    } else {
                        "WebLayer: hidden by rule"
                    };

                    Ok(
                        ContentDecision::hide(item.client_id.clone(), label, reason, confidence)
                            .with_matched_rule_ids(matched_rule_ids),
                    )
                },
            )
            .optional()
            .map_err(StorageError::from)
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
                tweets.latest_captured_at,
                tweets.latest_payload_json
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
                        capture_diagnostics: row
                            .get::<_, Option<String>>(16)?
                            .as_deref()
                            .and_then(capture_diagnostics_from_payload_json),
                    },
                    correction: None,
                },
            })
        })?;
        let mut items = Vec::new();

        for row in rows {
            let candidate = row?;
            let matched_rule_ids = parse_matched_rule_ids(&candidate.matched_rule_ids_json)?;
            if !matched_rule_ids.iter().any(|id| id == &rule_id) {
                continue;
            }

            items.push(candidate.caught);
        }
        items.extend(self.corrected_rule_catches(&rule_id)?);
        items.sort_by(|left, right| {
            catch_sort_time(right)
                .cmp(&catch_sort_time(left))
                .then_with(|| right.event_id.cmp(&left.event_id))
                .then_with(|| left.content.storage_key.cmp(&right.content.storage_key))
        });

        let total_matching = items.len();
        let items = items.into_iter().skip(offset).take(limit).collect();

        Ok(Some(RuleCatchPage {
            rule_id,
            total_matching,
            limit,
            offset,
            items,
        }))
    }

    fn corrected_rule_catches(&self, rule_id: &str) -> Result<Vec<RuleCatch>> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                annotations.id,
                annotations.storage_key,
                annotations.content_kind,
                annotations.annotation_type,
                annotations.annotation_key,
                annotations.value_json,
                annotations.confidence,
                annotations.source,
                annotations.created_at_unix_ms,
                annotations.updated_at_unix_ms,
                tweets.post_id,
                tweets.url,
                tweets.author_handle,
                COALESCE(tweets.text, ''),
                COALESCE(tweets.first_seen_at_unix_ms, annotations.created_at_unix_ms),
                COALESCE(tweets.last_seen_at_unix_ms, annotations.updated_at_unix_ms),
                COALESCE(tweets.seen_count, 0),
                tweets.latest_captured_at,
                tweets.latest_payload_json
            FROM content_annotations annotations
            LEFT JOIN tweets ON tweets.storage_key = annotations.storage_key
            WHERE annotations.annotation_type = ?1
                AND annotations.annotation_key = ?2
            ",
        )?;
        let rows =
            statement.query_map(params![FALSE_POSITIVE_ANNOTATION_TYPE, rule_id], |row| {
                let annotation = content_annotation_from_row(row)?;
                let tweet_text: String = row.get(13)?;
                let event_id = annotation
                    .value
                    .get("eventId")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                let caught_at_unix_ms = annotation
                    .value
                    .pointer("/originalDecision/createdAtUnixMs")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(annotation.created_at_unix_ms);

                Ok(RuleCatch {
                    event_id,
                    caught_at_unix_ms,
                    action: annotation
                        .value
                        .pointer("/originalDecision/action")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("hide")
                        .to_string(),
                    reason: annotation
                        .value
                        .pointer("/originalDecision/reason")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned),
                    confidence: annotation
                        .value
                        .pointer("/originalDecision/confidence")
                        .and_then(serde_json::Value::as_f64),
                    source: annotation
                        .value
                        .pointer("/originalDecision/source")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(annotation.source.as_str())
                        .to_string(),
                    content: StoredContentItem {
                        content_kind: "post".into(),
                        storage_key: annotation.storage_key.clone(),
                        content_id: row.get(10)?,
                        url: row.get(11)?,
                        author: row.get(12)?,
                        text: (!tweet_text.trim().is_empty())
                            .then_some(tweet_text)
                            .or_else(|| {
                                annotation
                                    .value
                                    .pointer("/post/text")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToOwned::to_owned)
                            })
                            .unwrap_or_default(),
                        snippet: None,
                        first_seen_at_unix_ms: row.get(14)?,
                        last_seen_at_unix_ms: row.get(15)?,
                        seen_count: row.get(16)?,
                        latest_captured_at: row.get(17)?,
                        capture_diagnostics: row
                            .get::<_, Option<String>>(18)?
                            .as_deref()
                            .and_then(capture_diagnostics_from_payload_json),
                    },
                    correction: Some(annotation),
                })
            })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    pub(in crate::storage) fn correct_rule_catch(
        &mut self,
        input: RuleCatchCorrectionInput,
    ) -> Result<Option<RuleCatchCorrection>> {
        let Some(rule_id) = clean_optional(Some(input.rule_id.as_str())) else {
            return Ok(None);
        };
        let Some(rule_detail) = self.rule_detail(&rule_id)? else {
            return Ok(None);
        };
        let Some(event) = self.rule_catch_event(input.event_id)? else {
            return Ok(None);
        };
        let matched_rule_ids = parse_matched_rule_ids(&event.matched_rule_ids_json)?;
        if event.action != "hide" || !matched_rule_ids.iter().any(|id| id == &rule_id) {
            return Ok(None);
        }

        let reason = clean_optional(Some(input.reason.as_str()))
            .unwrap_or_else(|| "Wanted to see this post.".into());
        let source =
            clean_optional(Some(input.source.as_str())).unwrap_or_else(|| "dashboard".into());
        let corrected_at_unix_ms = now_unix_ms();
        let remaining_matched_rule_ids = remaining_matched_rule_ids(&matched_rule_ids, &rule_id);
        let annotation = self.upsert_content_annotation(ContentAnnotationInput {
            storage_key: event.storage_key.clone(),
            content_kind: "post".into(),
            annotation_type: FALSE_POSITIVE_ANNOTATION_TYPE.into(),
            key: rule_id.clone(),
            value: json!({
                "verdict": "falsePositive",
                "ruleId": &rule_id,
                "eventId": event.id,
                "reason": &reason,
                "correctedAtUnixMs": corrected_at_unix_ms,
                "post": {
                    "storageKey": &event.storage_key,
                    "postId": &event.post_id,
                    "text": &event.text,
                },
                "originalDecision": {
                    "action": &event.action,
                    "matchedRuleIds": &matched_rule_ids,
                    "reason": &event.reason,
                    "confidence": event.confidence,
                    "source": &event.source,
                    "createdAtUnixMs": event.created_at_unix_ms,
                },
            }),
            confidence: None,
            source: source.clone(),
        })?;
        let rule = self.rule_with_false_positive_example(
            &rule_detail.rule,
            event.text.as_deref(),
            &reason,
            &source,
        )?;

        if remaining_matched_rule_ids.is_empty() {
            self.connection.execute(
                "
                DELETE FROM content_decision_events
                WHERE id = ?1
                ",
                params![event.id],
            )?;
        } else {
            let remaining_json = serde_json::to_string(&remaining_matched_rule_ids)?;
            self.connection.execute(
                "
                UPDATE content_decision_events
                SET matched_rule_ids_json = ?2
                WHERE id = ?1
                ",
                params![event.id, remaining_json],
            )?;
        }

        Ok(Some(RuleCatchCorrection {
            rule_id,
            event_id: event.id,
            storage_key: annotation.storage_key.clone(),
            content_id: event.post_id,
            corrected_at_unix_ms,
            removed_event: remaining_matched_rule_ids.is_empty(),
            remaining_matched_rule_ids,
            annotation,
            rule,
        }))
    }

    fn rule_catch_event(&self, event_id: i64) -> Result<Option<RuleCatchEvent>> {
        self.connection
            .query_row(
                "
                SELECT
                    events.id,
                    events.storage_key,
                    events.post_id,
                    events.created_at_unix_ms,
                    events.action,
                    events.matched_rule_ids_json,
                    events.reason,
                    events.confidence,
                    events.source,
                    tweets.text
                FROM content_decision_events events
                LEFT JOIN tweets ON tweets.storage_key = events.storage_key
                WHERE events.site = ?1
                    AND events.id = ?2
                ",
                params![SITE_DIR, event_id],
                |row| {
                    Ok(RuleCatchEvent {
                        id: row.get(0)?,
                        storage_key: row.get(1)?,
                        post_id: row.get(2)?,
                        created_at_unix_ms: row.get(3)?,
                        action: row.get(4)?,
                        matched_rule_ids_json: row.get(5)?,
                        reason: row.get(6)?,
                        confidence: row.get(7)?,
                        source: row.get(8)?,
                        text: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
    }

    fn rule_with_false_positive_example(
        &mut self,
        rule: &ContentRule,
        text: Option<&str>,
        reason: &str,
        source: &str,
    ) -> Result<ContentRule> {
        let Some(example) = false_positive_example(text, reason) else {
            return Ok(rule.clone());
        };
        let normalized_example_text = false_positive_example_key(text);
        let mut negative_examples = rule.examples.negative.clone();
        if let Some(existing) = negative_examples.iter_mut().find(|existing| {
            normalized_example_text
                .as_ref()
                .is_some_and(|text| normalize_text(existing).contains(text))
        }) {
            *existing = example;
        } else {
            negative_examples.push(example);
        }
        let overflow = negative_examples
            .len()
            .saturating_sub(MAX_RULE_NEGATIVE_EXAMPLES);
        if overflow > 0 {
            negative_examples.drain(0..overflow);
        }

        let Some(rule) = self.update_rule(RuleUpdateInput {
            id: rule.id.clone(),
            status: None,
            priority: None,
            title: None,
            instruction: None,
            source: source.to_string(),
            positive_examples: None,
            negative_examples: Some(negative_examples),
        })?
        else {
            return Ok(rule.clone());
        };

        Ok(rule)
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

struct RuleCatchEvent {
    id: i64,
    storage_key: String,
    post_id: Option<String>,
    created_at_unix_ms: i64,
    action: String,
    matched_rule_ids_json: String,
    reason: Option<String>,
    confidence: Option<f64>,
    source: String,
    text: Option<String>,
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

fn catch_sort_time(caught: &RuleCatch) -> i64 {
    caught
        .correction
        .as_ref()
        .map(|correction| correction.updated_at_unix_ms)
        .unwrap_or(caught.caught_at_unix_ms)
}

fn remaining_matched_rule_ids(matched_rule_ids: &[String], removed_rule_id: &str) -> Vec<String> {
    let mut remaining = Vec::new();
    for rule_id in matched_rule_ids {
        if rule_id == removed_rule_id || remaining.contains(rule_id) {
            continue;
        }
        remaining.push(rule_id.clone());
    }
    remaining
}

fn false_positive_example(text: Option<&str>, reason: &str) -> Option<String> {
    let text = normalize_text(text?);
    if text.is_empty() {
        return None;
    }

    let text = truncate_chars(&text, MAX_FALSE_POSITIVE_TEXT_CHARS);
    let reason = truncate_chars(&normalize_text(reason), MAX_FALSE_POSITIVE_REASON_CHARS);
    let example = if reason.is_empty() {
        format!("Do not match this post.\nPost: {text}")
    } else {
        format!("Do not match this post.\nReason to keep: {reason}\nPost: {text}")
    };

    Some(example)
}

fn false_positive_example_key(text: Option<&str>) -> Option<String> {
    let text = normalize_text(text?);
    (!text.is_empty()).then_some(truncate_chars(&text, MAX_FALSE_POSITIVE_EXAMPLE_KEY_CHARS))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    text.chars().take(max_chars).collect()
}
