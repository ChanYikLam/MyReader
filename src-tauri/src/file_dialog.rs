use std::path::PathBuf;

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::error::AppError;

/// 打开系统文件选择对话框，返回选中的文件路径（取消返回 None）。
///
/// 注意：`blocking_pick_file` 内部阻塞调用线程，必须在非主线程执行
/// （命令层用 `spawn_blocking`，菜单事件里用独立线程）。
pub fn pick_markdown_file(app: &AppHandle) -> Result<Option<PathBuf>, AppError> {
    let picked = app
        .dialog()
        .file()
        .set_title("打开 Markdown 文件")
        .add_filter("Markdown", &["md", "markdown", "mdown"])
        .add_filter("纯文本", &["txt"])
        .blocking_pick_file();

    let Some(file_path) = picked else {
        return Ok(None);
    };

    let path = file_path.into_path().map_err(|error| AppError::Dialog {
        detail: error.to_string(),
    })?;

    Ok(Some(path))
}
