import { reportBootEvent } from "./ipc";
import { installRevealWatchdog, requestReveal } from "./reveal";

/** rAF 探针超时：隐藏窗口若不触发 rAF，用超时兜底继续启动流程。 */
const RAF_PROBE_TIMEOUT_MS = 500;
/** 字体就绪等待上限；超时只记录，不阻塞首帧（看门狗仍兜底）。 */
const FONTS_TIMEOUT_MS = 2000;

function elapsedMs(): number {
  return performance.now();
}

/**
 * 统计在隐藏窗口下 rAF 是否能正常触发。
 * 目的：验证「visible:false + backgroundThrottling:disabled」时布局/合成链路可用，
 * 结论记录到 boot 日志（SPEC §19 风险表第一种风险）。
 */
async function probeAnimationFrames(
  targetFrames: number,
  timeoutMs: number,
): Promise<{ frames: number; elapsedMs: number; timedOut: boolean }> {
  const startedAt = elapsedMs();
  return await new Promise((resolve) => {
    let frames = 0;
    const timer = window.setTimeout(() => {
      resolve({ frames, elapsedMs: elapsedMs() - startedAt, timedOut: true });
    }, timeoutMs);

    const step = (): void => {
      frames += 1;
      if (frames >= targetFrames) {
        window.clearTimeout(timer);
        resolve({
          frames,
          elapsedMs: elapsedMs() - startedAt,
          timedOut: false,
        });
        return;
      }
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  });
}

/** 等待 document.fonts.ready，带超时兜底。 */
async function probeFontsReady(
  timeoutMs: number,
): Promise<{ ready: boolean; elapsedMs: number; timedOut: boolean }> {
  const startedAt = elapsedMs();
  return await new Promise((resolve) => {
    const timer = window.setTimeout(() => {
      resolve({ ready: false, elapsedMs: elapsedMs() - startedAt, timedOut: true });
    }, timeoutMs);

    void document.fonts.ready.then(() => {
      window.clearTimeout(timer);
      resolve({ ready: true, elapsedMs: elapsedMs() - startedAt, timedOut: false });
    });
  });
}

/**
 * 启动编排（M1 版本）。
 *
 * 当前只包含：启动态注入校验 → 看门狗 → rAF/字体探针 → 请求显示。
 * 文档渲染、公式、滚动恢复等就绪条件在后续里程碑加入（SPEC §9.3）。
 */
export async function boot(): Promise<void> {
  const processStartedAt = window.__BOOT_STARTED_AT_MS__;
  const sinceProcessStartMs =
    typeof processStartedAt === "number" && processStartedAt > 0
      ? Math.round(Date.now() - processStartedAt)
      : null;

  reportBootEvent(
    "module-start",
    elapsedMs(),
    JSON.stringify({
      sinceProcessStartMs,
      visibility: document.visibilityState,
      viewport: [window.innerWidth, window.innerHeight],
      devicePixelRatio: window.devicePixelRatio,
      theme: document.documentElement.dataset.theme,
      themeMode: document.documentElement.dataset.themeMode,
    }),
  );

  installRevealWatchdog();

  // 实测结论（2026-10-17）：隐藏窗口下 rAF 被 WebKit 节流，不会触发。
  // 因此 reveal 前的就绪判定不能依赖 rAF；该探针改为后台记录，不阻塞首帧。
  void probeAnimationFrames(2, RAF_PROBE_TIMEOUT_MS).then((result) => {
    reportBootEvent("raf-probe", elapsedMs(), JSON.stringify(result));
  });

  const fontsProbe = await probeFontsReady(FONTS_TIMEOUT_MS);
  reportBootEvent("fonts-probe", elapsedMs(), JSON.stringify(fontsProbe));

  requestReveal("boot");
}
