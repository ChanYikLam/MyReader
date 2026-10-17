use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};

use crate::diagnostics;
use crate::file_dialog;
use crate::settings::{self, ThemeMode};

/// 菜单项 id（用于事件分发）。
pub const MENU_FILE_OPEN: &str = "file.open";
pub const MENU_FILE_CLOSE: &str = "file.close";
pub const MENU_THEME_SYSTEM: &str = "view.theme.system";
pub const MENU_THEME_LIGHT: &str = "view.theme.light";
pub const MENU_THEME_DARK: &str = "view.theme.dark";

/// 构建并安装原生菜单（SPEC §12）。
///
/// `theme_mode` 决定「显示 → 外观」菜单里的勾选状态；主题切换后重建菜单
/// 以更新单选标记（macOS 没有原生 radio 组，用 checked 状态模拟）。
///
/// macOS 上第一个子菜单是应用菜单（必须以应用名命名）。
pub fn install(app: &AppHandle, theme_mode: ThemeMode) -> tauri::Result<()> {
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

    // 显示菜单：外观三态（跟随系统 / 浅色 / 深色）
    let theme_system = CheckMenuItem::with_id(
        app,
        MENU_THEME_SYSTEM,
        "跟随系统",
        true,
        theme_mode == ThemeMode::System,
        None::<&str>,
    )?;
    let theme_light = CheckMenuItem::with_id(
        app,
        MENU_THEME_LIGHT,
        "浅色",
        true,
        theme_mode == ThemeMode::Light,
        None::<&str>,
    )?;
    let theme_dark = CheckMenuItem::with_id(
        app,
        MENU_THEME_DARK,
        "深色",
        true,
        theme_mode == ThemeMode::Dark,
        None::<&str>,
    )?;
    let appearance = Submenu::with_items(
        app,
        "外观",
        true,
        &[&theme_system, &theme_light, &theme_dark],
    )?;
    let view_menu = Submenu::with_items(app, "显示", true, &[&appearance])?;

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

    let menu = Menu::with_items(
        app,
        &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu],
    )?;
    app.set_menu(menu)?;

    Ok(())
}

/// 菜单事件分发。
pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_FILE_OPEN => request_open_dialog(app.clone()),
        MENU_FILE_CLOSE => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        MENU_THEME_SYSTEM => apply_theme_mode(app, ThemeMode::System),
        MENU_THEME_LIGHT => apply_theme_mode(app, ThemeMode::Light),
        MENU_THEME_DARK => apply_theme_mode(app, ThemeMode::Dark),
        _ => {}
    }
}

/// 切换主题：持久化 → 原生标题栏 → 通知前端 → 更新菜单勾选。
///
/// 内容区颜色由前端在同一任务内改 `data-theme`（纯 CSS 变量），
/// 原生标题栏由这里同步，避免"内容深了、标题栏还是浅的"。
fn apply_theme_mode(app: &AppHandle, mode: ThemeMode) {
    if let Err(error) = settings::save_theme_mode(app, mode) {
        diagnostics::boot_log(&format!(
            "theme-save-error mode={} detail={}",
            mode.as_str(),
            error
        ));
    }

    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.set_theme(mode.native_theme()) {
            diagnostics::boot_log(&format!(
                "theme-native-error mode={} detail={error}",
                mode.as_str()
            ));
        }
        let _ = window.emit("theme-changed", mode.as_str());
    }

    if let Err(error) = install(app, mode) {
        diagnostics::boot_log(&format!("theme-menu-rebuild-error detail={error}"));
    }
}

/// 在独立线程打开文件对话框（阻塞 API 不能跑在主线程上），
/// 选中后把路径推给前端统一走 open_path 流程（SPEC §12）。
fn request_open_dialog(app: AppHandle) {
    std::thread::spawn(move || match file_dialog::pick_markdown_file(&app) {
        Ok(Some(path)) => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("open-file", path.display().to_string());
            }
        }
        Ok(None) => {}
        Err(error) => {
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(payload) = serde_json::to_value(&error) {
                    let _ = window.emit("open-error", payload);
                }
            }
        }
    });
}
