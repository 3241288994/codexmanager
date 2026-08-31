use super::{evaluate_snapshot, Availability};
use codexmanager_core::storage::UsageSnapshotRecord;

/// 函数 `snap`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - primary_used: 参数 primary_used
/// - primary_window: 参数 primary_window
/// - secondary_used: 参数 secondary_used
/// - secondary_window: 参数 secondary_window
///
/// # 返回
/// 返回函数执行结果
fn snap(
    primary_used: Option<f64>,
    primary_window: Option<i64>,
    secondary_used: Option<f64>,
    secondary_window: Option<i64>,
) -> UsageSnapshotRecord {
    UsageSnapshotRecord {
        account_id: "acc-1".to_string(),
        used_percent: primary_used,
        window_minutes: primary_window,
        resets_at: None,
        secondary_used_percent: secondary_used,
        secondary_window_minutes: secondary_window,
        secondary_resets_at: None,
        credits_json: None,
        captured_at: 0,
    }
}

/// 函数 `availability_marks_missing_primary_unavailable`
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
fn availability_marks_missing_primary_unavailable() {
    let record = snap(None, Some(300), Some(10.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable(_)
    ));
}

/// 函数 `availability_marks_missing_secondary_available_when_both_secondary_fields_absent`
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
fn availability_marks_missing_secondary_available_when_both_secondary_fields_absent() {
    let record = snap(Some(10.0), Some(300), None, None);
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Available
    ));
}

/// 函数 `availability_marks_partial_secondary_missing_available`
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
fn availability_marks_partial_secondary_missing_available() {
    let record = snap(Some(10.0), Some(300), None, Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Available
    ));
}

/// 函数 `availability_marks_exhausted_secondary_unavailable`
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
fn availability_marks_exhausted_secondary_unavailable() {
    let record = snap(Some(10.0), Some(300), Some(100.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Unavailable(_)
    ));
}

/// 函数 `availability_marks_ok_available`
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
fn availability_marks_ok_available() {
    let record = snap(Some(10.0), Some(300), Some(20.0), Some(10080));
    assert!(matches!(
        evaluate_snapshot(&record),
        Availability::Available
    ));
}
