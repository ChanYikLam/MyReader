//! 图片资源解析：本地相对路径、远程 URL、尺寸探测（SPEC §5 第 5 步）。

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Url;

/// 图片加载方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    /// 本地文件（经 Tauri asset 协议加载）。
    Local,
    /// http/https 远程图片（由前端预加载，CSP 已允许）。
    Remote,
    /// 其它协议或无意义地址，不加载（data:/file:/javascript: 等）。
    Unsupported,
}

/// 一张图片的解析结果，前端按 `src` 与渲染后的 `<img>` 对应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageResource {
    /// Markdown 中的原始 src（与 HTML 里 `<img src>` 的值一致）。
    pub src: String,
    pub kind: ImageKind,
    /// 本地图片的绝对路径（kind = local 时存在）。
    pub path: Option<String>,
    /// 本地文件是否存在（缺失时前端用占位盒展示）。
    pub exists: bool,
    /// 原始像素尺寸；读不到（如 SVG/损坏文件）时为 None。
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// 解析文档里出现的全部图片（按文档顺序）。
pub fn resolve_images(document_path: &Path, srcs: &[String]) -> Vec<ImageResource> {
    // 相对文档路径（无父目录）时以当前工作目录为基准；Windows 上没有 "/" 根。
    let document_dir = document_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok());
    // 必须用目录 URL（带结尾斜杠），否则 join 会把最后一级目录替换掉。
    let base_url = document_dir
        .as_deref()
        .and_then(|dir| Url::from_directory_path(dir).ok());

    srcs.iter()
        .map(|src| resolve_image(src, base_url.as_ref()))
        .collect()
}

fn resolve_image(src: &str, base_url: Option<&Url>) -> ImageResource {
    let unsupported = ImageResource {
        src: src.to_string(),
        kind: ImageKind::Unsupported,
        path: None,
        exists: false,
        width: None,
        height: None,
    };

    let trimmed = src.trim();
    if trimmed.is_empty() {
        return unsupported;
    }

    // 绝对 URL：只支持 http/https（其余协议一律不加载）。
    if let Ok(url) = Url::parse(trimmed) {
        if matches!(url.scheme(), "http" | "https") {
            return ImageResource {
                src: src.to_string(),
                kind: ImageKind::Remote,
                path: None,
                exists: false,
                width: None,
                height: None,
            };
        }
        return unsupported;
    }

    // 相对路径：以文档所在目录为基准解析（支持 ./ ../ 与 URL 百分号编码）。
    let Some(base) = base_url else {
        return unsupported;
    };
    // Windows 上 Markdown 常用反斜杠写图片路径（如 images\a.png）；
    // 直接交给 URL join 会被当成非法字符，先归一为正斜杠。
    let relative = if cfg!(windows) {
        trimmed.replace('\\', "/")
    } else {
        trimmed.to_string()
    };
    let Ok(joined) = base.join(&relative) else {
        return unsupported;
    };
    let Ok(path) = joined.to_file_path() else {
        return unsupported;
    };

    let exists = path.is_file();
    let (width, height) = if exists {
        probe_dimensions(&path)
    } else {
        (None, None)
    };

    ImageResource {
        src: src.to_string(),
        kind: ImageKind::Local,
        path: Some(path.display().to_string()),
        exists,
        width,
        height,
    }
}

/// 只读图片文件头获取像素尺寸；SVG 等不支持的格式返回 (None, None)。
fn probe_dimensions(path: &Path) -> (Option<u32>, Option<u32>) {
    match imagesize::size(path) {
        Ok(size) => (
            u32::try_from(size.width).ok(),
            u32::try_from(size.height).ok(),
        ),
        Err(_) => (None, None),
    }
}

