use super::*;

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    /// 函数 `set`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - key: 参数 key
    /// - value: 参数 value
    ///
    /// # 返回
    /// 返回函数执行结果
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, original }
    }

    /// 函数 `clear`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - key: 参数 key
    ///
    /// # 返回
    /// 返回函数执行结果
    fn clear(key: &'static str) -> Self {
        let original = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    /// 函数 `drop`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 无
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
        // Runtime settings are cached globally; return the cache to the restored
        // environment as well so this test cannot leak into the next one.
        let _ = std::panic::catch_unwind(super::reload_from_env);
    }
}

/// 函数 `reload_from_env_updates_timeout_and_proxy`
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
fn reload_from_env_updates_timeout_and_proxy() {
    let _guard = crate::test_env_guard();
    let _timeout_guard = EnvGuard::set(ENV_UPSTREAM_TOTAL_TIMEOUT_MS, "777");
    let _stream_timeout_guard = EnvGuard::set(ENV_UPSTREAM_STREAM_TIMEOUT_MS, "888");
    let _inflight_guard = EnvGuard::set(ENV_ACCOUNT_MAX_INFLIGHT, "4");
    let _strict_allowlist_guard = EnvGuard::set(ENV_STRICT_REQUEST_PARAM_ALLOWLIST, "0");
    let _request_compression_guard = EnvGuard::set(ENV_ENABLE_REQUEST_COMPRESSION, "0");
    let _image_enabled_guard = EnvGuard::set(ENV_CODEX_IMAGE_GENERATION_ENABLED, "0");
    let _image_auto_inject_guard = EnvGuard::set(ENV_CODEX_IMAGE_GENERATION_AUTO_INJECT_TOOL, "1");
    let _image_main_model_guard = EnvGuard::set(ENV_CODEX_IMAGE_MAIN_MODEL, "gpt-5.4");
    let _image_tool_model_guard = EnvGuard::set(ENV_CODEX_IMAGE_TOOL_MODEL, "gpt-image-2");
    let _client_id_guard = EnvGuard::set(ENV_TOKEN_EXCHANGE_CLIENT_ID, "client-id-123");
    let _issuer_guard = EnvGuard::set(ENV_TOKEN_EXCHANGE_ISSUER, "https://issuer.example");
    let _proxy_guard = EnvGuard::set(ENV_UPSTREAM_PROXY_URL, "socks5://127.0.0.1:7890");

    reload_from_env();

    assert_eq!(upstream_total_timeout(), Some(Duration::from_millis(777)));
    assert_eq!(upstream_stream_timeout(), Some(Duration::from_millis(888)));
    assert_eq!(account_max_inflight_limit(), 4);
    assert!(!strict_request_param_allowlist_enabled());
    assert!(!request_compression_enabled());
    assert!(!codex_image_generation_enabled());
    assert!(codex_image_generation_auto_inject_tool_enabled());
    assert_eq!(current_codex_image_main_model(), "gpt-5.4");
    assert_eq!(current_codex_image_tool_model(), "gpt-image-2");
    assert_eq!(token_exchange_client_id(), "client-id-123");
    assert_eq!(
        token_exchange_default_issuer(),
        "https://issuer.example".to_string()
    );
    assert_eq!(
        upstream_proxy_url().as_deref(),
        Some("socks5h://127.0.0.1:7890")
    );
}

