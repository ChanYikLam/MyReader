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
}
