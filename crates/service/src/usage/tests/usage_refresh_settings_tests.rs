use super::*;

#[test]
fn validate_background_tasks_settings_rejects_invalid_warmup_cron_expression() {
    let patch = BackgroundTasksSettingsPatch {
        warmup_cron_expression: Some("99 99 99 99 99".to_string()),
        ..BackgroundTasksSettingsPatch::default()
    };
    let err = validate_background_tasks_settings_patch(&patch)
        .expect_err("invalid cron should be rejected");

    assert!(!err.trim().is_empty());
}

#[test]
fn validate_background_tasks_settings_rejects_enabled_empty_warmup_cron_expression() {
    let patch = BackgroundTasksSettingsPatch {
        warmup_cron_enabled: Some(true),
        warmup_cron_expression: Some("   ".to_string()),
        ..BackgroundTasksSettingsPatch::default()
    };
    let err = validate_background_tasks_settings_patch(&patch)
        .expect_err("enabled empty cron should be rejected");

    assert!(err.contains("required"));
}

#[test]
fn validate_background_tasks_settings_allows_disabling_existing_invalid_warmup_cron() {
    let _guard = crate::test_env_guard();
    set_background_tasks_settings(BackgroundTasksSettingsPatch {
        warmup_cron_enabled: Some(true),
        warmup_cron_expression: Some("99 99 99 99 99".to_string()),
        ..BackgroundTasksSettingsPatch::default()
    });

    let patch = BackgroundTasksSettingsPatch {
        warmup_cron_enabled: Some(false),
        ..BackgroundTasksSettingsPatch::default()
    };

    assert!(validate_background_tasks_settings_patch(&patch).is_ok());

    set_background_tasks_settings(BackgroundTasksSettingsPatch {
        warmup_cron_enabled: Some(false),
        warmup_cron_expression: Some(String::new()),
        ..BackgroundTasksSettingsPatch::default()
    });
}

#[test]
fn reload_background_tasks_runtime_from_env_notifies_warmup_cron_changes() {
    let _guard = crate::test_env_guard();
    std::env::remove_var("CODEXMANAGER_WARMUP_CRON_ENABLED");
    std::env::remove_var("CODEXMANAGER_WARMUP_CRON_EXPRESSION");
    reload_background_tasks_runtime_from_env();
    let previous_version = warmup_cron_signal_version();

    std::env::set_var("CODEXMANAGER_WARMUP_CRON_ENABLED", "1");
    std::env::set_var("CODEXMANAGER_WARMUP_CRON_EXPRESSION", "0 0 * * *");
    reload_background_tasks_runtime_from_env();

    assert_ne!(warmup_cron_signal_version(), previous_version);

    std::env::remove_var("CODEXMANAGER_WARMUP_CRON_ENABLED");
    std::env::remove_var("CODEXMANAGER_WARMUP_CRON_EXPRESSION");
    reload_background_tasks_runtime_from_env();
}
