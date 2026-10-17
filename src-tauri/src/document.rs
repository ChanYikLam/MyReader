use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::media;
use crate::settings::ScrollPosition;
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
    /// 图片资源（本地路径/尺寸或远程标记），前端据此设置 src 与占位尺寸。
    pub images: Vec<media::ImageResource>,
    /// 记忆的滚动位置（首帧同步恢复，SPEC §9.3 第 6 项）；由命令层填充。
    pub saved_scroll: Option<ScrollPosition>,
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

    let media_started = std::time::Instant::now();
    let images = media::resolve_images(&loaded.path, &rendered.images);
    let media_ms = elapsed_ms(media_started);

    let file_name = loaded
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| loaded.path.display().to_string());

    let total_ms = elapsed_ms(total_started);

    // 启动诊断：Rust 侧文档管线的分阶段耗时（SPEC §9.9/§14）。
    crate::diagnostics::boot_log(&format!(
        "document-loaded read_ms={read_ms} parse_ms={parse_ms} sanitize_ms={sanitize_ms} \
         media_ms={media_ms} total_ms={total_ms}"
    ));

    Ok(DocumentPayload {
        path: loaded.path.display().to_string(),
        file_name,
        html,
        headings: rendered.headings,
        images,
        saved_scroll: None,
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
    use std::path::PathBuf;

    use super::*;
    use crate::media::ImageKind;
    use crate::test_support::TempDir;

    /// 仓库根目录下的验收夹具（tests/fixtures/）。
    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures")
            .join(name)
    }

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

    #[test]
    fn keeps_gfm_extensions_after_sanitize() {
        let dir = TempDir::new();
        let path = dir.write_file(
            "gfm.md",
            "| 左 | 中 |\n| :-- | :-: |\n| a | b |\n\n- [x] 完成\n- [ ] 未完成\n\n正文[^1]\n\n[^1]: 脚注\n\n~~删除~~\n"
                .as_bytes(),
        );

        let document = load_document(&path).expect("加载应成功");

        assert!(document.html.contains("<table>"), "html={}", document.html);
        assert!(document.html.contains(r#"align="center""#));
        assert!(document.html.contains(r#"type="checkbox""#));
        assert!(document.html.contains("task-list"));
        assert!(document.html.contains("footnotes"));
        assert!(document.html.contains("data-footnotes"));
        assert!(document.html.contains("footnote-backref"));
        assert!(document.html.contains("<del>删除</del>"));
    }

    #[test]
    fn renders_all_elements_fixture() {
        let document = load_document(&fixture("elements.md")).expect("夹具应能加载");

        // 表格 + 对齐
        assert!(document.html.contains("<table>"), "html={}", document.html);
        assert!(document.html.contains(r#"align="center""#));
        // 任务列表
        assert!(document.html.contains(r#"type="checkbox""#));
        assert!(document.html.contains("task-list"));
        // 代码高亮（class 模式）
        assert!(document.html.contains("syntax-highlighting"));
        assert!(document.html.contains("hl-source"));
        assert!(document.html.contains("language-rust"));
        // 删除线 / 脚注 / 嵌套引用
        assert!(document.html.contains("<del>"));
        assert!(document.html.contains("footnotes"));
        assert!(document.html.contains("footnote-backref"));
        assert!(document.html.matches("<blockquote>").count() >= 2);
        // 图片标签
        assert!(document.html.contains("<img"));
    }

    #[test]
    fn resolves_images_in_fixture() {
        let document = load_document(&fixture("elements.md")).expect("夹具应能加载");

        let find = |src: &str| {
            document
                .images
                .iter()
                .find(|image| image.src == src)
                .unwrap_or_else(|| panic!("缺少图片资源 {src}: {:?}", document.images))
        };

        let photo = find("images/photo.png");
        assert_eq!(photo.kind, ImageKind::Local);
        assert!(photo.exists);
        assert_eq!((photo.width, photo.height), (Some(320), Some(160)));
        assert!(photo
            .path
            .as_deref()
            .is_some_and(|p| p.ends_with("photo.png")));

        let svg = find("images/icon.svg");
        assert_eq!(svg.kind, ImageKind::Local);
        assert!(svg.exists);
        // SVG 不探测尺寸 → 前端用固定占位盒
        assert_eq!((svg.width, svg.height), (None, None));

        let missing = find("images/missing.png");
        assert_eq!(missing.kind, ImageKind::Local);
        assert!(!missing.exists);

        let remote = find("https://example.com/remote.png");
        assert_eq!(remote.kind, ImageKind::Remote);
        assert!(remote.path.is_none());
    }

    #[test]
    fn renders_math_fixture_source_nodes() {
        let document = load_document(&fixture("math.md")).expect("夹具应能加载");

        // 行内/块级公式都以源码文本节点交给前端 KaTeX
        assert!(document.html.contains(r#"data-math-style="inline""#));
        assert!(document.html.contains(r#"data-math-style="display""#));
        assert!(document.html.contains("E = mc^2"));
        assert!(document.html.contains("\\begin{pmatrix}"));
        assert!(document.html.contains("\\begin{aligned}"));
        assert!(document.html.contains("\\begin{cases}"));

        // 代码域豁免
        assert!(document.html.contains("<code>$x$</code>"));
        assert!(document.html.contains("$y = 1 + 2$"));

        // \$ 转义
        assert!(document.html.contains("$5"));

        // 非法公式源码原样透传；后续正文必须保留
        assert!(document.html.contains("\\frac{1}"));
        assert!(document.html.contains("后面的这段话必须正常显示"));

        // trust:false 在 KaTeX 侧处理，源码节点仍需保留供前端渲染
        assert!(document.html.contains("\\href{javascript:alert(1)}"));

        let math_nodes = document.html.matches("data-math-style").count();
        assert!(math_nodes >= 10, "公式节点数量异常: {math_nodes}");
    }

    #[test]
    fn renders_basic_fixture() {
        let document = load_document(&fixture("basic.md")).expect("夹具应能加载");

        // 标题层级与大纲锚点
        assert_eq!(document.headings.len(), 6);
        assert_eq!(document.headings[0].level, 1);
        assert_eq!(document.headings[0].text, "MyReader 功能验收");
        assert_eq!(document.headings[5].level, 6);
        for heading in &document.headings {
            assert!(
                document.html.contains(&format!("id=\"{}\"", heading.id)),
                "HTML 缺少标题锚点 {}",
                heading.id
            );
        }

        // 行内元素
        assert!(document.html.contains("<strong>粗体</strong>"));
        assert!(document.html.contains("<em>斜体</em>"));
        assert!(document.html.contains("<code>行内代码</code>"));
        assert!(document.html.contains("href=\"https://example.com\""));

        // 引用（含嵌套）与列表（含嵌套）
        assert!(document.html.contains("<blockquote>"));
        assert!(document.html.matches("<blockquote>").count() >= 2);
        assert!(document.html.contains("<ol>"));
        assert!(document.html.contains("<ul>"));

        // 分隔线（ammonia 会按 HTML5 序列化为 <hr>）与代码块语言 class
        // （未知语言回退为无高亮纯文本）
        assert!(document.html.contains("<hr>"));
        assert!(document.html.contains("language-rust"));
        assert!(document.html.contains("language-python"));
        assert!(document.html.contains("language-unknownlang"));
        assert!(document.html.contains("unknown language should fall back"));
    }

    #[test]
    fn renders_security_fixture_safely() {
        let document = load_document(&fixture("security.md")).expect("夹具应能加载");

        // 原始 HTML 以转义文本呈现，不产生真实标签
        assert!(!document.html.contains("<script"));
        assert!(document.html.contains("&lt;script&gt;"));
        assert!(!document.html.contains("<img"));

        // 危险协议被剥离，正常链接保留
        assert!(!document.html.contains("href=\"javascript:"));
        assert!(document.html.contains("href=\"https://example.com\""));
        assert!(document.html.contains("javascript 链接"));
    }
}
