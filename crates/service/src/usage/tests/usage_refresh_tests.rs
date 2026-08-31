use super::{
    clear_pending_usage_refresh_tasks_for_tests, enqueue_usage_refresh_with_worker,
    next_usage_poll_cursor, notify_usage_refresh_completed, reset_usage_poll_cursor_for_tests,
    resolve_token_refresh_issuer, run_token_refresh_task, set_usage_refresh_completed_handler,
    should_retry_usage_refresh_with_token, subscribe_usage_refresh_completed,
    token_refresh_access_exp_cutoff, token_refresh_due_cutoff, token_refresh_schedule,
    usage_poll_batch_indices,
};
use codexmanager_core::storage::{now_ts, Account, Storage, Token};
use std::collections::HashSet;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn usage_refresh_completed_handler_receives_notification() {
    let _guard = crate::test_env_guard();
    let (tx, rx) = mpsc::channel();
    set_usage_refresh_completed_handler(move |event| {
        let _ = tx.send(event);
    });

    notify_usage_refresh_completed("test-notify", 2, 3);
    let event = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("usage refresh completed event");
    assert_eq!(event.source, "test-notify");
    assert_eq!(event.processed, 2);
    assert_eq!(event.total, 3);
    assert!(event.completed_at > 0);
}

#[test]
fn usage_refresh_completed_subscriber_receives_notification() {
    let _guard = crate::test_env_guard();
    let rx = subscribe_usage_refresh_completed();

    notify_usage_refresh_completed("test-subscribe", 1, 1);
    let event = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("usage refresh completed event");
    assert_eq!(event.source, "test-subscribe");
    assert_eq!(event.processed, 1);
    assert_eq!(event.total, 1);
    assert!(event.completed_at > 0);
}

/// 函数 `enqueue_usage_refresh_for_same_account_is_deduplicated_until_finish`
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
fn enqueue_usage_refresh_for_same_account_is_deduplicated_until_finish() {
    let _guard = crate::test_env_guard();
    clear_pending_usage_refresh_tasks_for_tests();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let first = enqueue_usage_refresh_with_worker("acc-dedup", move |_| {
        let _ = started_tx.send(());
        let _ = release_rx.recv();
    });
    assert!(first);
    started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("worker started");

    let second = enqueue_usage_refresh_with_worker("acc-dedup", |_| {});
    assert!(!second);

    let _ = release_tx.send(());
    std::thread::sleep(Duration::from_millis(20));

    let third = enqueue_usage_refresh_with_worker("acc-dedup", |_| {});
    assert!(third);
    std::thread::sleep(Duration::from_millis(20));
    clear_pending_usage_refresh_tasks_for_tests();
}

/// 函数 `enqueue_usage_refresh_for_different_accounts_keeps_queue_progress`
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
fn enqueue_usage_refresh_for_different_accounts_keeps_queue_progress() {
    let _guard = crate::test_env_guard();
    clear_pending_usage_refresh_tasks_for_tests();
    let (started_tx, started_rx) = mpsc::channel::<String>();
    let (release_tx, release_rx) = mpsc::channel();
    let started_tx_first = started_tx.clone();

    let first = enqueue_usage_refresh_with_worker("acc-a", move |_| {
        let _ = started_tx_first.send("acc-a".to_string());
        let _ = release_rx.recv_timeout(Duration::from_secs(1));
    });
    assert!(first);

    let started_tx = started_tx.clone();
    let second = enqueue_usage_refresh_with_worker("acc-b", move |_| {
        let _ = started_tx.send("acc-b".to_string());
    });
    assert!(second);

    let first_started = started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("first task should start");
    let _ = release_tx.send(());
    let second_started = started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("second task should start");

    let seen: HashSet<String> = [first_started, second_started].into_iter().collect();
    assert_eq!(seen.len(), 2);
    assert!(seen.contains("acc-a"));
    assert!(seen.contains("acc-b"));

    std::thread::sleep(Duration::from_millis(20));
    clear_pending_usage_refresh_tasks_for_tests();
}

