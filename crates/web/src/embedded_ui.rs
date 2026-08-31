#[cfg(feature = "embedded-ui")]
use include_dir::{include_dir, Dir};

#[cfg(feature = "embedded-ui")]
static DIST_DIR: Dir<'static> = include_dir!("$OUT_DIR/codexmanager-web-dist");
#[cfg(feature = "embedded-ui")]
const _DIST_FINGERPRINT: &str = env!("CODEXMANAGER_WEB_DIST_FINGERPRINT");

/// 函数 `has_embedded_ui`
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
#[cfg(feature = "embedded-ui")]
pub fn has_embedded_ui() -> bool {
    // apps/out 至少应包含 index.html
    DIST_DIR.get_file("index.html").is_some()
}

/// 函数 `read_asset_bytes`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - path: 参数 path
///
/// # 返回
/// 返回函数执行结果
#[cfg(feature = "embedded-ui")]
pub fn read_asset_bytes(path: &str) -> Option<&'static [u8]> {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let file = DIST_DIR.get_file(path)?;
    Some(file.contents())
}

/// 函数 `guess_mime`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - path: 参数 path
///
/// # 返回
/// 返回函数执行结果
#[cfg(feature = "embedded-ui")]
pub fn guess_mime(path: &str) -> String {
    let path = path.trim_start_matches('/');
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
}

/// 函数 `has_embedded_ui`
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
#[cfg(not(feature = "embedded-ui"))]
pub fn has_embedded_ui() -> bool {
    false
}

/// 函数 `read_asset_bytes`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - _path: 参数 _path
///
/// # 返回
/// 返回函数执行结果
#[cfg(not(feature = "embedded-ui"))]
pub fn read_asset_bytes(_path: &str) -> Option<&'static [u8]> {
    None
}

/// 函数 `guess_mime`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - _path: 参数 _path
///
/// # 返回
/// 返回函数执行结果
#[cfg(not(feature = "embedded-ui"))]
pub fn guess_mime(_path: &str) -> String {
    "application/octet-stream".to_string()
}
