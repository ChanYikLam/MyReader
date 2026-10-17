pub mod boot;
pub mod commands;
pub mod diagnostics;
pub mod document;
pub mod error;
pub mod file_dialog;
pub mod fs;
pub mod highlight;
pub mod media;
pub mod menu;
pub mod parser;
pub mod sanitize;
pub mod settings;
pub mod state;
#[cfg(test)]
pub mod test_support;
pub mod theme;

use tauri::{Emitter, Manager};

/// 把 panic 信息写入启动诊断日志（launchd 启动的应用 stderr 不可见）。
fn install_panic_logger() {
    std::panic::set_hook(Box::new(|info| {
        diagnostics::boot_log(&format!("PANIC: {info}"));
        diagnostics::boot_log(&format!(
            "PANIC-BACKTRACE: {}",
            std::backtrace::Backtrace::force_capture()
        ));
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_logger();

    let started_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();

    let startup_state =
        state::StartupState::new(state::initial_path_from_args(std::env::args().skip(1)));

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // 在 build 阶段注册状态：RunEvent::Opened 可能在 setup 之前到达。
        .manage(startup_state)
        .manage(state::WindowState::default())
        .setup(move |app| {
            highlight::prewarm();

            // 窗口创建前读取持久化主题：菜单勾选 + __BOOT__ 注入都用它，
            // 保证首帧主题与菜单状态一致、无闪变（SPEC §9.2）。
            let theme_mode = settings::load_theme_mode(app.handle());
            menu::install(app.handle(), theme_mode)?;
            app.on_menu_event(menu::handle_event);

            let boot_state = boot::BootState::new(
                theme_mode,
                settings::load_font_size(app.handle()),
                settings::load_sidebar_open(app.handle()),
            );
            boot::create_main_window(app.handle(), &boot_state, started_at_ms)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::open_path,
            commands::pick_file,
            commands::open_external,
            commands::reveal_window,
            commands::report_boot_event,
            commands::set_window_title,
            commands::set_window_background,
            commands::set_sidebar_open,
            commands::save_scroll,
        ])
        .build(tauri::generate_context!())
        .expect("MyReader 初始化失败");

    app.run(|app_handle, event| match event {
        // Cmd+W / 菜单「关闭窗口」：隐藏主窗口而不是销毁，
        // 这样文件关联/再次打开时还能复用同一个窗口与前端状态。
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "main" => {
            api.prevent_close();
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        // Finder 双击 / `open -a` 打开文件时 macOS 投递的打开事件。
        tauri::RunEvent::Opened { urls } => {
            let Some(path) = urls.iter().find_map(|url| url.to_file_path().ok()) else {
                return;
            };

            // 先暂存，保证前端启动稍晚也能在 bootstrap 里取到。
            if let Some(startup_state) = app_handle.try_state::<state::StartupState>() {
                startup_state.set_pending_open_path(path.clone());
            }

            if let Some(window) = app_handle.get_webview_window("main") {
                // 启动阶段（尚未 reveal）不提前显示，避免破坏首帧完整性。
                let revealed = app_handle
                    .try_state::<state::WindowState>()
                    .is_some_and(|window_state| window_state.is_revealed());
                if revealed {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = window.emit("open-file", path.display().to_string());
            }
        }
        // 点击 Dock 图标 / 窗口被隐藏后重新激活。
        tauri::RunEvent::Reopen { .. } => {
            let revealed = app_handle
                .try_state::<state::WindowState>()
                .is_some_and(|window_state| window_state.is_revealed());
            if revealed {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
        _ => {}
    });
}
