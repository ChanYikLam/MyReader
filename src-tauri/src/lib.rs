pub mod boot;
pub mod commands;
pub mod diagnostics;
pub mod error;
pub mod fs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();

    tauri::Builder::default()
        .setup(move |app| {
            let boot_state = boot::BootState::default();
            boot::create_main_window(app.handle(), &boot_state, started_at_ms)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::reveal_window,
            commands::report_boot_event,
        ])
        .run(tauri::generate_context!())
        .expect("MyReader 启动失败");
}
