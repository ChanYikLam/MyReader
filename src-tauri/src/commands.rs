use tauri::WebviewWindow;

use crate::diagnostics;

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
