use serde::Serialize;
use tauri::{AppHandle, WebviewWindowBuilder};

use crate::settings::{self, ThemeMode};

/// 前端在首帧前就需要的启动状态。
///
/// 由 Rust 在创建隐藏窗口时通过 `initialization_script` 同步注入到
/// `window.__BOOT__`（见 SPEC §9.1）。M4 起这些字段会来自 store 持久化数据，
/// 当前为默认值。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootState {
    /// 主题模式：system / light / dark（来自 store，缺失时 system）。
    pub theme_mode: ThemeMode,
    /// 正文字号（px，13–24，SPEC §18 A9）。
    pub font_size: u32,
    /// 侧栏开合；None = 用户未手动设置，前端按窗口宽度决定默认值。
    pub sidebar_open: Option<bool>,
}

impl BootState {
    pub fn new(theme_mode: ThemeMode, font_size: u32, sidebar_open: Option<bool>) -> Self {
        Self {
            theme_mode,
            font_size: settings::clamp_font_size(font_size),
            sidebar_open,
        }
    }
}

impl Default for BootState {
    fn default() -> Self {
        Self::new(ThemeMode::System, settings::DEFAULT_FONT_SIZE, None)
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
        // 原生标题栏主题：手动浅/深色立即生效；跟随系统时交给 macOS。
        .theme(state.theme_mode.native_theme())
        // 导航守卫：应用内只允许本地页面；外链一律由前端拦截后交给
        // 系统浏览器，绝不在应用内跳转（SPEC §12/§13）。
        //
        // 平台差异：macOS 的应用页面走 `tauri://localhost`，Windows 的
        // WebView2 把自定义协议映射为 `http://tauri.localhost`（见
        // tauri::protocol::origin）。只放行 localhost / 127.0.0.1 会把
        // Windows 的首次加载也拦掉，表现为黑屏。
        .on_navigation(|url| match url.scheme() {
            "tauri" => true,
            "http" | "https" => matches!(
                url.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost")
            ),
            _ => false,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 锁定注入前端的 JSON 契约（index.html 内联脚本读取 themeMode/fontSize）。
    #[test]
    fn boot_state_serializes_for_frontend() {
        let value = serde_json::to_value(BootState::new(ThemeMode::Dark, 19, Some(true)))
            .expect("可序列化");
        assert_eq!(value["themeMode"], "dark");
        assert_eq!(value["fontSize"], 19);
        assert_eq!(value["sidebarOpen"], true);

        let default_value = serde_json::to_value(BootState::default()).expect("可序列化");
        assert_eq!(default_value["themeMode"], "system");
    }
}
