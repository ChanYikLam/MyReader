import { listen } from "@tauri-apps/api/event";

import { renderDocument, renderError } from "./document";
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
    renderDocument(payload);
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
