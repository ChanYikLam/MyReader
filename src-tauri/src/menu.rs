use std::path::Path;

use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};

use crate::diagnostics;
use crate::file_dialog;
use crate::settings::{self, RecentFile, ThemeMode};

/// 菜单项 id（用于事件分发）。
pub const MENU_FILE_OPEN: &str = "file.open";
pub const MENU_FILE_CLOSE: &str = "file.close";
pub const MENU_RECENT_PREFIX: &str = "file.recent.";
pub const MENU_RECENT_CLEAR: &str = "file.recent.clear";
pub const MENU_THEME_SYSTEM: &str = "view.theme.system";
pub const MENU_THEME_LIGHT: &str = "view.theme.light";
pub const MENU_THEME_DARK: &str = "view.theme.dark";
pub const MENU_TOGGLE_SIDEBAR: &str = "view.sidebar.toggle";
pub const MENU_FONT_INCREASE: &str = "view.font.increase";
pub const MENU_FONT_DECREASE: &str = "view.font.decrease";
pub const MENU_FONT_RESET: &str = "view.font.reset";

/// 重新构建菜单（读取当前 store 状态：主题勾选、最近文件）。
pub fn rebuild(app: &AppHandle) {
    let theme_mode = settings::load_theme_mode(app);
    if let Err(error) = install(app, theme_mode) {
        diagnostics::boot_log(&format!("menu-rebuild-error detail={error}"));
    }
}

/// 构建并安装原生菜单（SPEC §12）。
///
/// `theme_mode` 决定「显示 → 外观」的勾选状态；最近文件从 store 读取。
/// macOS 上第一个子菜单是应用菜单（必须以应用名命名）。
pub fn install(app: &AppHandle, theme_mode: ThemeMode) -> tauri::Result<()> {
    let separator = PredefinedMenuItem::separator(app)?;
    let recent = settings::load_recent(app);

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

    // 文件菜单（含动态「打开最近」）
    let open = MenuItem::with_id(app, MENU_FILE_OPEN, "打开…", true, Some("CmdOrCtrl+O"))?;
    let close_window =
        MenuItem::with_id(app, MENU_FILE_CLOSE, "关闭窗口", true, Some("CmdOrCtrl+W"))?;
    let recent_menu = build_recent_menu(app, &recent)?;
    let file_menu = Submenu::with_items(
        app,
        "文件",
        true,
        &[&open, &recent_menu, &separator, &close_window],
    )?;

    // 编辑菜单：只读阅读器也需要复制/全选
    let copy = PredefinedMenuItem::copy(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;
    let edit_menu = Submenu::with_items(app, "编辑", true, &[&copy, &select_all])?;

    // 显示菜单：外观三态 + 侧栏 + 字号
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

    let toggle_sidebar = MenuItem::with_id(
        app,
        MENU_TOGGLE_SIDEBAR,
        "切换侧栏",
        true,
        Some("CmdOrCtrl+B"),
    )?;
    let font_increase = MenuItem::with_id(
        app,
        MENU_FONT_INCREASE,
        "增大字号",
        true,
        Some("CmdOrCtrl+="),
    )?;
    let font_decrease = MenuItem::with_id(
        app,
        MENU_FONT_DECREASE,
        "减小字号",
        true,
        Some("CmdOrCtrl+-"),
    )?;
    let font_reset =
        MenuItem::with_id(app, MENU_FONT_RESET, "实际大小", true, Some("CmdOrCtrl+0"))?;
    let view_menu = Submenu::with_items(
        app,
        "显示",
        true,
        &[
            &appearance,
            &separator,
            &toggle_sidebar,
            &separator,
            &font_increase,
            &font_decrease,
            &font_reset,
        ],
    )?;

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

/// 动态「打开最近」子菜单（最多 10 条 + 清除）。
fn build_recent_menu(app: &AppHandle, recent: &[RecentFile]) -> tauri::Result<Submenu<tauri::Wry>> {
    let separator = PredefinedMenuItem::separator(app)?;
    let clear = MenuItem::with_id(app, MENU_RECENT_CLEAR, "清除最近记录", true, None::<&str>)?;

    if recent.is_empty() {
        let empty = MenuItem::with_id(
            app,
            "file.recent.empty",
            "（暂无最近文件）",
            false,
            None::<&str>,
        )?;
        return Submenu::with_items(app, "打开最近", true, &[&empty, &separator, &clear]);
    }

    let items: Vec<MenuItem<tauri::Wry>> = recent
        .iter()
        .enumerate()
        .map(|(index, file)| {
            MenuItem::with_id(
                app,
                format!("{MENU_RECENT_PREFIX}{index}"),
                recent_label(file),
                true,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;

    let mut refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        Vec::with_capacity(items.len() + 2);
    for item in &items {
        refs.push(item);
    }
    refs.push(&separator);
    refs.push(&clear);

    Submenu::with_items(app, "打开最近", true, &refs)
}

/// 菜单里的最近文件展示文案：文件名 + 父目录名（同名文件可区分）。
fn recent_label(file: &RecentFile) -> String {
    let parent = Path::new(&file.path)
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());

    match parent {
        Some(parent) if !parent.is_empty() => format!("{} — {}", file.name, parent),
        _ => file.name.clone(),
    }
}

/// 菜单事件分发。
pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();

    match id.as_str() {
        MENU_FILE_OPEN => request_open_dialog(app.clone()),
        MENU_FILE_CLOSE => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        MENU_RECENT_CLEAR => {
            if let Err(error) = settings::clear_recent(app) {
                diagnostics::boot_log(&format!("recent-clear-error detail={error}"));
            }
            rebuild(app);
        }
        MENU_THEME_SYSTEM => apply_theme_mode(app, ThemeMode::System),
        MENU_THEME_LIGHT => apply_theme_mode(app, ThemeMode::Light),
        MENU_THEME_DARK => apply_theme_mode(app, ThemeMode::Dark),
        MENU_TOGGLE_SIDEBAR => emit_event(app, "toggle-sidebar", ()),
        MENU_FONT_INCREASE => adjust_font_size(app, 1),
        MENU_FONT_DECREASE => adjust_font_size(app, -1),
        MENU_FONT_RESET => set_font_size(app, settings::DEFAULT_FONT_SIZE),
        _ => {
            if let Some(index) = id.strip_prefix(MENU_RECENT_PREFIX) {
                open_recent_file(app, index);
            }
        }
    }
}

/// 切换主题：持久化 → 原生标题栏 → 通知前端 → 更新菜单勾选。
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

    rebuild(app);
}

fn adjust_font_size(app: &AppHandle, delta: i32) {
    let current = settings::load_font_size(app) as i32;
    let next = settings::clamp_font_size((current + delta).max(0) as u32);
    set_font_size(app, next);
}

fn set_font_size(app: &AppHandle, size: u32) {
    let size = settings::clamp_font_size(size);
    if let Err(error) = settings::save_font_size(app, size) {
        diagnostics::boot_log(&format!("font-size-save-error detail={error}"));
    }
    emit_event(app, "font-size-changed", size);
}

fn emit_event<S: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: S) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(event, payload);
    }
}

fn open_recent_file(app: &AppHandle, index: &str) {
    let Ok(index) = index.parse::<usize>() else {
        return;
    };
    let recent = settings::load_recent(app);
    let Some(file) = recent.get(index) else {
        return;
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("open-file", file.path.clone());
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
