use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// 应用启动期状态 / 运行期「待打开文件」暂存。
///
/// 来源有三类：命令行参数、Finder 文件关联（`RunEvent::Opened`）、
/// 菜单或拖拽（后续里程碑）。前端在 bootstrap 时消费一次；
/// 运行期间的新请求通过 `open-file` 事件推给前端（SPEC §12）。
#[derive(Debug)]
pub struct StartupState {
    pending_open_path: Mutex<Option<PathBuf>>,
}

impl StartupState {
    pub fn new(initial_path: Option<PathBuf>) -> Self {
        Self {
            pending_open_path: Mutex::new(initial_path),
        }
    }

    /// 取出待打开路径（一次性消费）。
    pub fn take_pending_open_path(&self) -> Option<PathBuf> {
        self.pending_open_path
            .lock()
            .ok()
            .and_then(|mut guard| guard.take())
    }

    /// 记录运行期收到的打开请求。
    pub fn set_pending_open_path(&self, path: PathBuf) {
        if let Ok(mut guard) = self.pending_open_path.lock() {
            *guard = Some(path);
        }
    }
}

/// 主窗口显示状态：区分「启动中（未 reveal）」与「已就绪（可能被隐藏）」。
///
/// 启动期间收到的文件关联事件不能提前 show 窗口（会破坏首帧完整性），
/// 由前端首帧流程统一 reveal；只有 reveal 之后的再次打开才主动唤起窗口。
#[derive(Debug, Default)]
pub struct WindowState {
    revealed: AtomicBool,
}

impl WindowState {
    pub fn mark_revealed(&self) {
        self.revealed.store(true, Ordering::SeqCst);
    }

    pub fn is_revealed(&self) -> bool {
        self.revealed.load(Ordering::SeqCst)
    }
}

/// 关闭主窗口时的全屏收尾状态。
///
/// macOS 原生全屏下直接 `hide()` 会打断 Space 过渡：全屏 Space 被保留但里面
/// 没有窗口，屏幕只剩黑屏、用户也无法正常退出（electron#20263、tauri#12056
/// 是同类问题）。因此全屏窗口收到关闭请求时必须「先退出全屏 → 等过渡完成 →
/// 再隐藏」。
///
/// Tauri 未暴露「离开全屏」事件，且 `is_fullscreen()` 在请求退全屏的瞬间就会
/// 变成 false，不能作为过渡完成信号，所以隐藏由调用方延迟执行；本状态机负责
/// 把过渡期间的重复关闭请求收敛为一次隐藏：
/// - `plan` 首次对全屏窗口返回 `ExitFullscreenThenHide`；
/// - 流程进行中再次收到关闭请求返回 `AlreadyPending`（不得做第二次 hide）；
/// - `settle(false)` 返回 true 时由调用方执行唯一一次 `hide()`；
/// - `settle(true)` 表示退出过渡被打断/用户又进入全屏，放弃本次隐藏；
/// - `cancel()` 用于窗口被重新唤起时放弃延迟隐藏。
#[derive(Debug, Default)]
pub struct FullscreenCloseState {
    pending: AtomicBool,
}

/// `FullscreenCloseState::plan` 给出的关闭动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDisposition {
    /// 普通（非全屏）窗口：立即隐藏。
    HideNow,
    /// 全屏窗口：先退出全屏，等过渡结束后再隐藏。
    ExitFullscreenThenHide,
    /// 已在「退出全屏 → 隐藏」流程中：忽略重复的关闭请求。
    AlreadyPending,
}

impl FullscreenCloseState {
    /// 根据窗口是否全屏给出关闭动作，并登记「等待退全屏」状态。
    ///
    /// 流程进行中一律返回 `AlreadyPending`：此时 `is_fullscreen()` 可能已经
    /// 变成 false，若再次 `hide()` 仍会打断正在播放的 Space 过渡。
    pub fn plan(&self, fullscreen: bool) -> CloseDisposition {
        if self.pending.load(Ordering::SeqCst) {
            return CloseDisposition::AlreadyPending;
        }
        if !fullscreen {
            return CloseDisposition::HideNow;
        }
        if self.pending.swap(true, Ordering::SeqCst) {
            CloseDisposition::AlreadyPending
        } else {
            CloseDisposition::ExitFullscreenThenHide
        }
    }

    /// 延迟隐藏收尾：仍在全屏（过渡被打断/用户又进入全屏）时放弃隐藏；
    /// 返回 true 表示调用方应执行隐藏。
    pub fn settle(&self, still_fullscreen: bool) -> bool {
        if still_fullscreen {
            self.pending.store(false, Ordering::SeqCst);
            return false;
        }
        self.pending.swap(false, Ordering::SeqCst)
    }

