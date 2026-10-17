use std::io::Write;
use std::path::PathBuf;

/// 启动链路诊断日志。
///
/// 始终写到 stderr（从终端直接运行时可见）。写入文件的规则：
/// - 设置了 `MYREADER_BOOT_LOG` 环境变量时，写入该路径；
/// - debug 构建额外默认写入 `~/Library/Logs/MyReader/boot.log`，
///   便于自动化首帧/性能验证在应用由 launchd 启动时收集数据（SPEC §9.9）。
pub fn boot_log(line: &str) {
    eprintln!("[boot] {line}");

    let Some(path) = log_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

fn log_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("MYREADER_BOOT_LOG") {
        return Some(PathBuf::from(path));
    }
    if cfg!(debug_assertions) {
        let home = std::env::var_os("HOME")?;
        return Some(PathBuf::from(home).join("Library/Logs/MyReader/boot.log"));
    }
    None
}
