//! 单元测试共用的临时文件工具（不参与非测试构建）。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// 进程内的唯一序号：并行测试在同一微秒创建 TempDir 时避免路径冲突。
static TEMP_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 测试用临时目录，`Drop` 时清理。
#[derive(Debug)]
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let unique = format!(
            "myreader-test-{}-{}-{}",
            std::process::id(),
            sequence,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("创建测试临时目录");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// 在临时目录写入一个文件并返回其路径。
    pub fn write_file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        let mut file = std::fs::File::create(&path).expect("创建测试文件");
        file.write_all(bytes).expect("写入测试文件");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
