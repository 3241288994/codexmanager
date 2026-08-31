use std::collections::HashSet;

use reqwest::blocking::Client;

use super::model::{GitHubAsset, GitHubRelease};
use super::runtime::{normalize_version, resolve_github_token, USER_AGENT};

/// 函数 `extract_tag_from_release_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - url: 参数 url
///
/// # 返回
/// 返回函数执行结果
fn extract_tag_from_release_url(url: &str) -> Option<String> {
    let marker = "/releases/tag/";
    let (_, tail) = url.split_once(marker)?;
    let tag = tail
        .split(['?', '#', '/'])
        .next()
        .map(|v| v.trim())
        .unwrap_or("");
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

/// 函数 `normalize_release_asset_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - raw: 参数 raw
/// - repo: 参数 repo
///
/// # 返回
/// 返回函数执行结果
fn normalize_release_asset_url(raw: &str, repo: &str) -> Option<String> {
    let href = raw.trim().replace("&amp;", "&");
    if href.is_empty() {
        return None;
    }

    let absolute = if href.starts_with("https://github.com/") {
        href
    } else if href.starts_with("http://github.com/") {
        href.replacen("http://", "https://", 1)
    } else if href.starts_with("//github.com/") {
        format!("https:{href}")
    } else if href.starts_with('/') {
        format!("https://github.com{href}")
    } else {
        return None;
    };

    let marker = format!("/{repo}/releases/download/");
    if absolute.contains(&marker) {
        Some(absolute)
    } else {
        None
    }
}

/// 函数 `asset_name_from_download_url`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - url: 参数 url
///
/// # 返回
/// 返回函数执行结果
fn asset_name_from_download_url(url: &str) -> Option<String> {
    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    let name = without_query.rsplit('/').next().unwrap_or("").trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// 函数 `parse_release_assets_from_html`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - html: 参数 html
/// - repo: 参数 repo
///
/// # 返回
/// 返回函数执行结果
fn parse_release_assets_from_html(html: &str, repo: &str) -> Vec<GitHubAsset> {
    let mut assets = Vec::new();
    let mut seen = HashSet::new();
    let mut cursor = html;
    loop {
        let Some(idx) = cursor.find("href=\"") else {
            break;
        };
        cursor = &cursor[idx + 6..];
        let Some(end_idx) = cursor.find('"') else {
            break;
        };

        let href = &cursor[..end_idx];
        if let Some(url) = normalize_release_asset_url(href, repo) {
            if let Some(name) = asset_name_from_download_url(&url) {
                let key = name.to_ascii_lowercase();
                if seen.insert(key) {
                    assets.push(GitHubAsset {
                        name,
                        browser_download_url: url,
                    });
                }
            }
        }
        cursor = &cursor[end_idx + 1..];
    }
    assets
}

/// 函数 `fetch_release_assets_from_expanded_fragment`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - client: 参数 client
/// - repo: 参数 repo
/// - tag: 参数 tag
///
/// # 返回
/// 返回函数执行结果
fn fetch_release_assets_from_expanded_fragment(
    client: &Client,
    repo: &str,
    tag: &str,
) -> Result<Vec<GitHubAsset>, String> {
    let url = format!("https://github.com/{repo}/releases/expanded_assets/{tag}");
    let html = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .map_err(|err| format!("请求扩展资产列表失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("扩展资产列表响应异常：{err}"))?
        .text()
        .map_err(|err| format!("读取扩展资产列表失败：{err}"))?;
    Ok(parse_release_assets_from_html(&html, repo))
}

/// 函数 `fetch_latest_release_via_html`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - client: 参数 client
/// - repo: 参数 repo
///
/// # 返回
/// 返回函数执行结果
fn fetch_latest_release_via_html(client: &Client, repo: &str) -> Result<GitHubRelease, String> {
    let url = format!("https://github.com/{repo}/releases/latest");
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .map_err(|err| format!("请求最新发布页跳转失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("最新发布页跳转响应异常：{err}"))?;

    let final_url = response.url().as_str().to_string();
    let tag = extract_tag_from_release_url(&final_url)
        .ok_or_else(|| format!("无法从 GitHub Releases 地址解析最新标签：{final_url}"))?;
    let html = response
        .text()
        .map_err(|err| format!("读取最新发布页失败：{err}"))?;
    let mut assets = parse_release_assets_from_html(&html, repo);
    if assets.is_empty() {
        if let Ok(expanded_assets) = fetch_release_assets_from_expanded_fragment(client, repo, &tag)
        {
            if !expanded_assets.is_empty() {
                assets = expanded_assets;
            }
        }
    }

    Ok(GitHubRelease {
        tag_name: tag,
        name: None,
        published_at: None,
        draft: false,
        prerelease: false,
        assets,
    })
}

/// 函数 `select_release_for_channel`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - releases: 参数 releases
/// - include_prerelease: 参数 include_prerelease
///
/// # 返回
/// 返回函数执行结果
fn select_release_for_channel(
    releases: Vec<GitHubRelease>,
    include_prerelease: bool,
) -> Result<GitHubRelease, String> {
    let mut selected = None;

    for release in releases {
        if release.draft {
            continue;
        }
        if !include_prerelease && release.prerelease {
            continue;
        }

        let version = match normalize_version(&release.tag_name) {
            Ok(value) => value,
            Err(_) => continue,
        };

        match &selected {
            Some((best_version, _)) if version <= *best_version => {}
            _ => selected = Some((version, release)),
        }
    }

    selected.map(|(_, release)| release).ok_or_else(|| {
        if include_prerelease {
            "未找到可用的稳定版或预发布版本".to_string()
        } else {
            "未找到可用的稳定版发布".to_string()
        }
    })
}

/// 函数 `fetch_latest_release`
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
pub(super) fn fetch_latest_release(
    client: &Client,
    repo: &str,
    include_prerelease: bool,
) -> Result<GitHubRelease, String> {
    if !repo.contains('/') {
        return Err(format!("更新仓库配置无效 '{repo}'，应为 owner/repo 格式"));
    }
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=20");
    let mut req = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json");
    if let Some(token) = resolve_github_token() {
        req = req.bearer_auth(token);
    }

    let release = match req.send() {
        Ok(resp) => match resp.error_for_status() {
            Ok(ok_resp) => {
                let releases = ok_resp
                    .json::<Vec<GitHubRelease>>()
                    .map_err(|err| format!("解析发布列表失败：{err}"))?;
                select_release_for_channel(releases, include_prerelease)?
            }
            Err(api_err) => {
                if include_prerelease {
                    return Err(format!(
                        "发布列表 API 请求失败（{api_err}）；预发布通道不支持 HTML 回退，请重试或配置 CODEXMANAGER_GITHUB_TOKEN"
                    ));
                }
                fetch_latest_release_via_html(client, repo).map_err(|fallback_err| {
                    format!(
                        "最新发布 API 请求失败（{api_err}）；回退解析发布页面也失败（{fallback_err}）"
                    )
                })?
            }
        },
        Err(api_transport_err) => {
            if include_prerelease {
                return Err(format!(
                    "发布列表请求失败（{api_transport_err}）；预发布通道不支持 HTML 回退，请重试或配置 CODEXMANAGER_GITHUB_TOKEN"
                ));
            }
            fetch_latest_release_via_html(client, repo).map_err(|fallback_err| {
                format!(
                    "最新发布请求失败（{api_transport_err}）；回退解析发布页面也失败（{fallback_err}）"
                )
            })?
        }
    };

    Ok(release)
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_release_asset_url, parse_release_assets_from_html, select_release_for_channel,
    };
    use crate::commands::updater::model::GitHubRelease;

    /// 函数 `release_selection_respects_channel`
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
    fn release_selection_respects_channel() {
        let releases = vec![
            GitHubRelease {
                tag_name: "v0.1.9-beta.1".to_string(),
                name: None,
                published_at: None,
                draft: false,
                prerelease: true,
                assets: vec![],
            },
            GitHubRelease {
                tag_name: "v0.1.8".to_string(),
                name: None,
                published_at: None,
                draft: false,
                prerelease: false,
                assets: vec![],
            },
        ];

        let stable = select_release_for_channel(releases.clone(), false).expect("stable release");
        let prerelease = select_release_for_channel(releases, true).expect("prerelease release");

        assert_eq!(stable.tag_name, "v0.1.8");
        assert_eq!(prerelease.tag_name, "v0.1.9-beta.1");
    }

    /// 函数 `parse_release_assets_filters_repo_and_deduplicates`
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
    fn parse_release_assets_filters_repo_and_deduplicates() {
        let html = r#"
<a href="/owner/project/releases/download/v0.1.8/CodexManager.exe">ok</a>
<a href="https://github.com/owner/project/releases/download/v0.1.8/CodexManager.exe?download=1">dup</a>
<a href="/someone/else/releases/download/v0.1.8/not-ours.zip">skip</a>
"#;
        let assets = parse_release_assets_from_html(html, "owner/project");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "CodexManager.exe");
    }

    /// 函数 `release_asset_url_requires_target_repo`
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
    fn release_asset_url_requires_target_repo() {
        assert!(normalize_release_asset_url(
            "/owner/project/releases/download/v0.1.8/file.zip",
            "owner/project"
        )
        .is_some());
        assert!(normalize_release_asset_url(
            "/other/repo/releases/download/v0.1.8/file.zip",
            "owner/project"
        )
        .is_none());
    }
}
