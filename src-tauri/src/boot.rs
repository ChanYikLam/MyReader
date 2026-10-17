use serde::Serialize;
use tauri::{AppHandle, WebviewWindowBuilder};

/// 前端在首帧前就需要的启动状态。
///
/// 由 Rust 在创建隐藏窗口时通过 `initialization_script` 同步注入到
/// `window.__BOOT__`（见 SPEC §9.1）。M4 起这些字段会来自 store 持久化数据，
/// 当前为默认值。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootState {
    /// 主题模式："system" | "light" | "dark"。
    /// M4 接入 store 持久化后改为强类型枚举（见 SPEC §10/§11）。
    pub theme_mode: &'static str,
    pub font_size: u32,
}

impl Default for BootState {
    fn default() -> Self {
        Self {
            theme_mode: "system",
            font_size: 17,
        }
    }
}

/// 按 `tauri.conf.json` 的主窗口配置创建隐藏窗口，并注入启动态。
///
/// 窗口配置 `create: false` + `visible: false`：窗口由本函数手动创建，
/// 在 frontend 调用 `reveal_window` 之前不可见（SPEC §9.1/§9.5）。
pub fn create_main_window(
    app: &AppHandle,
    state: &BootState,
    started_at_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let window_config = app
        .config()
        .app
        .windows
        .first()
        .cloned()
        .ok_or("tauri.conf.json 缺少主窗口配置")?;

    // 该脚本在页面任何脚本之前执行（WKUserScript documentStart），
    // 保证内联主题脚本与后续模块脚本都能同步读到启动态。
    let init_script = format!(
        "window.__BOOT__ = {}; window.__BOOT_STARTED_AT_MS__ = {};",
        serde_json::to_string(state)?,
        started_at_ms
    );

    let window = WebviewWindowBuilder::from_config(app, &window_config)?
        .initialization_script(init_script)
        .build()?;

    // 诊断：确认窗口按配置尺寸创建（首帧机制调试用，见 SPEC §9.9）。
    crate::diagnostics::boot_log(&format!(
        "window-created inner_size={:?} outer_size={:?} scale_factor={:?} visible={:?}",
        window.inner_size(),
        window.outer_size(),
        window.scale_factor(),
        window.is_visible(),
    ));

    Ok(())
}
