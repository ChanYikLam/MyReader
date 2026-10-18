use std::path::Path;

use serde::Serialize;
use tauri::{Emitter, Manager, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

use crate::diagnostics;
use crate::document::{self, DocumentPayload};
use crate::error::AppError;
use crate::file_dialog;
use crate::highlight;
use crate::media;
use crate::settings::{self, RecentFile};
use crate::state::{StartupState, WindowState};
use crate::theme;

/// 启动引导载荷：命令启动时前端需要的初始数据。
///
/// 命令行参数 / Finder 文件关联在启动期解析为 `initial_document`；
/// 无文件时返回 None（前端显示欢迎页）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub initial_document: Option<DocumentPayload>,
    /// 明暗两套代码高亮 CSS（按 data-theme 作用域），首帧前注入 <head>。
    pub highlight_css: String,
    /// 最近打开列表（欢迎页展示，最多 10 条）。
    pub recent: Vec<RecentFile>,
}

/// 前端启动时调用一次，拿到首帧所需的全部数据。
///
/// 若启动时带文件（命令行参数 / 后续的文件关联），在此完成读取与解析；
/// 失败时返回 AppError，由前端的错误页承载（SPEC §9.6）。
#[tauri::command]
pub async fn bootstrap(
    app: tauri::AppHandle,
    state: tauri::State<'_, StartupState>,
) -> Result<BootstrapPayload, AppError> {
    let initial_path = state.take_pending_open_path();

    let initial_document = tauri::async_runtime::spawn_blocking(move || {
        initial_path
            .as_deref()
            .map(document::load_document)
            .transpose()
    })
    .await
    .map_err(|error| AppError::Internal {
        detail: error.to_string(),
    })??;

    let mut initial_document = initial_document;
    if let Some(document) = initial_document.as_mut() {
        finish_document_load(&app, document);
    }

    Ok(BootstrapPayload {
        initial_document,
        highlight_css: highlight::theme_css().to_string(),
        recent: settings::load_recent(&app),
    })
}

/// 打开指定路径的文档：读取/解析/净化在阻塞线程池执行，避免卡住 UI。
#[tauri::command]
pub async fn open_path(app: tauri::AppHandle, path: String) -> Result<DocumentPayload, AppError> {
    let payload =
        tauri::async_runtime::spawn_blocking(move || document::load_document(Path::new(&path)))
            .await
            .map_err(|error| AppError::Internal {
                detail: error.to_string(),
            })??;

    let mut payload = payload;
    finish_document_load(&app, &mut payload);
    Ok(payload)
}

/// 文档加载成功后的收尾：滚动位置记忆、最近列表、图片 asset scope。
fn finish_document_load(app: &tauri::AppHandle, document: &mut DocumentPayload) {
    document.saved_scroll = settings::load_scroll(app, &document.path);
    match settings::push_recent(app, &document.path, &document.file_name) {
        Ok(_) => crate::menu::rebuild(app),
        Err(error) => diagnostics::boot_log(&format!("recent-save-error detail={error}")),
    }
    register_image_scopes(app, document);
}

/// 切换主题三态（UI 按钮调用）：持久化 → 原生标题栏 → 广播 → 菜单勾选同步。
#[tauri::command]
pub fn set_theme_mode(
    app: tauri::AppHandle,
    window: WebviewWindow,
    mode: String,
) -> Result<(), AppError> {
    let theme_mode = settings::ThemeMode::parse(&mode).ok_or_else(|| AppError::Internal {
        detail: format!("未知主题模式：{mode}"),
    })?;

    settings::save_theme_mode(&app, theme_mode)?;
    if let Err(error) = window.set_theme(theme_mode.native_theme()) {
        diagnostics::boot_log(&format!(
            "theme-native-error mode={} detail={error}",
            theme_mode.as_str()
        ));
    }
    let _ = window.emit("theme-changed", theme_mode.as_str());
    crate::menu::rebuild(&app);
    Ok(())
}

/// 菜单/键盘快捷键共用的字号调整：delta=0 表示恢复默认字号。
///
/// 返回并广播新字号，前端只负责应用到 CSS 变量（持久化在 Rust 侧）。
#[tauri::command]
pub fn change_font_size(app: tauri::AppHandle, delta: i32) -> Result<u32, AppError> {
    let current = settings::load_font_size(&app) as i32;
    let next = if delta == 0 {
        settings::DEFAULT_FONT_SIZE
    } else {
        settings::clamp_font_size((current + delta).max(1) as u32)
    };

    settings::save_font_size(&app, next)?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("font-size-changed", next);
    }
    Ok(next)
}

