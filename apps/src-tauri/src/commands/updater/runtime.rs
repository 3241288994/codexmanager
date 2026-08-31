use reqwest::blocking::Client;
use semver::Version;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) const PORTABLE_MARKER_FILE: &str = ".codexmanager-portable";
pub(super) const USER_AGENT: &str = "CodexManager-Updater";

#[cfg(target_os = "windows")]
pub(super) const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 函数 `now_unix_secs`
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
pub(super) fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_secs())
        .unwrap_or(0)
}

/// 函数 `resolve_update_repo`
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
pub(super) fn resolve_update_repo() -> Result<Option<String>, String> {
    let Some(value) = std::env::var("CODEXMANAGER_UPDATE_REPO").ok() else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    normalize_update_repo_value(&value).map(Some)
}

fn normalize_update_repo_value(value: &str) -> Result<String, String> {
    let mut parts = value.trim().split('/');
    let owner = parts.next().unwrap_or_default();
    let repository = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || owner.is_empty()
        || repository.is_empty()
        || !owner
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        || !repository
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(
            "CODEXMANAGER_UPDATE_REPO 必须是 GitHub owner/repository，且只能包含字母、数字、连字符、下划线或点"
                .to_string(),
        );
    }
    Ok(format!("{owner}/{repository}"))
}

fn require_update_repo_value(repo: Option<String>) -> Result<String, String> {
    repo.ok_or_else(|| {
        "自动更新未配置；发布自己的 GitHub Release 后设置 CODEXMANAGER_UPDATE_REPO=owner/repo"
            .to_string()
    })
}

pub(super) fn require_update_repo() -> Result<String, String> {
    require_update_repo_value(resolve_update_repo()?)
}

/// 函数 `normalize_version`
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
pub(super) fn normalize_version(input: &str) -> Result<Version, String> {
    let normalized = input.trim().trim_start_matches(['v', 'V']);
    Version::parse(normalized).map_err(|err| format!("版本号无效 '{input}'：{err}"))
}

/// 函数 `current_exe_path`
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
pub(super) fn current_exe_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|err| format!("解析当前可执行文件路径失败：{err}"))
}

/// 函数 `current_mode_and_marker`
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
pub(super) fn current_mode_and_marker() -> Result<(String, bool, PathBuf, PathBuf), String> {
    let exe = current_exe_path()?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| "解析可执行文件所在目录失败".to_string())?
        .to_path_buf();
    let marker = exe_dir.join(PORTABLE_MARKER_FILE);
    let by_marker = marker.is_file();
    let by_exe_name = exe
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase().contains("-portable"))
        .unwrap_or(false);
    let is_portable = by_marker || by_exe_name;
    let mode = if is_portable { "portable" } else { "installer" }.to_string();
    Ok((mode, is_portable, exe, marker))
}

/// 函数 `env_flag`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - name: 参数 name
///
/// # 返回
/// 返回函数执行结果
fn env_flag(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// 函数 `should_include_prerelease_updates_with_override`
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
pub(super) fn should_include_prerelease_updates_with_override(
    _current_version: &Version,
    override_value: Option<bool>,
) -> bool {
    override_value.unwrap_or(false)
}

/// 函数 `should_include_prerelease_updates`
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
pub(super) fn should_include_prerelease_updates(current_version: &Version) -> bool {
    should_include_prerelease_updates_with_override(
        current_version,
        env_flag("CODEXMANAGER_UPDATE_PRERELEASE"),
    )
}

/// 函数 `http_client`
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
pub(super) fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("创建 HTTP 客户端失败：{err}"))
}

/// 函数 `resolve_github_token`
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
pub(super) fn resolve_github_token() -> Option<String> {
    for key in ["CODEXMANAGER_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use semver::Version;

    use super::{
        normalize_update_repo_value, normalize_version, require_update_repo_value,
        should_include_prerelease_updates_with_override,
    };

    #[test]
    fn updater_is_disabled_without_an_explicit_repository() {
        let err =
            require_update_repo_value(None).expect_err("unconfigured updater should be disabled");

        assert!(err.contains("CODEXMANAGER_UPDATE_REPO"));
    }

    #[test]
    fn update_repository_requires_a_safe_owner_and_repository() {
        assert_eq!(
            normalize_update_repo_value(" owner-name/repository.name ")
                .expect("valid GitHub repository"),
            "owner-name/repository.name"
        );
        assert!(normalize_update_repo_value("owner").is_err());
        assert!(normalize_update_repo_value("owner/repo/extra").is_err());
        assert!(normalize_update_repo_value("owner/../repo").is_err());
    }

    /// 函数 `prerelease_channel_defaults_to_stable_latest`
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
    fn prerelease_channel_defaults_to_stable_latest() {
        let stable = Version::parse("0.1.8").expect("stable version");
        let beta = Version::parse("0.1.8-beta.1").expect("beta version");

        assert!(!should_include_prerelease_updates_with_override(
            &stable, None
        ));
        assert!(!should_include_prerelease_updates_with_override(
            &beta, None
        ));
        assert!(should_include_prerelease_updates_with_override(
            &stable,
            Some(true)
        ));
        assert!(!should_include_prerelease_updates_with_override(
            &beta,
            Some(false)
        ));
    }

    /// 函数 `normalize_version_accepts_v_prefix`
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
    fn normalize_version_accepts_v_prefix() {
        let version = normalize_version(" v0.1.8 ").expect("normalized version");
        assert_eq!(version, Version::parse("0.1.8").expect("expected version"));
    }
}
