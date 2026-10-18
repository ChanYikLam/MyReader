/**
 * 字号控制（SPEC §18 A9）：13–24px，主修饰键 + = / - / 0 由前端统一处理，
 * 设置值由 Rust 持久化，前端只负责应用到 CSS 变量。
 */

import { listen } from "@tauri-apps/api/event";

import { changeFontSize } from "./ipc";
import { IS_MAC } from "./platform";

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
 * 安装字号快捷键（macOS：⌘= / ⌘+ / ⌘⇧= / 小键盘 + 增大，⌘- 减小，⌘0 复位；
 * Windows：Ctrl 对应组合）。
 *
 * 由前端统一处理而不是菜单 accelerator：既覆盖 Shift 变体/小键盘，
 * 又能 preventDefault 阻止 WebView 的页面缩放（否则与字号调节冲突）。
 */
export function installFontSizeShortcuts(): void {
  window.addEventListener("keydown", (event) => {
    const primaryModifier = IS_MAC ? event.metaKey : event.ctrlKey;
    const secondaryModifier = IS_MAC ? event.ctrlKey : event.metaKey;
    if (!primaryModifier || secondaryModifier || event.altKey) {
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
