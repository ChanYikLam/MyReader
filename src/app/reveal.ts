import { reportBootEvent, revealWindow } from "./ipc";

/** 首帧看门狗：超时强制显示窗口，绝不把用户卡在不可见状态（SPEC §9.5）。 */
const WATCHDOG_MS = 4000;

let revealRequested = false;

/** 安装看门狗。必须在启动流程最开始调用。 */
export function installRevealWatchdog(): void {
  window.setTimeout(() => {
    if (revealRequested) {
      return;
    }
    reportBootEvent("watchdog-fired", performance.now());
    requestReveal("watchdog");
  }, WATCHDOG_MS);
}

/**
 * 请求显示窗口。幂等：重复调用只有第一次生效。
 * 这是全应用唯一的「show」语义入口（SPEC §9.5）。
 */
export function requestReveal(reason: string): void {
  if (revealRequested) {
    return;
  }
  revealRequested = true;

  reportBootEvent("reveal-request", performance.now(), `reason=${reason}`);

  void revealWindow()
    .then(() => {
      document.documentElement.dataset.revealed = "true";
      reportBootEvent(
        "reveal-done",
        performance.now(),
        `visibility=${document.visibilityState}`,
      );
      requestAnimationFrame(() => {
        reportBootEvent(
          "first-frame-after-show",
          performance.now(),
          `visibility=${document.visibilityState}`,
        );
      });
    })
    .catch((error: unknown) => {
      reportBootEvent("reveal-error", performance.now(), String(error));
    });
}
