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

// ---------------------------------------------------------------------------
// 字号 / 侧栏 / 最近文件 / 滚动位置（SPEC §11）
// ---------------------------------------------------------------------------

const FONT_SIZE_KEY: &str = "typography.fontSize";
const SIDEBAR_OPEN_KEY: &str = "ui.sidebarOpen";
const RECENT_KEY: &str = "recent";
const SCROLL_KEY: &str = "scroll";

pub const DEFAULT_FONT_SIZE: u32 = 17;
pub const MIN_FONT_SIZE: u32 = 13;
pub const MAX_FONT_SIZE: u32 = 24;
/// 「文件 → 打开最近」最多保留条数。
pub const MAX_RECENT_FILES: usize = 10;
/// 滚动位置记录上限（按最近使用 LRU 淘汰）。
const MAX_SCROLL_ENTRIES: usize = 200;

/// 最近打开文件条目（File → Open Recent 与欢迎页共用）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    /// Unix 毫秒时间戳，用于排序/去重。
    pub opened_at: u64,
}

/// 每个文件的滚动位置记忆。
///
/// 同时记录文档高度与视口宽度：窗口尺寸/字号变化导致高度变化时，
/// 前端可按比例回退到近似位置（SPEC §11）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScrollPosition {
    pub top: f64,
    pub doc_height: f64,
    pub viewport_width: f64,
    pub updated_at: u64,
}

pub fn clamp_font_size(size: u32) -> u32 {
    size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
}

fn open_store(
    app: &AppHandle,
) -> Result<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>, AppError> {
    app.store(STORE_FILE).map_err(|error| AppError::Settings {
        detail: error.to_string(),
    })
}

fn save_store(store: &tauri_plugin_store::Store<tauri::Wry>) -> Result<(), AppError> {
    store.save().map_err(|error| AppError::Settings {
        detail: error.to_string(),
    })
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

// ---- 字号 ----

pub fn load_font_size(app: &AppHandle) -> u32 {
    match open_store(app) {
        Ok(store) => font_size_from_value(store.get(FONT_SIZE_KEY).as_ref()),
        Err(_) => DEFAULT_FONT_SIZE,
    }
}

pub fn save_font_size(app: &AppHandle, size: u32) -> Result<(), AppError> {
    let store = open_store(app)?;
    store.set(FONT_SIZE_KEY, serde_json::json!(clamp_font_size(size)));
    save_store(&store)
}

pub fn font_size_from_value(value: Option<&serde_json::Value>) -> u32 {
    value
        .and_then(serde_json::Value::as_u64)
        .map(|size| clamp_font_size(u32::try_from(size).unwrap_or(DEFAULT_FONT_SIZE)))
        .unwrap_or(DEFAULT_FONT_SIZE)
}

// ---- 侧栏 ----

/// None = 尚未手动设置，前端按窗口宽度决定默认展开/收起（SPEC §18 A3）。
pub fn load_sidebar_open(app: &AppHandle) -> Option<bool> {
    let store = open_store(app).ok()?;
    store
        .get(SIDEBAR_OPEN_KEY)
        .and_then(|value| value.as_bool())
}

pub fn save_sidebar_open(app: &AppHandle, open: bool) -> Result<(), AppError> {
    let store = open_store(app)?;
    store.set(SIDEBAR_OPEN_KEY, serde_json::json!(open));
    save_store(&store)
}

// ---- 最近文件 ----

pub fn load_recent(app: &AppHandle) -> Vec<RecentFile> {
    match open_store(app) {
        Ok(store) => recent_from_value(store.get(RECENT_KEY).as_ref()),
        Err(_) => Vec::new(),
    }
}

pub fn recent_from_value(value: Option<&serde_json::Value>) -> Vec<RecentFile> {
    let Some(items) = value.and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };

    let mut files: Vec<RecentFile> = items
        .iter()
        .filter_map(|item| serde_json::from_value(item.clone()).ok())
        .collect();
    files.sort_by_key(|file| std::cmp::Reverse(file.opened_at));
    files.dedup_by(|left, right| left.path == right.path);
    files.truncate(MAX_RECENT_FILES);
    files
}

/// 记录一次成功打开：去重、置顶、截断到 10 条，返回更新后的列表。
pub fn push_recent(app: &AppHandle, path: &str, name: &str) -> Result<Vec<RecentFile>, AppError> {
    let store = open_store(app)?;
    let mut files = recent_from_value(store.get(RECENT_KEY).as_ref());
    let updated = updated_recent(files.split_off(0), path, name, now_unix_ms());
    store.set(
        RECENT_KEY,
        serde_json::to_value(&updated).unwrap_or_default(),
    );
    save_store(&store)?;
    Ok(updated)
}

