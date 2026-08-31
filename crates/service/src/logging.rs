use std::sync::Once;

static LOGGER_INIT: Once = Once::new();

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn init_logging() {
    LOGGER_INIT.call_once(|| {
        let filter = non_empty_env("RUST_LOG")
            .or_else(|| non_empty_env("CODEXMANAGER_LOG"))
            .unwrap_or_else(|| "info".to_string());
        let mut builder = env_logger::Builder::new();
        builder.parse_filters(&filter);
        builder.format_timestamp_secs();

        if let Some(style) =
            non_empty_env("RUST_LOG_STYLE").or_else(|| non_empty_env("CODEXMANAGER_LOG_STYLE"))
        {
            builder.parse_write_style(&style);
        }

        let _ = builder.try_init();
    });
}
