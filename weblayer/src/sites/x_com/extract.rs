use crate::core::{ContentItem, DomAnalysisBatch, DomCommandTarget, DomElementSnapshot};
use serde_json::{json, Value};
use tracing::{trace, Level};

pub(super) fn extract_items(batch: &DomAnalysisBatch) -> Vec<ExtractedItem> {
    let mut page_status_href = status_href_from_page(&batch.page.url);
    let mut extracted_items = Vec::new();

    for element in &batch.elements {
        let Some(extracted) = extract_item(batch, element, page_status_href.as_deref()) else {
            continue;
        };

        page_status_href = None;
        extracted_items.push(extracted);
    }

    extracted_items
}

fn extract_item(
    batch: &DomAnalysisBatch,
    element: &DomElementSnapshot,
    page_status_href: Option<&str>,
) -> Option<ExtractedItem> {
    if !has_post_region_evidence(element) {
        return None;
    }

    let metadata = x_metadata(element);
    let metadata_post_id = metadata.and_then(|metadata| metadata_string(metadata, "postId"));
    let status_href = find_status_href(element).or_else(|| page_status_href.map(ToOwned::to_owned));
    let post_id = metadata_post_id.or_else(|| {
        status_href
            .as_deref()
            .and_then(x_status_id)
            .map(ToOwned::to_owned)
    });
    let post_id = post_id?;

    let author = metadata
        .and_then(|metadata| metadata_string(metadata, "authorHandle"))
        .or_else(|| status_href.as_deref().and_then(author_handle));
    let visible_index = metadata.and_then(|metadata| metadata_i64(metadata, "visibleIndex"));
    let parent_post_id = metadata.and_then(|metadata| metadata_string(metadata, "parentPostId"));
    let reply_ancestor_post_ids = metadata
        .map(|metadata| metadata_string_list(metadata, "replyAncestorPostIds"))
        .unwrap_or_default();
    let replying_to_handles = metadata
        .map(|metadata| metadata_string_list(metadata, "replyingToHandles"))
        .unwrap_or_default();
    let post_text = metadata.and_then(|metadata| metadata_string(metadata, "postText"));
    let relationship = XPostRelationship {
        post_id: post_id.clone(),
        author_handle: author.clone(),
        visible_index,
        parent_post_id,
        reply_ancestor_post_ids,
        replying_to_handles,
    };

    let item = ContentItem {
        client_id: element.client_id.clone(),
        content_id: Some(post_id.clone()),
        url: status_href,
        author,
        text: post_text.clone().unwrap_or_else(|| element.text.clone()),
        captured_at: element
            .captured_at
            .clone()
            .or_else(|| batch.page.captured_at.clone()),
        kind: Some("post".into()),
        metadata: json!({
            "pageUrl": batch.page.url,
            "pageTitle": batch.page.title,
            "selector": element.selector,
            "tagName": element.tag_name,
            "role": element.role,
            "snapshotHash": element.snapshot_hash,
            "xCom": {
                "postId": relationship.post_id.clone(),
                "authorHandle": relationship.author_handle.clone(),
                "visibleIndex": relationship.visible_index,
                "parentPostId": relationship.parent_post_id.clone(),
                "replyAncestorPostIds": relationship.reply_ancestor_post_ids.clone(),
                "replyingToHandles": relationship.replying_to_handles.clone(),
                "postText": post_text,
                "snapshotText": element.text,
            },
        }),
    };
    trace_identified_post(&item);

    let target = DomCommandTarget {
        client_id: element.client_id.clone(),
        selector: element.selector.clone(),
        must_match_snapshot_hash: element.snapshot_hash.clone(),
    };

    Some(ExtractedItem {
        item,
        target,
        relationship,
    })
}

fn x_metadata(element: &DomElementSnapshot) -> Option<&Value> {
    element
        .metadata
        .get("xCom")
        .or_else(|| element.metadata.get("x.com"))
}

fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    let value = metadata.get(key)?.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn metadata_i64(metadata: &Value, key: &str) -> Option<i64> {
    metadata.get(key)?.as_i64()
}

fn metadata_string_list(metadata: &Value, key: &str) -> Vec<String> {
    metadata
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn has_post_region_evidence(element: &DomElementSnapshot) -> bool {
    element
        .tag_name
        .as_deref()
        .is_some_and(|tag_name| tag_name.eq_ignore_ascii_case("article"))
        || element
            .role
            .as_deref()
            .is_some_and(|role| role.eq_ignore_ascii_case("article"))
        || has_root_attribute(element, "data-testid", "tweet")
}

fn has_root_attribute(element: &DomElementSnapshot, name: &str, value: &str) -> bool {
    element.attributes.iter().any(|attribute| {
        attribute.name.eq_ignore_ascii_case(name) && attribute.value.eq_ignore_ascii_case(value)
    })
}

fn trace_identified_post(item: &ContentItem) {
    if !tracing::enabled!(target: "weblayer_daemon::sites::x_com", Level::TRACE) {
        return;
    }

    if let Ok(post_json) = serde_json::to_string(item) {
        trace!(
            target: "weblayer_daemon::sites::x_com",
            client_id = item.client_id.as_str(),
            content_id = item.content_id.as_deref(),
            url = item.url.as_deref(),
            post = %post_json,
            "identified X post"
        );
    }
}

fn find_status_href(element: &DomElementSnapshot) -> Option<String> {
    element
        .links
        .iter()
        .find_map(|link| x_status_id(&link.href).map(|_| link.href.clone()))
}

fn status_href_from_page(url: &str) -> Option<String> {
    x_status_id(url).map(|_| url.to_string())
}

fn x_status_id(url: &str) -> Option<&str> {
    let marker = "/status/";
    let start = url.find(marker)? + marker.len();
    let rest = &url[start..];
    let end = rest
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(rest.len());

    (end > 0).then_some(&rest[..end])
}

fn author_handle(url: &str) -> Option<String> {
    let without_scheme = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let path = without_scheme.split_once('/')?.1;
    let handle = path.split_once("/status/")?.0.trim_matches('/');

    (!handle.is_empty()).then(|| format!("@{handle}"))
}

pub(super) struct ExtractedItem {
    pub(super) item: ContentItem,
    pub(super) target: DomCommandTarget,
    pub(super) relationship: XPostRelationship,
}

#[derive(Debug, Clone)]
pub(super) struct XPostRelationship {
    pub(super) post_id: String,
    pub(super) author_handle: Option<String>,
    pub(super) visible_index: Option<i64>,
    pub(super) parent_post_id: Option<String>,
    pub(super) reply_ancestor_post_ids: Vec<String>,
    pub(super) replying_to_handles: Vec<String>,
}
