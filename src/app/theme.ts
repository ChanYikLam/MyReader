/**
 * 主题三态控制（SPEC §10）。
 *
 * 首帧主题由 index.html 内联脚本 + `__BOOT__` 同步决定，本模块负责运行期：
 * - 菜单切换（Rust 发 `theme-changed` 事件）；
 * - 跟随系统时监听 `prefers-color-scheme` 实时切换。
 *
 * 只改 `<html data-theme>` 与 CSS 变量，颜色切换是纯 CSS，没有时序窗口。
 */

import { listen } from "@tauri-apps/api/event";

import { setWindowBackground } from "./ipc";
import type { ThemeMode } from "./ipc";

const DARK_MEDIA_QUERY = "(prefers-color-scheme: dark)";

let currentMode: ThemeMode = readInitialMode();

/** 应用主题模式（system 时解析系统外观）。 */
export function applyThemeMode(mode: ThemeMode): void {
  currentMode = mode;

  const dark =
    mode === "dark" ||
    (mode === "system" && window.matchMedia(DARK_MEDIA_QUERY).matches);

  const root = document.documentElement;
  root.dataset.theme = dark ? "dark" : "light";
  root.dataset.themeMode = mode;
  root.style.colorScheme = dark ? "dark" : "light";

  // 原生窗口背景同步（窗口可见时避免边缘/过渡露出白底）。
  void setWindowBackground(dark ? "dark" : "light").catch((error: unknown) => {
    console.error("同步窗口背景色失败", error);
  });
}

/** 安装运行期主题监听（菜单切换 + 系统外观变化）。 */
export function installThemeListener(): void {
  const media = window.matchMedia(DARK_MEDIA_QUERY);
  media.addEventListener("change", () => {
    // 手动浅/深色不受系统外观影响（验收 11）。
    if (currentMode === "system") {
      applyThemeMode("system");
    }
  });

  void listen<string>("theme-changed", (event) => {
    const mode = normalizeThemeMode(event.payload);
    if (mode !== null) {
      applyThemeMode(mode);
    }
  });
}

function readInitialMode(): ThemeMode {
  return normalizeThemeMode(document.documentElement.dataset.themeMode) ?? "system";
}

function normalizeThemeMode(value: unknown): ThemeMode | null {
  return value === "system" || value === "light" || value === "dark" ? value : null;
}