pub fn updated_recent(
    mut files: Vec<RecentFile>,
    path: &str,
    name: &str,
    opened_at: u64,
) -> Vec<RecentFile> {
    files.retain(|file| file.path != path);
    files.insert(
        0,
        RecentFile {
            path: path.to_string(),
            name: name.to_string(),
            opened_at,
        },
    );
    files.truncate(MAX_RECENT_FILES);
    files
}

pub fn clear_recent(app: &AppHandle) -> Result<(), AppError> {
    let store = open_store(app)?;
    store.delete(RECENT_KEY);
    save_store(&store)
}

// ---- 滚动位置 ----

pub fn load_scroll(app: &AppHandle, path: &str) -> Option<ScrollPosition> {
    let store = open_store(app).ok()?;
    let map = store.get(SCROLL_KEY)?;
    scroll_from_value(&map, path)
}

pub fn scroll_from_value(map: &serde_json::Value, path: &str) -> Option<ScrollPosition> {
    serde_json::from_value(map.get(path)?.clone()).ok()
}

pub fn save_scroll(
    app: &AppHandle,
    path: &str,
    mut position: ScrollPosition,
) -> Result<(), AppError> {
    let store = open_store(app)?;
    let mut map = store
        .get(SCROLL_KEY)
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    position.updated_at = now_unix_ms();
    map.insert(
        path.to_string(),
        serde_json::to_value(position).unwrap_or_default(),
    );

    // 超出上限时按 updatedAt 淘汰最久未用的记录。
    if map.len() > MAX_SCROLL_ENTRIES {
        let mut entries: Vec<(String, u64)> = map
            .iter()
            .map(|(key, value)| {
                let updated = value
                    .get("updatedAt")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default();
                (key.clone(), updated)
            })
            .collect();
        entries.sort_by_key(|(_, updated)| *updated);
        let remove_count = map.len() - MAX_SCROLL_ENTRIES;
        for (key, _) in entries.into_iter().take(remove_count) {
            map.remove(&key);
        }
    }

    store.set(SCROLL_KEY, serde_json::Value::Object(map));
    save_store(&store)
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

    #[test]
    fn font_size_clamps_to_supported_range() {
        assert_eq!(font_size_from_value(None), DEFAULT_FONT_SIZE);
        assert_eq!(font_size_from_value(Some(&json!(11))), MIN_FONT_SIZE);
        assert_eq!(font_size_from_value(Some(&json!(40))), MAX_FONT_SIZE);
        assert_eq!(font_size_from_value(Some(&json!(19))), 19);
        assert_eq!(font_size_from_value(Some(&json!("19"))), DEFAULT_FONT_SIZE);
    }

    #[test]
    fn recent_dedupes_and_caps_at_ten() {
        let mut files: Vec<RecentFile> = (0..12)
            .map(|index| RecentFile {
                path: format!("/tmp/{index}.md"),
                name: format!("{index}.md"),
                opened_at: index,
            })
            .collect();

        files = updated_recent(files, "/tmp/3.md", "3.md", 99);
        assert_eq!(files.len(), MAX_RECENT_FILES);
        assert_eq!(files[0].path, "/tmp/3.md");
        assert_eq!(files[0].opened_at, 99);
        assert_eq!(
            files.iter().filter(|file| file.path == "/tmp/3.md").count(),
            1
        );
    }

    #[test]
    fn recent_from_value_sorts_and_ignores_garbage() {
        let value = json!([
            { "path": "/a.md", "name": "a.md", "openedAt": 1 },
            { "path": "/b.md", "name": "b.md", "openedAt": 5 },
            { "oops": true }
        ]);

        let files = recent_from_value(Some(&value));
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "/b.md");
        assert_eq!(recent_from_value(None).len(), 0);
    }

    #[test]
    fn scroll_positions_roundtrip_through_value() {
        let value = json!({
            "/tmp/a.md": {
                "top": 1234.5,
                "docHeight": 5600.0,
                "viewportWidth": 1000.0,
                "updatedAt": 42
            }
        });

        let position = scroll_from_value(&value, "/tmp/a.md").expect("应能读取");
        assert_eq!(position.top, 1234.5);
        assert_eq!(position.doc_height, 5600.0);
        assert_eq!(position.updated_at, 42);
        assert!(scroll_from_value(&value, "/tmp/missing.md").is_none());
    }
}