/// 函数 `reload_from_env_defaults_keep_request_gate_legacy_unbounded`
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
fn reload_from_env_defaults_keep_request_gate_legacy_unbounded() {
    let _guard = crate::test_env_guard();
    let _account_guard = EnvGuard::clear(ENV_ACCOUNT_MAX_INFLIGHT);
    let _strict_guard = EnvGuard::clear(ENV_STRICT_REQUEST_PARAM_ALLOWLIST);
    let _gate_guard = EnvGuard::clear(ENV_REQUEST_GATE_WAIT_TIMEOUT_MS);
    let _front_proxy_guard = EnvGuard::clear(ENV_FRONT_PROXY_MAX_BODY_BYTES);
    let _stream_guard = EnvGuard::clear(ENV_UPSTREAM_STREAM_TIMEOUT_MS);
    let _request_compression_guard = EnvGuard::clear(ENV_ENABLE_REQUEST_COMPRESSION);
    let _image_enabled_guard = EnvGuard::clear(ENV_CODEX_IMAGE_GENERATION_ENABLED);
    let _image_auto_inject_guard = EnvGuard::clear(ENV_CODEX_IMAGE_GENERATION_AUTO_INJECT_TOOL);
    let _image_main_model_guard = EnvGuard::clear(ENV_CODEX_IMAGE_MAIN_MODEL);
    let _image_tool_model_guard = EnvGuard::clear(ENV_CODEX_IMAGE_TOOL_MODEL);

    reload_from_env();

    assert_eq!(account_max_inflight_limit(), 0);
    assert!(!strict_request_param_allowlist_enabled());
    assert_eq!(request_gate_wait_timeout(), None);
    assert_eq!(front_proxy_max_body_bytes(), 0);
    assert_eq!(
        upstream_stream_timeout(),
        Some(Duration::from_millis(300_000))
    );
    assert!(request_compression_enabled());
    assert!(codex_image_generation_enabled());
    assert!(!codex_image_generation_auto_inject_tool_enabled());
    assert_eq!(current_codex_image_main_model(), "gpt-5.4-mini");
    assert_eq!(current_codex_image_tool_model(), "gpt-image-2");
}

/// 函数 `parse_proxy_list_env_limits_to_five_entries`
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
fn parse_proxy_list_env_limits_to_five_entries() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(
        ENV_PROXY_LIST,
        "http://p1:8080,http://p2:8080;http://p3:8080\nhttp://p4:8080\rhttp://p5:8080,http://p6:8080",
    );
    let parsed = parse_proxy_list_env();
    assert_eq!(parsed.len(), MAX_UPSTREAM_PROXY_POOL_SIZE);
    assert_eq!(parsed.first().map(String::as_str), Some("http://p1:8080"));
    assert_eq!(parsed.last().map(String::as_str), Some("http://p5:8080"));
}

/// 函数 `parse_proxy_list_env_normalizes_socks_entries`
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
fn parse_proxy_list_env_normalizes_socks_entries() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(
        ENV_PROXY_LIST,
        "socks5://127.0.0.1:7890,socks://127.0.0.1:7891,https://socks5://127.0.0.1:7892",
    );

    let parsed = parse_proxy_list_env();

    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0], "socks5h://127.0.0.1:7890");
    assert_eq!(parsed[1], "socks5h://127.0.0.1:7891");
    assert_eq!(parsed[2], "socks5h://127.0.0.1:7892");
}

/// 函数 `stable_proxy_index_is_deterministic`
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
fn stable_proxy_index_is_deterministic() {
    let _guard = crate::test_env_guard();
    let idx1 = stable_proxy_index("account-42", 5);
    let idx2 = stable_proxy_index("account-42", 5);
    assert_eq!(idx1, idx2);
    assert!(idx1.expect("index") < 5);
}

/// 函数 `set_upstream_proxy_url_updates_env_and_cache`
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
fn set_upstream_proxy_url_updates_env_and_cache() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(ENV_UPSTREAM_PROXY_URL, "");

    let applied = set_upstream_proxy_url(Some("http://127.0.0.1:7890")).expect("set proxy");
    assert_eq!(applied.as_deref(), Some("http://127.0.0.1:7890"));
    assert_eq!(
        std::env::var(ENV_UPSTREAM_PROXY_URL).ok().as_deref(),
        Some("http://127.0.0.1:7890")
    );
    assert_eq!(
        upstream_proxy_url().as_deref(),
        Some("http://127.0.0.1:7890")
    );

    let cleared = set_upstream_proxy_url(None).expect("clear proxy");
    assert!(cleared.is_none());
    assert_eq!(std::env::var(ENV_UPSTREAM_PROXY_URL).ok(), None);
    assert_eq!(upstream_proxy_url(), None);
}

/// 函数 `set_upstream_proxy_url_normalizes_socks_scheme`
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
fn set_upstream_proxy_url_normalizes_socks_scheme() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(ENV_UPSTREAM_PROXY_URL, "");

    let applied =
        set_upstream_proxy_url(Some("https://socks5://127.0.0.1:7890")).expect("set proxy");

    assert_eq!(applied.as_deref(), Some("socks5h://127.0.0.1:7890"));
    assert_eq!(
        std::env::var(ENV_UPSTREAM_PROXY_URL).ok().as_deref(),
        Some("socks5h://127.0.0.1:7890")
    );
}

