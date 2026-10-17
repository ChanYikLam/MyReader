use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter};

use crate::file_dialog;

/// 菜单项 id（用于事件分发）。
pub const MENU_FILE_OPEN: &str = "file.open";
pub const MENU_FILE_CLOSE: &str = "file.close";

/// 构建并安装原生菜单（SPEC §12）。
///
/// macOS 上第一个子菜单是应用菜单（必须以应用名命名）。
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let separator = PredefinedMenuItem::separator(app)?;

    // 应用菜单
    let about = PredefinedMenuItem::about(app, Some("关于 MyReader"), None)?;
    let hide = PredefinedMenuItem::hide(app, None)?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let show_all = PredefinedMenuItem::show_all(app, None)?;
    let quit = PredefinedMenuItem::quit(app, Some("退出 MyReader"))?;
    let app_menu = Submenu::with_items(
        app,
        "MyReader",
        true,
        &[
            &about,
            &separator,
            &hide,
            &hide_others,
            &show_all,
            &separator,
            &quit,
        ],
    )?;

    // 文件菜单
    let open = MenuItem::with_id(app, MENU_FILE_OPEN, "打开…", true, Some("CmdOrCtrl+O"))?;
    let close_window =
        MenuItem::with_id(app, MENU_FILE_CLOSE, "关闭窗口", true, Some("CmdOrCtrl+W"))?;
    let file_menu = Submenu::with_items(app, "文件", true, &[&open, &separator, &close_window])?;

    // 编辑菜单：只读阅读器也需要复制/全选
    let copy = PredefinedMenuItem::copy(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;
    let edit_menu = Submenu::with_items(app, "编辑", true, &[&copy, &select_all])?;

    // 窗口菜单
    let minimize = PredefinedMenuItem::minimize(app, None)?;
    let zoom = PredefinedMenuItem::maximize(app, None)?;
    let bring_all_to_front = PredefinedMenuItem::bring_all_to_front(app, None)?;
    let window_menu = Submenu::with_items(
        app,
        "窗口",
        true,
        &[&minimize, &zoom, &separator, &bring_all_to_front],
    )?;

    let menu = Menu::with_items(app, &[&app_menu, &file_menu, &edit_menu, &window_menu])?;
    app.set_menu(menu)?;

    Ok(())
}

/// 菜单事件分发。
pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_FILE_OPEN => request_open_dialog(app.clone()),
        MENU_FILE_CLOSE => {
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                let _ = window.close();
            }
        }
        _ => {}
    }
}

/// 在独立线程打开文件对话框（阻塞 API 不能跑在主线程上），
/// 选中后把路径推给前端统一走 open_path 流程（SPEC §12）。
fn request_open_dialog(app: AppHandle) {
    std::thread::spawn(move || match file_dialog::pick_markdown_file(&app) {
        Ok(Some(path)) => {
            if let Some(window) = tauri::Manager::get_webview_window(&app, "main") {
                let _ = window.emit("open-file", path.display().to_string());
            }
        }
        Ok(None) => {}
        Err(error) => {
            if let Some(window) = tauri::Manager::get_webview_window(&app, "main") {
                if let Ok(payload) = serde_json::to_value(&error) {
                    let _ = window.emit("open-error", payload);
                }
            }
        }
    });
}
