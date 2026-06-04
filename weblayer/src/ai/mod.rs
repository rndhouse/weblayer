mod codex_app;
mod summary_cache;

use crate::{
    core::ContentItem,
    storage::{ContentRule, RuleDecisionStats, RuleSetProposalChange, XDislikedPost},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};
use summary_cache::SummaryCache;
use tokio::sync::{watch, Mutex as AsyncMutex};
use tracing::{debug, warn, Level};

const CODEX_ENABLED_ENV: &str = "WEBLAYER_CODEX_APP_ENABLED";

/// Shared AI analyzer used by site handlers.
#[derive(Clone)]
pub struct AiAnalyzer {
    codex_app: Option<Arc<codex_app::CodexAppAnalyzer>>,
    x_summary_cache: Arc<Mutex<SummaryCache>>,
    x_inflight_opinions: Arc<AsyncMutex<HashMap<String, watch::Sender<bool>>>>,
}

impl AiAnalyzer {
    /// Builds the analyzer from local daemon environment variables.
    pub fn from_env() -> Self {
        let codex_enabled = env_flag_default(CODEX_ENABLED_ENV, true);

        let codex_app = codex_enabled.then(|| Arc::new(codex_app::CodexAppAnalyzer::from_env()));
        Self {
            codex_app,
            x_summary_cache: Arc::new(Mutex::new(SummaryCache::from_env())),
            x_inflight_opinions: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }

    /// Gets Codex opinions for X content.
    pub async fn x_opinions(
        &self,
        items: &[ContentItem],
        rules: &[AiContentRule],
    ) -> Option<Vec<AiOpinion>> {
        let mut opinions = Vec::new();
        let mut misses = Vec::new();
        let now = Instant::now();
        let rule_scope = rule_cache_scope(rules);

        {
            let mut cache = self
                .x_summary_cache
                .lock()
                .expect("X summary cache mutex should not be poisoned");

            for item in items {
                if let Some(hit) = cache.get(item, &rule_scope, now) {
                    opinions.push(hit);
                } else {
                    misses.push(item.clone());
                }
            }
        }

        if misses.is_empty() {
            return Some(opinions);
        }

        let Some(codex_app) = self.codex_app.as_ref() else {
            return (!opinions.is_empty()).then_some(opinions);
        };

        let prepared_misses = self.prepare_x_opinion_misses(&misses, &rule_scope).await;

        for item in &prepared_misses.fresh_items {
            debug_x_agent_query_item(item);
        }

        let mut uncacheable_opinions = Vec::new();
        if !prepared_misses.fresh_items.is_empty() {
            match codex_app
                .x_opinions(&prepared_misses.fresh_items, rules)
                .await
            {
                Ok(fresh_opinions) => {
                    let fresh_items_by_client_id: HashMap<_, _> = prepared_misses
                        .fresh_items
                        .iter()
                        .map(|item| (item.client_id.as_str(), item))
                        .collect();
                    let now = Instant::now();

                    {
                        let mut cache = self
                            .x_summary_cache
                            .lock()
                            .expect("X summary cache mutex should not be poisoned");

                        for opinion in &fresh_opinions {
                            if let Some(item) =
                                fresh_items_by_client_id.get(opinion.client_id.as_str())
                            {
                                if SummaryCache::cache_key_for_item(item, &rule_scope).is_some() {
                                    cache.insert(item, &rule_scope, opinion, now);
                                } else {
                                    uncacheable_opinions.push(opinion.clone());
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    warn!(%error, "codex app-server opinion unavailable");
                }
            }
        }

        self.complete_x_opinion_misses(&prepared_misses.leader_cache_keys)
            .await;

        for mut waiter in prepared_misses.waiters {
            if !*waiter.borrow() {
                let _ = waiter.changed().await;
            }
        }

        {
            let now = Instant::now();
            let mut cache = self
                .x_summary_cache
                .lock()
                .expect("X summary cache mutex should not be poisoned");

            for item in misses
                .iter()
                .filter(|item| SummaryCache::cache_key_for_item(item, &rule_scope).is_some())
            {
                if let Some(hit) = cache.get(item, &rule_scope, now) {
                    opinions.push(hit);
                }
            }
        }

        opinions.extend(uncacheable_opinions);
        (!opinions.is_empty()).then_some(opinions)
    }

    async fn prepare_x_opinion_misses(
        &self,
        misses: &[ContentItem],
        rule_scope: &str,
    ) -> PreparedOpinionMisses {
        let mut fresh_items = Vec::new();
        let mut leader_cache_keys = Vec::new();
        let mut waiters = Vec::new();
        let mut local_cache_keys = HashSet::new();
        let mut inflight = self.x_inflight_opinions.lock().await;

        for item in misses {
            let Some(cache_key) = SummaryCache::cache_key_for_item(item, rule_scope) else {
                fresh_items.push(item.clone());
                continue;
            };

            if !local_cache_keys.insert(cache_key.clone()) {
                if let Some(waiter) = inflight.get(cache_key.as_str()) {
                    waiters.push(waiter.subscribe());
                }
                continue;
            }

            if let Some(waiter) = inflight.get(cache_key.as_str()) {
                waiters.push(waiter.subscribe());
                continue;
            }

            let (sender, _receiver) = watch::channel(false);
            inflight.insert(cache_key.clone(), sender);
            leader_cache_keys.push(cache_key);
            fresh_items.push(item.clone());
        }

        PreparedOpinionMisses {
            fresh_items,
            leader_cache_keys,
            waiters,
        }
    }

    async fn complete_x_opinion_misses(&self, leader_cache_keys: &[String]) {
        if leader_cache_keys.is_empty() {
            return;
        }

        let mut waiters = Vec::new();
        {
            let mut inflight = self.x_inflight_opinions.lock().await;
            for cache_key in leader_cache_keys {
                if let Some(waiter) = inflight.remove(cache_key) {
                    waiters.push(waiter);
                }
            }
        }

        for waiter in waiters {
            let _ = waiter.send(true);
        }
    }

    /// Gets X opinions only when every requested item is already cached.
    pub fn cached_x_opinions(
        &self,
        items: &[ContentItem],
        rules: &[AiContentRule],
    ) -> Option<Vec<AiOpinion>> {
        let now = Instant::now();
        let rule_scope = rule_cache_scope(rules);
        let mut cache = self
            .x_summary_cache
            .lock()
            .expect("X summary cache mutex should not be poisoned");
        let mut opinions = Vec::with_capacity(items.len());

        for item in items {
            opinions.push(cache.get(item, &rule_scope, now)?);
        }

        Some(opinions)
    }

    /// Gets a Codex-generated proposal for reconciling X rules with feedback.
    pub async fn x_rule_set_proposal(
        &self,
        feedback: &[XDislikedPost],
        active_rules: &[ContentRule],
        rule_stats: &[RuleDecisionStats],
    ) -> Option<Vec<RuleSetProposalChange>> {
        let Some(codex_app) = self.codex_app.as_ref() else {
            return None;
        };

        match codex_app
            .x_rule_set_proposal(feedback, active_rules, rule_stats)
            .await
        {
            Ok(changes) => Some(changes),
            Err(error) => {
                warn!(%error, "codex app-server rule proposal unavailable");
                None
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests_with_x_summaries(summaries: &[(&ContentItem, &str, f32)]) -> Self {
        let mut cache = SummaryCache::from_env();
        let now = Instant::now();

        for (item, summary, confidence) in summaries {
            cache.insert(
                item,
                &rule_cache_scope(&[]),
                &AiOpinion {
                    client_id: item.client_id.clone(),
                    action: AiAction::Keep,
                    opinion: (*summary).into(),
                    confidence: *confidence,
                    matched_rule_ids: Vec::new(),
                },
                now,
            );
        }

        Self {
            codex_app: None,
            x_summary_cache: Arc::new(Mutex::new(cache)),
            x_inflight_opinions: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }
}

struct PreparedOpinionMisses {
    fresh_items: Vec<ContentItem>,
    leader_cache_keys: Vec<String>,
    waiters: Vec<watch::Receiver<bool>>,
}

fn debug_x_agent_query_item(item: &ContentItem) {
    if !tracing::enabled!(target: "weblayer_daemon::ai", Level::DEBUG) {
        return;
    }

    debug!(
        target: "weblayer_daemon::ai",
        post_url = item.url.as_deref(),
        client_id = item.client_id.as_str(),
        content_id = item.content_id.as_deref(),
        author = item.author.as_deref(),
        "querying Codex app-server for X post"
    );
}

fn env_flag_default(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
        .unwrap_or(default)
}

/// Active user rule sent to the AI analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiContentRule {
    /// Stable rule ID.
    pub id: String,
    /// Rule priority. Lower numbers run earlier.
    pub priority: i64,
    /// Short human-readable rule name.
    pub title: String,
    /// Agent-facing instruction text.
    pub instruction: String,
    /// Rule update timestamp.
    pub updated_at_unix_ms: i64,
    /// Examples that should match this rule.
    pub positive_examples: Vec<String>,
    /// Examples that should not match this rule.
    pub negative_examples: Vec<String>,
}

impl From<ContentRule> for AiContentRule {
    fn from(rule: ContentRule) -> Self {
        Self {
            id: rule.id,
            priority: rule.priority,
            title: rule.title,
            instruction: rule.instruction,
            updated_at_unix_ms: rule.updated_at_unix_ms,
            positive_examples: rule.examples.positive,
            negative_examples: rule.examples.negative,
        }
    }
}

/// AI opinion attached to one analyzed content item.
#[derive(Debug, Clone)]
pub struct AiOpinion {
    /// Client-generated ID from the analyzed content item.
    pub client_id: String,
    /// Rule-driven action to apply to the item.
    pub action: AiAction,
    /// Short opinion suitable for a browser label.
    pub opinion: String,
    /// Model confidence on a `0.0..=1.0` scale.
    pub confidence: f32,
    /// Active rule IDs that caused a hide decision.
    pub matched_rule_ids: Vec<String>,
}

/// AI action for an analyzed content item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAction {
    /// Leave the item visible.
    Keep,
    /// Hide the item because it matches at least one active rule.
    Hide,
}

fn rule_cache_scope(rules: &[AiContentRule]) -> String {
    if rules.is_empty() {
        return "none".into();
    }

    let mut text = String::new();
    for rule in rules {
        text.push_str(&rule.id);
        text.push('\n');
        text.push_str(&rule.priority.to_string());
        text.push('\n');
        text.push_str(&rule.title);
        text.push('\n');
        text.push_str(&rule.instruction);
        text.push('\n');
        for example in &rule.positive_examples {
            text.push_str("+ ");
            text.push_str(example);
            text.push('\n');
        }
        for example in &rule.negative_examples {
            text.push_str("- ");
            text.push_str(example);
            text.push('\n');
        }
    }

    format!("{:016x}", stable_hash(&text))
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tokio::time::{timeout, Duration};

    fn item(client_id: &str, content_id: &str) -> ContentItem {
        ContentItem {
            client_id: client_id.into(),
            content_id: Some(content_id.into()),
            url: Some(format!("https://x.com/user/status/{content_id}")),
            author: Some("@user".into()),
            text: "post text".into(),
            captured_at: None,
            kind: Some("post".into()),
            metadata: Value::Null,
        }
    }

    #[tokio::test]
    async fn opinion_misses_dedupe_duplicate_posts_in_one_batch() {
        let analyzer = AiAnalyzer::for_tests_with_x_summaries(&[]);
        let rule_scope = rule_cache_scope(&[]);
        let misses = vec![item("first-client", "123"), item("second-client", "123")];

        let prepared = analyzer
            .prepare_x_opinion_misses(&misses, &rule_scope)
            .await;

        assert_eq!(prepared.fresh_items.len(), 1);
        assert_eq!(prepared.fresh_items[0].client_id, "first-client");
        assert_eq!(prepared.leader_cache_keys.len(), 1);
        assert_eq!(
            prepared.waiters.len(),
            1,
            "the duplicate post should wait for the same in-flight opinion"
        );

        analyzer
            .complete_x_opinion_misses(&prepared.leader_cache_keys)
            .await;
        let mut waiter = prepared.waiters.into_iter().next().unwrap();
        timeout(Duration::from_millis(50), async {
            if !*waiter.borrow() {
                waiter.changed().await.expect("waiter should be signaled");
            }
        })
        .await
        .expect("duplicate waiter should not hang");
    }

    #[tokio::test]
    async fn opinion_misses_join_existing_inflight_post() {
        let analyzer = AiAnalyzer::for_tests_with_x_summaries(&[]);
        let rule_scope = rule_cache_scope(&[]);

        let first = analyzer
            .prepare_x_opinion_misses(&[item("first-client", "123")], &rule_scope)
            .await;
        let second = analyzer
            .prepare_x_opinion_misses(&[item("second-client", "123")], &rule_scope)
            .await;

        assert_eq!(first.fresh_items.len(), 1);
        assert_eq!(second.fresh_items.len(), 0);
        assert_eq!(second.leader_cache_keys.len(), 0);
        assert_eq!(
            second.waiters.len(),
            1,
            "a concurrent duplicate should join the existing in-flight opinion"
        );

        analyzer
            .complete_x_opinion_misses(&first.leader_cache_keys)
            .await;
        let mut waiter = second.waiters.into_iter().next().unwrap();
        timeout(Duration::from_millis(50), async {
            if !*waiter.borrow() {
                waiter.changed().await.expect("waiter should be signaled");
            }
        })
        .await
        .expect("in-flight waiter should not hang");
    }
}
