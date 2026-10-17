/**
 * 字号控制（SPEC §18 A9）：13–24px，⌘+/⌘-/⌘0 由原生菜单驱动，
 * 设置值由 Rust 持久化，前端只负责应用到 CSS 变量。
 */

import { listen } from "@tauri-apps/api/event";

export const MIN_FONT_SIZE = 13;
export const MAX_FONT_SIZE = 24;
export const DEFAULT_FONT_SIZE = 17;

/** 应用正文字号（钳制到合法区间）。 */
export function applyFontSize(size: number): void {
  const clamped = Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.round(size)));
  document.documentElement.style.setProperty("--content-font-size", `${clamped}px`);
}

/** 监听菜单字号变化事件（启动早期调用）。 */
export function installFontSizeListener(): void {
  void listen<number>("font-size-changed", (event) => {
    applyFontSize(event.payload);
  });
}
