/**
 * 目录侧栏（SPEC §12 阅读体验）。
 *
 * 大纲来自 Rust 侧 payload.headings（与标题 id 完全一致）；
 * 首帧前完成 DOM 构建与开合状态应用（index.html 内联脚本设置 data-sidebar），
 * 之后的开合/跳转都是用户操作，不产生首帧布局跳动。
 */

import { listen } from "@tauri-apps/api/event";

import { setSidebarOpen } from "./ipc";
import type { DocumentPayload, Heading } from "./types";

/** 大纲只收 h1–h3（SPEC §18 A3/A15）。 */
const MAX_OUTLINE_LEVEL = 3;

const SIDEBAR_ID = "outline-sidebar";
const TOGGLE_ID = "sidebar-toggle";

let activeObserver: IntersectionObserver | null = null;
let outlineItems = new Map<string, HTMLElement>();
let sidebarInstalled = false;
/** 用户侧栏偏好：null = 未设置（按窗口宽度决定默认值）。 */
let preferredOpen: boolean | null = window.__BOOT__?.sidebarOpen ?? null;

/** 构建/重建侧栏（文档加载后调用；欢迎页/错误页调用 clearSidebar）。 */
export function buildSidebar(payload: DocumentPayload): void {
  const app = requireApp();
  clearSidebar();

  const headings = payload.headings.filter(
    (heading) => heading.level <= MAX_OUTLINE_LEVEL,
  );

  const sidebar = document.createElement("aside");
  sidebar.id = SIDEBAR_ID;
  sidebar.className = "sidebar";
  sidebar.setAttribute("aria-label", "目录");

  const header = document.createElement("div");
  header.className = "sidebar-header";

  const title = document.createElement("span");
  title.className = "sidebar-title";
  title.textContent = "目录";

  const collapse = document.createElement("button");
  collapse.type = "button";
  collapse.className = "sidebar-collapse";
  collapse.textContent = "‹";
  collapse.title = "收起侧栏（⌘B）";
  collapse.addEventListener("click", () => setSidebarVisible(false));

  header.append(title, collapse);

  const nav = document.createElement("nav");
  nav.className = "outline";
  if (headings.length === 0) {
    nav.classList.add("outline-empty");
    nav.textContent = "（无标题）";
  }

  outlineItems = new Map();
  for (const heading of headings) {
    const item = createOutlineItem(heading);
    nav.append(item);
    outlineItems.set(heading.id, item);
  }

  sidebar.append(header, nav);

  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.id = TOGGLE_ID;
  toggle.className = "sidebar-toggle";
  toggle.textContent = "☰";
  toggle.title = "显示/隐藏目录（⌘B）";
  toggle.addEventListener("click", () => toggleSidebar());

  app.prepend(sidebar);
  app.append(toggle);

  applyInitialVisibility();
  installActiveHeadingObserver(headings);
}

/** 移除侧栏与按钮（欢迎页/错误页使用）。 */
export function clearSidebar(): void {
  activeObserver?.disconnect();
  activeObserver = null;
  outlineItems = new Map();

  document.getElementById(SIDEBAR_ID)?.remove();
  document.getElementById(TOGGLE_ID)?.remove();
  document.documentElement.dataset.sidebar = "closed";
}

/** 切换侧栏并持久化（菜单 ⌘B / 悬浮按钮共用）。 */
export function toggleSidebar(): void {
  const open = document.documentElement.dataset.sidebar !== "open";
  setSidebarVisible(open);
}

/** 安装菜单「切换侧栏」事件监听（启动早期调用）。 */
export function installSidebarListener(): void {
  if (sidebarInstalled) {
    return;
  }
  sidebarInstalled = true;

  void listen("toggle-sidebar", () => {
    toggleSidebar();
  });
}

/** 首帧/换文件后的侧栏可见性：用户偏好优先，否则按窗口宽度决定。 */
function applyInitialVisibility(): void {
  const open = preferredOpen ?? window.innerWidth >= 900;
  document.documentElement.dataset.sidebar = open ? "open" : "closed";
}

function setSidebarVisible(open: boolean): void {
  preferredOpen = open;
  document.documentElement.dataset.sidebar = open ? "open" : "closed";
  void setSidebarOpen(open).catch((error: unknown) => {
    console.error("保存侧栏状态失败", error);
  });
}

function createOutlineItem(heading: Heading): HTMLButtonElement {
  const item = document.createElement("button");
  item.type = "button";
  item.className = `outline-item outline-level-${heading.level}`;
  item.textContent = heading.text;
  item.title = heading.text;
  item.addEventListener("click", () => {
    scrollToHeading(heading.id);
    setActiveItem(heading.id);
  });
  return item;
}

function scrollToHeading(id: string): void {
  const target = document.getElementById(id);
  target?.scrollIntoView({ block: "start" });
}

/** 滚动时高亮当前可见标题（首帧之后才安装，不参与首帧）。 */
function installActiveHeadingObserver(headings: Heading[]): void {
  if (headings.length === 0) {
    return;
  }

  const elements = headings
    .map((heading) => document.getElementById(heading.id))
    .filter((element): element is HTMLElement => element !== null);
  if (elements.length === 0) {
    return;
  }

  activeObserver = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          const id = entry.target.id;
          if (outlineItems.has(id)) {
            setActiveItem(id);
          }
        }
      }
    },
    { rootMargin: "-40% 0px -55% 0px", threshold: 0 },
  );

  for (const element of elements) {
    activeObserver.observe(element);
  }
}

function setActiveItem(id: string): void {
  for (const [itemId, element] of outlineItems) {
    element.classList.toggle("is-active", itemId === id);
  }
}

function requireApp(): HTMLElement {
  const app = document.querySelector<HTMLElement>("#app");
  if (app === null) {
    throw new Error("缺少 #app 容器");
  }
  return app;
}
