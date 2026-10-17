use std::path::Path;

use serde::Serialize;
use tauri::{Manager, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

use crate::diagnostics;
use crate::document::{self, DocumentPayload};
use crate::error::AppError;
use crate::file_dialog;
use crate::highlight;
use crate::media;
use crate::state::StartupState;
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

    if let Some(document) = initial_document.as_ref() {
        register_image_scopes(&app, document);
    }

    Ok(BootstrapPayload {
        initial_document,
        highlight_css: highlight::theme_css().to_string(),
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

    register_image_scopes(&app, &payload);
    Ok(payload)
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
