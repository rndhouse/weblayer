use super::extract::ExtractedItem;
use crate::core::{ContentDecision, DecisionAction, DomCommand, FeedbackContext};
use crate::storage::ContentStore;
use std::collections::{HashMap, HashSet};
use tracing::warn;

pub(super) fn commands_from_decisions(
    extracted_items: Vec<ExtractedItem>,
    decisions: Vec<ContentDecision>,
    feedback_context: &FeedbackContext,
    content_store: &ContentStore,
) -> Vec<DomCommand> {
    let decisions = expand_hide_decisions(&extracted_items, decisions);
    let mut decisions_by_id: HashMap<_, _> = decisions
        .into_iter()
        .map(|decision| (decision.client_id.clone(), decision))
        .collect();

    extracted_items
        .into_iter()
        .flat_map(|extracted| {
            let decision = decisions_by_id.remove(&extracted.item.client_id);
            let item_feedback_context = decision
                .as_ref()
                .map(|decision| feedback_context.with_decision(decision))
                .unwrap_or_else(|| feedback_context.clone());
            let mut commands = feedback_control_command(
                content_store,
                extracted.target.clone(),
                &item_feedback_context,
            )
            .into_iter()
            .collect::<Vec<_>>();

            if let Some(decision) = decision {
                commands.push(DomCommand::from_decision(decision, extracted.target));
            }

            commands
        })
        .collect()
}

pub(super) fn expand_hide_decisions(
    extracted_items: &[ExtractedItem],
    decisions: Vec<ContentDecision>,
) -> Vec<ContentDecision> {
    let descendants_by_post_id = descendants_by_post_id(extracted_items);
    let item_by_post_id: HashMap<_, _> = extracted_items
        .iter()
        .map(|extracted| (extracted.relationship.post_id.as_str(), extracted))
        .collect();
    let mut decisions_by_client_id: HashMap<_, _> = decisions
        .into_iter()
        .map(|decision| (decision.client_id.clone(), decision))
        .collect();
    let hidden_post_ids: Vec<_> = extracted_items
        .iter()
        .filter_map(|extracted| {
            let decision = decisions_by_client_id.get(&extracted.item.client_id)?;
            matches!(decision.action, DecisionAction::Hide)
                .then(|| extracted.relationship.post_id.clone())
        })
        .collect();
    let mut hidden_descendant_ids = HashSet::new();

    for post_id in hidden_post_ids {
        collect_descendant_post_ids(
            post_id.as_str(),
            &descendants_by_post_id,
            &mut hidden_descendant_ids,
        );
    }

    for post_id in hidden_descendant_ids {
        let Some(extracted) = item_by_post_id.get(post_id.as_str()) else {
            continue;
        };
        let client_id = extracted.item.client_id.clone();
        let should_insert = decisions_by_client_id
            .get(client_id.as_str())
            .is_none_or(|decision| !matches!(decision.action, DecisionAction::Hide));

        if should_insert {
            decisions_by_client_id.insert(
                client_id.clone(),
                ContentDecision::hide(client_id, "WebLayer: hidden", "Reply to hidden post", 1.0),
            );
        }
    }

    decisions_by_client_id.into_values().collect()
}

fn descendants_by_post_id(extracted_items: &[ExtractedItem]) -> HashMap<String, Vec<String>> {
    let ordered_items = ordered_items(extracted_items);
    let mut known_post_ids = HashSet::<String>::new();
    let mut author_post_ids = HashMap::<String, Vec<String>>::new();
    let mut descendants = HashMap::<String, Vec<String>>::new();

    for extracted in ordered_items {
        let post_id = extracted.relationship.post_id.clone();
        let parent_post_id = inferred_parent_post_id(extracted, &known_post_ids, &author_post_ids);

        if let Some(parent_post_id) = parent_post_id {
            descendants
                .entry(parent_post_id)
                .or_default()
                .push(post_id.clone());
        }

        known_post_ids.insert(post_id.clone());
        if let Some(author_handle) =
            normalized_handle(extracted.relationship.author_handle.as_ref())
        {
            author_post_ids
                .entry(author_handle)
                .or_default()
                .push(post_id);
        }
    }

    descendants
}

fn ordered_items(extracted_items: &[ExtractedItem]) -> Vec<&ExtractedItem> {
    let mut indexed_items: Vec<_> = extracted_items
        .iter()
        .enumerate()
        .map(|(index, extracted)| {
            (
                extracted.relationship.visible_index.unwrap_or(index as i64),
                index,
                extracted,
            )
        })
        .collect();
    indexed_items.sort_by_key(|(visible_index, index, _)| (*visible_index, *index));
    indexed_items
        .into_iter()
        .map(|(_, _, extracted)| extracted)
        .collect()
}

fn inferred_parent_post_id(
    extracted: &ExtractedItem,
    known_post_ids: &HashSet<String>,
    author_post_ids: &HashMap<String, Vec<String>>,
) -> Option<String> {
    if let Some(parent_post_id) = extracted.relationship.parent_post_id.as_ref() {
        if known_post_ids.contains(parent_post_id) {
            return Some(parent_post_id.clone());
        }
    }

    for ancestor_post_id in extracted.relationship.reply_ancestor_post_ids.iter().rev() {
        if known_post_ids.contains(ancestor_post_id) {
            return Some(ancestor_post_id.clone());
        }
    }

    extracted
        .relationship
        .replying_to_handles
        .iter()
        .filter_map(|handle| normalized_handle(Some(handle)))
        .filter_map(|handle| {
            author_post_ids
                .get(handle.as_str())
                .and_then(|ids| ids.last())
        })
        .last()
        .cloned()
}

fn collect_descendant_post_ids(
    post_id: &str,
    descendants_by_post_id: &HashMap<String, Vec<String>>,
    output: &mut HashSet<String>,
) {
    let Some(children) = descendants_by_post_id.get(post_id) else {
        return;
    };

    for child_id in children {
        if output.insert(child_id.clone()) {
            collect_descendant_post_ids(child_id, descendants_by_post_id, output);
        }
    }
}

fn normalized_handle(value: Option<&String>) -> Option<String> {
    let value = value?.trim().trim_start_matches('@');
    (!value.is_empty()).then(|| format!("@{}", value.to_ascii_lowercase()))
}

fn feedback_control_command(
    content_store: &ContentStore,
    target: crate::core::DomCommandTarget,
    feedback_context: &FeedbackContext,
) -> Option<DomCommand> {
    match content_store.store_x_feedback_context(feedback_context) {
        Ok(context_id) => Some(DomCommand::feedback_control_with_context_id(
            target, context_id,
        )),
        Err(error) => {
            warn!(%error, "failed to store feedback context for X command");
            None
        }
    }
}
