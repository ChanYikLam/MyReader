import { setWindowTitle } from "./ipc";
import type { AppErrorPayload, DocumentPayload } from "./types";

const APP_NAME = "MyReader";

function requireContentRoot(): HTMLElement {
  const root = document.querySelector<HTMLElement>("#content-root");
  if (root === null) {
    throw new Error("缺少 #content-root 容器");
  }
  return root;
}

/**
 * 渲染文档。
 *
 * `payload.html` 已在 Rust 侧完成 ammonia 白名单净化（SPEC §13），
 * 这里只做一次 DOM 替换；窗口此时仍不可见（reveal 在 boot 流程末尾）。
 */
export function renderDocument(payload: DocumentPayload): void {
  const root = requireContentRoot();

  const article = document.createElement("article");
  article.className = "markdown-body";
  article.innerHTML = payload.html;

  root.replaceChildren(article);
  applyWindowTitle(payload.fileName);
}

/** 无文件启动时的欢迎页（SPEC §18 A2）。 */
export function renderWelcome(): void {
  const root = requireContentRoot();

  const section = document.createElement("section");
  section.className = "welcome";

  const title = document.createElement("h1");
  title.className = "welcome-title";
  title.textContent = APP_NAME;

  const subtitle = document.createElement("p");
  subtitle.className = "welcome-subtitle";
  subtitle.textContent = "一个只读的 Markdown 阅读器";

  const hint = document.createElement("p");
  hint.className = "welcome-hint";
  hint.textContent = "打开一个 Markdown 文件开始阅读";

  section.append(title, subtitle, hint);
  root.replaceChildren(section);
  applyWindowTitle(null);
}

/** 错误页：启动失败/文件打不开时展示，窗口仍会正常显示（SPEC §9.6）。 */
export function renderError(error: AppErrorPayload): void {
  const root = requireContentRoot();

  const section = document.createElement("section");
  section.className = "error-page";

  const title = document.createElement("h1");
  title.textContent = "无法打开文档";

  const message = document.createElement("p");
  message.className = "error-message";
  message.textContent = error.message;

  section.append(title, message);

  if (typeof error.detail === "string" && error.detail.length > 0) {
    const detail = document.createElement("pre");
    detail.className = "error-detail";
    detail.textContent = error.detail;
    section.append(detail);
  }

  root.replaceChildren(section);
  applyWindowTitle(null);
}

function applyWindowTitle(fileName: string | null): void {
  document.title = fileName === null ? APP_NAME : `${fileName} — ${APP_NAME}`;
  void setWindowTitle(fileName ?? APP_NAME).catch((error: unknown) => {
    console.error("设置窗口标题失败", error);
  });
}
