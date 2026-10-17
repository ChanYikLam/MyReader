use comrak::nodes::{AstNode, NodeValue};
use comrak::options::Plugins;
use comrak::{format_html_with_plugins, parse_document, Anchorizer, Arena, Options};
use serde::Serialize;

use crate::highlight;

/// 一个标题的大纲条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heading {
    /// 标题级别（1–6）。
    pub level: u8,
    /// 标题纯文本（用于大纲展示）。
    pub text: String,
    /// 与渲染 HTML 中标题 `id` 完全一致的锚点。
    pub id: String,
}

/// Markdown 解析结果。
#[derive(Debug, Clone)]
pub struct RenderedMarkdown {
    /// comrak 渲染出的 HTML（未净化，调用方必须交给 `sanitize` 处理）。
    pub html: String,
    /// 文档大纲（含兜底 id 与标题锚点一致）。
    pub headings: Vec<Heading>,
}

/// 解析并渲染 Markdown。
///
/// 安全前提：`render.unsafe_ = false`，内联 HTML 会被转义为文本；
/// 渲染结果仍需在命令层做 ammonia 白名单净化（SPEC §13）。
pub fn render_markdown(markdown: &str) -> RenderedMarkdown {
    let options = build_options();
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);

    let headings = collect_headings(root);

    // 代码块交给 syntect 输出 class-based 高亮 HTML（未知语言自动回退纯文本）。
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(highlight::adapter());

    // 输出到 String 不会失败，这里仍按 fmt::Result 处理以保持 API 完整。
    let mut html = String::with_capacity(markdown.len() + markdown.len() / 2);
    if format_html_with_plugins(root, &options, &mut html, &plugins).is_err() {
        html.clear();
    }

    RenderedMarkdown { html, headings }
}

/// 构造 comrak 选项（SPEC §5 渲染管线）。
fn build_options() -> Options<'static> {
    let mut options = Options::default();

    // 安全优先：内联 HTML 一律转义为文本（escape = true），不执行（验收 12）。
    // unsafe_ 保持 false（默认），escape 让它转义而不是整段丢弃。
    options.render.r#unsafe = false;
    options.render.escape = true;
    // 输出 <pre><code class="language-xxx">，M2 起交给 syntect 做 class-based 高亮。
    options.render.github_pre_lang = false;
    // 标题 id 由 comrak 生成（GFM 风格 slug + 去重），作为大纲跳转锚点。
    // 空前缀 = 纯 slug；与 collect_headings 中的 Anchorizer 保持一致。
    options.extension.header_id_prefix = Some(String::new());

    options
}

/// 按文档顺序提取标题，并用与 comrak 渲染完全相同的规则生成锚点。
///
/// 注意：`Anchorizer` 必须按文档顺序调用，才能与 comrak 渲染时的去重后缀
/// （`-1`、`-2`……）保持一致（见 comrak::Anchorizer 文档）。
fn collect_headings<'a>(root: &'a AstNode<'a>) -> Vec<Heading> {
    let mut anchorizer = Anchorizer::new();
    let mut headings = Vec::new();

    for node in root.descendants() {
        let level = {
            let data = node.data();
            match &data.value {
                NodeValue::Heading(heading) => Some(heading.level),
                _ => None,
            }
        };
        let Some(level) = level else {
            continue;
        };

        let text = node.collect_text();
        let id = anchorizer.anchorize(&text);
        headings.push(Heading { level, text, id });
    }

    headings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_blocks() {
        let markdown = "# 标题\n\n段落 **粗体** 与 *斜体*。\n\n> 引用\n\n- 项目\n- 项目二\n\n---\n";
        let rendered = render_markdown(markdown);

        assert!(rendered.html.contains("<h1 id=\""));
        assert!(rendered.html.contains("<strong>粗体</strong>"));
        assert!(rendered.html.contains("<em>斜体</em>"));
        assert!(rendered.html.contains("<blockquote>"));
        assert!(rendered.html.contains("<ul>"));
        assert!(rendered.html.contains("<hr />"));
    }

    #[test]
    fn extracts_heading_outline_with_matching_ids() {
        let markdown = "# 一级标题\n\n## 二级 标题\n\n### 重复\n\n### 重复\n";
        let rendered = render_markdown(markdown);

        let outline: Vec<(&str, u8, &str)> = rendered
            .headings
            .iter()
            .map(|heading| (heading.text.as_str(), heading.level, heading.id.as_str()))
            .collect();

        assert_eq!(
            outline,
            vec![
                ("一级标题", 1, "一级标题"),
                ("二级 标题", 2, "二级-标题"),
                ("重复", 3, "重复"),
                ("重复", 3, "重复-1"),
            ]
        );

        // HTML 中的 id 必须与大纲 id 一致，保证侧栏跳转可定位。
        for heading in &rendered.headings {
            assert!(
                rendered.html.contains(&format!("id=\"{}\"", heading.id)),
                "HTML 缺少标题 id: {}",
                heading.id
            );
        }
    }

    #[test]
    fn escapes_inline_html() {
        let rendered = render_markdown("<script>alert(1)</script>\n");

        assert!(rendered.html.contains("&lt;script&gt;"));
        assert!(!rendered.html.contains("<script>"));
    }

    #[test]
    fn renders_highlighted_code_fence() {
        let markdown = "```rust\nfn main() {}\n```\n";
        let rendered = render_markdown(markdown);

        // syntect class 模式的输出：pre 带 syntax-highlighting，code 带语言 class，
        // token 按 scope 拆成带 hl- 前缀的 class（如 hl-storage/hl-function）。
        assert!(rendered.html.contains("<code class=\"language-rust\">"));
        assert!(rendered.html.contains("syntax-highlighting"));
        assert!(
            rendered.html.contains("hl-source"),
            "html={}",
            rendered.html
        );
        assert!(rendered.html.contains("hl-storage"));
        assert!(rendered.html.contains("main"));
    }

    #[test]
    fn unknown_language_falls_back_without_error() {
        let rendered = render_markdown("```notalang\nplain text\n```\n");

        assert!(rendered.html.contains("language-notalang"));
        assert!(rendered.html.contains("syntax-highlighting"));
        assert!(rendered.html.contains("plain text"));
    }

    #[test]
    fn heading_text_collects_inline_markup() {
        let rendered = render_markdown("# **粗** `code`\n");

        assert_eq!(rendered.headings.len(), 1);
        assert_eq!(rendered.headings[0].text, "粗 code");
    }

    #[test]
    fn empty_document_renders_empty_html() {
        let rendered = render_markdown("");

        assert!(rendered.html.is_empty());
        assert!(rendered.headings.is_empty());
    }
}
