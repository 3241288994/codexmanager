use crate::app_settings::{
    get_persisted_app_setting, normalize_optional_text, save_persisted_app_setting,
    APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY,
};
use rand::RngCore;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;

const ENV_WEB_ACCESS_PASSWORD: &str = "CODEXMANAGER_WEB_ACCESS_PASSWORD";
const ENV_WEB_ACCESS_PASSWORD_FILE: &str = "CODEXMANAGER_WEB_ACCESS_PASSWORD_FILE";
const ENV_WEB_ACCESS_PASSWORD_RESET: &str = "CODEXMANAGER_WEB_ACCESS_PASSWORD_RESET";

/// 函数 `current_web_access_password_hash`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn current_web_access_password_hash() -> Option<String> {
    get_persisted_app_setting(APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY)
}

/// 函数 `web_access_password_configured`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn web_access_password_configured() -> bool {
    current_web_access_password_hash().is_some()
}

/// 函数 `set_web_access_password`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - password: 参数 password
///
/// # 返回
/// 返回函数执行结果
pub fn set_web_access_password(password: Option<&str>) -> Result<bool, String> {
    match normalize_optional_text(password) {
        Some(value) => {
            let hashed = hash_web_access_password(&value);
            save_persisted_app_setting(APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY, Some(&hashed))?;
            Ok(true)
        }
        None => {
            save_persisted_app_setting(APP_SETTING_WEB_ACCESS_PASSWORD_HASH_KEY, Some(""))?;
            Ok(false)
        }
    }
}

/// Initializes the Web access password from a private deployment secret.
///
/// A configured database password always wins unless the explicit reset flag
/// is set. This lets a Compose secret bootstrap a new volume without making
/// every container restart invalidate existing browser sessions.
pub fn bootstrap_web_access_password_from_env() -> Result<bool, String> {
    let reset_requested = env_flag(ENV_WEB_ACCESS_PASSWORD_RESET);
    if web_access_password_configured() && !reset_requested {
        return Ok(false);
    }

    let Some(password) = read_bootstrap_password()? else {
        if reset_requested {
            return Err(format!(
                "{ENV_WEB_ACCESS_PASSWORD_RESET} is set but no {ENV_WEB_ACCESS_PASSWORD_FILE} or {ENV_WEB_ACCESS_PASSWORD} value is available"
            ));
        }
        return Ok(false);
    };

    set_web_access_password(Some(&password))?;
    if crate::current_web_auth_mode() == "none" {
        let _ = crate::set_web_auth_mode("password")?;
    }
    log::info!("web access password initialized from private deployment configuration");
    Ok(true)
}

