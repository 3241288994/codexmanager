use super::super::{ConversationBinding, Storage};

/// 函数 `sample_binding`
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
fn sample_binding() -> ConversationBinding {
    ConversationBinding {
        platform_key_hash: "key-hash-1".to_string(),
        conversation_id: "conv-1".to_string(),
        account_id: "acc-1".to_string(),
        thread_epoch: 1,
        thread_anchor: "thread-anchor-1".to_string(),
        status: "active".to_string(),
        last_model: Some("gpt-5.4".to_string()),
        last_switch_reason: None,
        created_at: 100,
        updated_at: 100,
        last_used_at: 100,
    }
}

/// 函数 `conversation_binding_roundtrip_and_touch`
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
fn conversation_binding_roundtrip_and_touch() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let binding = sample_binding();
    storage
        .upsert_conversation_binding(&binding)
        .expect("insert binding");

    let loaded = storage
        .get_conversation_binding("key-hash-1", "conv-1")
        .expect("load binding")
        .expect("binding exists");
    assert_eq!(loaded.account_id, "acc-1");
    assert_eq!(loaded.thread_anchor, "thread-anchor-1");
    assert_eq!(loaded.last_model.as_deref(), Some("gpt-5.4"));

    let touched = storage
        .touch_conversation_binding("key-hash-1", "conv-1", "acc-1", Some("gpt-5.5"), 200)
        .expect("touch binding");
    assert!(touched);

    let touched_loaded = storage
        .get_conversation_binding("key-hash-1", "conv-1")
        .expect("reload binding")
        .expect("binding exists");
    assert_eq!(touched_loaded.last_model.as_deref(), Some("gpt-5.5"));
    assert_eq!(touched_loaded.last_used_at, 200);
    assert_eq!(touched_loaded.updated_at, 200);
}

/// 函数 `conversation_binding_upsert_rebinds_existing_pair`
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
fn conversation_binding_upsert_rebinds_existing_pair() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let mut binding = sample_binding();
    storage
        .upsert_conversation_binding(&binding)
        .expect("insert binding");

    binding.account_id = "acc-2".to_string();
    binding.thread_epoch = 2;
    binding.thread_anchor = "thread-anchor-2".to_string();
    binding.last_switch_reason = Some("automatic_failover".to_string());
    binding.updated_at = 300;
    binding.last_used_at = 300;
    storage
        .upsert_conversation_binding(&binding)
        .expect("rebind binding");

    let loaded = storage
        .get_conversation_binding("key-hash-1", "conv-1")
        .expect("load rebound binding")
        .expect("binding exists");
    assert_eq!(loaded.account_id, "acc-2");
    assert_eq!(loaded.thread_epoch, 2);
    assert_eq!(loaded.thread_anchor, "thread-anchor-2");
    assert_eq!(
        loaded.last_switch_reason.as_deref(),
        Some("automatic_failover")
    );
    assert_eq!(loaded.created_at, 100);
    assert_eq!(loaded.updated_at, 300);
}

/// 函数 `conversation_binding_delete_helpers_remove_rows`
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
fn conversation_binding_delete_helpers_remove_rows() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init schema");

    let mut first = sample_binding();
    let second = ConversationBinding {
        platform_key_hash: "key-hash-1".to_string(),
        conversation_id: "conv-2".to_string(),
        account_id: "acc-1".to_string(),
        thread_epoch: 1,
        thread_anchor: "thread-anchor-2".to_string(),
        status: "active".to_string(),
        last_model: None,
        last_switch_reason: None,
        created_at: 100,
        updated_at: 100,
        last_used_at: 90,
    };
    first.last_used_at = 80;

    storage
        .upsert_conversation_binding(&first)
        .expect("insert first binding");
    storage
        .upsert_conversation_binding(&second)
        .expect("insert second binding");

    let removed = storage
        .delete_stale_conversation_bindings(85)
        .expect("delete stale bindings");
    assert_eq!(removed, 1);
    assert!(storage
        .get_conversation_binding("key-hash-1", "conv-1")
        .expect("load deleted binding")
        .is_none());

    storage
        .delete_conversation_bindings_for_account("acc-1")
        .expect("delete account bindings");
    assert!(storage
        .get_conversation_binding("key-hash-1", "conv-2")
        .expect("load remaining binding")
        .is_none());
}
