//! 原生窗口外观与 CSS 主题的对应关系（SPEC §10）。
//!
//! 深色冷启动"无白色闪屏"要求：窗口 show 之前把原生窗口背景色设成
//! 与页面 `--bg` 相同的颜色，这样即使 WebView 首帧还没提交，用户看到的
//! 也是主题底色而不是系统窗口白底。
//!
//! 常量必须与 index.html 内联 CSS 的 `html[data-theme] { --bg }` 保持同步。

use tauri::window::Color;

/// 浅色主题页面背景（#ffffff）。
pub const LIGHT_BACKGROUND: Color = Color(0xFF, 0xFF, 0xFF, 0xFF);
/// 深色主题页面背景（#0d1117）。
pub const DARK_BACKGROUND: Color = Color(0x0D, 0x11, 0x17, 0xFF);

/// 把解析后的主题名（"light" | "dark"）映射为窗口背景色。
pub fn background_color(theme: &str) -> Option<Color> {
    match theme {
        "light" => Some(LIGHT_BACKGROUND),
        "dark" => Some(DARK_BACKGROUND),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_resolved_themes() {
        assert_eq!(background_color("light"), Some(LIGHT_BACKGROUND));
        assert_eq!(background_color("dark"), Some(DARK_BACKGROUND));
    }

    #[test]
    fn rejects_unresolved_or_unknown_theme() {
        assert_eq!(background_color("system"), None);
        assert_eq!(background_color(""), None);
        assert_eq!(background_color("purple"), None);
    }
}