/// 函数 `set_upstream_stream_timeout_ms_updates_env_and_cache`
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
fn set_upstream_stream_timeout_ms_updates_env_and_cache() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(ENV_UPSTREAM_STREAM_TIMEOUT_MS, "1800000");

    let applied = set_upstream_stream_timeout_ms(432100);

    assert_eq!(applied, 432100);
    assert_eq!(current_upstream_stream_timeout_ms(), 432100);
    assert_eq!(
        upstream_stream_timeout(),
        Some(Duration::from_millis(432100))
    );
    assert_eq!(
        std::env::var(ENV_UPSTREAM_STREAM_TIMEOUT_MS)
            .ok()
            .as_deref(),
        Some("432100")
    );
}

#[test]
fn set_upstream_total_timeout_ms_updates_env_and_cache() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(ENV_UPSTREAM_TOTAL_TIMEOUT_MS, "0");

    let applied = set_upstream_total_timeout_ms(120000);

    assert_eq!(applied, 120000);
    assert_eq!(current_upstream_total_timeout_ms(), 120000);
    assert_eq!(
        upstream_total_timeout(),
        Some(Duration::from_millis(120000))
    );
    assert_eq!(
        std::env::var(ENV_UPSTREAM_TOTAL_TIMEOUT_MS).ok().as_deref(),
        Some("120000")
    );
}

/// 函数 `normalize_model_slug_maps_legacy_gpt_5_4_pro_to_gpt_5_4`
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
fn normalize_model_slug_maps_legacy_gpt_5_4_pro_to_gpt_5_4() {
    let _guard = crate::test_env_guard();

    let actual = normalize_model_slug("gpt-5.4-pro").expect("normalize model");

    assert_eq!(actual, "gpt-5.4");
}

/// 函数 `normalize_model_slug_accepts_auto`
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
fn normalize_model_slug_accepts_auto() {
    let _guard = crate::test_env_guard();

    let actual = normalize_model_slug("auto").expect("normalize model");

    assert_eq!(actual, "auto");
}

#[test]
fn set_model_forward_rules_updates_env_cache_and_matching() {
    let _guard = crate::test_env_guard();
    let _rules_guard = EnvGuard::clear(ENV_MODEL_FORWARD_RULES);

    let applied = set_model_forward_rules("spark*=gpt-5.4-mini\nclaude-sonnet-4*=gpt-5.4")
        .expect("set model forward rules");

    assert_eq!(applied, "spark*=gpt-5.4-mini\nclaude-sonnet-4*=gpt-5.4");
    assert_eq!(current_model_forward_rules(), applied);
    assert_eq!(
        std::env::var(ENV_MODEL_FORWARD_RULES).ok().as_deref(),
        Some(applied.as_str())
    );
    assert_eq!(
        resolve_forwarded_model("spark"),
        Some("gpt-5.4-mini".to_string())
    );
    assert_eq!(
        resolve_forwarded_model("claude-sonnet-4-20250514"),
        Some("gpt-5.4".to_string())
    );
    assert_eq!(resolve_forwarded_model("gpt-5.4"), None);
}

#[test]
fn set_model_forward_rules_preserves_case_while_matching_case_insensitively() {
    let _guard = crate::test_env_guard();
    let _rules_guard = EnvGuard::clear(ENV_MODEL_FORWARD_RULES);

    let applied = set_model_forward_rules("Spark*=GPT-5.4-mini\nClaude-Sonnet-4*=Gemini-2.5-Pro")
        .expect("set mixed-case model forward rules");

    assert_eq!(
        applied,
        "Spark*=GPT-5.4-mini\nClaude-Sonnet-4*=Gemini-2.5-Pro"
    );
    assert_eq!(current_model_forward_rules(), applied);
    assert_eq!(
        std::env::var(ENV_MODEL_FORWARD_RULES).ok().as_deref(),
        Some(applied.as_str())
    );
    assert_eq!(
        resolve_forwarded_model("spark-lite"),
        Some("GPT-5.4-mini".to_string())
    );
    assert_eq!(
        resolve_forwarded_model("claude-sonnet-4-20250514"),
        Some("Gemini-2.5-Pro".to_string())
    );
}

#[test]
fn builtin_model_forward_rule_does_not_remap_codex_long_tail_slug() {
    let _guard = crate::test_env_guard();
    let _rules_guard = EnvGuard::clear(ENV_MODEL_FORWARD_RULES);

    assert_eq!(resolve_forwarded_model("gpt-5.3-codex-spark"), None);
}

