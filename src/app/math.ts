/**
 * 数学公式渲染（SPEC §7/§9.3）。
 *
 * comrak 在解析阶段把 `$...$` / `$$...$$` 输出为
 * `<span data-math-style="inline|display">源码</span>`；本模块在前端用 KaTeX
 * 把它们替换为排版结果。首帧策略：**全部公式在 reveal 前同步渲染**
 * （SPEC §9.3 决策，200 公式实测远低于预算），这样块级公式高度精确、
 * 滚动恢复与滚动条长度都稳定，无需事后懒渲染。
 */

import katex from "katex";

/** 公式就绪等待上限（字体加载），超时只记录不阻塞首帧。 */
const MATH_FONT_TIMEOUT_MS = 2000;

/** KaTeX 固定配置（SPEC §7.2）：不信任 LaTeX 指令、出错降级为错误提示。 */
const KATEX_OPTIONS = {
  throwOnError: false,
  trust: false,
  strict: "ignore",
  output: "htmlAndMathml",
} as const;

export interface MathRenderStats {
  total: number;
  inline: number;
  display: number;
  failed: number;
  elapsedMs: number;
}

export interface MathFontStats {
  faces: number;
  elapsedMs: number;
  timedOut: boolean;
}

/** 渲染正文中的全部公式节点，返回统计信息（用于启动日志/验收）。 */
export function renderMath(article: HTMLElement): MathRenderStats {
  const startedAt = performance.now();
  const stats: MathRenderStats = {
    total: 0,
    inline: 0,
    display: 0,
    failed: 0,
    elapsedMs: 0,
  };

  const nodes = Array.from(article.querySelectorAll<HTMLElement>("[data-math-style]"));

  for (const node of nodes) {
    const source = node.textContent ?? "";
    const displayMode = node.dataset.mathStyle === "display";

    stats.total += 1;
    if (displayMode) {
      stats.display += 1;
    } else {
      stats.inline += 1;
    }

    // 注意：display 模式也用 <span>（CSS 设 display:block），
    // 否则在段落 <p> 内插入 <div> 属于非法嵌套，可能被浏览器重排。
    const wrapper = document.createElement("span");
    wrapper.className = displayMode ? "math-display" : "math-inline";

    try {
      wrapper.innerHTML = katex.renderToString(source, {
        ...KATEX_OPTIONS,
        displayMode,
      });

      // throwOnError:false 时 KaTeX 输出红色 .katex-error 节点；
      // 再补一个可见的错误提示（title 里是具体解析错误）。
      if (wrapper.querySelector(".katex-error") !== null) {
        stats.failed += 1;
        decorateMathError(wrapper);
      }
    } catch (error) {
      // 理论上 throwOnError:false 不会抛；兜底保证单个公式失败不影响整篇。
      stats.failed += 1;
      renderMathError(wrapper, source, error);
    }

    node.replaceWith(wrapper);
  }

  stats.elapsedMs = Math.round((performance.now() - startedAt) * 100) / 100;
  return stats;
}

/**
 * 显式加载全部 @font-face（KaTeX 19 个 woff2 等），再等 document.fonts.ready。
 *
 * 隐藏窗口下 WebKit 不一定因为布局自动触发字体加载（M1 实测 rAF 也会被节流），
 * 因此必须显式 load，保证窗口出现前公式字体已就位、不出现字体回退跳变。
 */
export async function warmUpMathFonts(): Promise<MathFontStats> {
  const startedAt = performance.now();
  const faces = Array.from(document.fonts);
  let timedOut = false;

  await Promise.race([
    Promise.allSettled(faces.map((face) => face.load())),
    new Promise<void>((resolve) => {
      window.setTimeout(() => {
        timedOut = true;
        resolve();
      }, MATH_FONT_TIMEOUT_MS);
    }),
  ]);
  await document.fonts.ready;

  return {
    faces: faces.length,
    elapsedMs: Math.round((performance.now() - startedAt) * 100) / 100,
    timedOut,
  };
}

/** KaTeX 已渲染出错误节点：追加可见提示。 */
function decorateMathError(wrapper: HTMLElement): void {
  const errorNode = wrapper.querySelector<HTMLElement>(".katex-error");
  const message = errorNode?.getAttribute("title") ?? "公式语法错误";

  const hint = document.createElement("span");
  hint.className = "math-error-hint";
  hint.textContent = "公式错误";
  hint.title = message;
  wrapper.append(hint);
}

/** 兜底错误渲染：红色源码 + 错误提示（不中断文档其他部分）。 */
function renderMathError(
  wrapper: HTMLElement,
  source: string,
  error: unknown,
): void {
  wrapper.classList.add("math-error");

  const code = document.createElement("code");
  code.className = "math-error-source";
  code.textContent = source;

  const hint = document.createElement("span");
  hint.className = "math-error-hint";
  hint.textContent = "公式语法错误";
  hint.title = error instanceof Error ? error.message : String(error);

  wrapper.append(code, hint);
}
