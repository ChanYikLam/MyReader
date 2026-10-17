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

/** 弹出系统文件选择对话框；取消时返回 null。 */
export async function pickFile(): Promise<string | null> {
  return await invoke<string | null>("pick_file");
}

/** 用系统默认浏览器/邮件客户端打开外链（Rust 侧做协议白名单校验）。 */
export async function openExternal(url: string): Promise<void> {
  await invoke("open_external", { url });
}

/** 设置原生窗口标题（macOS 标题栏显示文件名，SPEC §18 A14）。 */
export async function setWindowTitle(title: string): Promise<void> {
  await invoke("set_window_title", { title });
}

/**
 * 显示主窗口：全应用唯一的 show 调用入口（SPEC §9.5）。
 *
 * `backgroundTheme` 是前端解析后的 "light" | "dark"：Rust 会在 show 之前
 * 把原生窗口背景色设置成主题底色（深色冷启动不闪白）。
 */
export async function revealWindow(backgroundTheme: string): Promise<void> {
  await invoke("reveal_window", { backgroundTheme });
}

/** 运行期主题切换时同步原生窗口背景色。 */
export async function setWindowBackground(theme: string): Promise<void> {
  await invoke("set_window_background", { theme });
}
