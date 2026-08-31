use codexmanager_core::storage::{ApiKey, Storage};

use crate::storage_helpers::{hash_platform_key, open_storage, StorageHandle};

/// 函数 `open_storage_or_error`
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
pub(super) fn open_storage_or_error() -> Result<StorageHandle, super::LocalValidationError> {
    open_storage().ok_or_else(|| {
        super::LocalValidationError::new(
            500,
            crate::gateway::bilingual_error("存储不可用", "storage unavailable"),
        )
    })
}

/// 函数 `load_active_api_key`
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
pub(super) fn load_active_api_key(
    storage: &Storage,
    platform_key: &str,
    request_url: &str,
    debug: bool,
) -> Result<ApiKey, super::LocalValidationError> {
    let key_hash = hash_platform_key(platform_key);
    let api_key = storage.find_api_key_by_hash(&key_hash).map_err(|err| {
        super::LocalValidationError::new(
            500,
            crate::gateway::bilingual_error("读取存储失败", format!("storage read failed: {err}")),
        )
    })?;

    let Some(api_key) = api_key else {
        if debug {
            log::warn!(
                "event=gateway_auth_invalid path={} status=403 key_hash_prefix={}",
                request_url,
                &key_hash[..8]
            );
        }
        return Err(super::LocalValidationError::new(
            403,
            crate::gateway::MISSING_AUTH_JSON_OPENAI_API_KEY_ERROR,
        ));
    };

    if api_key.status != "active" {
        if debug {
            log::warn!(
                "event=gateway_auth_disabled path={} status=403 key_id={}",
                request_url,
                api_key.id
            );
        }
        return Err(super::LocalValidationError::new(
            403,
            crate::gateway::bilingual_error("API Key 已禁用", "api key disabled"),
        ));
    }

    if let Some(limit) = storage
        .find_api_key_quota_limit(&api_key.id)
        .map_err(|err| {
            super::LocalValidationError::new(
                500,
                crate::gateway::bilingual_error(
                    "读取存储失败",
                    format!("storage read failed: {err}"),
                ),
            )
        })?
    {
        let used = storage
            .api_key_total_token_usage(&api_key.id)
            .map_err(|err| {
                super::LocalValidationError::new(
                    500,
                    crate::gateway::bilingual_error(
                        "读取用量失败",
                        format!("read api key usage failed: {err}"),
                    ),
                )
            })?;
        if limit > 0 && used >= limit {
            if debug {
                log::warn!(
                    "event=gateway_auth_quota_exhausted path={} status=429 key_id={} used={} limit={}",
                    request_url,
                    api_key.id,
                    used,
                    limit
                );
            }
            return Err(super::LocalValidationError::new(
                429,
                crate::gateway::bilingual_error(
                    "API Key 额度已用尽",
                    format!("api key quota exhausted: used {used}, limit {limit}"),
                ),
            ));
        }
    }

    crate::wallet_precheck_for_api_key(storage, &api_key.id).map_err(|err| {
        if err.contains("余额不足") {
            // 中文注释：CLI 会对 429 做重试并最终显示 exceeded retry limit，
            // 本地分发额度不足需要保留可读错误文案，所以使用非重试状态码。
            return super::LocalValidationError::new(402, "额度不足，请联系管理员");
        }
        super::LocalValidationError::new(
            403,
            crate::gateway::bilingual_error(err, "wallet precheck failed"),
        )
    })?;

    Ok(api_key)
}
