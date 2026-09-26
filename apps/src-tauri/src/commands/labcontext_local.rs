use reqwest::blocking::Client;
use reqwest::Url;
use rfd::FileDialog;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const DEFAULT_LOCAL_ADMIN_URL: &str = "http://127.0.0.1:1455/admin";
static SELECTED_ROOT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn selected_root() -> &'static Mutex<Option<PathBuf>> {
    SELECTED_ROOT.get_or_init(|| Mutex::new(None))
}

fn validate_local_admin_base_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    let parsed = Url::parse(value)
        .map_err(|err| format!("invalid CODEXMANAGER_LOCAL_LABCONTEXT_ADMIN_URL: {err}"))?;
    let loopback = parsed
        .host_str()
        .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"));
    if parsed.scheme() != "http" || !loopback {
        return Err("本地 LabContext 管理地址必须使用 HTTP 回环地址".to_string());
    }
    Ok(value.trim_end_matches('/').to_string())
}

fn local_admin_base_url() -> Result<String, String> {
    let value = std::env::var("CODEXMANAGER_LOCAL_LABCONTEXT_ADMIN_URL")
        .unwrap_or_else(|_| DEFAULT_LOCAL_ADMIN_URL.to_string());
    validate_local_admin_base_url(&value)
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "无法确定本机用户目录".to_string())
}

fn local_token_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CODEXMANAGER_LOCAL_LABCONTEXT_ADMIN_TOKEN_FILE") {
        return Ok(PathBuf::from(path));
    }
    Ok(home_dir()?.join(".local/state/labcontext/admin.token"))
}

fn local_token() -> Result<String, String> {
    let path = local_token_path()?;
    let value = fs::read_to_string(&path).map_err(|err| {
        format!(
            "读取本机 LabContext 管理 token 失败（{}）：{err}",
            path.display()
        )
    })?;
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err("本机 LabContext 管理 token 为空".to_string());
    }
    Ok(value)
}

fn local_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .no_proxy()
        .build()
        .map_err(|err| format!("创建本机 LabContext 连接失败：{err}"))
}

fn snake_to_camel(key: &str) -> String {
    let mut output = String::with_capacity(key.len());
    let mut uppercase = false;
    for ch in key.chars() {
        if ch == '_' {
            uppercase = true;
        } else if uppercase {
            output.extend(ch.to_uppercase());
            uppercase = false;
        } else {
            output.push(ch);
        }
    }
    output
}

fn camel_to_snake(key: &str) -> String {
    let mut output = String::with_capacity(key.len() + 4);
    for ch in key.chars() {
        if ch.is_ascii_uppercase() {
            output.push('_');
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

fn map_keys(value: Value, mapper: fn(&str) -> String) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (mapper(&key), map_keys(value, mapper)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| map_keys(value, mapper))
                .collect(),
        ),
        other => other,
    }
}

fn decode(response: reqwest::blocking::Response) -> Result<Value, String> {
    let status = response.status();
    let value: Value = response
        .json()
        .map_err(|err| format!("解析本机 LabContext 响应失败：{err}"))?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("本机 LabContext 请求失败");
        return Err(format!("{message}（HTTP {}）", status.as_u16()));
    }
    Ok(map_keys(value, snake_to_camel))
}

fn local_get(path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    let mut url = Url::parse(&format!(
        "{}/{}",
        local_admin_base_url()?,
        path.trim_start_matches('/')
    ))
    .map_err(|err| format!("创建本机 LabContext URL 失败：{err}"))?;
    url.query_pairs_mut().extend_pairs(query.iter().copied());
    let response = local_client()?
        .get(url)
        .bearer_auth(local_token()?)
        .send()
        .map_err(|err| format!("连接本机 LabContext 失败：{err}"))?;
    decode(response)
}

fn local_post(path: &str, payload: Value) -> Result<Value, String> {
    let url = format!(
        "{}/{}",
        local_admin_base_url()?,
        path.trim_start_matches('/')
    );
    let response = local_client()?
        .post(url)
        .bearer_auth(local_token()?)
        .json(&map_keys(payload, camel_to_snake))
        .send()
        .map_err(|err| format!("连接本机 LabContext 失败：{err}"))?;
    decode(response)
}

fn canonical_workspace_root(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("无法读取所选工作区目录（{}）：{err}", path.display()))?;
    if !canonical.is_dir() {
        return Err("所选工作区不是目录".to_string());
    }
    if canonical.parent().is_none() {
        return Err("不能把文件系统根目录授权为工作区".to_string());
    }
    if let Ok(home) = home_dir()
        .and_then(|path| fs::canonicalize(path).map_err(|err| format!("解析用户目录失败：{err}")))
    {
        if canonical == home {
            return Err("不能把整个用户目录授权为工作区，请选择具体项目目录".to_string());
        }
    }
    Ok(canonical)
}

fn remember_selected_root(path: &Path) -> Result<PathBuf, String> {
    let canonical = canonical_workspace_root(path)?;
    let mut selected = selected_root()
        .lock()
        .map_err(|_| "本地目录授权状态不可用".to_string())?;
    *selected = Some(canonical.clone());
    Ok(canonical)
}

