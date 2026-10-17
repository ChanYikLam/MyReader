use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 允许打开的文件扩展名（需求「打开文件」）。
pub const SUPPORTED_EXTENSIONS: [&str; 4] = ["md", "markdown", "mdown", "txt"];

/// 单文件体积上限：32 MB（SPEC §18 A7，防 OOM/卡死）。
pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

const MEBIBYTE: u64 = 1024 * 1024;
/// 二进制嗅探只看开头这么多字节，避免大文件全量扫描。
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// 读取成功的文档原文与规范化元数据。
#[derive(Debug, Clone)]
pub struct LoadedFile {
    /// 规范化（canonicalize）后的绝对路径，作为滚动位置等状态的 key。
    pub path: PathBuf,
    pub text: String,
    pub size_bytes: u64,
}

/// 按应用规则读取一个 Markdown/文本文件。
///
/// 规则：扩展名白名单 → 必须存在且是普通文件 → 体积上限 → 二进制嗅探
/// → 去 UTF-8 BOM → 非法 UTF-8 宽松解码（SPEC §18 A6）。
pub fn load_markdown_file(path: &Path) -> Result<LoadedFile, AppError> {
    let display_path = path.display().to_string();

    let canonical = std::fs::canonicalize(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound {
                path: display_path.clone(),
            }
        } else {
            AppError::Io {
                path: display_path.clone(),
                source,
            }
        }
    })?;

    ensure_supported_extension(&canonical)?;

    let metadata = std::fs::metadata(&canonical).map_err(|source| AppError::Io {
        path: display_path.clone(),
        source,
    })?;

    if !metadata.is_file() {
        return Err(AppError::NotAFile { path: display_path });
    }

    ensure_size_within_limit(metadata.len())?;

    let bytes = std::fs::read(&canonical).map_err(|source| AppError::Io {
        path: display_path.clone(),
        source,
    })?;

    if looks_binary(&bytes) {
        return Err(AppError::BinaryFile { path: display_path });
    }

    Ok(LoadedFile {
        text: decode_text(bytes),
        path: canonical,
        size_bytes: metadata.len(),
    })
}

/// 扩展名白名单校验（大小写不敏感）。
pub fn ensure_supported_extension(path: &Path) -> Result<(), AppError> {
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase);

    match extension {
        Some(extension) if SUPPORTED_EXTENSIONS.contains(&extension.as_str()) => Ok(()),
        _ => Err(AppError::UnsupportedType {
            path: path.display().to_string(),
        }),
    }
}

/// 体积上限校验。
pub fn ensure_size_within_limit(size_bytes: u64) -> Result<(), AppError> {
    if size_bytes > MAX_FILE_BYTES {
        return Err(AppError::TooLarge {
            size_mb: size_bytes.div_ceil(MEBIBYTE),
            limit_mb: MAX_FILE_BYTES / MEBIBYTE,
        });
    }
    Ok(())
}

/// 前 8 KB 出现 NUL 字节即视为二进制（与多数编辑器的启发式一致）。
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_SNIFF_BYTES).any(|byte| *byte == 0)
}

/// 去 UTF-8 BOM；非法 UTF-8 用替换字符宽松解码，保证文件仍可阅读。
fn decode_text(mut bytes: Vec<u8>) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let len = bytes.len();
        bytes.copy_within(3.., 0);
        bytes.truncate(len - 3);
    }

    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn loads_utf8_file() {
        let dir = TempDir::new();
        let path = dir.write_file("a.md", "# 标题\n正文".as_bytes());

        let loaded = load_markdown_file(&path).expect("应读取成功");
        assert_eq!(loaded.text, "# 标题\n正文");
        assert_eq!(loaded.size_bytes, "# 标题\n正文".len() as u64);
        assert!(loaded.path.is_absolute());
    }

    #[test]
    fn strips_utf8_bom() {
        let dir = TempDir::new();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("hello".as_bytes());
        let path = dir.write_file("bom.md", &bytes);

        let loaded = load_markdown_file(&path).expect("应读取成功");
        assert_eq!(loaded.text, "hello");
    }

    #[test]
    fn lossy_decodes_invalid_utf8() {
        let dir = TempDir::new();
        let path = dir.write_file("gbk.md", &[0x61, 0xFF, 0x62]);

        let loaded = load_markdown_file(&path).expect("应读取成功");
        assert!(loaded.text.starts_with('a'));
        assert!(loaded.text.ends_with('b'));
        assert!(loaded.text.contains('\u{FFFD}'));
    }

    #[test]
    fn rejects_binary_file() {
        let dir = TempDir::new();
        let path = dir.write_file("bin.md", &[0x00, 0x01, 0x02, 0x03]);

        let error = load_markdown_file(&path).expect_err("二进制文件应被拒绝");
        assert_eq!(error.code(), "binary_file");
    }

    #[test]
    fn rejects_unsupported_extension() {
        let dir = TempDir::new();
        let path = dir.write_file("doc.pdf", b"hello");

        let error = load_markdown_file(&path).expect_err("后缀不在白名单应被拒绝");
        assert_eq!(error.code(), "unsupported_type");
    }

    #[test]
    fn accepts_uppercase_extension() {
        let dir = TempDir::new();
        let path = dir.write_file("README.MD", b"hello");

        let loaded = load_markdown_file(&path).expect("大写后缀应可用");
        assert_eq!(loaded.text, "hello");
    }

    #[test]
    fn accepts_txt_extension() {
        let dir = TempDir::new();
        let path = dir.write_file("notes.txt", b"plain");

        let loaded = load_markdown_file(&path).expect("txt 应可用");
        assert_eq!(loaded.text, "plain");
    }

    #[test]
    fn reports_missing_file() {
        let dir = TempDir::new();
        let path = dir.path().join("missing.md");

        let error = load_markdown_file(&path).expect_err("不存在的文件应报错");
        assert_eq!(error.code(), "not_found");
    }

    #[test]
    fn enforces_size_limit() {
        assert!(ensure_size_within_limit(MAX_FILE_BYTES).is_ok());

        let error = ensure_size_within_limit(MAX_FILE_BYTES + 1).expect_err("超限应报错");
        assert_eq!(error.code(), "too_large");
    }
}
