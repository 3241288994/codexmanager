use reqwest::blocking::Client;
use reqwest::Url;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_ADMIN_URL: &str = "http://127.0.0.1:1455/admin";

fn is_allowed_admin_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
        || host.eq_ignore_ascii_case("host.docker.internal")
}

fn validate_admin_base_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    let parsed = Url::parse(value).map_err(|err| format!("invalid LABCONTEXT_ADMIN_URL: {err}"))?;
    if parsed.scheme() != "http" || !parsed.host_str().is_some_and(is_allowed_admin_host) {
        return Err(
            "LabContext admin URL must use HTTP on loopback or host.docker.internal".to_string(),
        );
    }
    Ok(value.trim_end_matches('/').to_string())
}

fn admin_base_url() -> Result<String, String> {
    let value =
        std::env::var("LABCONTEXT_ADMIN_URL").unwrap_or_else(|_| DEFAULT_ADMIN_URL.to_string());
    validate_admin_base_url(&value)
}

fn token_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("LABCTX_ADMIN_TOKEN_FILE") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not available".to_string())?;
    Ok(PathBuf::from(home).join(".local/state/labcontext/admin.token"))
}

fn token() -> Result<String, String> {
    let path = token_path()?;
    let value = fs::read_to_string(&path).map_err(|err| {
        format!(
            "read LabContext admin token failed ({}): {err}",
            path.display()
        )
    })?;
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err("LabContext admin token is empty".to_string());
    }
    Ok(value)
}

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .no_proxy()
        .build()
        .map_err(|err| format!("build LabContext client failed: {err}"))
}

fn decode(response: reqwest::blocking::Response) -> Result<Value, String> {
    let status = response.status();
    let value: Value = response
        .json()
        .map_err(|err| format!("decode LabContext response failed: {err}"))?;
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("LabContext request failed");
        return Err(format!("{message} (HTTP {})", status.as_u16()));
    }
    Ok(map_keys(value, snake_to_camel))
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

pub(crate) fn get(path: &str) -> Result<Value, String> {
    let url = format!("{}/{}", admin_base_url()?, path.trim_start_matches('/'));
    let response = client()?
        .get(url)
        .bearer_auth(token()?)
        .send()
        .map_err(|err| format!("connect to LabContext failed: {err}"))?;
    decode(response)
}

pub(crate) fn get_with_query(path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    let mut url = Url::parse(&format!(
        "{}/{}",
        admin_base_url()?,
        path.trim_start_matches('/')
    ))
    .map_err(|err| format!("build LabContext URL failed: {err}"))?;
    url.query_pairs_mut().extend_pairs(query.iter().copied());
    let response = client()?
        .get(url)
        .bearer_auth(token()?)
        .send()
        .map_err(|err| format!("connect to LabContext failed: {err}"))?;
    decode(response)
}

pub(crate) fn post(path: &str, payload: Value) -> Result<Value, String> {
    let url = format!("{}/{}", admin_base_url()?, path.trim_start_matches('/'));
    let response = client()?
        .post(url)
        .bearer_auth(token()?)
        .json(&map_keys(payload, camel_to_snake))
        .send()
        .map_err(|err| format!("connect to LabContext failed: {err}"))?;
    decode(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_admin_url_is_loopback() {
        let parsed = Url::parse(DEFAULT_ADMIN_URL).unwrap();
        assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    }

    #[test]
    fn admin_url_accepts_only_local_transport_hosts() {
        assert_eq!(
            validate_admin_base_url("http://host.docker.internal:1455/admin").unwrap(),
            "http://host.docker.internal:1455/admin"
        );
        assert!(validate_admin_base_url("https://host.docker.internal/admin").is_err());
        assert!(validate_admin_base_url("http://labcontext.example/admin").is_err());
    }

    #[test]
    fn maps_json_keys_in_both_directions() {
        let value = serde_json::json!({"workspace_id": "hvs", "nested_value": [{"job_id": "j"}]});
        let camel = map_keys(value, snake_to_camel);
        assert_eq!(camel["workspaceId"], "hvs");
        assert_eq!(camel["nestedValue"][0]["jobId"], "j");
        assert_eq!(map_keys(camel, camel_to_snake)["workspace_id"], "hvs");
    }
}