/// 函数 `schedule_prefers_exp_minus_ahead`
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
fn schedule_prefers_exp_minus_ahead() {
    let now = now_ts();
    let token = Token {
        account_id: "acc-1".to_string(),
        id_token: "id".to_string(),
        access_token: "a.eyJleHAiOjQxMDI0NDQ4MDB9.s".to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: now - 10,
    };
    let (exp, scheduled_at) = token_refresh_schedule(&token, now, 3600, 2700);
    assert_eq!(exp, Some(4_102_444_800));
    assert_eq!(scheduled_at, 4_102_441_200);
}

#[test]
fn schedule_prefers_refresh_token_exp_when_it_expires_first() {
    let now = now_ts();
    let token = Token {
        account_id: "acc-refresh-exp-first".to_string(),
        id_token: "id".to_string(),
        access_token: "a.eyJleHAiOjQxMDI0NDQ4MDB9.s".to_string(),
        refresh_token: "r.eyJleHAiOjQxMDI0NDMwMDB9.s".to_string(),
        api_key_access_token: None,
        last_refresh: now - 10,
    };
    let (exp, scheduled_at) = token_refresh_schedule(&token, now, 3600, 2700);
    assert_eq!(exp, Some(4_102_444_800));
    assert_eq!(scheduled_at, 4_102_439_400);
}

/// 函数 `schedule_falls_back_to_last_refresh_when_exp_missing`
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
fn schedule_falls_back_to_last_refresh_when_exp_missing() {
    let now = now_ts();
    let token = Token {
        account_id: "acc-2".to_string(),
        id_token: "id".to_string(),
        access_token: "no-jwt".to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: now - 5000,
    };
    let (exp, scheduled_at) = token_refresh_schedule(&token, now, 300, 2700);
    assert_eq!(exp, None);
    assert_eq!(scheduled_at, now);
}

/// 函数 `schedule_skips_when_refresh_token_is_empty`
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
fn schedule_skips_when_refresh_token_is_empty() {
    let now = now_ts();
    let token = Token {
        account_id: "acc-empty-refresh".to_string(),
        id_token: "id".to_string(),
        access_token: "a.eyJleHAiOjQxMDI0NDQ4MDB9.s".to_string(),
        refresh_token: String::new(),
        api_key_access_token: None,
        last_refresh: now - 10,
    };
    let (exp, scheduled_at) = token_refresh_schedule(&token, now, 600, 2700);
    assert_eq!(exp, None);
    assert_eq!(scheduled_at, i64::MAX);
}

