use codexmanager_core::storage::Storage;
use serde::Serialize;
use std::collections::HashSet;

use crate::account_plan::resolve_effective_account_plan;
use crate::storage_helpers::open_storage;

const MANAGED_ACCOUNT_STATUSES: &[&str] = &[
    "active",
    "unavailable",
    "banned",
    "limited",
    "disabled",
    "inactive",
    "unknown",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteAccountsResult {
    requested: usize,
    deleted: usize,
    failed: usize,
    deleted_account_ids: Vec<String>,
    failed_account_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteUnavailableFreeResult {
    scanned: usize,
    deleted: usize,
    skipped_available: usize,
    skipped_disabled: usize,
    skipped_non_free: usize,
    skipped_missing_usage: usize,
    skipped_missing_token: usize,
    deleted_account_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteAccountsByStatusesResult {
    scanned: usize,
    deleted: usize,
    skipped_status: usize,
    target_statuses: Vec<String>,
    deleted_account_ids: Vec<String>,
}

pub(crate) fn delete_account(account_id: &str) -> Result<(), String> {
    let account_id = required_account_id(account_id)?;
    let mut storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    ensure_account_exists(&storage, account_id)?;
    storage
        .delete_account(account_id)
        .map_err(|err| format!("delete account failed: {err}"))
}

pub(crate) fn delete_accounts(account_ids: Vec<String>) -> Result<DeleteAccountsResult, String> {
    let ids = normalized_unique_ids(account_ids);
    if ids.is_empty() {
        return Err("missing accountIds".to_string());
    }

    let mut storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let mut result = DeleteAccountsResult {
        requested: ids.len(),
        deleted: 0,
        failed: 0,
        deleted_account_ids: Vec::new(),
        failed_account_ids: Vec::new(),
    };

    for account_id in ids {
        let exists = storage
            .find_account_by_id(&account_id)
            .map_err(|err| format!("read account failed: {err}"))?
            .is_some();
        if !exists {
            result.failed += 1;
            result.failed_account_ids.push(account_id);
            continue;
        }
        match storage.delete_account(&account_id) {
            Ok(()) => {
                result.deleted += 1;
                result.deleted_account_ids.push(account_id);
            }
            Err(_) => {
                result.failed += 1;
                result.failed_account_ids.push(account_id);
            }
        }
    }
    Ok(result)
}

pub(crate) fn delete_unavailable_free_accounts() -> Result<DeleteUnavailableFreeResult, String> {
    let mut storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let accounts = storage
        .list_accounts()
        .map_err(|err| format!("list accounts failed: {err}"))?;
    let mut result = DeleteUnavailableFreeResult {
        scanned: accounts.len(),
        deleted: 0,
        skipped_available: 0,
        skipped_disabled: 0,
        skipped_non_free: 0,
        skipped_missing_usage: 0,
        skipped_missing_token: 0,
        deleted_account_ids: Vec::new(),
    };
    let mut candidates = Vec::new();

    for account in accounts {
        let status = account.status.trim().to_ascii_lowercase();
        if status == "disabled" {
            result.skipped_disabled += 1;
            continue;
        }
        if status != "unavailable" && status != "banned" {
            result.skipped_available += 1;
            continue;
        }
        let token = storage
            .find_token_by_account_id(&account.id)
            .map_err(|err| format!("read token failed: {err}"))?;
        let snapshot = storage
            .latest_usage_snapshot_for_account(&account.id)
            .map_err(|err| format!("read usage snapshot failed: {err}"))?;
        let subscription = storage
            .find_account_subscription(&account.id)
            .map_err(|err| format!("read account subscription failed: {err}"))?;
        let Some(token) = token else {
            result.skipped_missing_token += 1;
            continue;
        };
        let Some(plan) =
            resolve_effective_account_plan(Some(&token), snapshot.as_ref(), subscription.as_ref())
        else {
            if snapshot.is_none() {
                result.skipped_missing_usage += 1;
            } else {
                result.skipped_non_free += 1;
            }
            continue;
        };
        if plan.normalized == "free" {
            candidates.push(account.id);
        } else {
            result.skipped_non_free += 1;
        }
    }

    for account_id in candidates {
        storage
            .delete_account(&account_id)
            .map_err(|err| format!("delete account failed: {err}"))?;
        result.deleted += 1;
        result.deleted_account_ids.push(account_id);
    }
    Ok(result)
}

pub(crate) fn delete_accounts_by_statuses(
    statuses: Vec<String>,
) -> Result<DeleteAccountsByStatusesResult, String> {
    let target_statuses = normalize_statuses(statuses)?;
    let mut storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let accounts = storage
        .list_accounts()
        .map_err(|err| format!("list accounts failed: {err}"))?;
    let target_set = target_statuses
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let candidate_ids = accounts
        .iter()
        .filter(|account| target_set.contains(account.status.trim().to_ascii_lowercase().as_str()))
        .map(|account| account.id.clone())
        .collect::<Vec<_>>();
    let mut result = DeleteAccountsByStatusesResult {
        scanned: accounts.len(),
        deleted: 0,
        skipped_status: 0,
        target_statuses,
        deleted_account_ids: Vec::new(),
    };
    for account_id in candidate_ids {
        storage
            .delete_account(&account_id)
            .map_err(|err| format!("delete account failed: {err}"))?;
        result.deleted += 1;
        result.deleted_account_ids.push(account_id);
    }
    result.skipped_status = result.scanned.saturating_sub(result.deleted);
    Ok(result)
}

pub(crate) fn update_account(
    account_id: &str,
    sort: Option<i64>,
    status: Option<&str>,
    label: Option<&str>,
    note: Option<&str>,
    tags: Option<&str>,
    has_note: bool,
    has_tags: bool,
) -> Result<(), String> {
    let account_id = required_account_id(account_id)?;
    let mut storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    ensure_account_exists(&storage, account_id)?;

    if let Some(label) = label {
        let label = label.trim();
        if label.is_empty() {
            return Err("account label cannot be empty".to_string());
        }
        storage
            .update_account_label(account_id, label)
            .map_err(|err| format!("update account label failed: {err}"))?;
    }
    if let Some(sort) = sort {
        storage
            .update_account_sort(account_id, sort)
            .map_err(|err| format!("update account sort failed: {err}"))?;
    }
    if let Some(status) = status {
        let normalized = normalize_status(status)?;
        storage
            .update_account_status(account_id, &normalized)
            .map_err(|err| format!("update account status failed: {err}"))?;
    }
    if has_note || has_tags {
        let current = storage
            .find_account_metadata(account_id)
            .map_err(|err| format!("read account metadata failed: {err}"))?;
        let next_note = if has_note {
            note
        } else {
            current.as_ref().and_then(|value| value.note.as_deref())
        };
        let next_tags = if has_tags {
            tags
        } else {
            current.as_ref().and_then(|value| value.tags.as_deref())
        };
        storage
            .upsert_account_metadata(account_id, next_note, next_tags)
            .map_err(|err| format!("update account metadata failed: {err}"))?;
    }
    Ok(())
}

fn required_account_id(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("missing accountId".to_string())
    } else {
        Ok(value)
    }
}

fn ensure_account_exists(storage: &Storage, account_id: &str) -> Result<(), String> {
    if storage
        .find_account_by_id(account_id)
        .map_err(|err| format!("read account failed: {err}"))?
        .is_some()
    {
        Ok(())
    } else {
        Err("account not found".to_string())
    }
}

fn normalized_unique_ids(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && seen.insert(value.clone()))
        .collect()
}

fn normalize_statuses(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let statuses = values
        .into_iter()
        .map(|value| normalize_status(&value))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>();
    if statuses.is_empty() {
        Err("missing cleanup statuses".to_string())
    } else {
        Ok(statuses)
    }
}

fn normalize_status(value: &str) -> Result<String, String> {
    let normalized = value.trim().to_ascii_lowercase();
    if MANAGED_ACCOUNT_STATUSES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(format!("unsupported account status: {value}"))
    }
}
