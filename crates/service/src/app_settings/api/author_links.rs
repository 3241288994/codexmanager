use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuthorLinkItem {
    pub key: String,
    pub name: String,
    pub description: String,
    pub href: String,
    pub action_label: String,
    pub image_src: Option<String>,
    pub image_alt: Option<String>,
}

fn trim_text(value: &str) -> String {
    value.trim().to_string()
}

fn trim_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub(super) fn default_author_sponsors() -> Vec<AuthorLinkItem> {
    Vec::new()
}

pub(super) fn default_author_server_recommendations() -> Vec<AuthorLinkItem> {
    Vec::new()
}

pub(super) fn normalize_author_link_items(items: Vec<AuthorLinkItem>) -> Vec<AuthorLinkItem> {
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| AuthorLinkItem {
            key: {
                let normalized = trim_text(&item.key);
                if normalized.is_empty() {
                    format!("item-{}", index + 1)
                } else {
                    normalized
                }
            },
            name: trim_text(&item.name),
            description: trim_text(&item.description),
            href: trim_text(&item.href),
            action_label: trim_text(&item.action_label),
            image_src: trim_optional_text(item.image_src),
            image_alt: trim_optional_text(item.image_alt),
        })
        .collect()
}

pub(super) fn load_author_link_items(
    settings: &HashMap<String, String>,
    key: &str,
    defaults: &[AuthorLinkItem],
) -> Vec<AuthorLinkItem> {
    settings
        .get(key)
        .and_then(|raw| serde_json::from_str::<Vec<AuthorLinkItem>>(raw).ok())
        .map(normalize_author_link_items)
        .unwrap_or_else(|| defaults.to_vec())
}

pub(super) fn serialize_author_link_items(items: &[AuthorLinkItem]) -> Result<String, String> {
    serde_json::to_string(items).map_err(|err| format!("serialize author links failed: {err}"))
}
