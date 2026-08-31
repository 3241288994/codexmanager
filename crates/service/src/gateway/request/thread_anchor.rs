use codexmanager_core::storage::ConversationBinding;

use super::incoming_headers::IncomingHeaderSnapshot;

fn normalize_anchor(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn has_native_thread_anchor(headers: &IncomingHeaderSnapshot) -> bool {
    normalize_anchor(headers.conversation_id()).is_some()
        || normalize_anchor(headers.turn_state()).is_some()
}

pub(crate) fn resolve_local_conversation_id_with_sticky_fallback(
    headers: &IncomingHeaderSnapshot,
    allow_sticky_fallback: bool,
) -> Option<String> {
    normalize_anchor(headers.conversation_id()).or_else(|| {
        if !allow_sticky_fallback || normalize_anchor(headers.turn_state()).is_some() {
            return None;
        }
        super::upstream::header_profile::derive_sticky_conversation_id_from_headers(headers)
    })
}

pub(crate) fn resolve_fallback_thread_anchor(
    headers: &IncomingHeaderSnapshot,
    local_conversation_id: Option<&str>,
    binding: Option<&ConversationBinding>,
) -> Option<String> {
    if has_native_thread_anchor(headers) {
        return None;
    }
    super::conversation_binding::effective_thread_anchor(local_conversation_id, binding)
}

#[cfg(test)]
mod tests {
    use super::{
        has_native_thread_anchor, resolve_fallback_thread_anchor,
        resolve_local_conversation_id_with_sticky_fallback,
    };
    use axum::http::{HeaderMap, HeaderValue};
    use codexmanager_core::storage::ConversationBinding;

    fn sample_headers(
        conversation_id: Option<&str>,
        turn_state: Option<&str>,
        x_api_key: Option<&str>,
    ) -> crate::gateway::IncomingHeaderSnapshot {
        let mut headers = HeaderMap::new();
        if let Some(conversation_id) = conversation_id {
            headers.insert(
                "conversation_id",
                HeaderValue::from_str(conversation_id).expect("conversation header"),
            );
        }
        if let Some(turn_state) = turn_state {
            headers.insert(
                "x-codex-turn-state",
                HeaderValue::from_str(turn_state).expect("turn-state header"),
            );
        }
        if let Some(x_api_key) = x_api_key {
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(x_api_key).expect("api key header"),
            );
        }
        crate::gateway::IncomingHeaderSnapshot::from_http_headers(&headers)
    }

    fn sample_binding() -> ConversationBinding {
        ConversationBinding {
            platform_key_hash: "hash".to_string(),
            conversation_id: "sticky-conversation".to_string(),
            account_id: "acc_1".to_string(),
            thread_epoch: 2,
            thread_anchor: "thread-anchor-2".to_string(),
            status: "active".to_string(),
            last_model: None,
            last_switch_reason: None,
            created_at: 1,
            updated_at: 1,
            last_used_at: 1,
        }
    }

    #[test]
    fn native_thread_anchor_detects_turn_state_without_conversation_id() {
        let headers = sample_headers(None, Some("turn-state-1"), Some("pk_test"));

        assert!(has_native_thread_anchor(&headers));
    }

    #[test]
    fn sticky_fallback_is_disabled_when_turn_state_exists() {
        let headers = sample_headers(None, Some("turn-state-1"), Some("pk_test"));

        let actual = resolve_local_conversation_id_with_sticky_fallback(&headers, true);

        assert_eq!(actual, None);
    }

    #[test]
    fn fallback_thread_anchor_is_suppressed_when_native_anchor_exists() {
        let headers = sample_headers(Some("conversation-1"), None, Some("pk_test"));

        let actual = resolve_fallback_thread_anchor(
            &headers,
            Some("conversation-1"),
            Some(&sample_binding()),
        );

        assert_eq!(actual, None);
    }
}
