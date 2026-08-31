use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GitHubAsset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GitHubRelease {
    pub(crate) tag_name: String,
    pub(crate) name: Option<String>,
    pub(crate) published_at: Option<String>,
    pub(crate) draft: bool,
    pub(crate) prerelease: bool,
    pub(crate) assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResponse {
    pub repo: String,
    pub mode: String,
    pub is_portable: bool,
    pub has_update: bool,
    pub can_prepare: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_tag: String,
    pub release_name: Option<String>,
    pub published_at: Option<String>,
    pub reason: Option<String>,
    pub checked_at_unix_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrepareResponse {
    pub prepared: bool,
    pub mode: String,
    pub is_portable: bool,
    pub release_tag: String,
    pub latest_version: String,
    pub asset_name: String,
    pub asset_path: String,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActionResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PendingUpdate {
    pub(crate) mode: String,
    pub(crate) is_portable: bool,
    pub(crate) release_tag: String,
    pub(crate) latest_version: String,
    pub(crate) asset_name: String,
    pub(crate) asset_path: String,
    pub(crate) installer_path: Option<String>,
    pub(crate) staging_dir: Option<String>,
    pub(crate) prepared_at_unix_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusResponse {
    pub repo: Option<String>,
    pub mode: String,
    pub is_portable: bool,
    pub current_version: String,
    pub current_exe_path: String,
    pub portable_marker_path: String,
    pub pending: Option<PendingUpdate>,
    pub last_check: Option<UpdateCheckResponse>,
    pub last_error: Option<String>,
}
