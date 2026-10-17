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
 * - 远程图片：本提交先交给浏览器默认加载，远程预加载在后续提交实现。
 */

import { convertFileSrc } from "@tauri-apps/api/core";

import type { DocumentPayload, ImageResource } from "./types";

/** 视口内图片等待上限：超时也继续（占位尺寸已定，不阻塞首帧太久）。 */
const IN_VIEW_IMAGE_TIMEOUT_MS = 300;

/** 图片处理统计（写入启动日志，便于验收与性能诊断）。 */
export interface ImagePreparationStats {
  total: number;
  local: number;
  remote: number;
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
    placeholder: 0,
    missing: 0,
  };
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
        // 远程图片由后续提交接入预加载与固定占位；暂按浏览器默认行为加载。
        stats.remote += 1;
        img.loading = isInViewport(img) ? "eager" : "lazy";
        break;
      default:
        decorateMissing(img, img.alt);
        stats.missing += 1;
    }
  }

  for (const frame of Array.from(article.querySelectorAll(".image-frame"))) {
    stats.placeholder += 1;
    void frame;
  }

  await Promise.allSettled(loads);
  return stats;
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
  img.removeAttribute("src");
  const frame = wrapImageFrame(img, true);
  frame.dataset.alt = alt.length > 0 ? alt : "图片无法显示";
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