/// 函数 `usage_refresh_retry_skips_when_refresh_token_is_empty`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-12
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn usage_refresh_retry_skips_when_refresh_token_is_empty() {
    let token = Token {
        account_id: "acc-empty-refresh".to_string(),
        id_token: "id".to_string(),
        access_token: "access".to_string(),
        refresh_token: String::new(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    };

    assert!(!should_retry_usage_refresh_with_token(
        &token,
        "usage endpoint status 401 Unauthorized"
    ));
    assert!(!should_retry_usage_refresh_with_token(
        &token,
        "usage endpoint status 403 Forbidden"
    ));
}

#[test]
fn usage_refresh_retry_skips_region_blocked_errors() {
    let token = Token {
        account_id: "acc-region-blocked-retry".to_string(),
        id_token: "id".to_string(),
        access_token: "access".to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    };

    assert!(!should_retry_usage_refresh_with_token(
        &token,
        "usage endpoint failed: status=403 Forbidden body=code=unsupported_country_region_territory cf_ray=ray-HKG",
    ));
    assert!(should_retry_usage_refresh_with_token(
        &token,
        "usage endpoint status 403 Forbidden"
    ));
}

/// 函数 `due_cutoff_includes_next_poll_window_and_buffer`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-06
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn due_cutoff_includes_next_poll_window_and_buffer() {
    let now = now_ts();
    assert_eq!(token_refresh_due_cutoff(now, 600), now + 660);
}

/// 函数 `access_exp_cutoff_includes_refresh_ahead_window`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-26
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn access_exp_cutoff_includes_refresh_ahead_window() {
    assert_eq!(token_refresh_access_exp_cutoff(1_000, 3600), 4_600);
}

/// 函数 `due_cutoff_covers_boundary_when_poll_interval_matches_refresh_ahead`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-06
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn due_cutoff_covers_boundary_when_poll_interval_matches_refresh_ahead() {
    let exp = 4_102_444_800;
    let now = exp - 7_260;
    let token = Token {
        account_id: "acc-boundary".to_string(),
        id_token: "id".to_string(),
        access_token: "a.eyJleHAiOjQxMDI0NDQ4MDB9.s".to_string(),
        refresh_token: "refresh".to_string(),
        api_key_access_token: None,
        last_refresh: now - 10,
    };
    let (_, scheduled_at) = token_refresh_schedule(&token, now, 3600, 2700);

    assert_eq!(scheduled_at, exp - 3600);
    assert!(scheduled_at > now);
    assert!(scheduled_at <= token_refresh_due_cutoff(now, 3600));
}

/// 函数 `token_refresh_issuer_uses_account_issuer`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-26
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn token_refresh_issuer_uses_account_issuer() {
    let now = now_ts();
    let account = Account {
        id: "acc-custom-issuer".to_string(),
        label: "custom issuer".to_string(),
        issuer: "https://custom-issuer.example".to_string(),
        chatgpt_account_id: None,
        workspace_id: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    };

    assert_eq!(
        resolve_token_refresh_issuer(Some(&account), "https://auth.openai.com"),
        "https://custom-issuer.example"
    );
}

/// 函数 `token_refresh_issuer_falls_back_to_default`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-26
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn token_refresh_issuer_falls_back_to_default() {
    let now = now_ts();
    let account = Account {
        id: "acc-empty-issuer".to_string(),
        label: "empty issuer".to_string(),
        issuer: "  ".to_string(),
        chatgpt_account_id: None,
        workspace_id: None,
        group_name: None,
        sort: 0,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    };

    assert_eq!(
        resolve_token_refresh_issuer(Some(&account), "https://auth.openai.com"),
        "https://auth.openai.com"
    );
    assert_eq!(
        resolve_token_refresh_issuer(None, "https://auth.openai.com"),
        "https://auth.openai.com"
    );
}

/// 函数 `run_token_refresh_task_skips_empty_refresh_token`
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
fn run_token_refresh_task_skips_empty_refresh_token() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let now = now_ts();
    let mut token = Token {
        account_id: "acc-empty-refresh".to_string(),
        id_token: "id".to_string(),
        access_token: "access".to_string(),
        refresh_token: String::new(),
        api_key_access_token: None,
        last_refresh: now,
    };

    let refreshed =
        run_token_refresh_task(&storage, &mut token, "https://auth.openai.com", "codex-cli");
    assert!(!refreshed);
}

/// 函数 `usage_poll_batch_indices_rotate_from_cursor`
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
fn usage_poll_batch_indices_rotate_from_cursor() {
    reset_usage_poll_cursor_for_tests();
    assert_eq!(usage_poll_batch_indices(5, 4, 3), vec![4, 0, 1]);
    assert_eq!(usage_poll_batch_indices(3, 1, 10), vec![1, 2, 0]);
}

/// 函数 `usage_poll_cursor_advances_by_processed_count`
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
fn usage_poll_cursor_advances_by_processed_count() {
    reset_usage_poll_cursor_for_tests();
    assert_eq!(next_usage_poll_cursor(5, 4, 2), 1);
    assert_eq!(next_usage_poll_cursor(5, 1, 5), 1);
    assert_eq!(next_usage_poll_cursor(0, 7, 3), 0);
}
