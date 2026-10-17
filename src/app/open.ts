import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import { renderDocument, renderError } from "./document";
import { prepareImages } from "./images";
import { renderMath, warmUpMathFonts } from "./math";
import { openPath, pickFile } from "./ipc";
import { toAppErrorPayload } from "./types";
import type { AppErrorPayload } from "./types";

/** 文档视图上的「打开文件…」操作（欢迎页/错误页复用）。 */
export function openFileAction(): () => void {
  return () => {
    void promptOpenFile();
  };
}

/** 打开并渲染一个文件；失败时显示错误页（旧内容保持可见，SPEC §9.7）。 */
export async function openDocument(path: string): Promise<void> {
  try {
    const payload = await openPath(path);
    const article = renderDocument(payload);
    // 同步渲染公式：同一任务内替换 DOM，不会出现 LaTeX 原文中间帧。
    const mathStats = renderMath(article);
    if (mathStats.total > 0) {
      void warmUpMathFonts();
    }
    void prepareImages(payload);
  } catch (error) {
    renderError(toAppErrorPayload(error), {
      onOpenFile: openFileAction(),
    });
  }
}

/** 弹出系统文件对话框并打开选中文件。 */
export async function promptOpenFile(): Promise<void> {
  try {
    const path = await pickFile();
    if (path !== null) {
      await openDocument(path);
    }
  } catch (error) {
    renderError(toAppErrorPayload(error), {
      onOpenFile: openFileAction(),
    });
  }
}

let listenersInstalled = false;

/**
 * 安装窗口级事件监听（必须在 bootstrap 之前调用）。
 *
 * - `open-file`：Finder 双击、菜单/`open -a` 打开文件；
 * - `open-error`：菜单打开对话框失败等后端错误。
 *
 * 冷启动时文件关联事件可能先于前端就绪到达：若在前端启动前就已投递，
 * 路径会暂存在 Rust 的 StartupState 中，由 bootstrap 消费（SPEC §12）。
 */
export async function installOpenFileListeners(): Promise<void> {
  if (listenersInstalled) {
    return;
  }
  listenersInstalled = true;

  await listen<string>("open-file", (event) => {
    void openDocument(event.payload);
  });

  await listen<AppErrorPayload>("open-error", (event) => {
    renderError(event.payload, { onOpenFile: openFileAction() });
  });
}

let dragDropInstalled = false;

/**
 * 安装拖拽打开监听：把文件拖到窗口上即打开（多文件取第一个，
 * 需求「打开文件」）。
 *
 * 注：Tauri 的原生拖放事件通过 webview 事件通道投递，
 * 不触发浏览器默认的 HTML5 drop（dragDropEnabled=true）。
 */
export async function installDragDropListener(): Promise<void> {
  if (dragDropInstalled) {
    return;
  }
  dragDropInstalled = true;

  await getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type !== "drop") {
      return;
    }
    const [firstPath] = event.payload.paths;
    if (firstPath !== undefined) {
      void openDocument(firstPath);
    }
  });
}
