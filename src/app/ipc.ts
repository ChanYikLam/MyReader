import { invoke } from "@tauri-apps/api/core";

import type { BootstrapPayload, DocumentPayload } from "./types";

/** 主题三态（与 src-tauri/src/boot.rs 的 themeMode 对应）。 */
export type ThemeMode = "system" | "light" | "dark";

/** 记录启动链路计时；失败不影响主流程。 */
export function reportBootEvent(
  event: string,
  elapsedMs: number,
  detail?: string,
): void {
  void invoke("report_boot_event", { event, elapsedMs, detail }).catch(
    (error: unknown) => {
      console.error("report_boot_event 失败", error);
    },
  );
}

/** 获取启动引导数据（首帧需要的一切）。 */
export async function fetchBootstrap(): Promise<BootstrapPayload> {
  return await invoke<BootstrapPayload>("bootstrap");
}

/** 打开指定路径的文档（Rust 侧完成读取/解析/净化）。 */
export async function openPath(path: string): Promise<DocumentPayload> {
  return await invoke<DocumentPayload>("open_path", { path });
}

/** 设置原生窗口标题（macOS 标题栏显示文件名，SPEC §18 A14）。 */
export async function setWindowTitle(title: string): Promise<void> {
  await invoke("set_window_title", { title });
}

/** 显示主窗口：全应用唯一的 show 调用入口（SPEC §9.5）。 */
export async function revealWindow(): Promise<void> {
  await invoke("reveal_window");
}