fn ensure_selected_root(path: &Path) -> Result<PathBuf, String> {
    let canonical = canonical_workspace_root(path)?;
    let selected = selected_root()
        .lock()
        .map_err(|_| "本地目录授权状态不可用".to_string())?;
    if selected.as_ref() != Some(&canonical) {
        return Err("请先使用系统文件夹选择器授权本地项目目录".to_string());
    }
    Ok(canonical)
}

fn workspace_id(params: &Value) -> Result<&str, String> {
    params
        .get("workspaceId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "缺少 workspaceId".to_string())
}

fn call_local_operation(operation: &str, params: Value) -> Result<Value, String> {
    match operation {
        "overview" => local_get("overview", &[]),
        "setDefaultWorkspace" => local_post("default-workspace", params),
        "refreshWorkspace" => local_post("refresh-workspace", params),
        "testTool" => local_post("test-tool", params),
        "setToolPolicy" => local_post("tool-policy", params),
        "deleteWorkspace" => local_post("delete-workspace", params),
        "setWorkspaceOverview" => local_post("workspace-overview", params),
        "generateWorkspaceOverview" => local_post("generate-workspace-overview", params),
        "setWorkerConfig" => local_post("worker-config", params),
        "getResearchMap" => {
            let id = workspace_id(&params)?;
            local_get("research-map", &[("workspace_id", id)])
        }
        "initializeResearchMap" => local_post("research-map/initialize", params),
        "saveResearchMapLayout" => local_post("research-map/layout", params),
        "applyResearchMapPatch" => local_post("research-map/patch", params),
        "reviewResearchMap" => local_post("research-map/review", params),
        "researchMapProposalAction" => local_post("research-map/proposal", params),
        _ => Err("不支持的本机 LabContext 操作".to_string()),
    }
}

fn result_envelope(value: Value) -> Value {
    serde_json::json!({ "result": value })
}

#[tauri::command]
pub async fn app_labcontext_pick_local_workspace_directory() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(|| {
        *selected_root()
            .lock()
            .map_err(|_| "本地目录授权状态不可用".to_string())? = None;
        let Some(path) = FileDialog::new()
            .set_title("选择本地项目目录")
            .pick_folder()
        else {
            return Ok(result_envelope(serde_json::json!({
                "canceled": true,
                "path": null,
            })));
        };
        let canonical = remember_selected_root(&path)?;
        Ok(result_envelope(serde_json::json!({
            "canceled": false,
            "path": canonical.to_string_lossy(),
        })))
    })
    .await
    .map_err(|err| format!("选择本地工作区目录失败：{err}"))?
}

#[tauri::command]
pub async fn app_labcontext_local_call(
    operation: String,
    params: Option<Value>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        call_local_operation(&operation, params.unwrap_or_else(|| serde_json::json!({})))
            .map(result_envelope)
    })
    .await
    .map_err(|err| format!("本机 LabContext 操作失败：{err}"))?
}

#[tauri::command]
pub async fn app_labcontext_local_upsert_workspace(
    name: String,
    root: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let name = name.trim();
        if name.is_empty() {
            return Err("工作区名称不能为空".to_string());
        }
        let canonical = ensure_selected_root(Path::new(&root))?;
        let value = local_post(
            "workspaces",
            serde_json::json!({
                "name": name,
                "root": canonical.to_string_lossy(),
            }),
        )?;
        if let Ok(mut selected) = selected_root().lock() {
            *selected = None;
        }
        Ok(result_envelope(value))
    })
    .await
    .map_err(|err| format!("添加本地工作区失败：{err}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn local_admin_url_is_strictly_loopback() {
        assert!(validate_local_admin_base_url("http://127.0.0.1:1455/admin").is_ok());
        assert!(validate_local_admin_base_url("http://localhost:1455/admin/").is_ok());
        assert!(validate_local_admin_base_url("https://127.0.0.1/admin").is_err());
        assert!(validate_local_admin_base_url("http://host.docker.internal:1455/admin").is_err());
        assert!(validate_local_admin_base_url("http://example.com/admin").is_err());
    }

    #[test]
    fn local_operation_allowlist_excludes_workspace_registration() {
        assert!(call_local_operation("upsertWorkspace", serde_json::json!({})).is_err());
        assert!(call_local_operation("arbitraryRequest", serde_json::json!({})).is_err());
    }

    #[test]
    fn maps_labcontext_keys_without_touching_values() {
        let value = serde_json::json!({"workspace_id":"a_b", "nested_value":[{"job_id":"j"}]});
        let camel = map_keys(value, snake_to_camel);
        assert_eq!(camel["workspaceId"], "a_b");
        assert_eq!(camel["nestedValue"][0]["jobId"], "j");
        assert_eq!(map_keys(camel, camel_to_snake)["workspace_id"], "a_b");
    }

    #[test]
    fn registration_requires_a_picker_authorized_canonical_directory() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("codexmanager-local-workspace-{suffix}"));
        fs::create_dir_all(&root).unwrap();

        assert!(ensure_selected_root(&root).is_err());
        let selected = remember_selected_root(&root).unwrap();
        assert_eq!(ensure_selected_root(&root).unwrap(), selected);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filesystem_root_is_never_a_workspace() {
        let current = std::env::current_dir().unwrap();
        let root = current.ancestors().last().unwrap();
        assert!(canonical_workspace_root(root).is_err());
    }
}
