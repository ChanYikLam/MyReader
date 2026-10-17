import { invoke } from "@tauri-apps/api/core";

/** 主题三态（与 src-tauri/src/boot.rs::ThemeMode 对应）。 */
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

/** 显示主窗口：全应用唯一的 show 调用入口（SPEC §9.5）。 */
export async function revealWindow(): Promise<void> {
  await invoke("reveal_window");
}
