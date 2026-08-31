use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::thread;

use super::{ensure_background_tasks_config_loaded, USAGE_REFRESH_WORKERS};

static PENDING_USAGE_REFRESH_TASKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static USAGE_REFRESH_EXECUTOR: OnceLock<UsageRefreshExecutor> = OnceLock::new();

/// 函数 `enqueue_usage_refresh_with_worker`
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
pub(crate) fn enqueue_usage_refresh_with_worker<F>(account_id: &str, worker: F) -> bool
where
    F: FnOnce(String) + Send + 'static,
{
    let id = account_id.trim();
    if id.is_empty() {
        return false;
    }
    if !mark_usage_refresh_task_pending(id) {
        return false;
    }
    let task = UsageRefreshTask {
        account_id: id.to_string(),
        worker: Box::new(worker),
    };
    if usage_refresh_executor().sender.send(task).is_err() {
        clear_usage_refresh_task_pending(id);
        return false;
    }
    true
}

struct UsageRefreshTask {
    account_id: String,
    worker: Box<dyn FnOnce(String) + Send + 'static>,
}

struct UsageRefreshExecutor {
    sender: Sender<UsageRefreshTask>,
}

impl UsageRefreshExecutor {
    /// 函数 `new`
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
    fn new() -> Self {
        let worker_count = usage_refresh_worker_count();
        let (sender, receiver) = unbounded::<UsageRefreshTask>();
        for index in 0..worker_count {
            let receiver = receiver.clone();
            let _ = thread::Builder::new()
                .name(format!("usage-refresh-worker-{index}"))
                .spawn(move || usage_refresh_worker_loop(receiver));
        }
        Self { sender }
    }
}

/// 函数 `usage_refresh_executor`
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
fn usage_refresh_executor() -> &'static UsageRefreshExecutor {
    USAGE_REFRESH_EXECUTOR.get_or_init(UsageRefreshExecutor::new)
}

/// 函数 `usage_refresh_worker_loop`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - receiver: 参数 receiver
///
/// # 返回
/// 无
fn usage_refresh_worker_loop(receiver: Receiver<UsageRefreshTask>) {
    while let Ok(task) = receiver.recv() {
        let UsageRefreshTask { account_id, worker } = task;
        let account_id_for_clear = account_id.clone();
        // worker 若 panic 需要强制清理 pending，避免该账号后续刷新被永久去重。
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            worker(account_id);
        }));
        clear_usage_refresh_task_pending(&account_id_for_clear);
    }
}

/// 函数 `usage_refresh_worker_count`
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
fn usage_refresh_worker_count() -> usize {
    ensure_background_tasks_config_loaded();
    USAGE_REFRESH_WORKERS.load(Ordering::Relaxed).max(1)
}

/// 函数 `mark_usage_refresh_task_pending`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - account_id: 参数 account_id
///
/// # 返回
/// 返回函数执行结果
fn mark_usage_refresh_task_pending(account_id: &str) -> bool {
    let mutex = PENDING_USAGE_REFRESH_TASKS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut pending = crate::lock_utils::lock_recover(mutex, "pending_usage_refresh_tasks");
    pending.insert(account_id.to_string())
}

/// 函数 `clear_usage_refresh_task_pending`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - account_id: 参数 account_id
///
/// # 返回
/// 无
fn clear_usage_refresh_task_pending(account_id: &str) {
    let Some(mutex) = PENDING_USAGE_REFRESH_TASKS.get() else {
        return;
    };
    let mut pending = crate::lock_utils::lock_recover(mutex, "pending_usage_refresh_tasks");
    pending.remove(account_id);
}

/// 函数 `clear_pending_usage_refresh_tasks_for_tests`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
#[cfg(test)]
pub(crate) fn clear_pending_usage_refresh_tasks_for_tests() {
    if let Some(mutex) = PENDING_USAGE_REFRESH_TASKS.get() {
        let mut pending = crate::lock_utils::lock_recover(mutex, "pending_usage_refresh_tasks");
        pending.clear();
    }
}
