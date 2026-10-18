/**
 * 图片准备（SPEC §9.3 第 5 项 / §9.4）。
 *
 * 目标：窗口显示前，视口内图片要么已加载，要么有确定尺寸的占位；
 * 任何图片加载后都不得改变高度（无 CLS）。
 *
 * - 本地图片：src 换成 Tauri asset 协议 URL；能探测到尺寸就写
 *   width/height 属性（浏览器按固有比例预留空间），探测不到（SVG 等）
 *   用固定 16:10 占位盒（object-fit: contain，加载后盒子高度不变）；
 * - 缺失/非法图片：固定占位盒 + alt 文案；
 * - 远程图片：首帧前统一预加载（整体上限 800ms，SPEC §18 A4）：
 *   成功则写 width/height 预留高度；失败/超时用固定 16:10 占位盒。
 */

import { convertFileSrc } from "@tauri-apps/api/core";

import type { DocumentPayload, ImageResource } from "./types";

/** 视口内本地图片等待上限：超时也继续（占位尺寸已定，不阻塞首帧太久）。 */
const IN_VIEW_IMAGE_TIMEOUT_MS = 300;
/** 全部远程图片预加载的总时间上限（SPEC §18 A4）。 */
const REMOTE_PRELOAD_TIMEOUT_MS = 800;

/** 图片处理统计（写入启动日志，便于验收与性能诊断）。 */
export interface ImagePreparationStats {
  total: number;
  local: number;
  remote: number;
  remoteLoaded: number;
  remotePlaceholder: number;
  placeholder: number;
  missing: number;
}

/** 在正式 reveal 前准备全部图片；视口内图片会等待加载/decode。 */
export async function prepareImages(
  payload: DocumentPayload,
): Promise<ImagePreparationStats> {
  const stats: ImagePreparationStats = {
    total: 0,
    local: 0,
    remote: 0,
    remoteLoaded: 0,
    remotePlaceholder: 0,
    placeholder: 0,
    missing: 0,
  };
  const remoteTasks: Array<Promise<void>> = [];
  const remoteDeadline = performance.now() + REMOTE_PRELOAD_TIMEOUT_MS;
  const article = document.querySelector<HTMLElement>(".markdown-body");
  if (article === null) {
    return stats;
  }

  const resources = new Map<string, ImageResource>();
  for (const resource of payload.images) {
    resources.set(resource.src, resource);
  }

  const loads: Promise<void>[] = [];

  for (const img of Array.from(article.querySelectorAll<HTMLImageElement>("img"))) {
    stats.total += 1;
    const src = img.getAttribute("src");
    if (src === null) {
      decorateMissing(img, img.alt);
      stats.missing += 1;
      continue;
    }

    const resource = resources.get(src);
    if (resource === undefined) {
      decorateMissing(img, img.alt);
      stats.missing += 1;
      continue;
    }

    switch (resource.kind) {
      case "local":
        stats.local += 1;
        if (!resource.exists || resource.path === null) {
          stats.missing += 1;
        }
        prepareLocalImage(img, resource);
        if (isInViewport(img)) {
          loads.push(waitForImage(img, IN_VIEW_IMAGE_TIMEOUT_MS));
        } else {
          img.loading = "lazy";
        }
        break;
      case "remote":
        stats.remote += 1;
        img.loading = "eager";
        img.decoding = "async";
        remoteTasks.push(
          prepareRemoteImage(img, remoteDeadline).then((outcome) => {
            if (outcome === "loaded") {
              stats.remoteLoaded += 1;
            } else {
              stats.remotePlaceholder += 1;
            }
          }),
        );
        break;
      default:
        decorateMissing(img, img.alt);
        stats.missing += 1;
    }
  }

  await Promise.allSettled([...loads, ...remoteTasks]);

  // 远程超时/未知尺寸在异步阶段才生成占位盒，统计放在等待之后。
  stats.placeholder = article.querySelectorAll(".image-frame").length;
  return stats;
}

