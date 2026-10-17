// KaTeX 样式静态引入：Vite 会把它打进构建产物（CSS + 本地 woff2 字体），
// 不走 CDN、离线可用（SPEC §8）；必须在首屏样式阶段加载，
// 不能等公式渲染后再异步引入，否则会出现公式字体/布局闪动。
import "katex/dist/katex.min.css";

import "./styles/app.css";
import "./styles/markdown.css";

import { boot } from "./app/boot";
import { renderError } from "./app/document";
import { openFileAction } from "./app/open";
import { requestReveal } from "./app/reveal";
import { toAppErrorPayload } from "./app/types";

void boot().catch((error: unknown) => {
  console.error("启动流程异常", error);
  // 启动异常也要让窗口出现，由错误页承载信息（SPEC §9.6）。
  try {
    renderError(toAppErrorPayload(error), { onOpenFile: openFileAction() });
  } finally {
    requestReveal("boot-error");
  }
});
