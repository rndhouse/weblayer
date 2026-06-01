use super::{rule_curation, AppState};
use crate::{
    core::{
        DebugStatsMetric, DebugStatsPanel, DebugStatsSection, DomAnalysisBatch, DomCommand,
        DomElementSnapshot, FeedbackKind, PageSnapshot,
    },
    sites,
    storage::{ContentStore, RuleQuery, StorageError, XDislikeQuery},
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

pub(super) async fn analyze_dom(
    State(state): State<AppState>,
    Json(batch): Json<DomAnalysisBatch>,
) -> Json<DomAnalyzeResponse> {
    if state.log_captured_content {
        log_dom_batch(&batch);
    }

    let mut commands = sites::analyze_dom(&batch, &state.ai_analyzer, &state.content_store).await;
    append_debug_stats_command(&state, &batch.page.url, &mut commands);
    rule_curation::schedule_x_rule_curation(state);

    Json(DomAnalyzeResponse { commands })
}

pub(super) async fn dom_feedback(
    State(state): State<AppState>,
    Json(request): Json<DomFeedbackRequest>,
) -> Result<Json<DomFeedbackResponse>, super::error::ApiError> {
    let DomFeedbackRequest {
        feedback,
        page,
        element,
        reason,
        feedback_context_id,
    } = request;
    let batch = DomAnalysisBatch {
        page,
        elements: vec![element],
    };
    if state.log_captured_content {
        log_dom_batch(&batch);
    }

    let mut commands = sites::apply_feedback(
        &batch,
        feedback,
        reason.as_str(),
        feedback_context_id.as_str(),
        &state.content_store,
    )?;
    append_debug_stats_command(&state, &batch.page.url, &mut commands);
    rule_curation::schedule_x_rule_curation(state);

    Ok(Json(DomFeedbackResponse { commands }))
}

pub(super) fn append_debug_stats_command(
    state: &AppState,
    page_url: &str,
    commands: &mut Vec<DomCommand>,
) {
    if !state.x_debug_stats || !is_x_com_url(page_url) {
        return;
    }

    match x_debug_stats_command(&state.content_store) {
        Ok(command) => commands.push(command),
        Err(error) => {
            warn!(%error, "failed to build X debug stats command");
        }
    }
}

fn x_debug_stats_command(content_store: &ContentStore) -> Result<DomCommand, StorageError> {
    let content_stats = content_store.x_content_stats()?;
    let active_feedback = content_store
        .x_dislikes(XDislikeQuery {
            active: Some(true),
            unprocessed: None,
            limit: 1,
            offset: 0,
        })?
        .total_matching;
    let active_rules = content_store
        .x_rules(RuleQuery {
            status: Some("active".into()),
            limit: 1,
            offset: 0,
        })?
        .total_matching;
    let curation_status = content_store.x_rule_curation_status()?;
    let mut rule_stats = content_store.x_rule_decision_stats()?;
    rule_stats.sort_by(|left, right| {
        right
            .hide_count
            .cmp(&left.hide_count)
            .then_with(|| right.matched_count.cmp(&left.matched_count))
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });

    let mut sections = vec![
        DebugStatsSection {
            title: "Storage".into(),
            metrics: vec![
                metric("Unique posts", content_stats.unique_items),
                metric("Post encounters", content_stats.total_encounters),
                metric("Active feedback", active_feedback),
                metric("Active rules", active_rules),
            ],
        },
        DebugStatsSection {
            title: "Rule curation".into(),
            metrics: vec![
                metric(
                    "Queued feedback",
                    curation_status.unprocessed_feedback_count,
                ),
                metric(
                    "Encounters since curation",
                    curation_status.encounters_since_last_run,
                ),
            ],
        },
    ];

    let rule_metrics = rule_stats
        .into_iter()
        .take(5)
        .map(|stats| DebugStatsMetric {
            label: stats.rule_id,
            value: stats.hide_count.to_string(),
            detail: Some(format!("{} matches", stats.matched_count)),
        })
        .collect::<Vec<_>>();

    sections.push(DebugStatsSection {
        title: "Rule catches".into(),
        metrics: if rule_metrics.is_empty() {
            vec![DebugStatsMetric {
                label: "Recorded rule hides".into(),
                value: "0".into(),
                detail: None,
            }]
        } else {
            rule_metrics
        },
    });

    Ok(DomCommand::debug_stats(DebugStatsPanel {
        site: "x.com".into(),
        title: "WebLayer stats".into(),
        generated_at_unix_ms: now_unix_ms(),
        sections,
    }))
}

fn metric(label: &str, value: usize) -> DebugStatsMetric {
    DebugStatsMetric {
        label: label.into(),
        value: value.to_string(),
        detail: None,
    }
}

fn is_x_com_url(page_url: &str) -> bool {
    let Some(host) = url_host(page_url) else {
        return false;
    };
    let host = host.trim_start_matches("www.").to_ascii_lowercase();

    host == "x.com"
        || host.ends_with(".x.com")
        || host == "twitter.com"
        || host.ends_with(".twitter.com")
}

fn url_host(page_url: &str) -> Option<&str> {
    let trimmed = page_url.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let end = without_scheme
        .find(['/', '?', '#'])
        .unwrap_or(without_scheme.len());
    let host = without_scheme[..end]
        .split('@')
        .next_back()?
        .split(':')
        .next()?;

    (!host.is_empty()).then_some(host)
}

pub(super) fn log_dom_batch(batch: &DomAnalysisBatch) {
    let received_at_unix_ms = now_unix_ms();

    for element in &batch.elements {
        match serde_json::to_string(element) {
            Ok(element_json) => {
                info!(
                    target: "weblayer_daemon::captured_dom",
                    page_url = batch.page.url.as_str(),
                    client_id = element.client_id.as_str(),
                    selector = element.selector.as_deref(),
                    snapshot_hash = element.snapshot_hash.as_deref(),
                    received_at_unix_ms,
                    element = %element_json,
                    "captured DOM region"
                );
            }
            Err(error) => {
                warn!(%error, "failed to serialize DOM snapshot for logging");
            }
        }
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Response for the DOM snapshot analysis endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomAnalyzeResponse {
    /// Commands for the extension's generic DOM executor.
    pub commands: Vec<DomCommand>,
}

/// Request for applying user feedback to one captured DOM region.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomFeedbackRequest {
    /// Feedback signal chosen by the user.
    pub feedback: FeedbackKind,
    /// Optional user-supplied reason for the feedback.
    #[serde(default)]
    pub reason: String,
    /// Snapshot metadata for the live page.
    pub page: PageSnapshot,
    /// DOM region that received feedback.
    pub element: DomElementSnapshot,
    /// Opaque daemon-side rule context ID emitted with the feedback control.
    pub feedback_context_id: String,
}

/// Response for a DOM feedback request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomFeedbackResponse {
    /// Commands for the extension's generic DOM executor.
    pub commands: Vec<DomCommand>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{AnalysisBatch, ContentItem, DomCommandAction};
    use serde_json::Value;
    use std::path::PathBuf;

    fn temp_data_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "weblayer-dom-api-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn item() -> ContentItem {
        ContentItem {
            client_id: "client-1".into(),
            content_id: Some("123".into()),
            url: Some("https://x.com/user/status/123".into()),
            author: Some("@user".into()),
            text: "hello".into(),
            captured_at: None,
            kind: Some("post".into()),
            metadata: Value::Null,
        }
    }

    #[test]
    fn x_debug_stats_command_reports_storage_counts() {
        let data_dir = temp_data_dir("debug-stats-counts");
        let store = ContentStore::with_data_dir(&data_dir).expect("store should open");
        store
            .record_x_batch(&AnalysisBatch::new("x.com", vec![item()]))
            .expect("content should store");

        let command = x_debug_stats_command(&store).expect("debug stats should build");
        let stats = command
            .debug_stats
            .expect("debug stats payload should be present");
        let storage = stats
            .sections
            .iter()
            .find(|section| section.title == "Storage")
            .expect("storage section should exist");

        assert!(matches!(command.action, DomCommandAction::ShowDebugStats));
        assert_eq!(stats.site, "x.com");
        assert!(storage
            .metrics
            .iter()
            .any(|metric| metric.label == "Post encounters" && metric.value == "1"));

        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn x_debug_stats_only_targets_x_hosts() {
        assert!(is_x_com_url("https://x.com/home"));
        assert!(is_x_com_url("https://mobile.twitter.com/home"));
        assert!(!is_x_com_url("https://example.com/home"));
    }
}
