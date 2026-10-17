pub mod boot;
pub mod commands;
pub mod diagnostics;
pub mod document;
pub mod error;
pub mod file_dialog;
pub mod fs;
pub mod menu;
pub mod parser;
pub mod sanitize;
pub mod state;
#[cfg(test)]
pub mod test_support;

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
        // 在 build 阶段注册状态：RunEvent::Opened 可能在 setup 之前到达。
        .manage(startup_state)
        .setup(move |app| {
            menu::install(app.handle())?;
            app.on_menu_event(menu::handle_event);

            let boot_state = boot::BootState::default();
            boot::create_main_window(app.handle(), &boot_state, started_at_ms)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::open_path,
            commands::pick_file,
            commands::reveal_window,
            commands::report_boot_event,
            commands::set_window_title,
        ])
        .build(tauri::generate_context!())
        .expect("MyReader 初始化失败");

    app.run(|app_handle, event| {
        // Finder 双击 / `open -a` 打开文件时 macOS 投递的打开事件。
        if let tauri::RunEvent::Opened { urls } = event {
            let Some(path) = urls.iter().find_map(|url| url.to_file_path().ok()) else {
                return;
            };

            // 先暂存，保证前端启动稍晚也能在 bootstrap 里取到。
            if let Some(startup_state) = app_handle.try_state::<state::StartupState>() {
                startup_state.set_pending_open_path(path.clone());
            }

            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("open-file", path.display().to_string());
            }
        }
    });
}
