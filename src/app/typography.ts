/**
 * 字号控制（SPEC §18 A9）：13–24px，⌘+/⌘-/⌘0 由原生菜单驱动，
 * 设置值由 Rust 持久化，前端只负责应用到 CSS 变量。
 */

import { listen } from "@tauri-apps/api/event";

import { changeFontSize } from "./ipc";

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

/**
 * 安装 Cmd 字号快捷键（⌘= / ⌘+ / ⌘⇧= / 小键盘 + 增大，⌘- 减小，⌘0 复位）。
 *
 * 由前端统一处理而不是菜单 accelerator：既覆盖 Shift 变体/小键盘，
 * 又能 preventDefault 阻止 WebKit 的页面缩放（否则与字号调节冲突）。
 */
export function installFontSizeShortcuts(): void {
  window.addEventListener("keydown", (event) => {
    if (!event.metaKey || event.ctrlKey || event.altKey) {
      return;
    }

    if (event.key === "0" || event.code === "Numpad0") {
      event.preventDefault();
      void changeFontSize(0);
      return;
    }

    const increase =
      event.key === "=" ||
      event.key === "+" ||
      event.code === "NumpadAdd";
    if (increase) {
      event.preventDefault();
      void changeFontSize(1);
      return;
    }

    if (event.key === "-" || event.code === "NumpadSubtract") {
      event.preventDefault();
      void changeFontSize(-1);
    }
  });
}
