//! 代码高亮：comrak 官方 `SyntectAdapter`（class 模式）+ 明暗两套主题 CSS。
//!
//! 设计（SPEC §5/§10）：
//! - Rust 侧只输出 class-based HTML（`hl-k`、`hl-s`…），不烘焙颜色；
//! - 两份主题 CSS 用 `html[data-theme="light|dark"]` 作用域隔离，
//!   前端切换主题时纯 CSS 生效，没有 JS 时序风险。

use std::sync::OnceLock;

use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use syntect::highlighting::{Color, Theme, ThemeSet};
use syntect::html::{css_for_theme_with_class_style, ClassStyle};

/// class 前缀：`hl-k`、`hl-s` 等（必须与 adapter 配置一致）。
pub const CLASS_PREFIX: &str = "hl-";

/// 浅色代码高亮主题（syntect 内置）。
pub const LIGHT_THEME: &str = "InspiredGitHub";
/// 深色代码高亮主题（syntect 内置）。
pub const DARK_THEME: &str = "base16-ocean.dark";

/// 进程内单例：SyntaxSet/ThemeSet 加载成本高，只初始化一次（约几十毫秒）。
pub fn adapter() -> &'static SyntectAdapter {
    static ADAPTER: OnceLock<SyntectAdapter> = OnceLock::new();
    ADAPTER.get_or_init(|| {
        SyntectAdapterBuilder::new()
            .css_with_class_prefix(CLASS_PREFIX)
            .build()
    })
}

/// 预热语法集/主题集（约 100–200 ms）。与窗口创建、前端加载并行，
/// 避免首次文档解析被加载成本拖慢（SPEC §14 冷启动预算）。
pub fn prewarm() {
    std::thread::spawn(|| {
        let _ = adapter();
    });
}

/// 明暗两套代码高亮 CSS（含代码块背景/前景 CSS 变量），进程内只生成一次。
pub fn theme_css() -> &'static str {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(build_theme_css)
}

fn build_theme_css() -> String {
    let theme_set = ThemeSet::load_defaults();
    let mut css =
        String::from("/* MyReader 代码高亮：明暗两套 class 主题（按 data-theme 作用域） */\n");

    for (mode, name) in [("light", LIGHT_THEME), ("dark", DARK_THEME)] {
        let Some(theme) = theme_set.themes.get(name) else {
            continue;
        };

        css.push_str(&palette_rule(mode, theme));

        let Ok(rules) = css_for_theme_with_class_style(
            theme,
            ClassStyle::SpacedPrefixed {
                prefix: CLASS_PREFIX,
            },
        ) else {
            css.push_str(&format!("/* 生成 {name} 主题 CSS 失败 */\n"));
            continue;
        };
        css.push_str(&scope_css(&rules, &format!("html[data-theme=\"{mode}\"]")));
    }

    css
}

/// 代码块背景/前景色写在 CSS 变量里，供 markdown.css 的 pre/code 使用。
fn palette_rule(mode: &str, theme: &Theme) -> String {
    let fallback_background = if mode == "dark" { "#2b303b" } else { "#ffffff" };
    let fallback_foreground = if mode == "dark" { "#c0c5ce" } else { "#323232" };

    let background = theme
        .settings
        .background
        .map(color_hex)
        .unwrap_or_else(|| fallback_background.to_string());
    let foreground = theme
        .settings
        .foreground
        .map(color_hex)
        .unwrap_or_else(|| fallback_foreground.to_string());

    format!(
        "html[data-theme=\"{mode}\"] {{\n  --code-bg: {background};\n  --code-fg: {foreground};\n}}\n\n"
    )
}

fn color_hex(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

/// 给 syntect 生成的每条规则加主题作用域前缀。
///
/// syntect 输出格式固定：`选择器 {` / 属性行 / `}`，选择器可能逗号分隔。
fn scope_css(css: &str, scope: &str) -> String {
    let mut scoped_css = String::with_capacity(css.len() + css.len() / 3);

    for line in css.lines() {
        let trimmed = line.trim_end();
        if trimmed.ends_with('{') && !trimmed.starts_with("/*") {
            let selector = trimmed.trim_end_matches('{').trim_end();
            let scoped: Vec<String> = selector
                .split(',')
                .map(|part| format!("{scope} {}", part.trim()))
                .collect();
            scoped_css.push_str(&scoped.join(", "));
            scoped_css.push_str(" {\n");
        } else {
            scoped_css.push_str(line);
            scoped_css.push('\n');
        }
    }

    scoped_css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_css_scopes_both_modes() {
        let css = theme_css();

        assert!(css.contains("html[data-theme=\"light\"] .hl-"));
        assert!(css.contains("html[data-theme=\"dark\"] .hl-"));
        assert!(css.contains("--code-bg:"));
        assert!(css.contains("--code-fg:"));
    }

    #[test]
    fn theme_css_contains_known_palettes() {
        let css = theme_css();

        // InspiredGitHub 背景为白色；base16-ocean.dark 背景 #2b303b。
        assert!(css.contains("--code-bg: #ffffff;"));
        assert!(css.contains("--code-bg: #2b303b;"));
    }

    #[test]
    fn scope_css_prefixes_every_selector() {
        let scoped = scope_css(
            ".hl-code {\n color: #fff;\n}\n\n.hl-k, .hl-s {\n color: #000;\n}\n",
            "SCOPE",
        );

        assert!(scoped.contains("SCOPE .hl-k, SCOPE .hl-s {"));
        assert!(scoped.contains("SCOPE .hl-code {"));
        assert!(!scoped.contains("\n.hl-k"));
    }
}