/// 本地图片路径（供命令层注册 asset 协议 scope）。
pub fn local_paths(resources: &[ImageResource]) -> impl Iterator<Item = PathBuf> + '_ {
    resources
        .iter()
        .filter(|resource| resource.kind == ImageKind::Local)
        .filter_map(|resource| resource.path.as_deref().map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    /// 1x1 透明 PNG。
    const SMALL_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn document_path(dir: &TempDir) -> PathBuf {
        dir.path().join("doc.md")
    }

    #[test]
    fn resolves_remote_image() {
        let dir = TempDir::new();
        let resources = resolve_images(
            &document_path(&dir),
            &["https://example.com/a.png".to_string()],
        );

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].kind, ImageKind::Remote);
        assert!(resources[0].path.is_none());
    }

    #[test]
    fn rejects_non_http_schemes() {
        let dir = TempDir::new();
        let resources = resolve_images(
            &document_path(&dir),
            &[
                "data:image/png;base64,AAAA".to_string(),
                "file:///etc/passwd".to_string(),
                "javascript:alert(1)".to_string(),
            ],
        );

        assert!(resources
            .iter()
            .all(|resource| resource.kind == ImageKind::Unsupported));
    }

    #[test]
    fn resolves_local_image_with_dimensions() {
        let dir = TempDir::new();
        std::fs::create_dir_all(dir.path().join("images")).unwrap();
        let image_path = dir.path().join("images/one.png");
        std::fs::write(&image_path, SMALL_PNG).unwrap();

        let resources = resolve_images(&document_path(&dir), &["images/one.png".to_string()]);

        assert_eq!(resources[0].kind, ImageKind::Local);
        assert!(resources[0].exists);
        assert_eq!(resources[0].width, Some(1));
        assert_eq!(resources[0].height, Some(1));
        let resolved = resources[0].path.as_deref().expect("本地图片应有路径");
        // 用 Path 比较，避免 Windows 反斜杠导致字符串断言失败。
        assert!(
            Path::new(resolved).ends_with(Path::new("images").join("one.png")),
            "path={resolved}"
        );
    }

    #[test]
    fn resolves_parent_directory_image() {
        let dir = TempDir::new();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        let image_path = dir.path().join("shared.png");
        std::fs::write(&image_path, SMALL_PNG).unwrap();

        let resources = resolve_images(
            &dir.path().join("docs/doc.md"),
            &["../shared.png".to_string()],
        );

        assert_eq!(resources[0].kind, ImageKind::Local);
        assert!(resources[0].exists);
    }

    /// Windows 上常见 `images\a.png` 写法，解析前要归一为正斜杠。
    #[cfg(windows)]
    #[test]
    fn resolves_backslash_relative_image_path() {
        let dir = TempDir::new();
        std::fs::create_dir_all(dir.path().join("images")).unwrap();
        std::fs::write(dir.path().join("images/one.png"), SMALL_PNG).unwrap();

        let resources = resolve_images(&document_path(&dir), &["images\\one.png".to_string()]);

        assert_eq!(resources[0].kind, ImageKind::Local);
        assert!(resources[0].exists);
    }

    #[test]
    fn missing_local_image_is_local_but_not_existing() {
        let dir = TempDir::new();
        let resources = resolve_images(&document_path(&dir), &["nope.png".to_string()]);

        assert_eq!(resources[0].kind, ImageKind::Local);
        assert!(!resources[0].exists);
        assert!(resources[0].path.is_some());
    }

    #[test]
    fn svg_has_no_dimensions() {
        let dir = TempDir::new();
        let svg_path = dir.path().join("icon.svg");
        std::fs::write(
            &svg_path,
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"></svg>"#,
        )
        .unwrap();

        let resources = resolve_images(&document_path(&dir), &["icon.svg".to_string()]);

        assert_eq!(resources[0].kind, ImageKind::Local);
        assert!(resources[0].exists);
        assert_eq!(resources[0].width, None);
        assert_eq!(resources[0].height, None);
    }

    #[test]
    fn resolves_percent_encoded_file_name() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("a b.png"), SMALL_PNG).unwrap();

        let resources = resolve_images(&document_path(&dir), &["a%20b.png".to_string()]);

        assert_eq!(resources[0].kind, ImageKind::Local);
        assert!(resources[0].exists);
    }

    #[test]
    fn local_paths_only_returns_local_files() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("a.png"), SMALL_PNG).unwrap();

        let resources = resolve_images(
            &document_path(&dir),
            &[
                "a.png".to_string(),
                "https://example.com/b.png".to_string(),
                "data:image/png;base64,AA".to_string(),
            ],
        );
        let paths: Vec<PathBuf> = local_paths(&resources).collect();

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("a.png"));
    }
}
