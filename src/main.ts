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
