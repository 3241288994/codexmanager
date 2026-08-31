use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// 函数 `main`
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
fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    emit_embedded_ui_tracking(&manifest_dir);
    compile_windows_icon(&manifest_dir);
}

/// 函数 `emit_embedded_ui_tracking`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - manifest_dir: 参数 manifest_dir
///
/// # 返回
/// 无
fn emit_embedded_ui_tracking(manifest_dir: &Path) {
    let dist_dir = manifest_dir.join("../../apps/out");
    let embedded_dir =
        PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("codexmanager-web-dist");
    println!("cargo:rerun-if-changed={}", dist_dir.display());

    let fingerprint = if dist_dir.join("index.html").is_file() {
        let fingerprint = fingerprint_tree(&dist_dir);
        if let Err(err) = mirror_tree(&dist_dir, &embedded_dir) {
            panic!(
                "failed to prepare embedded frontend dist from {} to {}: {err}",
                dist_dir.display(),
                embedded_dir.display()
            );
        }
        fingerprint
    } else {
        if embedded_dir.exists() {
            std::fs::remove_dir_all(&embedded_dir).unwrap_or_else(|err| {
                panic!(
                    "failed to clear embedded frontend dist {}: {err}",
                    embedded_dir.display()
                )
            });
        }
        std::fs::create_dir_all(&embedded_dir).unwrap_or_else(|err| {
            panic!(
                "failed to create empty embedded frontend dist {}: {err}",
                embedded_dir.display()
            )
        });
        "missing".to_string()
    };
    println!("cargo:rustc-env=CODEXMANAGER_WEB_DIST_FINGERPRINT={fingerprint}");
}

/// 函数 `fingerprint_tree`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - root: 参数 root
///
/// # 返回
/// 返回函数执行结果
fn fingerprint_tree(root: &Path) -> String {
    let mut pending = VecDeque::from([root.to_path_buf()]);
    let mut items = Vec::new();

    while let Some(dir) = pending.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push_back(path);
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            println!("cargo:rerun-if-changed={}", path.display());
            let modified = metadata
                .modified()
                .ok()
                .and_then(|ts| ts.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|ts| ts.as_secs())
                .unwrap_or_default();
            items.push(format!(
                "{}:{}:{}",
                relative.to_string_lossy().replace('\\', "/"),
                metadata.len(),
                modified
            ));
        }
    }

    items.sort();
    if items.is_empty() {
        "empty".to_string()
    } else {
        items.join("|")
    }
}

/// 函数 `mirror_tree`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-08
///
/// # 参数
/// - source: 前端静态资源目录
/// - target: Cargo 构建输出中的嵌入目录
///
/// # 返回
/// 返回函数执行结果
fn mirror_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    if target.exists() {
        std::fs::remove_dir_all(target)?;
    }
    std::fs::create_dir_all(target)?;

    let mut pending = VecDeque::from([source.to_path_buf()]);
    while let Some(dir) = pending.pop_front() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(source).map_err(std::io::Error::other)?;
            let destination = target.join(relative);
            if path.is_dir() {
                std::fs::create_dir_all(&destination)?;
                pending.push_back(path);
            } else {
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&path, &destination)?;
            }
        }
    }
    Ok(())
}

/// 函数 `compile_windows_icon`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - manifest_dir: 参数 manifest_dir
///
/// # 返回
/// 无
#[cfg(windows)]
fn compile_windows_icon(manifest_dir: &Path) {
    // 仅在主包构建时嵌入图标，避免作为依赖参与其它目标（例如桌面端）链接时引入资源冲突风险。
    if std::env::var_os("CARGO_PRIMARY_PACKAGE").is_none() {
        return;
    }

    let icon_path = manifest_dir.join("../../apps/src-tauri/icons/icon.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());

    if !icon_path.is_file() {
        panic!("Windows icon not found: {}", icon_path.display());
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path.to_string_lossy().as_ref());
    res.compile()
        .expect("failed to compile Windows resources (icon)");
}

/// 函数 `compile_windows_icon`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - _manifest_dir: 参数 _manifest_dir
///
/// # 返回
/// 无
#[cfg(not(windows))]
fn compile_windows_icon(_manifest_dir: &Path) {}
