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
#[cfg(windows)]
fn main() {
    // 关键：本 crate 既作为独立二进制发布（service 版），也会被桌面端（Tauri）作为依赖引用。
    // 若在依赖构建时也注入 Windows 资源，可能导致链接阶段资源冲突/损坏（例如 LNK1123）。
    // 仅在“主包构建”（`cargo build -p codexmanager-service` / workflow 打包）时才嵌入图标。
    if std::env::var_os("CARGO_PRIMARY_PACKAGE").is_none() {
        return;
    }

    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
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
#[cfg(not(windows))]
fn main() {}
