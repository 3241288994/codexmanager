/// 函数 `load_env_from_exe_dir`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 无
pub(crate) fn load_env_from_exe_dir() {
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(err) => {
            log::warn!("Failed to resolve current exe path: {}", err);
            return;
        }
    };
    let Some(exe_dir) = exe_path.parent() else {
        return;
    };

    let candidates = ["codexmanager.env", "CodexManager.env", ".env"];
    let mut chosen = None;
    for name in candidates {
        let p = exe_dir.join(name);
        if p.is_file() {
            chosen = Some(p);
            break;
        }
    }
    let Some(path) = chosen else {
        return;
    };

    let bytes = match std::fs::read(&path) {
        Ok(v) => v,
        Err(err) => {
            log::warn!("Failed to read env file {}: {}", path.display(), err);
            return;
        }
    };
    let content = String::from_utf8_lossy(&bytes);
    let mut applied = 0usize;
    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key_raw, value_raw)) = line.split_once('=') else {
            log::warn!(
                "Skip invalid env line {}:{} (missing '=')",
                path.display(),
                line_no
            );
            continue;
        };
        let key = key_raw.trim();
        if key.is_empty() {
            continue;
        }
        let mut value = value_raw.trim().to_string();
        if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
            || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
        {
            value = value[1..value.len() - 1].to_string();
        }

        if std::env::var_os(key).is_some() {
            continue;
        }
        std::env::set_var(key, value);
        applied += 1;
    }

    if applied > 0 {
        log::info!("Loaded {} env vars from {}", applied, path.display());
    }
}
