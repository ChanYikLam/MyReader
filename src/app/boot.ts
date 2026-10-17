import { renderDocument, renderError, renderWelcome } from "./document";
import type { DocumentPayload } from "./types";
import { installHighlightStyles } from "./highlight";
import { prepareImages } from "./images";
import { renderMath, warmUpMathFonts } from "./math";
import { buildSidebar, clearSidebar, installSidebarListener } from "./outline";
import { installScrollSaver, restoreScrollForDocument } from "./scroll";
import { installFontSizeListener } from "./typography";
import { installLinkHandler } from "./links";
import { installThemeListener } from "./theme";
import { fetchBootstrap, reportBootEvent } from "./ipc";
import {
  installDragDropListener,
  installOpenFileListeners,
  openDocument,
  openFileAction,
} from "./open";
import { installRevealWatchdog, requestReveal } from "./reveal";
import { toAppErrorPayload } from "./types";

/** rAF 探针超时：隐藏窗口下 rAF 会被节流（实测不触发），仅作后台诊断。 */
const RAF_PROBE_TIMEOUT_MS = 500;
/** 字体就绪等待上限；超时只记录，不阻塞首帧（看门狗仍兜底）。 */
const FONTS_TIMEOUT_MS = 2000;

function elapsedMs(): number {
  return performance.now();
}

/**
 * 统计在隐藏窗口下 rAF 是否能正常触发（后台诊断，不阻塞启动）。
 * 结论见 SPEC §19 风险表：WebKit 在不可见窗口下不调度 rAF。
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
 * 启动编排（M1）：启动态校验 → 看门狗 → 内容渲染（文档/欢迎页/错误页）
 * → 字体就绪 → 请求显示窗口。
 *
 * 后续里程碑在此追加：公式渲染、图片占位、滚动恢复、主题持久化（SPEC §9.3）。
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
  installLinkHandler();
  installThemeListener();
  installFontSizeListener();
  installSidebarListener();

  void probeAnimationFrames(2, RAF_PROBE_TIMEOUT_MS).then((result) => {
    reportBootEvent("raf-probe", elapsedMs(), JSON.stringify(result));
  });

  // 必须先装监听：冷启动的文件关联事件可能先于 bootstrap 到达（SPEC §12）。
  try {
    await installOpenFileListeners();
    await installDragDropListener();
  } catch (error) {
    reportBootEvent("open-file-listener-error", elapsedMs(), String(error));
  }

  let startupDocument: DocumentPayload | null = null;

  try {
    const startup = await fetchBootstrap();
    installHighlightStyles(startup.highlightCss);
    startupDocument = startup.initialDocument;

    if (startup.initialDocument !== null) {
      const article = renderDocument(startup.initialDocument);
      buildSidebar(startup.initialDocument);

      // 全部公式在 reveal 前同步渲染（SPEC §9.3 决策）：
      // 高度确定 → 滚动恢复精确、之后滚动条长度不跳变。
      const mathStats = renderMath(article);
      reportBootEvent("math-ready", elapsedMs(), JSON.stringify(mathStats));
      if (mathStats.total > 0) {
        const fontStats = await warmUpMathFonts();
        reportBootEvent("math-fonts", elapsedMs(), JSON.stringify(fontStats));
      }

      const imageStats = await prepareImages(startup.initialDocument);
      reportBootEvent("images-ready", elapsedMs(), JSON.stringify(imageStats));
      reportBootEvent(
        "content-ready",
        elapsedMs(),
        JSON.stringify({
          file: startup.initialDocument.fileName,
          sizeBytes: startup.initialDocument.sizeBytes,
          timings: startup.initialDocument.timings,
        }),
      );
    } else {
      clearSidebar();
      renderWelcome(
        {
          onOpenFile: openFileAction(),
          onOpenRecent: (path) => {
            void openDocument(path);
          },
        },
        startup.recent,
      );
      reportBootEvent("content-ready", elapsedMs(), JSON.stringify({ welcome: true }));
    }
  } catch (error) {
    const payload = toAppErrorPayload(error);
    clearSidebar();
    renderError(payload, { onOpenFile: openFileAction() });
    reportBootEvent("content-error", elapsedMs(), JSON.stringify(payload));
  }

  const fontsProbe = await probeFontsReady(FONTS_TIMEOUT_MS);
  reportBootEvent("fonts-probe", elapsedMs(), JSON.stringify(fontsProbe));

  // 布局已稳定（公式/图片/字体全部就绪）后同步恢复滚动位置，再显示窗口。
  if (startupDocument !== null) {
    restoreScrollForDocument(startupDocument);
    reportBootEvent(
      "scroll-restored",
      elapsedMs(),
      JSON.stringify({ scrollY: Math.round(window.scrollY) }),
    );
  }

  document.documentElement.dataset.ready = "true";
  requestReveal("boot");

  if (startupDocument !== null) {
    installScrollSaver(startupDocument);
  }
}
