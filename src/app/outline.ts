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

let outlineItems = new Map<string, HTMLElement>();
let headingEntries: Array<{ id: string; element: HTMLElement }> = [];
/** 缓存标题的文档绝对偏移（在 reveal 后/尺寸变化时重算），滚动时二分查找。 */
let headingOffsets: number[] = [];
let activeId: string | null = null;
let sidebarInstalled = false;
let revealListenerInstalled = false;
let scrollRefreshScheduled = false;
/** 用户侧栏偏好：null = 未设置（按窗口宽度决定默认值）。 */
let preferredOpen: boolean | null = window.__BOOT__?.sidebarOpen ?? null;

/** 构建/重建侧栏（文档加载后调用；欢迎页/错误页调用 clearSidebar）。 */
export function buildSidebar(payload: DocumentPayload): void {
  const app = requireApp();
  clearSidebar();
  activeId = null;

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
  headingEntries = [];
  for (const heading of headings) {
    const item = createOutlineItem(heading);
    nav.append(item);
    outlineItems.set(heading.id, item);

    const anchor = document.getElementById(heading.id);
    if (anchor !== null) {
      headingEntries.push({ id: heading.id, element: anchor });
    }
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
  installActiveHeadingObserver();
}

/** 移除侧栏与按钮（欢迎页/错误页使用）。 */
export function clearSidebar(): void {
  outlineItems = new Map();
  headingEntries = [];

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

/**
 * 首帧之后刷新一次高亮，然后由滚动驱动。
 *
 * 不用 IntersectionObserver 逐个观察：超大文档（数千标题）观察成本高，
 * 且隐藏窗口期间回调不可靠。改为 reveal 后缓存标题绝对偏移，滚动时二分查找。
 */
function installActiveHeadingObserver(): void {
  if (!revealListenerInstalled) {
    revealListenerInstalled = true;
    document.addEventListener(
      "myreader:revealed",
      () => {
        refreshHeadingOffsets();
        refreshActiveHeading();
      },
      { once: true },
    );
    window.addEventListener(
      "scroll",
      () => {
        if (scrollRefreshScheduled) {
          return;
        }
        scrollRefreshScheduled = true;
        window.requestAnimationFrame(() => {
          scrollRefreshScheduled = false;
          refreshActiveHeading();
        });
      },
      { passive: true },
    );
    window.addEventListener("resize", () => {
      refreshHeadingOffsets();
      refreshActiveHeading();
    });
  }
}

/** 缓存每个标题相对文档顶部的偏移（只在 reveal 后/尺寸变化时计算）。 */
function refreshHeadingOffsets(): void {
  const scrollY = window.scrollY;
  headingOffsets = headingEntries.map(
    ({ element }) => element.getBoundingClientRect().top + scrollY,
  );
}

/** 按滚动位置二分查找当前章节（最后一个偏移不超过阈值的标题）。 */
function refreshActiveHeading(): void {
  if (headingEntries.length === 0) {
    return;
  }
  if (headingOffsets.length !== headingEntries.length) {
    refreshHeadingOffsets();
  }

  const threshold = window.scrollY + 140;
  let low = 0;
  let high = headingOffsets.length - 1;
  let candidate = 0;
  while (low <= high) {
    const mid = (low + high) >> 1;
    if ((headingOffsets[mid] ?? 0) <= threshold) {
      candidate = mid;
      low = mid + 1;
    } else {
      high = mid - 1;
    }
  }

  setActiveItem(headingEntries[candidate]?.id ?? "");
}

function setActiveItem(id: string): void {
  if (activeId === id) {
    return;
  }
  if (activeId !== null) {
    outlineItems.get(activeId)?.classList.remove("is-active");
  }
  activeId = id;
  const element = outlineItems.get(id);
  if (element !== undefined) {
    element.classList.add("is-active");
    scrollNavItemIntoView(element);
  }
}

/** 只在侧栏内部滚动，不带动正文（目录长时保持当前项可见）。 */
function scrollNavItemIntoView(item: HTMLElement): void {
  const nav = item.parentElement;
  if (nav === null || nav.scrollHeight <= nav.clientHeight) {
    return;
  }
  const itemRect = item.getBoundingClientRect();
  const navRect = nav.getBoundingClientRect();
  nav.scrollTop += itemRect.top - navRect.top - nav.clientHeight / 2;
}

function requireApp(): HTMLElement {
  const app = document.querySelector<HTMLElement>("#app");
  if (app === null) {
    throw new Error("缺少 #app 容器");
  }
  return app;
}