    /// 放弃延迟隐藏，返回此前是否处于流程中。
    pub fn cancel(&self) -> bool {
        self.pending.swap(false, Ordering::SeqCst)
    }

    /// 是否正在等待「退出全屏后隐藏」。
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::SeqCst)
    }
}

/// 从命令行参数中解析要打开的 Markdown 文件。
///
/// macOS 启动时可能注入 `-psn_0_...` 之类的参数（Finder 启动），
/// 因此跳过所有以 `-` 开头的参数，取第一个普通参数。
pub fn initial_path_from_args<I>(args: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = String>,
{
    args.into_iter()
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn picks_first_non_option_argument() {
        assert_eq!(
            initial_path_from_args(args(&["-psn_0_12345", "/tmp/a.md"])),
            Some(PathBuf::from("/tmp/a.md"))
        );
    }

    #[test]
    fn returns_none_without_file_argument() {
        assert_eq!(initial_path_from_args(args(&["-psn_0_12345"])), None);
        assert_eq!(initial_path_from_args(Vec::<String>::new()), None);
    }

    #[test]
    fn picks_relative_path_as_is() {
        assert_eq!(
            initial_path_from_args(args(&["docs/readme.md"])),
            Some(PathBuf::from("docs/readme.md"))
        );
    }

    /// Windows 文件关联把参数以 `C:\docs\a.md` 形式传入，不能被当成选项丢掉。
    #[test]
    fn keeps_windows_style_absolute_path() {
        assert_eq!(
            initial_path_from_args(args(&["C:\\docs\\a.md"])),
            Some(PathBuf::from("C:\\docs\\a.md"))
        );
    }

    #[test]
    fn pending_path_is_consumed_once() {
        let state = StartupState::new(Some(PathBuf::from("/tmp/a.md")));
        assert_eq!(
            state.take_pending_open_path(),
            Some(PathBuf::from("/tmp/a.md"))
        );
        assert_eq!(state.take_pending_open_path(), None);
    }

    #[test]
    fn pending_path_can_be_set_at_runtime() {
        let state = StartupState::new(None);
        state.set_pending_open_path(PathBuf::from("/tmp/b.md"));
        assert_eq!(
            state.take_pending_open_path(),
            Some(PathBuf::from("/tmp/b.md"))
        );
    }

    #[test]
    fn non_fullscreen_close_hides_immediately() {
        let state = FullscreenCloseState::default();
        assert_eq!(state.plan(false), CloseDisposition::HideNow);
        assert!(!state.is_pending());
    }

    /// 全屏关闭必须先退出全屏：首次请求登记流程，重复请求不再重复触发。
    #[test]
    fn fullscreen_close_waits_for_exit_transition() {
        let state = FullscreenCloseState::default();

        assert_eq!(state.plan(true), CloseDisposition::ExitFullscreenThenHide);
        assert!(state.is_pending());
        assert_eq!(state.plan(true), CloseDisposition::AlreadyPending);
        // 请求退全屏后 is_fullscreen() 立即变 false，此时再次关闭也不能 hide。
        assert_eq!(state.plan(false), CloseDisposition::AlreadyPending);

        // 过渡完成后只放行一次隐藏。
        assert!(state.settle(false));
        assert!(!state.is_pending());
        assert!(!state.settle(false));
    }

    /// 延迟期间用户又进入全屏：放弃隐藏，窗口保持可见。
    #[test]
    fn settle_abandons_hide_when_fullscreen_again() {
        let state = FullscreenCloseState::default();
        assert_eq!(state.plan(true), CloseDisposition::ExitFullscreenThenHide);

        assert!(!state.settle(true));
        assert!(!state.is_pending());
        assert_eq!(state.plan(true), CloseDisposition::ExitFullscreenThenHide);
    }

    /// 窗口被重新唤起（Dock 点击/再次打开文件）时放弃延迟隐藏。
    #[test]
    fn cancel_abandons_delayed_hide() {
        let state = FullscreenCloseState::default();
        assert_eq!(state.plan(true), CloseDisposition::ExitFullscreenThenHide);

        assert!(state.cancel());
        assert!(!state.is_pending());
        assert!(!state.settle(false));
    }

    /// 取消后重新关闭必须能再次进入流程。
    #[test]
    fn close_can_be_requested_again_after_cancel() {
        let state = FullscreenCloseState::default();
        assert_eq!(state.plan(true), CloseDisposition::ExitFullscreenThenHide);
        state.cancel();
        assert_eq!(state.plan(true), CloseDisposition::ExitFullscreenThenHide);
        assert!(state.settle(false));
    }
}
