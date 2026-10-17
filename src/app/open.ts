import { listen } from "@tauri-apps/api/event";

import { renderDocument, renderError } from "./document";
import { openPath } from "./ipc";
import { toAppErrorPayload } from "./types";

/** 打开并渲染一个文件；失败时显示错误页（旧内容保持可见，SPEC §9.7）。 */
export async function openDocument(path: string): Promise<void> {
  try {
    const payload = await openPath(path);
    renderDocument(payload);
  } catch (error) {
    renderError(toAppErrorPayload(error));
  }
}

let listenerInstalled = false;

/**
 * 监听 Rust 侧 `open-file` 事件（Finder 双击、菜单打开等）。
 *
 * 必须在 bootstrap 之前安装：冷启动时文件关联事件可能先于前端就绪到达；
 * 若事件在前端启动前就已到达，路径会暂存在 Rust 的 StartupState 中，
 * 由 bootstrap 消费（SPEC §12）。
 */
export async function installOpenFileListener(): Promise<void> {
  if (listenerInstalled) {
    return;
  }
  listenerInstalled = true;

  await listen<string>("open-file", (event) => {
    void openDocument(event.payload);
  });
}
