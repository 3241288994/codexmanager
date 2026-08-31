use super::{Event, Storage};

/// 函数 `latest_account_status_reasons_returns_latest_reason_per_account`
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
fn latest_account_status_reasons_returns_latest_reason_per_account() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    storage
        .insert_event(&Event {
            account_id: Some("acc-1".to_string()),
            event_type: "account_status_update".to_string(),
            message: "status=unavailable reason=usage_http_401".to_string(),
            created_at: 10,
        })
        .expect("insert first");
    storage
        .insert_event(&Event {
            account_id: Some("acc-1".to_string()),
            event_type: "account_status_update".to_string(),
            message: "status=unavailable reason=account_deactivated".to_string(),
            created_at: 20,
        })
        .expect("insert second");
    storage
        .insert_event(&Event {
            account_id: Some("acc-2".to_string()),
            event_type: "account_status_update".to_string(),
            message: "status=unavailable reason=workspace_deactivated".to_string(),
            created_at: 15,
        })
        .expect("insert third");

    let reasons = storage
        .latest_account_status_reasons(&[
            "acc-1".to_string(),
            "acc-2".to_string(),
            "missing".to_string(),
        ])
        .expect("load reasons");

    assert_eq!(
        reasons.get("acc-1").map(String::as_str),
        Some("account_deactivated")
    );
    assert_eq!(
        reasons.get("acc-2").map(String::as_str),
        Some("workspace_deactivated")
    );
    assert!(!reasons.contains_key("missing"));
}
