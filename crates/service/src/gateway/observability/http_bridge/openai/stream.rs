use serde_json::Value;

use super::super::aggregate::collect_response_output_text;

#[derive(Debug, Clone, Default)]
pub(in super::super) struct OpenAIStreamMeta {
    pub(in super::super) response_id: Option<String>,
    pub(in super::super) model: Option<String>,
    pub(in super::super) created: Option<i64>,
}

pub(in super::super) fn apply_openai_stream_meta_defaults(
    mapped: &mut Value,
    meta: &OpenAIStreamMeta,
) {
    let Some(mapped_obj) = mapped.as_object_mut() else {
        return;
    };
    if let Some(id) = meta.response_id.as_deref() {
        let needs_id = mapped_obj
            .get("id")
            .and_then(Value::as_str)
            .is_none_or(|current| current.is_empty());
        if needs_id {
            mapped_obj.insert("id".to_string(), Value::String(id.to_string()));
        }
    }
    if let Some(model) = meta.model.as_deref() {
        let needs_model = mapped_obj
            .get("model")
            .and_then(Value::as_str)
            .is_none_or(|current| current.is_empty());
        if needs_model {
            mapped_obj.insert("model".to_string(), Value::String(model.to_string()));
        }
    }
    if let Some(created) = meta.created {
        let needs_created = mapped_obj
            .get("created")
            .and_then(Value::as_i64)
            .is_none_or(|current| current == 0);
        if needs_created {
            mapped_obj.insert("created".to_string(), Value::Number(created.into()));
        }
    }
}

pub(in super::super) fn extract_openai_completed_output_text(value: &Value) -> Option<String> {
    let response = value.get("response").unwrap_or(value);
    let mut output_text = String::new();
    collect_response_output_text(response, &mut output_text);
    let trimmed = output_text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
