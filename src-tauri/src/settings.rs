//! 应用设置持久化（tauri-plugin-store，SPEC §10/§11）。
//!
//! 首帧防闪要求：主题偏好在**创建窗口之前**由 Rust 从 store 读出，
//! 通过 initialization_script 同步注入 `window.__BOOT__`，
//! 前端不依赖异步读取（SPEC §9.1/§9.2）。

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::error::AppError;

/// store 文件名（位于 Tauri 应用配置目录）。
pub const STORE_FILE: &str = "settings.json";

const THEME_MODE_KEY: &str = "theme.mode";

/// 主题三态：跟随系统 / 浅色 / 深色。
///
/// 序列化为 `"system" | "light" | "dark"`，与前端 ThemeMode 及
/// index.html 内联脚本的约定一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// 原生窗口/标题栏主题；跟随系统时返回 None（由 macOS 决定）。
    pub fn native_theme(self) -> Option<tauri::Theme> {
        match self {
            Self::System => None,
            Self::Light => Some(tauri::Theme::Light),
            Self::Dark => Some(tauri::Theme::Dark),
        }
    }
}

/// 读取持久化的主题偏好；store 缺失/损坏/值非法时回退「跟随系统」。
pub fn load_theme_mode(app: &AppHandle) -> ThemeMode {
    match app.store(STORE_FILE) {
        Ok(store) => theme_mode_from_value(store.get(THEME_MODE_KEY).as_ref()),
        Err(_) => ThemeMode::System,
    }
}

/// 持久化主题偏好（store 开启自动保存，这里显式 save 确保落盘）。
pub fn save_theme_mode(app: &AppHandle, mode: ThemeMode) -> Result<(), AppError> {
    let store = app.store(STORE_FILE).map_err(|error| AppError::Settings {
        detail: error.to_string(),
    })?;

    store.set(
        THEME_MODE_KEY,
        serde_json::Value::String(mode.as_str().to_string()),
    );
    store.save().map_err(|error| AppError::Settings {
        detail: error.to_string(),
    })
}

/// 从 JSON 值解析主题偏好（独立函数便于单测）。
pub fn theme_mode_from_value(value: Option<&serde_json::Value>) -> ThemeMode {
    value
        .and_then(serde_json::Value::as_str)
        .and_then(ThemeMode::parse)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn theme_mode_roundtrip() {
        for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
            assert_eq!(ThemeMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(ThemeMode::parse("dark "), None);
    }

    #[test]
    fn invalid_or_missing_value_falls_back_to_system() {
        assert_eq!(theme_mode_from_value(None), ThemeMode::System);
        assert_eq!(theme_mode_from_value(Some(&json!(42))), ThemeMode::System);
        assert_eq!(
            theme_mode_from_value(Some(&json!("purple"))),
            ThemeMode::System
        );
    }

    #[test]
    fn parses_persisted_values() {
        assert_eq!(
            theme_mode_from_value(Some(&json!("light"))),
            ThemeMode::Light
        );
        assert_eq!(theme_mode_from_value(Some(&json!("dark"))), ThemeMode::Dark);
    }

    #[test]
    fn native_theme_mapping() {
        assert!(ThemeMode::System.native_theme().is_none());
        assert_eq!(ThemeMode::Light.native_theme(), Some(tauri::Theme::Light));
        assert_eq!(ThemeMode::Dark.native_theme(), Some(tauri::Theme::Dark));
    }
}
