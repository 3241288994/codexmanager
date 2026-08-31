use crate::storage_helpers::open_storage;
use std::collections::HashSet;

use super::runtime::run_plugin_task;
use super::store::rearm_enabled_interval_tasks_for_plugin;

const DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS: u64 = 5;

/// 函数 `run_due_tasks_once`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn run_due_tasks_once() -> u64 {
    let Some(storage) = open_storage() else {
        return DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS;
    };
    let now = codexmanager_core::storage::now_ts();
    if rearm_enabled_interval_tasks_for_plugin(&storage, None, now).is_err() {
        log::warn!("repair plugin task schedules failed");
    }
    let tasks = match storage.list_due_plugin_tasks(now, 100) {
        Ok(items) => items,
        Err(err) => {
            log::warn!("list due plugin tasks failed: {err}");
            return DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS;
        }
    };
    for task in tasks {
        let _ = run_plugin_task(&task.id, None);
    }

    let installs = match storage.list_plugin_installs() {
        Ok(items) => items,
        Err(err) => {
            log::warn!("list plugin installs for scheduler failed: {err}");
            return DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS;
        }
    };
    let enabled_plugin_ids: HashSet<String> = installs
        .into_iter()
        .filter(|install| install.status == "enabled")
        .map(|install| install.plugin_id)
        .collect();
    let tasks = match storage.list_plugin_tasks(None) {
        Ok(items) => items,
        Err(err) => {
            log::warn!("list plugin tasks for scheduler failed: {err}");
            return DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS;
        }
    };

    let mut next_sleep_secs = DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS;
    for task in tasks {
        if task.schedule_kind == "manual" || !task.enabled {
            continue;
        }
        if !enabled_plugin_ids.contains(&task.plugin_id) {
            continue;
        }
        let Some(next_run_at) = task.next_run_at else {
            continue;
        };
        if next_run_at <= now {
            return 1;
        }
        next_sleep_secs = next_sleep_secs.min((next_run_at - now) as u64);
    }

    next_sleep_secs.clamp(1, DEFAULT_PLUGIN_SCHEDULER_INTERVAL_SECS)
}
