/**
 * 平台判断与快捷键文案（macOS / Windows 桌面端）。
 *
 * Tauri 的 WebView UA：macOS 含 "Macintosh"，Windows 含 "Windows NT"。
 * 只区分主修饰键，不需要完整的平台枚举。
 */

export const IS_MAC = /Macintosh|Mac OS X/i.test(navigator.userAgent);

/** 主修饰键符号：macOS 用 ⌘，其它平台用 Ctrl。 */
export const MODIFIER_LABEL = IS_MAC ? "⌘" : "Ctrl";

/** 快捷键提示文案：macOS 为 ⌘O，Windows 为 Ctrl+O。 */
export function shortcutHint(key: string): string {
  return IS_MAC ? `⌘${key}` : `Ctrl+${key}`;
}
