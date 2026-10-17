use std::collections::{HashMap, HashSet};

use ammonia::Builder;

/// 允许保留的 HTML 标签：comrak 输出（GFM + 脚注 + 数学占位）与
/// syntect class 高亮所需的最小集合（SPEC §13）。
const ALLOWED_TAGS: &[&str] = &[
    "a",
    "blockquote",
    "br",
    "code",
    "del",
    "div",
    "em",
    "figcaption",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "img",
    "input",
    "kbd",
    "li",
    "ol",
    "p",
    "pre",
    "s",
    "section",
    "span",
    "strong",
    "sub",
    "sup",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "ul",
    // 脚注返回链接与内联 HTML 转义文本不需要额外标签。
    "wbr",
];

/// 对所有元素都允许的属性（不含事件属性）。
const GENERIC_ATTRIBUTES: &[&str] = &["class", "id", "title"];

/// 允许的属性前缀：`data-math-style`、`data-footnotes`、`aria-label` 等。
const GENERIC_ATTRIBUTE_PREFIXES: &[&str] = &["data-", "aria-"];

/// URL 协议白名单；相对 URL 由 ammonia 默认放行（图片在上层解析为本地路径）。
const URL_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// 对 comrak 渲染结果执行白名单净化（SPEC §13 第 2 层安全防线）。
///
/// 解析层已把原始内联 HTML 转义为文本，这里再做兜底：即使渲染管线未来变化，
/// 输出也不会带出 `<script>`、`on*`、`javascript:` 等危险内容。
pub fn sanitize_html(html: &str) -> String {
    sanitize_builder().clean(html).to_string()
}

fn sanitize_builder() -> Builder<'static> {
    let mut builder = Builder::default();

    builder.tags(ALLOWED_TAGS.iter().copied().collect::<HashSet<_>>());
    builder.clean_content_tags(HashSet::from(["script", "style"]));

    builder.generic_attributes(GENERIC_ATTRIBUTES.iter().copied().collect::<HashSet<_>>());
    builder.generic_attribute_prefixes(
        GENERIC_ATTRIBUTE_PREFIXES
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
    );

    builder.tag_attributes(HashMap::from([
        ("a", HashSet::from(["href"])),
        (
            "img",
            HashSet::from(["src", "alt", "width", "height", "loading", "decoding"]),
        ),
        (
            "input",
            HashSet::from(["type", "checked", "disabled", "value"]),
        ),
        ("td", HashSet::from(["align", "colspan", "rowspan"])),
        ("th", HashSet::from(["align", "colspan", "rowspan"])),
        ("ol", HashSet::from(["start"])),
    ]));

    builder.url_schemes(URL_SCHEMES.iter().copied().collect::<HashSet<_>>());

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_script_tags_and_content() {
        let cleaned = sanitize_html("<p>前</p><script>alert(1)</script><p>后</p>");

        assert!(!cleaned.contains("<script"));
        assert!(!cleaned.contains("alert(1)"));
        assert!(cleaned.contains("前"));
        assert!(cleaned.contains("后"));
    }

    #[test]
    fn strips_event_attributes() {
        let cleaned = sanitize_html(r#"<img src="a.png" onerror="alert(1)" onload="x()">"#);

        assert!(!cleaned.contains("onerror"));
        assert!(!cleaned.contains("onload"));
        assert!(cleaned.contains(r#"src="a.png""#));
    }

    #[test]
    fn strips_javascript_urls() {
        let cleaned = sanitize_html(r#"<a href="javascript:alert(1)">危险</a>"#);

        assert!(!cleaned.contains("javascript:"));
        assert!(!cleaned.contains("href="));
        assert!(cleaned.contains("危险"));
    }

    #[test]
    fn keeps_https_and_relative_links() {
        let cleaned = sanitize_html(
            r#"<p><a href="https://example.com/a">外链</a><a href="./本地.md">相对</a></p>"#,
        );

        assert!(cleaned.contains(r#"href="https://example.com/a""#));
        assert!(cleaned.contains(r#"href="./本地.md""#));
    }

    #[test]
    fn keeps_highlight_and_math_classes() {
        let cleaned = sanitize_html(
            r#"<pre class="syntax-highlighting"><code class="language-rust"><span class="hl-k">fn</span></code></pre><span data-math-style="inline">x^2</span>"#,
        );

        assert!(cleaned.contains(r#"class="syntax-highlighting""#));
        assert!(cleaned.contains(r#"class="language-rust""#));
        assert!(cleaned.contains(r#"class="hl-k""#));
        assert!(cleaned.contains(r#"data-math-style="inline""#));
    }

    #[test]
    fn keeps_heading_ids_and_anchor_links() {
        let cleaned = sanitize_html(
            r##"<h2 id="二级-标题">二级 标题<a href="#二级-标题" aria-label="链接" data-heading-content="二级 标题" class="anchor"></a></h2>"##,
        );

        assert!(cleaned.contains(r#"id="二级-标题""#));
        assert!(cleaned.contains(r#"class="anchor""#));
    }

    #[test]
    fn keeps_table_alignment_and_checkboxes() {
        let cleaned = sanitize_html(
            r#"<table><thead><tr><th align="center">h</th></tr></thead><tbody><tr><td align="center"><input type="checkbox" checked="" disabled="">x</td></tr></tbody></table>"#,
        );

        assert!(cleaned.contains(r#"align="center""#));
        assert!(cleaned.contains(r#"type="checkbox""#));
        assert!(cleaned.contains(r#"disabled=""#));
    }

    #[test]
    fn strips_style_attributes() {
        let cleaned = sanitize_html(r#"<div style="color:red">x</div>"#);

        assert!(!cleaned.contains("style="));
    }

    #[test]
    fn strips_unknown_tags_but_keeps_text() {
        let cleaned = sanitize_html("<marquee>滚动</marquee><iframe src=\"x\"></iframe>");

        assert!(!cleaned.contains("<marquee"));
        assert!(!cleaned.contains("<iframe"));
        assert!(cleaned.contains("滚动"));
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(sanitize_html(""), "");
    }
}
