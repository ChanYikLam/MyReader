/**
 * 滚动位置记忆（SPEC §9.3 第 6 项 / §11）。
 *
 * 首帧要求：窗口出现时就在记忆位置，不能从顶部滚动过去。
 * 因此恢复必须在 reveal 前、且所有影响布局的资源（公式/图片/字体）就绪后
 * 用同步 `scrollTo` 完成，不使用平滑动画。
 */

import { saveScroll } from "./ipc";
import type { DocumentPayload } from "./types";

const SAVE_DEBOUNCE_MS = 250;

let currentPath: string | null = null;
let removeScrollListener: (() => void) | null = null;
let saveTimer: number | null = null;

/** 文档加载完成、reveal 之前调用：同步恢复记忆位置。 */
export function restoreScrollForDocument(payload: DocumentPayload): void {
  const saved = payload.savedScroll;

  // 下一帧起保存新文档的滚动（避免恢复动作本身触发一次保存）。
  currentPath = payload.path;
  if (saved === null || saved.top <= 0) {
    return;
  }

  const target = resolveScrollTarget(saved);
  applyScrollTarget(target);
}

/**
 * 安装滚动保存监听（窗口可见后调用）。
 * 节流 250ms 写入 store；切换文件/隐藏窗口时立即补一次。
 */
export function installScrollSaver(payload: DocumentPayload): void {
  removeScrollListener?.();
  removeScrollListener = null;
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
  }

  const onScroll = (): void => {
    if (saveTimer !== null) {
      window.clearTimeout(saveTimer);
    }
    saveTimer = window.setTimeout(() => {
      saveTimer = null;
      persistCurrentScroll();
    }, SAVE_DEBOUNCE_MS);
  };

  const onHidden = (): void => {
    if (document.visibilityState === "hidden") {
      persistCurrentScroll();
    }
  };

  window.addEventListener("scroll", onScroll, { passive: true });
  document.addEventListener("visibilitychange", onHidden);

  removeScrollListener = () => {
    window.removeEventListener("scroll", onScroll);
    document.removeEventListener("visibilitychange", onHidden);
  };

  void payload;
}

/** 保存当前文档滚动位置（切换文件前调用，避免丢失）。 */
export function saveCurrentScrollNow(): void {
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
  }
  persistCurrentScroll();
}

function persistCurrentScroll(): void {
  if (currentPath === null) {
    return;
  }
  void saveScroll(
    currentPath,
    Math.round(window.scrollY * 100) / 100,
    document.documentElement.scrollHeight,
    window.innerWidth,
  ).catch((error: unknown) => {
    console.error("保存滚动位置失败", error);
  });
}

/**
 * 解析恢复目标：
 * 窗口宽度或文档高度明显变化（字号/窗口尺寸变化）时按比例回退，
 * 否则精确恢复像素位置。
 */
function resolveScrollTarget(saved: NonNullable<DocumentPayload["savedScroll"]>): number {
  const docHeight = document.documentElement.scrollHeight;
  const widthChanged = Math.abs(window.innerWidth - saved.viewportWidth) > 2;
  const heightChanged =
    saved.docHeight > 0 &&
    Math.abs(docHeight - saved.docHeight) / saved.docHeight > 0.02;

  if ((widthChanged || heightChanged) && saved.docHeight > 0) {
    return Math.max(0, (saved.top / saved.docHeight) * docHeight);
  }

  return Math.max(0, saved.top);
}

function applyScrollTarget(target: number): void {
  window.scrollTo(0, target);

  // 被文档高度 clamp 时（例如仍矮于记忆位置）按当前高度再对齐一次。
  if (Math.abs(window.scrollY - target) > 1) {
    const docHeight = document.documentElement.scrollHeight;
    window.scrollTo(0, Math.max(0, Math.min(target, docHeight)));
  }
}
