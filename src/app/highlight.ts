/**
 * 代码高亮主题 CSS 注入（SPEC §10）。
 *
 * Rust 侧生成明暗两套 class 规则并用 `html[data-theme="light|dark"]`
 * 作用域隔离，这里只负责在首帧（窗口仍隐藏）前把样式挂到 <head>；
 * 之后切换主题是纯 CSS 生效，不需要任何 JS 重新注入或时序配合。
 */
const STYLE_ELEMENT_ID = "myreader-highlight-theme";

let installedSheet: CSSStyleSheet | null = null;

export function installHighlightStyles(css: string): void {
  // 优先使用 CSSOM 构造样式表（adoptedStyleSheets）：
  // Tauri 的 CSP 会在 style-src 里注入 nonce，按 CSP 规范此时
  // 'unsafe-inline' 失效，动态创建的 <style> 会被拦截（sheet 为 null）。
  // 构造样式表走 CSSOM 路径，不受 style-src 限制，且 WebKit 支持良好。
  if (typeof CSSStyleSheet !== "undefined" && "replaceSync" in CSSStyleSheet.prototype) {
    if (installedSheet === null) {
      installedSheet = new CSSStyleSheet();
      document.adoptedStyleSheets = [...document.adoptedStyleSheets, installedSheet];
    }
    installedSheet.replaceSync(css);
    return;
  }

  // 兜底：老 WebKit 用 <style> 元素。
  const existing = document.getElementById(STYLE_ELEMENT_ID);
  if (existing !== null) {
    existing.textContent = css;
    return;
  }

  const style = document.createElement("style");
  style.id = STYLE_ELEMENT_ID;
  style.textContent = css;
  document.head.append(style);
}
