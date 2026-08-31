use super::{
    clear_storage_cache_for_tests, clear_storage_open_count_for_tests, open_storage_at_path,
    storage_open_count_for_tests,
};
use std::ffi::OsString;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

struct EnvGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

/// 函数 `unique_db_path`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - prefix: 参数 prefix
///
/// # 返回
/// 返回函数执行结果
fn unique_db_path(prefix: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{prefix}-{nonce}.db"))
        .to_string_lossy()
        .to_string()
}

/// 函数 `open_storage_reuses_cached_connection_in_same_thread`
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
fn open_storage_reuses_cached_connection_in_same_thread() {
    let _env_lock = crate::test_env_guard();
    let db_path = unique_db_path("codexmanager-open-storage-reuse");
    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);

    let storage = open_storage_at_path(&db_path).expect("open storage 1");
    storage.init().expect("init");
    drop(storage);

    let storage = open_storage_at_path(&db_path).expect("open storage 2");
    drop(storage);

    assert_eq!(storage_open_count_for_tests(&db_path), 1);

    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);
    let _ = std::fs::remove_file(&db_path);
}

/// 函数 `open_storage_reopens_when_db_path_changes`
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
fn open_storage_reopens_when_db_path_changes() {
    let _env_lock = crate::test_env_guard();
    let db_path_1 = unique_db_path("codexmanager-open-storage-path-1");
    let db_path_2 = unique_db_path("codexmanager-open-storage-path-2");
    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path_1);
    clear_storage_open_count_for_tests(&db_path_2);

    let storage = open_storage_at_path(&db_path_1).expect("open storage path 1");
    storage.init().expect("init 1");
    drop(storage);

    let storage = open_storage_at_path(&db_path_2).expect("open storage path 2");
    storage.init().expect("init 2");
    drop(storage);

    assert_eq!(storage_open_count_for_tests(&db_path_1), 1);
    assert_eq!(storage_open_count_for_tests(&db_path_2), 1);

    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path_1);
    clear_storage_open_count_for_tests(&db_path_2);
    let _ = std::fs::remove_file(&db_path_1);
    let _ = std::fs::remove_file(&db_path_2);
}

/// 函数 `open_storage_waits_for_bounded_pool_slot`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn open_storage_waits_for_bounded_pool_slot() {
    let _env_lock = crate::test_env_guard();
    let _max_guard = EnvGuard::set("CODEXMANAGER_STORAGE_MAX_CONNECTIONS", "2");
    let _idle_guard = EnvGuard::set("CODEXMANAGER_STORAGE_MAX_IDLE_CONNECTIONS", "2");
    let _timeout_guard = EnvGuard::set("CODEXMANAGER_STORAGE_ACQUIRE_TIMEOUT_MS", "3000");
    let db_path = unique_db_path("codexmanager-open-storage-bounded");
    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);

    let release_pair = Arc::new((Mutex::new(false), Condvar::new()));
    let mut holders = Vec::new();
    for _ in 0..2 {
        let db_path = db_path.clone();
        let release_pair = Arc::clone(&release_pair);
        holders.push(thread::spawn(move || {
            let storage = open_storage_at_path(&db_path).expect("open held storage");
            let (lock, condvar) = &*release_pair;
            let mut released = lock.lock().expect("release lock");
            while !*released {
                released = condvar.wait(released).expect("release wait");
            }
            drop(storage);
        }));
    }

    let wait_started = Instant::now();
    while storage_open_count_for_tests(&db_path) < 2
        && wait_started.elapsed() < Duration::from_secs(3)
    {
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(storage_open_count_for_tests(&db_path), 2);

    let mut waiters = Vec::new();
    for _ in 0..4 {
        let db_path = db_path.clone();
        waiters.push(thread::spawn(move || {
            let storage = open_storage_at_path(&db_path).expect("open waited storage");
            drop(storage);
        }));
    }

    thread::sleep(Duration::from_millis(50));
    assert_eq!(storage_open_count_for_tests(&db_path), 2);

    {
        let (lock, condvar) = &*release_pair;
        let mut released = lock.lock().expect("release lock");
        *released = true;
        condvar.notify_all();
    }

    for holder in holders {
        holder.join().expect("holder join");
    }
    for waiter in waiters {
        waiter.join().expect("waiter join");
    }

    assert_eq!(storage_open_count_for_tests(&db_path), 2);

    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);
    let _ = std::fs::remove_file(&db_path);
}

/// 函数 `open_storage_times_out_when_pool_is_exhausted`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn open_storage_times_out_when_pool_is_exhausted() {
    let _env_lock = crate::test_env_guard();
    let _max_guard = EnvGuard::set("CODEXMANAGER_STORAGE_MAX_CONNECTIONS", "1");
    let _idle_guard = EnvGuard::set("CODEXMANAGER_STORAGE_MAX_IDLE_CONNECTIONS", "1");
    let _timeout_guard = EnvGuard::set("CODEXMANAGER_STORAGE_ACQUIRE_TIMEOUT_MS", "50");
    let db_path = unique_db_path("codexmanager-open-storage-timeout");
    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);

    let storage = open_storage_at_path(&db_path).expect("open held storage");
    let waited = open_storage_at_path(&db_path);
    assert!(waited.is_none());
    assert_eq!(storage_open_count_for_tests(&db_path), 1);
    drop(storage);

    clear_storage_cache_for_tests();
    clear_storage_open_count_for_tests(&db_path);
    let _ = std::fs::remove_file(&db_path);
}
