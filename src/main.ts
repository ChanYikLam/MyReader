import "./styles/app.css";
import { boot } from "./app/boot";
import { requestReveal } from "./app/reveal";

void boot().catch((error: unknown) => {
  console.error("启动流程异常", error);
  // 启动异常也要让窗口出现，由错误页/骨架承载信息（SPEC §9.6）。
  requestReveal("boot-error");
});
