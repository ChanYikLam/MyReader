/**
 * 轻量顶部提示条（SPEC §9.7 第 5 步：运行中打开失败时保留旧内容，只提示错误）。
 */

const TOAST_ID = "app-toast";
const VISIBLE_CLASS = "is-visible";
const AUTO_DISMISS_MS = 5000;

let hideTimer: number | null = null;

export function showToast(message: string): void {
  const toast = ensureToast();
  toast.textContent = message;
  toast.classList.add(VISIBLE_CLASS);

  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
  }
  hideTimer = window.setTimeout(() => {
    hideTimer = null;
    toast.classList.remove(VISIBLE_CLASS);
  }, AUTO_DISMISS_MS);
}

function ensureToast(): HTMLElement {
  const existing = document.getElementById(TOAST_ID);
  if (existing !== null) {
    return existing;
  }

  const toast = document.createElement("div");
  toast.id = TOAST_ID;
  toast.className = "toast";
  toast.setAttribute("role", "status");
  document.body.append(toast);
  return toast;
}
