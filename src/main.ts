import "./styles/app.css";

/** 应用外壳的最小引导：后续里程碑在此接入文档渲染与首帧就绪流程。 */
function bootstrapShell(): void {
  const app = document.querySelector<HTMLElement>("#app");
  if (app === null) {
    throw new Error("缺少 #app 容器");
  }

  app.dataset.state = "ready";

  const status = document.querySelector<HTMLElement>("#boot-status");
  if (status !== null) {
    status.textContent = "工程骨架已就绪";
  }
}

bootstrapShell();