/// 侧栏开合持久化（首帧之外的用户操作由前端调用）。
#[tauri::command]
pub fn set_sidebar_open(app: tauri::AppHandle, open: bool) -> Result<(), AppError> {
    settings::save_sidebar_open(&app, open)
}

/// 保存当前文档滚动位置（前端滚动 debounce 后调用）。
#[tauri::command]
pub fn save_scroll(
    app: tauri::AppHandle,
    path: String,
    top: f64,
    doc_height: f64,
    viewport_width: f64,
) -> Result<(), AppError> {
    settings::save_scroll(
        &app,
        &path,
        settings::ScrollPosition {
            top,
            doc_height,
            viewport_width,
            updated_at: 0,
        },
    )
}

/// 把本地图片逐个加入 asset 协议作用域（运行期动态放行，不整盘开放）。
fn register_image_scopes(app: &tauri::AppHandle, document: &DocumentPayload) {
    let scope = app.asset_protocol_scope();
    for path in media::local_paths(&document.images) {
        let _ = scope.allow_file(path);
    }
}

/// 弹出系统文件对话框，返回选中的路径；取消时返回 None。
#[tauri::command]
pub async fn pick_file(app: tauri::AppHandle) -> Result<Option<String>, AppError> {
    let picked =
        tauri::async_runtime::spawn_blocking(move || file_dialog::pick_markdown_file(&app))
            .await
            .map_err(|error| AppError::Internal {
                detail: error.to_string(),
            })??;

    Ok(picked.map(|path| path.display().to_string()))
}

/// 显示主窗口。前端 `reveal.ts` 是唯一调用点（SPEC §9.5）。
///
/// `background_theme` 是前端解析后的 "light" | "dark"：show 之前先设置
/// 原生窗口背景色，深色冷启动时任何过渡帧都不会是系统白底。
#[tauri::command]
pub fn reveal_window(
    app: tauri::AppHandle,
    window: WebviewWindow,
    background_theme: Option<String>,
) -> Result<(), String> {
    if let Some(theme_name) = background_theme.as_deref() {
        if let Some(color) = theme::background_color(theme_name) {
            let _ = window.set_background_color(Some(color));
        }
    }

    window
        .show()
        .map_err(|error| format!("显示主窗口失败：{error}"))?;
    let _ = window.set_focus();
    if let Some(window_state) = app.try_state::<WindowState>() {
        window_state.mark_revealed();
    }
    diagnostics::boot_log(&format!(
        "reveal_window shown inner_size={:?} outer_size={:?}",
        window.inner_size(),
        window.outer_size(),
    ));
    Ok(())
}

/// 用系统默认浏览器/邮件客户端打开外部链接。
///
/// 协议白名单：仅 http/https/mailto（SPEC §13）；其余一律拒绝，
/// 前端 `links.ts` 也会在点击时做同样的拦截。
#[tauri::command]
pub fn open_external(app: tauri::AppHandle, url: String) -> Result<(), AppError> {
    let scheme = url
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !matches!(scheme.as_str(), "http" | "https" | "mailto") {
        return Err(AppError::BlockedUrl { url });
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| AppError::Internal {
            detail: error.to_string(),
        })
}

/// 运行期主题切换时同步原生窗口背景色（窗口已可见，避免露出白底）。
#[tauri::command]
pub fn set_window_background(window: WebviewWindow, theme: String) -> Result<(), String> {
    let color =
        crate::theme::background_color(&theme).ok_or_else(|| format!("未知主题：{theme}"))?;
    window
        .set_background_color(Some(color))
        .map_err(|error| format!("设置窗口背景色失败：{error}"))
}

/// 设置原生窗口标题（macOS 标题栏显示当前文件名，SPEC §18 A14）。
#[tauri::command]
pub fn set_window_title(window: WebviewWindow, title: String) -> Result<(), String> {
    window
        .set_title(&title)
        .map_err(|error| format!("设置窗口标题失败：{error}"))
}

/// 记录一条启动链路计时（前端在关键节点调用）。
#[tauri::command]
pub fn report_boot_event(event: String, elapsed_ms: f64, detail: Option<String>) {
    diagnostics::boot_log(&format!(
        "{elapsed_ms:9.2}ms {event} {}",
        detail.unwrap_or_default()
    ));
}
