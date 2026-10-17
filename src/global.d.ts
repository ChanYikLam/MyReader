/** 由 Rust 初始化脚本注入的启动态（见 src-tauri/src/boot.rs）。 */
interface BootState {
  themeMode: "system" | "light" | "dark";
  fontSize: number;
  /** 侧栏开合；null/undefined = 未手动设置，按窗口宽度决定默认值。 */
  sidebarOpen?: boolean | null;
}

interface Window {
  /** 同步注入的启动态，页面任何脚本之前即可读取。 */
  __BOOT__?: BootState;
  /** Rust 进程侧启动时间（Unix ms），用于冷启动耗时统计。 */
  __BOOT_STARTED_AT_MS__?: number;
}
