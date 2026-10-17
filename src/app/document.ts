import { setWindowTitle } from "./ipc";
import type { AppErrorPayload, DocumentPayload, RecentFile } from "./types";

const APP_NAME = "MyReader";

/** 视图上可用的操作。 */
export interface DocumentViewActions {
  onOpenFile?: () => void;
  onOpenRecent?: (path: string) => void;
}

function requireContentRoot(): HTMLElement {
  const root = document.querySelector<HTMLElement>("#content-root");
  if (root === null) {
    throw new Error("缺少 #content-root 容器");
  }
  return root;
}

function createOpenFileButton(actions: DocumentViewActions): HTMLElement | null {
  if (actions.onOpenFile === undefined) {
    return null;
  }

  const actionsRow = document.createElement("p");
  actionsRow.className = "view-actions";

  const button = document.createElement("button");
  button.type = "button";
  button.className = "open-file-button";
  button.textContent = "打开文件…";
  button.addEventListener("click", () => {
    actions.onOpenFile?.();
  });

  actionsRow.append(button);
  return actionsRow;
}

/**
 * 渲染文档。
 *
 * `payload.html` 已在 Rust 侧完成 ammonia 白名单净化（SPEC §13），
 * 这里只做一次 DOM 替换；窗口此时仍不可见（reveal 在 boot 流程末尾）。
 *
 * 返回 article 元素，供公式渲染/图片处理阶段复用。
 */
export function renderDocument(payload: DocumentPayload): HTMLElement {
  const root = requireContentRoot();

  const article = document.createElement("article");
  article.className = "markdown-body";
  article.innerHTML = payload.html;

  // 宽表格横向滚动：在构建阶段包一层容器（窗口仍隐藏，无布局闪烁）。
  wrapTables(article);

  root.replaceChildren(article);
  applyWindowTitle(payload.fileName);
  return article;
}

/** 无文件启动时的欢迎页（含最近打开列表，SPEC §18 A2）。 */
export function renderWelcome(
  actions: DocumentViewActions = {},
  recent: RecentFile[] = [],
): void {
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
  hint.textContent = "使用「文件 → 打开…」（⌘O）选择 Markdown 文件";

  section.append(title, subtitle, hint);

  const button = createOpenFileButton(actions);
  if (button !== null) {
    section.append(button);
  }

  const recentSection = createRecentSection(recent, actions);
  if (recentSection !== null) {
    section.append(recentSection);
  }

  root.replaceChildren(section);
  applyWindowTitle(null);
}

/** 最近打开列表（最多 10 条，点击直接打开）。 */
function createRecentSection(
  recent: RecentFile[],
  actions: DocumentViewActions,
): HTMLElement | null {
  if (recent.length === 0 || actions.onOpenRecent === undefined) {
    return null;
  }

  const section = document.createElement("section");
  section.className = "welcome-recent";

  const heading = document.createElement("h2");
  heading.className = "welcome-recent-title";
  heading.textContent = "最近打开";

  const list = document.createElement("ul");
  list.className = "recent-list";

  for (const file of recent) {
    const item = document.createElement("li");
    const button = document.createElement("button");
    button.type = "button";
    button.className = "recent-item";
    button.textContent = file.name;
    button.title = file.path;
    button.addEventListener("click", () => {
      actions.onOpenRecent?.(file.path);
    });
    item.append(button);
    list.append(item);
  }

  section.append(heading, list);
  return section;
}

/** 错误页：启动失败/文件打不开时展示，窗口仍会正常显示（SPEC §9.6）。 */
export function renderError(
  error: AppErrorPayload,
  actions: DocumentViewActions = {},
): void {
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

  const button = createOpenFileButton(actions);
  if (button !== null) {
    section.append(button);
  }

  root.replaceChildren(section);
  applyWindowTitle(null);
}

/** 把表格包进横向滚动容器（需求「表格（带对齐）」）。 */
function wrapTables(article: HTMLElement): void {
  for (const table of Array.from(article.querySelectorAll("table"))) {
    const wrapper = document.createElement("div");
    wrapper.className = "table-scroll";
    table.replaceWith(wrapper);
    wrapper.append(table);
  }
}

function applyWindowTitle(fileName: string | null): void {
  document.title = fileName === null ? APP_NAME : `${fileName} — ${APP_NAME}`;
  void setWindowTitle(fileName ?? APP_NAME).catch((error: unknown) => {
    console.error("设置窗口标题失败", error);
  });
}
