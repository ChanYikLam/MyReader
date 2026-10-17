use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::{fs, parser, sanitize};

/// 一次文档加载各阶段耗时（毫秒），用于性能验收与启动诊断（SPEC §9.9/§14）。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadTimings {
    pub read_ms: f64,
    pub parse_ms: f64,
    pub sanitize_ms: f64,
    pub total_ms: f64,
}

/// 送给前端的文档载荷。
///
/// 渲染/净化全部在 Rust 侧完成，前端只负责把 `html` 塞进容器
/// （SPEC 前端边界约束）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPayload {
    /// 规范化后的绝对路径。
    pub path: String,
    /// 文件名（窗口标题与欢迎页展示用）。
    pub file_name: String,
    /// 已净化的 HTML。
    pub html: String,
    /// 文档大纲（M5 侧栏使用）。
    pub headings: Vec<parser::Heading>,
    pub size_bytes: u64,
    pub timings: LoadTimings,
}

/// 读取 → 解析 → 净化，生成前端载荷。
pub fn load_document(path: &Path) -> Result<DocumentPayload, AppError> {
    let total_started = std::time::Instant::now();

    let loaded = fs::load_markdown_file(path)?;
    let read_ms = elapsed_ms(total_started);

    let parse_started = std::time::Instant::now();
    let rendered = parser::render_markdown(&loaded.text);
    let parse_ms = elapsed_ms(parse_started);

    let sanitize_started = std::time::Instant::now();
    let html = sanitize::sanitize_html(&rendered.html);
    let sanitize_ms = elapsed_ms(sanitize_started);

    let file_name = loaded
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| loaded.path.display().to_string());

    Ok(DocumentPayload {
        path: loaded.path.display().to_string(),
        file_name,
        html,
        headings: rendered.headings,
        size_bytes: loaded.size_bytes,
        timings: LoadTimings {
            read_ms,
            parse_ms,
            sanitize_ms,
            total_ms: elapsed_ms(total_started),
        },
    })
}

fn elapsed_ms(started: std::time::Instant) -> f64 {
    let millis = started.elapsed().as_secs_f64() * 1000.0;
    (millis * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn loads_document_with_html_and_outline() {
        let dir = TempDir::new();
        let path = dir.write_file(
            "doc.md",
            "# 标题\n\n正文 **粗体**\n\n## 小节\n\n```rust\nfn main() {}\n```\n".as_bytes(),
        );

        let document = load_document(&path).expect("加载应成功");

        assert_eq!(document.file_name, "doc.md");
        assert!(document.html.contains("<h1"));
        assert!(document.html.contains("<strong>粗体</strong>"));
        assert!(document.html.contains("language-rust"));
        assert_eq!(document.headings.len(), 2);
        assert_eq!(document.headings[0].text, "标题");
        assert!(document.timings.total_ms >= 0.0);
    }

    #[test]
    fn sanitizes_dangerous_markdown() {
        let dir = TempDir::new();
        let path = dir.write_file(
            "xss.md",
            b"<script>alert(1)</script>\n\n[click](javascript:alert(1))\n",
        );

        let document = load_document(&path).expect("加载应成功");

        assert!(!document.html.contains("<script"));
        assert!(!document.html.contains("javascript:"));
        assert!(document.html.contains("alert(1)"));
    }

    #[test]
    fn propagates_file_errors() {
        let dir = TempDir::new();
        let path = dir.path().join("missing.md");

        let error = load_document(&path).expect_err("应返回 AppError");
        assert_eq!(error.code(), "not_found");
    }
}
