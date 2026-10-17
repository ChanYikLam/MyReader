/** 与 Rust 侧 IPC 载荷一一对应的类型定义（字段名 camelCase，见 serde 配置）。 */

export interface Heading {
  level: number;
  text: string;
  id: string;
}

export interface LoadTimings {
  readMs: number;
  parseMs: number;
  sanitizeMs: number;
  totalMs: number;
}

export type ImageKind = "local" | "remote" | "unsupported";

/** 与 src-tauri/src/media.rs::ImageResource 对应。 */
export interface ImageResource {
  src: string;
  kind: ImageKind;
  path: string | null;
  exists: boolean;
  width: number | null;
  height: number | null;
}

/** 每个文件的滚动位置记忆（与 settings.rs::ScrollPosition 对应）。 */
export interface ScrollPosition {
  top: number;
  docHeight: number;
  viewportWidth: number;
  updatedAt: number;
}

/** 最近打开文件条目（File → Open Recent / 欢迎页）。 */
export interface RecentFile {
  path: string;
  name: string;
  openedAt: number;
}

export interface DocumentPayload {
  path: string;
  fileName: string;
  html: string;
  headings: Heading[];
  images: ImageResource[];
  savedScroll: ScrollPosition | null;
  sizeBytes: number;
  timings: LoadTimings;
}

export interface BootstrapPayload {
  initialDocument: DocumentPayload | null;
  /** 明暗两套代码高亮 CSS（按 data-theme 作用域）。 */
  highlightCss: string;
  /** 最近打开列表（欢迎页展示，最多 10 条）。 */
  recent: RecentFile[];
}

/** Rust 侧 AppError 序列化后的形状（SPEC §5）。 */
export interface AppErrorPayload {
  code: string;
  message: string;
  detail?: string | null;
}

export function isAppErrorPayload(value: unknown): value is AppErrorPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as { code?: unknown; message?: unknown };
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

/** 任意异常 → 统一的用户可读错误载荷。 */
export function toAppErrorPayload(error: unknown): AppErrorPayload {
  if (isAppErrorPayload(error)) {
    return error;
  }

  let message = "未知错误";
  if (error instanceof Error) {
    message = error.message;
  } else if (typeof error === "string") {
    message = error;
  }

  return { code: "internal", message, detail: null };
}
