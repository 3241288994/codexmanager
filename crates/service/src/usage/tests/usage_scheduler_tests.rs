use super::{parse_interval_secs, run_blocking_poll_loop_with_sleep};
use std::cell::{Cell, RefCell};
use std::time::Duration;

/// 函数 `blocking_poll_loop_runs_task_and_respects_interval`
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
fn blocking_poll_loop_runs_task_and_respects_interval() {
    let task_runs = Cell::new(0usize);
    let sleep_calls = RefCell::new(Vec::new());

    run_blocking_poll_loop_with_sleep(
        "test-loop",
        Duration::from_secs(3),
        Duration::ZERO,
        Duration::from_secs(30),
        &mut || {
            task_runs.set(task_runs.get() + 1);
            Ok(())
        },
        &mut |_| true,
        |duration| {
            sleep_calls.borrow_mut().push(duration);
            task_runs.get() < 3
        },
        &mut || Duration::ZERO,
    );

    assert_eq!(task_runs.get(), 3);
    assert_eq!(sleep_calls.borrow().len(), 3);
    assert!(sleep_calls
        .borrow()
        .iter()
        .all(|d| *d == Duration::from_secs(3)));
}

/// 函数 `blocking_poll_loop_calls_error_filter_before_sleep`
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
fn blocking_poll_loop_calls_error_filter_before_sleep() {
    let checks = RefCell::new(Vec::new());
    let runs = Cell::new(0usize);

    run_blocking_poll_loop_with_sleep(
        "test-loop",
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(30),
        &mut || {
            runs.set(runs.get() + 1);
            if runs.get() == 1 {
                Err("ignored".to_string())
            } else {
                Err("fatal".to_string())
            }
        },
        &mut |err| {
            checks.borrow_mut().push(err.to_string());
            !err.contains("ignored")
        },
        |_| runs.get() < 2,
        &mut || Duration::ZERO,
    );

    assert_eq!(runs.get(), 2);
    assert_eq!(
        checks.borrow().as_slice(),
        ["ignored".to_string(), "fatal".to_string()]
    );
}

/// 函数 `blocking_poll_loop_applies_failure_backoff_with_cap_and_reset`
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
fn blocking_poll_loop_applies_failure_backoff_with_cap_and_reset() {
    let runs = Cell::new(0usize);
    let sleep_calls = RefCell::new(Vec::new());

    run_blocking_poll_loop_with_sleep(
        "test-loop",
        Duration::from_secs(2),
        Duration::ZERO,
        Duration::from_secs(5),
        &mut || {
            runs.set(runs.get() + 1);
            if runs.get() <= 3 {
                Err("upstream timeout".to_string())
            } else {
                Ok(())
            }
        },
        &mut |_| true,
        |duration| {
            sleep_calls.borrow_mut().push(duration);
            runs.get() < 4
        },
        &mut || Duration::ZERO,
    );

    assert_eq!(
        sleep_calls.borrow().as_slice(),
        [
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(5),
            Duration::from_secs(2),
        ]
    );
}

/// 函数 `blocking_poll_loop_adds_jitter_on_top_of_base_delay`
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
fn blocking_poll_loop_adds_jitter_on_top_of_base_delay() {
    let runs = Cell::new(0usize);
    let sleep_calls = RefCell::new(Vec::new());
    let jitter_seq = RefCell::new(vec![Duration::from_secs(6), Duration::from_secs(2)]);

    run_blocking_poll_loop_with_sleep(
        "test-loop",
        Duration::from_secs(10),
        Duration::from_secs(5),
        Duration::from_secs(30),
        &mut || {
            runs.set(runs.get() + 1);
            Ok(())
        },
        &mut |_| true,
        |duration| {
            sleep_calls.borrow_mut().push(duration);
            runs.get() < 2
        },
        &mut || jitter_seq.borrow_mut().remove(0),
    );

    assert_eq!(
        sleep_calls.borrow().as_slice(),
        [Duration::from_secs(15), Duration::from_secs(12)]
    );
}

/// 函数 `parse_interval_secs_falls_back_and_applies_minimum`
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
fn parse_interval_secs_falls_back_and_applies_minimum() {
    assert_eq!(parse_interval_secs(None, 600, 30), 600);
    assert_eq!(parse_interval_secs(Some(""), 600, 30), 600);
    assert_eq!(parse_interval_secs(Some("abc"), 600, 30), 600);
    assert_eq!(parse_interval_secs(Some("5"), 600, 30), 30);
    assert_eq!(parse_interval_secs(Some("120"), 600, 30), 120);
}
