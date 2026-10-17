use std::path::Path;

use serde::Serialize;
use tauri::WebviewWindow;

use crate::diagnostics;
use crate::document::{self, DocumentPayload};
use crate::error::AppError;

/// 启动引导载荷：命令启动时前端需要的初始数据。
///
/// 命令行参数 / Finder 文件关联 / 最近文件在 M5–M6 接入，
/// 当前阶段无初始文档（显示欢迎页）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPayload {
    pub initial_document: Option<DocumentPayload>,
}

/// 前端启动时调用一次，拿到首帧所需的全部数据。
#[tauri::command]
pub fn bootstrap() -> BootstrapPayload {
    BootstrapPayload {
        initial_document: None,
    }
}

/// 打开指定路径的文档：读取/解析/净化在阻塞线程池执行，避免卡住 UI。
#[tauri::command]
pub async fn open_path(path: String) -> Result<DocumentPayload, AppError> {
    tauri::async_runtime::spawn_blocking(move || document::load_document(Path::new(&path)))
        .await
        .map_err(|error| AppError::Internal {
            detail: error.to_string(),
        })?
}

/// 显示主窗口。前端 `reveal.ts` 是唯一调用点（SPEC §9.5）。
#[tauri::command]
pub fn reveal_window(window: WebviewWindow) -> Result<(), String> {
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

/// 记录一条启动链路计时（前端在关键节点调用）。
#[tauri::command]
pub fn report_boot_event(event: String, elapsed_ms: f64, detail: Option<String>) {
    diagnostics::boot_log(&format!(
        "{elapsed_ms:9.2}ms {event} {}",
        detail.unwrap_or_default()
    ));
}