/**
 * 远程图片：在统一截止时间前等待加载，成功后写 width/height 预留高度；
 * 失败则显示 alt 占位，超时/未完成则套固定比例占位盒（后续加载不改变高度）。
 */
async function prepareRemoteImage(
  img: HTMLImageElement,
  deadline: number,
): Promise<"loaded" | "failed" | "timeout"> {
  const outcome = await waitForRemote(img, deadline);

  if (outcome === "loaded") {
    if (img.naturalWidth > 0 && img.naturalHeight > 0) {
      img.setAttribute("width", String(img.naturalWidth));
      img.setAttribute("height", String(img.naturalHeight));
    }
    await img.decode().catch(() => {
      // 解码失败保留默认渲染
    });
    return "loaded";
  }

  if (outcome === "failed") {
    decorateMissing(img, img.alt);
    return "failed";
  }

  wrapImageFrame(img);
  return "timeout";
}

function waitForRemote(
  img: HTMLImageElement,
  deadline: number,
): Promise<"loaded" | "failed" | "timeout"> {
  if (img.complete) {
    return Promise.resolve(img.naturalWidth > 0 ? "loaded" : "failed");
  }

  const remainingMs = Math.max(0, deadline - performance.now());

  return new Promise((resolve) => {
    let settled = false;
    const settle = (outcome: "loaded" | "failed" | "timeout"): void => {
      if (settled) {
        return;
      }
      settled = true;
      img.removeEventListener("load", handleLoad);
      img.removeEventListener("error", handleError);
      resolve(outcome);
    };

    const handleLoad = (): void => settle("loaded");
    const handleError = (): void => settle("failed");

    img.addEventListener("load", handleLoad);
    img.addEventListener("error", handleError);
    window.setTimeout(() => settle("timeout"), remainingMs);
  });
}

function prepareLocalImage(img: HTMLImageElement, resource: ImageResource): void {
  if (!resource.exists || resource.path === null) {
    decorateMissing(img, img.alt);
    return;
  }

  if (resource.width !== null && resource.height !== null) {
    // 有固有尺寸：写 width/height，由浏览器在加载前就预留正确高度。
    img.setAttribute("width", String(resource.width));
    img.setAttribute("height", String(resource.height));
  } else {
    // 尺寸未知（SVG/损坏）：固定比例占位盒，加载后高度不变。
    wrapImageFrame(img);
  }

  img.decoding = "async";
  img.src = convertFileSrc(resource.path);
}

/** 缺失/不支持的图片：固定比例占位盒 + alt 文案。 */
function decorateMissing(img: HTMLImageElement, alt: string): void {
  // 直接移除 <img>：损坏图标会把 alt 文本画在占位框外（溢出/裁切），
  // 改为由占位框的 ::after 统一展示 alt 文案。
  const frame = document.createElement("span");
  frame.className = "image-frame image-frame--missing";
  frame.dataset.alt = alt.length > 0 ? alt : "图片无法显示";
  img.replaceWith(frame);
}

function wrapImageFrame(img: HTMLImageElement, missing = false): HTMLElement {
  const frame = document.createElement("span");
  frame.className = missing ? "image-frame image-frame--missing" : "image-frame";
  img.replaceWith(frame);
  frame.append(img);
  return frame;
}

function isInViewport(element: Element): boolean {
  const rect = element.getBoundingClientRect();
  return rect.top <= window.innerHeight && rect.bottom >= 0;
}

async function waitForImage(img: HTMLImageElement, timeoutMs: number): Promise<void> {
  if (!img.complete || img.naturalWidth === 0) {
    await Promise.race([
      new Promise<void>((resolve) => {
        img.addEventListener("load", () => resolve(), { once: true });
        img.addEventListener("error", () => resolve(), { once: true });
      }),
      new Promise<void>((resolve) => window.setTimeout(resolve, timeoutMs)),
    ]);
  }
  await img.decode().catch(() => {
    // 解码失败时保留浏览器默认渲染，不阻塞首帧
  });
}