#[test]
fn explicit_model_forward_rule_can_remap_codex_long_tail_slug() {
    let _guard = crate::test_env_guard();
    let _rules_guard = EnvGuard::clear(ENV_MODEL_FORWARD_RULES);

    let applied = set_model_forward_rules("gpt-5.3-codex-spark*=gpt-5.4-mini")
        .expect("set explicit spark rule");

    assert_eq!(
        resolve_forwarded_model("gpt-5.3-codex-spark"),
        Some("gpt-5.4-mini".to_string())
    );
    assert_eq!(current_model_forward_rules(), applied);
}

#[test]
fn set_model_forward_rules_rejects_invalid_target_auto() {
    let _guard = crate::test_env_guard();

    let err = set_model_forward_rules("spark*=auto").expect_err("auto target should be rejected");

    assert!(err.contains("target model cannot be auto"));
}

#[test]
fn set_compact_model_forward_rules_updates_env_cache_and_matching() {
    let _guard = crate::test_env_guard();
    let _rules_guard = EnvGuard::clear(ENV_COMPACT_MODEL_FORWARD_RULES);

    let applied = set_compact_model_forward_rules("gpt-5.4=gpt-5.4-openai-compact")
        .expect("set compact model forward rules");

    assert_eq!(applied, "gpt-5.4=gpt-5.4-openai-compact");
    assert_eq!(current_compact_model_forward_rules(), applied);
    assert_eq!(
        std::env::var(ENV_COMPACT_MODEL_FORWARD_RULES)
            .ok()
            .as_deref(),
        Some(applied.as_str())
    );
    assert_eq!(
        resolve_compact_forwarded_model("gpt-5.4"),
        Some("gpt-5.4-openai-compact".to_string())
    );
    assert_eq!(resolve_compact_forwarded_model("gpt-5.4-mini"), None);
    assert_eq!(resolve_forwarded_model("gpt-5.4"), None);
}

#[test]
fn compact_api_path_reads_chat_completions_override_from_env() {
    let _guard = crate::test_env_guard();
    let _compact_api_path = EnvGuard::set(ENV_COMPACT_API_PATH, "/v1/chat/completions");

    reload_from_env();

    assert_eq!(current_compact_api_path(), "/v1/chat/completions");
    assert!(compact_api_path_uses_chat_completions());
}

/// 函数 `set_originator_updates_env_and_dynamic_user_agent`
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
fn set_originator_updates_env_and_dynamic_user_agent() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(ENV_ORIGINATOR, "codex_cli_rs");

    let applied = set_originator("codex_cli_rs_windows").expect("set originator");

    assert_eq!(applied, "codex_cli_rs_windows");
    assert_eq!(current_originator(), "codex_cli_rs_windows");
    assert_eq!(current_wire_originator(), "codex_cli_rs_windows");
    assert_eq!(
        std::env::var(ENV_ORIGINATOR).ok().as_deref(),
        Some("codex_cli_rs_windows")
    );
    let expected_prefix = format!(
        "codex_cli_rs_windows/{}",
        current_codex_user_agent_version()
    );
    assert!(current_codex_user_agent().contains(expected_prefix.as_str()));
}

/// 函数 `set_codex_user_agent_version_updates_env_and_user_agent`
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
fn set_codex_user_agent_version_updates_env_and_user_agent() {
    let _guard = crate::test_env_guard();

    let applied = set_codex_user_agent_version("0.102.1").expect("set codex user agent version");

    assert_eq!(applied, "0.102.1");
    assert_eq!(current_codex_user_agent_version(), "0.102.1");
    let expected_prefix = format!("{}/0.102.1", current_wire_originator());
    assert!(current_codex_user_agent().contains(expected_prefix.as_str()));
}

/// 函数 `set_residency_requirement_updates_env_and_cache`
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
fn set_residency_requirement_updates_env_and_cache() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::clear(ENV_RESIDENCY_REQUIREMENT);

    let applied = set_residency_requirement(Some("us")).expect("set residency requirement");
    assert_eq!(applied.as_deref(), Some("us"));
    assert_eq!(current_residency_requirement().as_deref(), Some("us"));
    assert_eq!(
        std::env::var(ENV_RESIDENCY_REQUIREMENT).ok().as_deref(),
        Some("us")
    );

    let cleared = set_residency_requirement(None).expect("clear residency requirement");
    assert!(cleared.is_none());
    assert_eq!(current_residency_requirement(), None);
    assert_eq!(std::env::var(ENV_RESIDENCY_REQUIREMENT).ok(), None);
}

