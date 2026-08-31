use std::collections::HashSet;

use codexmanager_core::rpc::types::ModelInfo;
use codexmanager_core::rpc::types::ModelsResponse;
use serde_json::{Map, Value};

fn default_input_modalities() -> Vec<String> {
    vec!["text".to_string(), "image".to_string()]
}

/// 函数 `parse_models_response`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - super: 参数 super
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn parse_models_response(body: &[u8]) -> ModelsResponse {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return ModelsResponse::default();
    };

    if let Ok(response) = serde_json::from_value::<ModelsResponse>(value.clone()) {
        if !response.models.is_empty() {
            return normalize_models_response(response);
        }
    }

    let mut models: Vec<ModelInfo> = Vec::new();
    let mut seen = HashSet::new();
    parse_models_array(
        value.get("models").and_then(|v| v.as_array()),
        &mut seen,
        &mut models,
    );
    parse_data_array(
        value.get("items").and_then(|v| v.as_array()),
        &mut seen,
        &mut models,
    );
    parse_data_array(
        value.get("data").and_then(|v| v.as_array()),
        &mut seen,
        &mut models,
    );
    parse_data_array(value.as_array(), &mut seen, &mut models);
    ModelsResponse {
        models,
        extra: Default::default(),
    }
}

/// 函数 `parse_models_array`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - models: 参数 models
/// - seen: 参数 seen
/// - items: 参数 items
///
/// # 返回
/// 无
fn parse_models_array(
    models: Option<&Vec<Value>>,
    seen: &mut HashSet<String>,
    items: &mut Vec<ModelInfo>,
) {
    let Some(models) = models else {
        return;
    };
    for item in models {
        let Ok(model) = serde_json::from_value::<ModelInfo>(item.clone()) else {
            continue;
        };
        push_model(model, seen, items);
    }
}

fn normalize_models_response(response: ModelsResponse) -> ModelsResponse {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for model in response.models {
        push_model(model, &mut seen, &mut items);
    }
    ModelsResponse {
        models: items,
        extra: response.extra,
    }
}

fn push_model(model: ModelInfo, seen: &mut HashSet<String>, items: &mut Vec<ModelInfo>) {
    let slug = model.slug.trim().to_string();
    if slug.is_empty() || !seen.insert(slug.clone()) {
        return;
    }

    let mut normalized = model;
    normalized.slug = slug;
    if normalized.display_name.trim().is_empty() {
        normalized.display_name = normalized.slug.clone();
    }
    items.push(normalized);
}

fn model_info_from_legacy_item(item: &Map<String, Value>) -> Option<ModelInfo> {
    let slug = item
        .get("id")
        .or_else(|| item.get("slug"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())?;
    let display_name = item
        .get("display_name")
        .or_else(|| item.get("displayName"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(slug);
    let description = item
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let mut extra = item.clone();
    extra.remove("id");
    extra.remove("slug");
    extra.remove("display_name");
    extra.remove("displayName");
    extra.remove("description");

    Some(ModelInfo {
        slug: slug.to_string(),
        display_name: display_name.to_string(),
        description,
        visibility: Some("list".to_string()),
        supported_in_api: true,
        input_modalities: default_input_modalities(),
        extra: extra.into_iter().collect(),
        ..Default::default()
    })
}

/// 函数 `parse_data_array`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - data: 参数 data
/// - seen: 参数 seen
/// - items: 参数 items
///
/// # 返回
/// 无
fn parse_data_array(
    data: Option<&Vec<Value>>,
    seen: &mut HashSet<String>,
    items: &mut Vec<ModelInfo>,
) {
    let Some(data) = data else {
        return;
    };
    for item in data {
        let Some(obj) = item.as_object() else {
            continue;
        };
        if let Some(model) = model_info_from_legacy_item(obj) {
            push_model(model, seen, items);
        }
    }
}

#[cfg(test)]
mod tests {
    use codexmanager_core::rpc::types::ModelsResponse;
    use serde_json::json;

    use super::parse_models_response;

    /// 函数 `parse_models_response_preserves_official_models_payload`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn parse_models_response_preserves_official_models_payload() {
        let body = json!({
            "models": [
                {
                    "slug": "gpt-5.4-mini",
                    "display_name": "GPT-5.4 Mini",
                    "supported_in_api": true,
                    "visibility": "list"
                }
            ]
        })
        .to_string();

        let response = parse_models_response(body.as_bytes());
        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].slug, "gpt-5.4-mini");
        assert_eq!(response.models[0].display_name, "GPT-5.4 Mini");
        assert!(response.models[0].supported_in_api);
        assert_eq!(response.models[0].visibility.as_deref(), Some("list"));
    }

    #[test]
    fn parse_models_response_falls_back_to_legacy_data_array() {
        let body = json!({
            "data": [
                { "id": "gpt-5.4-mini", "display_name": "GPT-5.4 Mini" },
                { "slug": "o3", "title": "OpenAI o3" }
            ]
        })
        .to_string();

        let response: ModelsResponse = parse_models_response(body.as_bytes());
        assert_eq!(response.models.len(), 2);
        assert_eq!(response.models[0].slug, "gpt-5.4-mini");
        assert_eq!(response.models[0].display_name, "GPT-5.4 Mini");
        assert!(response.models[0].supported_in_api);
        assert_eq!(
            response.models[0].input_modalities,
            vec!["text".to_string(), "image".to_string()]
        );
        assert_eq!(response.models[1].slug, "o3");
        assert_eq!(response.models[1].display_name, "o3");
    }

    #[test]
    fn parse_models_response_falls_back_to_items_array() {
        let body = json!({
            "items": [
                { "slug": "gpt-5.4", "display_name": "GPT-5.4" }
            ]
        })
        .to_string();

        let response: ModelsResponse = parse_models_response(body.as_bytes());
        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].slug, "gpt-5.4");
        assert_eq!(response.models[0].display_name, "GPT-5.4");
        assert!(response.models[0].supported_in_api);
    }
}
