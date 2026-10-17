/** 由 Rust 初始化脚本注入的启动态（见 src-tauri/src/boot.rs）。 */
interface BootState {
  themeMode: "system" | "light" | "dark";
  fontSize: number;
}

interface Window {
  /** 同步注入的启动态，页面任何脚本之前即可读取。 */
  __BOOT__?: BootState;
  /** Rust 进程侧启动时间（Unix ms），用于冷启动耗时统计。 */
  __BOOT_STARTED_AT_MS__?: number;
}