/// 函数 `set_request_compression_enabled_updates_env_and_cache`
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
fn set_request_compression_enabled_updates_env_and_cache() {
    let _guard = crate::test_env_guard();
    let _guard = EnvGuard::set(ENV_ENABLE_REQUEST_COMPRESSION, "1");

    let applied = set_request_compression_enabled(false);

    assert!(!applied);
    assert!(!request_compression_enabled());
    assert_eq!(
        std::env::var(ENV_ENABLE_REQUEST_COMPRESSION)
            .ok()
            .as_deref(),
        Some("0")
    );

    let reapplied = set_request_compression_enabled(true);
    assert!(reapplied);
    assert!(request_compression_enabled());
    assert_eq!(
        std::env::var(ENV_ENABLE_REQUEST_COMPRESSION)
            .ok()
            .as_deref(),
        Some("1")
    );
}

#[test]
fn terminal_user_agent_prefers_term_program_over_wt_session() {
    let _guard = crate::test_env_guard();
    let _term_program = EnvGuard::set("TERM_PROGRAM", "WindowsTerminal");
    let _term_program_version = EnvGuard::set("TERM_PROGRAM_VERSION", "1.21");
    let _wt_session = EnvGuard::set("WT_SESSION", "1");
    let _wezterm = EnvGuard::clear("WEZTERM_VERSION");
    let _iterm_session = EnvGuard::clear("ITERM_SESSION_ID");
    let _iterm_profile = EnvGuard::clear("ITERM_PROFILE");
    let _iterm_profile_name = EnvGuard::clear("ITERM_PROFILE_NAME");
    let _term_session = EnvGuard::clear("TERM_SESSION_ID");
    let _kitty = EnvGuard::clear("KITTY_WINDOW_ID");
    let _alacritty = EnvGuard::clear("ALACRITTY_SOCKET");
    let _konsole = EnvGuard::clear("KONSOLE_VERSION");
    let _gnome = EnvGuard::clear("GNOME_TERMINAL_SCREEN");
    let _vte = EnvGuard::clear("VTE_VERSION");
    let _term = EnvGuard::clear("TERM");

    assert_eq!(
        current_codex_terminal_user_agent_token(),
        "WindowsTerminal/1.21"
    );
}

#[test]
fn terminal_user_agent_detects_windows_terminal_from_wt_session() {
    let _guard = crate::test_env_guard();
    let _term_program = EnvGuard::clear("TERM_PROGRAM");
    let _term_program_version = EnvGuard::clear("TERM_PROGRAM_VERSION");
    let _wt_session = EnvGuard::set("WT_SESSION", "1");
    let _wezterm = EnvGuard::clear("WEZTERM_VERSION");
    let _iterm_session = EnvGuard::clear("ITERM_SESSION_ID");
    let _iterm_profile = EnvGuard::clear("ITERM_PROFILE");
    let _iterm_profile_name = EnvGuard::clear("ITERM_PROFILE_NAME");
    let _term_session = EnvGuard::clear("TERM_SESSION_ID");
    let _kitty = EnvGuard::clear("KITTY_WINDOW_ID");
    let _alacritty = EnvGuard::clear("ALACRITTY_SOCKET");
    let _konsole = EnvGuard::clear("KONSOLE_VERSION");
    let _gnome = EnvGuard::clear("GNOME_TERMINAL_SCREEN");
    let _vte = EnvGuard::clear("VTE_VERSION");
    let _term = EnvGuard::clear("TERM");

    assert_eq!(current_codex_terminal_user_agent_token(), "WindowsTerminal");
}

#[test]
fn terminal_user_agent_sanitizes_header_like_official_codex() {
    let _guard = crate::test_env_guard();
    let _term_program = EnvGuard::set("TERM_PROGRAM", "Weird Terminal()");
    let _term_program_version = EnvGuard::set("TERM_PROGRAM_VERSION", "1.2 beta");
    let _wt_session = EnvGuard::clear("WT_SESSION");
    let _term = EnvGuard::clear("TERM");

    assert_eq!(
        current_codex_terminal_user_agent_token(),
        "Weird_Terminal__/1.2_beta"
    );
}