fn read_bootstrap_password() -> Result<Option<String>, String> {
    if let Ok(raw_path) = std::env::var(ENV_WEB_ACCESS_PASSWORD_FILE) {
        let path = raw_path.trim();
        if !path.is_empty() {
            let value = fs::read_to_string(path).map_err(|err| {
                format!("read {ENV_WEB_ACCESS_PASSWORD_FILE} failed for {path}: {err}")
            })?;
            let password = value.trim().to_string();
            if password.is_empty() {
                return Err(format!(
                    "{ENV_WEB_ACCESS_PASSWORD_FILE} points to an empty file"
                ));
            }
            return Ok(Some(password));
        }
    }

    Ok(std::env::var(ENV_WEB_ACCESS_PASSWORD)
        .ok()
        .and_then(|value| normalize_optional_text(Some(&value))))
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// 函数 `web_auth_status_value`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
pub fn web_access_auth_status_value() -> Result<Value, String> {
    Ok(serde_json::json!({
        "passwordConfigured": web_access_password_configured(),
    }))
}

/// 函数 `verify_web_access_password`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - password: 参数 password
///
/// # 返回
/// 返回函数执行结果
pub fn verify_web_access_password(password: &str) -> bool {
    let Some(stored_hash) = current_web_access_password_hash() else {
        return true;
    };
    verify_password_hash(password, &stored_hash)
}

/// 函数 `build_web_access_session_token`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - password_hash: 参数 password_hash
/// - rpc_token: 参数 rpc_token
///
/// # 返回
/// 返回函数执行结果
pub fn build_web_access_session_token(password_hash: &str, rpc_token: &str) -> String {
    hex_sha256(format!("codexmanager-web-auth-session:{password_hash}:{rpc_token}").as_bytes())
}

/// 函数 `hash_web_access_password`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - password: 参数 password
///
/// # 返回
/// 返回函数执行结果
fn hash_web_access_password(password: &str) -> String {
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let salt_hex = hex_encode(&salt);
    let digest = hex_sha256(format!("{salt_hex}:{password}").as_bytes());
    format!("sha256${salt_hex}${digest}")
}

/// 函数 `verify_password_hash`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - password: 参数 password
/// - stored_hash: 参数 stored_hash
///
/// # 返回
/// 返回函数执行结果
fn verify_password_hash(password: &str, stored_hash: &str) -> bool {
    let mut parts = stored_hash.split('$');
    let Some(kind) = parts.next() else {
        return false;
    };
    let Some(salt_hex) = parts.next() else {
        return false;
    };
    let Some(expected_hash) = parts.next() else {
        return false;
    };
    if kind != "sha256" || parts.next().is_some() {
        return false;
    }
    super::rpc::constant_time_eq(
        hex_sha256(format!("{salt_hex}:{password}").as_bytes()).as_bytes(),
        expected_hash.as_bytes(),
    )
}

/// 函数 `hex_sha256`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - bytes: 参数 bytes
///
/// # 返回
/// 返回函数执行结果
fn hex_sha256(bytes: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_ref());
    let digest = hasher.finalize();
    hex_encode(digest.as_slice())
}

/// 函数 `hex_encode`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - bytes: 参数 bytes
///
/// # 返回
/// 返回函数执行结果
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct EnvGuard {
        name: &'static str,
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            Self { name, previous }
        }

        fn clear(name: &'static str) -> Self {
            let previous = std::env::var_os(name);
            std::env::remove_var(name);
            Self { name, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = self.previous.take() {
                std::env::set_var(self.name, value);
            } else {
                std::env::remove_var(self.name);
            }
        }
    }

    fn unique_temp_path(prefix: &str, suffix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}{suffix}", std::process::id()))
    }

    #[test]
    fn bootstrap_password_file_initializes_password_mode_once() {
        let _guard = crate::test_env_guard();
        let db_path = unique_temp_path("codexmanager-web-access", ".db");
        let secret_path = unique_temp_path("codexmanager-web-access", ".secret");
        let _ = fs::remove_file(&db_path);
        fs::write(&secret_path, "first test-only password\n").expect("write secret file");

        let _db_guard = EnvGuard::set("CODEXMANAGER_DB_PATH", db_path.as_os_str());
        let _password_guard = EnvGuard::clear(ENV_WEB_ACCESS_PASSWORD);
        let _password_file_guard =
            EnvGuard::set(ENV_WEB_ACCESS_PASSWORD_FILE, secret_path.as_os_str());
        let _reset_guard = EnvGuard::clear(ENV_WEB_ACCESS_PASSWORD_RESET);

        assert!(bootstrap_web_access_password_from_env().expect("bootstrap password"));
        assert!(verify_web_access_password("first test-only password"));
        assert_eq!(crate::current_web_auth_mode(), "password");

        fs::write(&secret_path, "replacement test-only password\n").expect("rewrite secret file");
        assert!(
            !bootstrap_web_access_password_from_env().expect("do not overwrite existing password")
        );
        assert!(verify_web_access_password("first test-only password"));
        assert!(!verify_web_access_password(
            "replacement test-only password"
        ));

        let _ = fs::remove_file(secret_path);
        let _ = fs::remove_file(db_path);
    }
}
