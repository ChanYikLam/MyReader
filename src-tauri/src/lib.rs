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

/// 把「打开文件」请求交给前端；启动期只暂存，不提前 show（SPEC §9.5）。
///
/// 三个来源共用：macOS 的 `RunEvent::Opened`、Windows 的单实例参数转发、
/// 运行期菜单/拖拽。前端在 bootstrap 或 `open-file` 事件里消费。
fn reveal_and_open(app: &tauri::AppHandle, path: std::path::PathBuf) {
    // 先暂存，保证前端启动稍晚也能在 bootstrap 里取到。
    if let Some(startup_state) = app.try_state::<state::StartupState>() {
        startup_state.set_pending_open_path(path.clone());
    }

    if let Some(window) = app.get_webview_window("main") {
        let revealed = app
            .try_state::<state::WindowState>()
            .is_some_and(|window_state| window_state.is_revealed());
        if revealed {
            show_main_window(app);
        }
        let _ = window.emit("open-file", path.display().to_string());
    }
}

/// 显示主窗口，并取消可能排队中的「退出全屏后隐藏」。
///
/// Dock 点击、再次打开文件、单实例转发都走这里：用户重新唤起窗口后，
/// 之前全屏关闭留下的延迟隐藏必须作废，否则窗口会在几秒后自己消失。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(close_state) = app.try_state::<state::FullscreenCloseState>() {
        close_state.cancel();
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 关窗请求的统一落点（红叉 / ⌘W / 菜单「关闭窗口」）。
///
/// 非全屏窗口直接隐藏；macOS 原生全屏窗口必须先退出全屏再隐藏
/// （全屏 Space 里直接 hide 会留下黑屏，见 tauri#12056）。
fn request_hide_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    #[cfg(not(target_os = "macos"))]
    {
        // Windows 的全屏只是无边框铺满屏幕，直接隐藏不会残留黑屏，保持原行为。
        let _ = window.hide();
    }

    #[cfg(target_os = "macos")]
    {
        let Some(close_state) = app.try_state::<state::FullscreenCloseState>() else {
            let _ = window.hide();
            return;
        };

        match close_state.plan(window.is_fullscreen().unwrap_or(false)) {
            state::CloseDisposition::HideNow => {
                let _ = window.hide();
            }
            state::CloseDisposition::ExitFullscreenThenHide => {
                if let Err(error) = window.set_fullscreen(false) {
                    close_state.cancel();
                    diagnostics::boot_log(&format!("fullscreen-close exit-error detail={error}"));
                    return; // 退全屏失败：保持可见，用户可再点一次关闭
                }
                diagnostics::boot_log("fullscreen-close exit-fullscreen-requested");
                schedule_fullscreen_hide(app.clone());
            }
            state::CloseDisposition::AlreadyPending => {
                // 过渡中重复点击关闭：忽略，等延迟隐藏完成。
            }
        }
    }
}

/// 退出全屏后延迟隐藏主窗口（macOS）。
///
/// Tauri 没有暴露「离开全屏」事件，而 `is_fullscreen()` 在请求退全屏时立刻
/// 变为 false，无法作为过渡完成信号（tauri#12056）。macOS 全屏切换动画约
/// 0.5s，这里沿用社区验证过的 2s 延迟；延迟结束后再确认状态，由状态机保证
/// 只隐藏一次，且窗口已被重新唤起时不隐藏。
#[cfg(target_os = "macos")]
fn schedule_fullscreen_hide(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));

        let app_for_main = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(close_state) = app_for_main.try_state::<state::FullscreenCloseState>() else {
                return;
            };
            if !close_state.is_pending() {
                return; // 已被新一次 show / 打开文件取消
            }

            let still_fullscreen = app_for_main
                .get_webview_window("main")
                .is_some_and(|window| window.is_fullscreen().unwrap_or(false));

            if !close_state.settle(still_fullscreen) {
                diagnostics::boot_log(&format!(
                    "fullscreen-close abandoned still_fullscreen={still_fullscreen}"
                ));
                return;
            }

            if let Some(window) = app_for_main.get_webview_window("main") {
                let _ = window.hide();
                diagnostics::boot_log("fullscreen-close hidden-after-exit");
            }
        });
    });
}

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
        // 单实例必须是第一个插件：Windows 双击关联文件会启动新进程，
        // 由它将命令行参数转发给已经运行的窗口。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(path) = state::initial_path_from_args(argv.into_iter().skip(1)) {
                reveal_and_open(app, path);
            } else {
                show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        // 关键：窗口可见性由前端 reveal 门禁唯一控制（SPEC §9.5）。
        // window-state 默认会恢复 visible 并在创建时 show()，绕过门禁导致
        // 首帧先出现空白窗口；这里只恢复尺寸/位置等，不恢复可见性。
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        & !tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .build(),
        )
        // 在 build 阶段注册状态：RunEvent::Opened 可能在 setup 之前到达。
        .manage(startup_state)
        .manage(state::WindowState::default())
        .manage(state::FullscreenCloseState::default())
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
            commands::change_font_size,
            commands::set_theme_mode,
        ])
        .build(tauri::generate_context!())
        .expect("MyReader 初始化失败");

    app.run(|app_handle, event| match event {
        // Cmd+W / 菜单「关闭窗口」/ macOS 红叉：隐藏主窗口而不是销毁，
        // 这样文件关联/再次打开时还能复用同一个窗口与前端状态。
        // 全屏时不能直接隐藏（会残留黑屏 Space），交给 request_hide_main_window。
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == "main" => {
            api.prevent_close();
            request_hide_main_window(app_handle);
        }
        // Finder 双击 / `open -a` 打开文件时 macOS 投递的打开事件。
        tauri::RunEvent::Opened { urls } => {
            let Some(path) = urls.iter().find_map(|url| url.to_file_path().ok()) else {
                return;
            };
            reveal_and_open(app_handle, path);
        }
        // 点击 Dock 图标 / 窗口被隐藏后重新激活。
        tauri::RunEvent::Reopen { .. } => {
            let revealed = app_handle
                .try_state::<state::WindowState>()
                .is_some_and(|window_state| window_state.is_revealed());
            if revealed {
                show_main_window(app_handle);
            }
        }
        _ => {}
    });
}
