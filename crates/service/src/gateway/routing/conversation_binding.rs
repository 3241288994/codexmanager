use codexmanager_core::storage::{now_ts, Account, ConversationBinding, Storage, Token};

#[derive(Debug, Clone)]
pub(crate) struct ConversationRoutingContext {
    pub(crate) platform_key_hash: String,
    pub(crate) conversation_id: String,
    pub(crate) source: RouteConversationSource,
    pub(crate) existing_binding: Option<ConversationBinding>,
    pub(crate) binding_selected: bool,
    pub(crate) bound_account_selectable: bool,
    pub(crate) manual_preferred_account_id: Option<String>,
    pub(crate) next_thread_epoch: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteConversationSource {
    NativeConversation,
    StickyFallback,
    PromptCacheKey,
    PromptCacheKeyExistingOnly,
}

impl RouteConversationSource {
    pub(crate) fn is_prompt_cache_key(self) -> bool {
        matches!(
            self,
            Self::PromptCacheKey | Self::PromptCacheKeyExistingOnly
        )
    }

    fn allows_initial_binding_create(self) -> bool {
        !matches!(self, Self::PromptCacheKeyExistingOnly)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateRotationSource {
    ConversationBinding,
    ManualPreferredAccount,
    RouteStrategy,
}

impl CandidateRotationSource {
    /// 函数 `as_str`
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
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ConversationBinding => "conversation_bound",
            Self::ManualPreferredAccount => "manual_preferred_account",
            Self::RouteStrategy => "route_strategy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CandidateRotationPlan {
    pub(crate) source: CandidateRotationSource,
    pub(crate) strategy_label: &'static str,
    pub(crate) strategy_applied: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ConversationThreadAttempt {
    pub(crate) thread_anchor: String,
    pub(crate) thread_epoch: i64,
    pub(crate) reset_session_affinity: bool,
}

/// 函数 `normalize_conversation_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - conversation_id: 参数 conversation_id
///
/// # 返回
/// 返回函数执行结果
fn normalize_conversation_id(conversation_id: Option<&str>) -> Option<String> {
    conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// 函数 `load_conversation_binding`
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
pub(crate) fn load_conversation_binding(
    storage: &Storage,
    platform_key_hash: &str,
    conversation_id: Option<&str>,
) -> Result<Option<ConversationBinding>, String> {
    let Some(conversation_id) = normalize_conversation_id(conversation_id) else {
        return Ok(None);
    };
    storage
        .get_conversation_binding(platform_key_hash, conversation_id.as_str())
        .map_err(|err| format!("load conversation binding failed: {err}"))
}

/// 函数 `effective_thread_anchor`
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
pub(crate) fn effective_thread_anchor(
    conversation_id: Option<&str>,
    binding: Option<&ConversationBinding>,
) -> Option<String> {
    binding
        .map(|item| item.thread_anchor.clone())
        .or_else(|| normalize_conversation_id(conversation_id))
}

/// 函数 `rotate_to_bound_account`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - candidates: 参数 candidates
/// - binding: 参数 binding
///
/// # 返回
/// 返回函数执行结果
fn rotate_to_bound_account(
    candidates: &mut [(Account, Token)],
    binding: &ConversationBinding,
) -> bool {
    rotate_to_account_id(candidates, binding.account_id.as_str())
}

/// 函数 `rotate_to_account_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - candidates: 参数 candidates
/// - account_id: 参数 account_id
///
/// # 返回
/// 返回函数执行结果
fn rotate_to_account_id(candidates: &mut [(Account, Token)], account_id: &str) -> bool {
    let Some(index) = candidates
        .iter()
        .position(|(account, _)| account.id == account_id)
    else {
        return false;
    };
    if index > 0 {
        candidates.rotate_left(index);
    }
    true
}

/// 函数 `derive_next_thread_epoch`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - existing_binding: 参数 existing_binding
///
/// # 返回
/// 返回函数执行结果
fn derive_next_thread_epoch(existing_binding: Option<&ConversationBinding>) -> Option<i64> {
    existing_binding.map(|binding| binding.thread_epoch + 1)
}

/// 函数 `switch_reason_for_account`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - routing: 参数 routing
/// - account_id: 参数 account_id
///
/// # 返回
/// 返回函数执行结果
fn switch_reason_for_account(
    routing: &ConversationRoutingContext,
    account_id: &str,
) -> &'static str {
    if routing
        .manual_preferred_account_id
        .as_deref()
        .is_some_and(|manual_id| manual_id == account_id)
    {
        "manual_account_switch"
    } else {
        "automatic_account_switch"
    }
}

/// 函数 `prepare_conversation_routing`
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
#[cfg(test)]
pub(crate) fn prepare_conversation_routing(
    platform_key_hash: &str,
    conversation_id: Option<&str>,
    existing_binding: Option<&ConversationBinding>,
    candidates: &mut Vec<(Account, Token)>,
) -> Option<ConversationRoutingContext> {
    prepare_conversation_routing_with_source(
        platform_key_hash,
        conversation_id,
        existing_binding,
        candidates,
        RouteConversationSource::StickyFallback,
    )
}

pub(crate) fn prepare_conversation_routing_with_source(
    platform_key_hash: &str,
    conversation_id: Option<&str>,
    existing_binding: Option<&ConversationBinding>,
    candidates: &mut Vec<(Account, Token)>,
    source: RouteConversationSource,
) -> Option<ConversationRoutingContext> {
    let conversation_id = normalize_conversation_id(conversation_id)?;
    let existing_binding = existing_binding.cloned();
    let bound_account_selectable = existing_binding.as_ref().is_some_and(|binding| {
        candidates
            .iter()
            .any(|(account, _)| account.id == binding.account_id)
    });
    let manual_preferred_account_id = super::manual_preferred_account()
        .filter(|account_id| rotate_to_account_id(candidates.as_mut_slice(), account_id));
    let binding_selected = if let Some(account_id) = manual_preferred_account_id.as_deref() {
        existing_binding
            .as_ref()
            .is_some_and(|binding| binding.account_id == account_id)
    } else {
        existing_binding
            .as_ref()
            .is_some_and(|binding| rotate_to_bound_account(candidates.as_mut_slice(), binding))
    };
    let next_thread_epoch = derive_next_thread_epoch(existing_binding.as_ref());

    Some(ConversationRoutingContext {
        platform_key_hash: platform_key_hash.to_string(),
        conversation_id,
        source,
        existing_binding,
        binding_selected,
        bound_account_selectable,
        manual_preferred_account_id,
        next_thread_epoch,
    })
}

/// 函数 `apply_candidate_rotation`
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
pub(crate) fn apply_candidate_rotation(
    candidates: &mut Vec<(Account, Token)>,
    routing: Option<&ConversationRoutingContext>,
    key_id: &str,
    model_for_log: Option<&str>,
) -> CandidateRotationPlan {
    if routing
        .as_ref()
        .is_some_and(|routing| routing.binding_selected)
    {
        return CandidateRotationPlan {
            source: CandidateRotationSource::ConversationBinding,
            strategy_label: "conversation_bound",
            strategy_applied: false,
        };
    }
    if routing
        .as_ref()
        .and_then(|routing| routing.manual_preferred_account_id.as_deref())
        .is_some()
    {
        return CandidateRotationPlan {
            source: CandidateRotationSource::ManualPreferredAccount,
            strategy_label: "manual_preferred_account",
            strategy_applied: false,
        };
    }
    let manual_preferred_account_id = super::manual_preferred_account();
    super::apply_route_strategy(candidates, key_id, model_for_log);
    if manual_preferred_account_id
        .as_deref()
        .is_some_and(|account_id| {
            candidates
                .iter()
                .any(|(account, _)| account.id == account_id)
        })
    {
        return CandidateRotationPlan {
            source: CandidateRotationSource::ManualPreferredAccount,
            strategy_label: "manual_preferred_account",
            strategy_applied: false,
        };
    }
    CandidateRotationPlan {
        source: CandidateRotationSource::RouteStrategy,
        strategy_label: super::current_route_strategy(),
        strategy_applied: true,
    }
}

/// 函数 `resolve_attempt_thread`
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
pub(crate) fn resolve_attempt_thread(
    routing: Option<&ConversationRoutingContext>,
    account: &Account,
) -> Option<ConversationThreadAttempt> {
    let routing = routing?;
    if routing.source.is_prompt_cache_key() {
        return None;
    }
    match routing.existing_binding.as_ref() {
        Some(binding) if binding.account_id == account.id => Some(ConversationThreadAttempt {
            thread_anchor: binding.thread_anchor.clone(),
            thread_epoch: binding.thread_epoch,
            reset_session_affinity: false,
        }),
        Some(binding) => Some(ConversationThreadAttempt {
            // 同一会话的 prompt_cache_key 必须稳定；账号切换只重置 session affinity。
            thread_anchor: binding.thread_anchor.clone(),
            thread_epoch: routing
                .next_thread_epoch
                .unwrap_or(binding.thread_epoch + 1),
            reset_session_affinity: true,
        }),
        None => Some(ConversationThreadAttempt {
            thread_anchor: routing.conversation_id.clone(),
            thread_epoch: 1,
            reset_session_affinity: false,
        }),
    }
}

/// 函数 `record_conversation_binding_terminal_response`
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
pub(crate) fn record_conversation_binding_terminal_response(
    storage: &Storage,
    routing: Option<&ConversationRoutingContext>,
    account: &Account,
    model: Option<&str>,
    status_code: u16,
) -> Result<(), String> {
    let Some(routing) = routing else {
        return Ok(());
    };
    let attempt_thread = resolve_attempt_thread(Some(routing), account);

    let now = now_ts();
    match routing.existing_binding.as_ref() {
        Some(binding) if binding.account_id == account.id => storage
            .touch_conversation_binding(
                routing.platform_key_hash.as_str(),
                routing.conversation_id.as_str(),
                account.id.as_str(),
                model,
                now,
            )
            .map(|_| ())
            .map_err(|err| format!("touch conversation binding failed: {err}")),
        Some(_) if routing.source.is_prompt_cache_key() && routing.bound_account_selectable => {
            Ok(())
        }
        Some(binding) if status_code < 400 => {
            let (thread_epoch, thread_anchor) = if routing.source.is_prompt_cache_key() {
                (binding.thread_epoch + 1, binding.thread_anchor.clone())
            } else {
                let attempt_thread = attempt_thread
                    .ok_or_else(|| "missing conversation thread for rebound account".to_string())?;
                (attempt_thread.thread_epoch, attempt_thread.thread_anchor)
            };
            let mut next = binding.clone();
            next.account_id = account.id.clone();
            next.thread_epoch = thread_epoch;
            next.thread_anchor = thread_anchor;
            next.last_model = model.map(str::to_string);
            next.last_switch_reason =
                Some(switch_reason_for_account(routing, account.id.as_str()).to_string());
            next.updated_at = now;
            next.last_used_at = now;
            storage
                .upsert_conversation_binding(&next)
                .map_err(|err| format!("rebind conversation binding failed: {err}"))
        }
        None if status_code < 400 && routing.source.allows_initial_binding_create() => {
            let (thread_epoch, thread_anchor) = if routing.source.is_prompt_cache_key() {
                (1, routing.conversation_id.clone())
            } else {
                let attempt_thread = attempt_thread
                    .ok_or_else(|| "missing conversation thread for initial binding".to_string())?;
                (attempt_thread.thread_epoch, attempt_thread.thread_anchor)
            };
            let binding = ConversationBinding {
                platform_key_hash: routing.platform_key_hash.clone(),
                conversation_id: routing.conversation_id.clone(),
                account_id: account.id.clone(),
                thread_epoch,
                thread_anchor,
                status: "active".to_string(),
                last_model: model.map(str::to_string),
                last_switch_reason: None,
                created_at: now,
                updated_at: now,
                last_used_at: now,
            };
            storage
                .upsert_conversation_binding(&binding)
                .map_err(|err| format!("create conversation binding failed: {err}"))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_candidate_rotation, effective_thread_anchor, prepare_conversation_routing,
        prepare_conversation_routing_with_source, record_conversation_binding_terminal_response,
        resolve_attempt_thread, CandidateRotationSource, RouteConversationSource,
    };
    use codexmanager_core::storage::{Account, ConversationBinding, Storage, Token};

    /// 函数 `sample_account`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - id: 参数 id
    /// - sort: 参数 sort
    ///
    /// # 返回
    /// 返回函数执行结果
    fn sample_account(id: &str, sort: i64) -> Account {
        Account {
            id: id.to_string(),
            label: id.to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: None,
            workspace_id: None,
            group_name: None,
            sort,
            status: "active".to_string(),
            created_at: 1,
            updated_at: 1,
        }
    }

    /// 函数 `sample_token`
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
    fn sample_token(account_id: &str) -> Token {
        Token {
            account_id: account_id.to_string(),
            id_token: String::new(),
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            api_key_access_token: None,
            last_refresh: 1,
        }
    }

    /// 函数 `sample_binding`
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
    fn sample_binding(account_id: &str) -> ConversationBinding {
        ConversationBinding {
            platform_key_hash: "key-hash-1".to_string(),
            conversation_id: "conv-1".to_string(),
            account_id: account_id.to_string(),
            thread_epoch: 1,
            thread_anchor: "thread-anchor-1".to_string(),
            status: "active".to_string(),
            last_model: Some("gpt-5.4".to_string()),
            last_switch_reason: None,
            created_at: 1,
            updated_at: 1,
            last_used_at: 1,
        }
    }

    /// 函数 `prepare_conversation_routing_rotates_bound_account_first`
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
    fn prepare_conversation_routing_rotates_bound_account_first() {
        let mut candidates = vec![
            (sample_account("acc-1", 0), sample_token("acc-1")),
            (sample_account("acc-2", 1), sample_token("acc-2")),
        ];
        let binding = sample_binding("acc-2");

        let actual = prepare_conversation_routing(
            "key-hash-1",
            Some("conv-1"),
            Some(&binding),
            &mut candidates,
        )
        .expect("routing context");

        assert!(actual.binding_selected);
        assert_eq!(candidates[0].0.id, "acc-2");
        assert_eq!(candidates[1].0.id, "acc-1");
    }

    /// 函数 `effective_thread_anchor_prefers_existing_binding_anchor`
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
    fn effective_thread_anchor_prefers_existing_binding_anchor() {
        let binding = sample_binding("acc-1");

        let actual = effective_thread_anchor(Some("conv-1"), Some(&binding));

        assert_eq!(actual.as_deref(), Some("thread-anchor-1"));
    }

    /// 函数 `resolve_attempt_thread_uses_next_generation_for_switched_account`
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
    fn resolve_attempt_thread_keeps_anchor_for_switched_account() {
        let binding = sample_binding("acc-1");
        let routing = prepare_conversation_routing(
            "key-hash-1",
            Some("conv-1"),
            Some(&binding),
            &mut vec![(sample_account("acc-2", 0), sample_token("acc-2"))],
        )
        .expect("routing context");

        let actual =
            resolve_attempt_thread(Some(&routing), &sample_account("acc-2", 0)).expect("thread");

        assert!(actual.reset_session_affinity);
        assert_eq!(actual.thread_epoch, 2);
        assert_eq!(actual.thread_anchor, binding.thread_anchor);
    }

    #[test]
    fn prompt_cache_route_binding_does_not_create_attempt_thread() {
        let binding = sample_binding("acc-1");
        let routing = prepare_conversation_routing_with_source(
            "key-hash-1",
            Some("pck:v1:abcdef"),
            Some(&binding),
            &mut vec![(sample_account("acc-1", 0), sample_token("acc-1"))],
            RouteConversationSource::PromptCacheKey,
        )
        .expect("routing context");

        let actual = resolve_attempt_thread(Some(&routing), &sample_account("acc-1", 0));

        assert!(actual.is_none());
    }

    #[test]
    fn prompt_cache_route_binding_records_without_attempt_thread() {
        let storage = Storage::open_in_memory().expect("open in memory");
        storage.init().expect("init schema");
        let mut candidates = vec![(sample_account("acc-1", 0), sample_token("acc-1"))];
        let routing = prepare_conversation_routing_with_source(
            "key-hash-1",
            Some("pck:v1:abcdef"),
            None,
            &mut candidates,
            RouteConversationSource::PromptCacheKey,
        )
        .expect("routing context");

        record_conversation_binding_terminal_response(
            &storage,
            Some(&routing),
            &candidates[0].0,
            Some("gpt-5.5"),
            200,
        )
        .expect("record prompt cache route binding");

        let created = storage
            .get_conversation_binding("key-hash-1", "pck:v1:abcdef")
            .expect("load binding")
            .expect("binding exists");
        assert_eq!(created.account_id, "acc-1");
        assert_eq!(created.thread_anchor, "pck:v1:abcdef");
    }

    #[test]
    fn prompt_cache_route_binding_rotates_bound_account_first() {
        let mut binding = sample_binding("acc-2");
        binding.conversation_id = "pck:v1:abcdef".to_string();
        binding.thread_anchor = "pck:v1:abcdef".to_string();
        let mut candidates = vec![
            (sample_account("acc-1", 0), sample_token("acc-1")),
            (sample_account("acc-2", 1), sample_token("acc-2")),
        ];

        let routing = prepare_conversation_routing_with_source(
            "key-hash-1",
            Some("pck:v1:abcdef"),
            Some(&binding),
            &mut candidates,
            RouteConversationSource::PromptCacheKey,
        )
        .expect("routing context");

        assert!(routing.binding_selected);
        assert_eq!(candidates[0].0.id, "acc-2");
        assert_eq!(candidates[1].0.id, "acc-1");
    }

    #[test]
    fn prompt_cache_route_binding_does_not_rebind_after_selected_binding_failover_success() {
        let storage = Storage::open_in_memory().expect("open in memory");
        storage.init().expect("init schema");
        let mut binding = sample_binding("acc-1");
        binding.conversation_id = "pck:v1:abcdef".to_string();
        binding.thread_anchor = "pck:v1:abcdef".to_string();
        storage
            .upsert_conversation_binding(&binding)
            .expect("seed binding");
        let mut candidates = vec![
            (sample_account("acc-1", 0), sample_token("acc-1")),
            (sample_account("acc-2", 1), sample_token("acc-2")),
        ];
        let routing = prepare_conversation_routing_with_source(
            "key-hash-1",
            Some("pck:v1:abcdef"),
            Some(&binding),
            &mut candidates,
            RouteConversationSource::PromptCacheKey,
        )
        .expect("routing context");
        assert!(routing.binding_selected);

        record_conversation_binding_terminal_response(
            &storage,
            Some(&routing),
            &candidates[1].0,
            Some("gpt-5.5"),
            200,
        )
        .expect("record failover success");

        let actual = storage
            .get_conversation_binding("key-hash-1", "pck:v1:abcdef")
            .expect("load binding")
            .expect("binding exists");
        assert_eq!(actual.account_id, "acc-1");
        assert_eq!(actual.thread_anchor, "pck:v1:abcdef");
        assert_eq!(actual.thread_epoch, 1);
    }

    #[test]
    fn prompt_cache_route_binding_rebinds_when_bound_account_is_not_selected() {
        let storage = Storage::open_in_memory().expect("open in memory");
        storage.init().expect("init schema");
        let mut binding = sample_binding("acc-1");
        binding.conversation_id = "pck:v1:abcdef".to_string();
        binding.thread_anchor = "pck:v1:abcdef".to_string();
        storage
            .upsert_conversation_binding(&binding)
            .expect("seed binding");
        let mut candidates = vec![(sample_account("acc-2", 0), sample_token("acc-2"))];
        let routing = prepare_conversation_routing_with_source(
            "key-hash-1",
            Some("pck:v1:abcdef"),
            Some(&binding),
            &mut candidates,
            RouteConversationSource::PromptCacheKey,
        )
        .expect("routing context");
        assert!(!routing.binding_selected);

        record_conversation_binding_terminal_response(
            &storage,
            Some(&routing),
            &candidates[0].0,
            Some("gpt-5.5"),
            200,
        )
        .expect("record stale rebind success");

        let actual = storage
            .get_conversation_binding("key-hash-1", "pck:v1:abcdef")
            .expect("load binding")
            .expect("binding exists");
        assert_eq!(actual.account_id, "acc-2");
        assert_eq!(actual.thread_anchor, "pck:v1:abcdef");
        assert_eq!(actual.thread_epoch, 2);
    }

    #[test]
    fn prompt_cache_existing_only_route_binding_does_not_create_initial_binding() {
        let storage = Storage::open_in_memory().expect("open in memory");
        storage.init().expect("init schema");
        let mut candidates = vec![(sample_account("acc-1", 0), sample_token("acc-1"))];
        let routing = prepare_conversation_routing_with_source(
            "key-hash-1",
            Some("pck:v1:abcdef"),
            None,
            &mut candidates,
            RouteConversationSource::PromptCacheKeyExistingOnly,
        )
        .expect("routing context");

        record_conversation_binding_terminal_response(
            &storage,
            Some(&routing),
            &candidates[0].0,
            Some("gpt-5.5"),
            200,
        )
        .expect("record existing-only success");

        let actual = storage
            .get_conversation_binding("key-hash-1", "pck:v1:abcdef")
            .expect("load binding");
        assert!(actual.is_none());
    }

    /// 函数 `apply_candidate_rotation_reports_binding_source_when_binding_selected`
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
    fn apply_candidate_rotation_reports_binding_source_when_binding_selected() {
        let binding = sample_binding("acc-1");
        let mut routing = prepare_conversation_routing(
            "key-hash-1",
            Some("conv-1"),
            Some(&binding),
            &mut vec![
                (sample_account("acc-2", 0), sample_token("acc-2")),
                (sample_account("acc-1", 1), sample_token("acc-1")),
            ],
        )
        .expect("routing context");
        routing.manual_preferred_account_id = Some("acc-1".to_string());
        let mut candidates = vec![
            (sample_account("acc-2", 0), sample_token("acc-2")),
            (sample_account("acc-1", 1), sample_token("acc-1")),
        ];

        let plan = apply_candidate_rotation(
            &mut candidates,
            Some(&routing),
            "key-hash-1",
            Some("gpt-5.4"),
        );

        assert_eq!(plan.source, CandidateRotationSource::ConversationBinding);
        assert_eq!(plan.strategy_label, "conversation_bound");
        assert!(!plan.strategy_applied);
        assert_eq!(candidates[0].0.id, "acc-2");
    }

    #[test]
    fn apply_candidate_rotation_reports_manual_preferred_source() {
        let mut routing = prepare_conversation_routing(
            "key-hash-1",
            Some("conv-1"),
            None,
            &mut vec![
                (sample_account("acc-2", 0), sample_token("acc-2")),
                (sample_account("acc-1", 1), sample_token("acc-1")),
            ],
        )
        .expect("routing context");
        routing.manual_preferred_account_id = Some("acc-2".to_string());
        let mut candidates = vec![
            (sample_account("acc-2", 0), sample_token("acc-2")),
            (sample_account("acc-1", 1), sample_token("acc-1")),
        ];

        let plan = apply_candidate_rotation(
            &mut candidates,
            Some(&routing),
            "key-hash-1",
            Some("gpt-5.4"),
        );

        assert_eq!(plan.source, CandidateRotationSource::ManualPreferredAccount);
        assert_eq!(plan.strategy_label, "manual_preferred_account");
        assert!(!plan.strategy_applied);
    }

    /// 函数 `terminal_response_creates_and_rebinds_conversation_binding_on_success`
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
    fn terminal_response_creates_and_rebinds_conversation_binding_on_success() {
        let storage = Storage::open_in_memory().expect("open in memory");
        storage.init().expect("init schema");

        let mut candidates = vec![(sample_account("acc-1", 0), sample_token("acc-1"))];
        let routing =
            prepare_conversation_routing("key-hash-1", Some("conv-1"), None, &mut candidates)
                .expect("routing context");
        record_conversation_binding_terminal_response(
            &storage,
            Some(&routing),
            &candidates[0].0,
            Some("gpt-5.4"),
            200,
        )
        .expect("create binding");

        let created = storage
            .get_conversation_binding("key-hash-1", "conv-1")
            .expect("load binding")
            .expect("binding exists");
        assert_eq!(created.account_id, "acc-1");

        let rebound_context = prepare_conversation_routing(
            "key-hash-1",
            Some("conv-1"),
            Some(&created),
            &mut vec![(sample_account("acc-2", 0), sample_token("acc-2"))],
        )
        .expect("rebound routing");
        record_conversation_binding_terminal_response(
            &storage,
            Some(&rebound_context),
            &sample_account("acc-2", 0),
            Some("gpt-5.5"),
            200,
        )
        .expect("rebind binding");

        let rebound = storage
            .get_conversation_binding("key-hash-1", "conv-1")
            .expect("reload binding")
            .expect("binding exists");
        assert_eq!(rebound.account_id, "acc-2");
        assert_eq!(rebound.thread_epoch, 2);
        assert_eq!(rebound.thread_anchor, created.thread_anchor);
        assert_eq!(rebound.last_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(
            rebound.last_switch_reason.as_deref(),
            Some("automatic_account_switch")
        );
    }
}
