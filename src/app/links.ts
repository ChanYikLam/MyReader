/** 外链打开与站内锚点跳转（SPEC §12/§13）。 */

import { openExternal } from "./ipc";

const EXTERNAL_URL = /^(https?:|mailto:)/i;

let installed = false;

/**
 * 在正文容器上安装点击拦截：
 * - `#anchor` → 站内滚动（脚注/大纲/标题锚点）；
 * - http/https/mailto → 系统默认浏览器（Rust 侧再校验协议白名单）；
 * - 其余（javascript: 等）→ 直接忽略；
 * - 任何情况都阻止 webview 内导航。
 *
 * 使用事件委托，正文 DOM 被整体替换后依然有效。
 */
export function installLinkHandler(): void {
  if (installed) {
    return;
  }
  installed = true;

  const root = document.querySelector<HTMLElement>("#content-root");
  if (root === null) {
    throw new Error("缺少 #content-root 容器");
  }

  root.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) {
      return;
    }
    const anchor = target.closest("a");
    if (!(anchor instanceof HTMLAnchorElement)) {
      return;
    }

    event.preventDefault();
    handleAnchorClick(anchor);
  });

  // 中键/辅助键同样不应用内跳转
  root.addEventListener("auxclick", (event) => {
    const target = event.target;
    if (target instanceof Element && target.closest("a") !== null) {
      event.preventDefault();
    }
  });
}

function handleAnchorClick(anchor: HTMLAnchorElement): void {
  const href = anchor.getAttribute("href");
  if (href === null || href.length === 0) {
    return;
  }

  if (href.startsWith("#")) {
    scrollToAnchor(href.slice(1));
    return;
  }

  if (EXTERNAL_URL.test(href)) {
    void openExternal(href).catch((error: unknown) => {
      console.error("打开外部链接失败", error);
    });
  }
  // 其它协议（javascript:、file: 等）静默忽略
}

function scrollToAnchor(rawId: string): void {
  let id = rawId;
  try {
    id = decodeURIComponent(rawId);
  } catch {
    // 非法百分号编码时保留原文
  }

  const element = document.getElementById(id);
  element?.scrollIntoView({ block: "start" });
}
