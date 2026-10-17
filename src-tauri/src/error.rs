use serde::ser::{Serialize, SerializeStruct, Serializer};

/// 应用统一错误类型。
///
/// 内部用 thiserror 保留结构化原因；转 IPC 时序列化为
/// `{ code, message, detail }`：`message` 是可直接展示给用户的中文文案，
/// `code` 供前端按类别处理（SPEC §5/§9.6）。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("文件不存在：{path}")]
    NotFound { path: String },

    #[error("文件过大（{size_mb} MB），上限为 {limit_mb} MB")]
    TooLarge { size_mb: u64, limit_mb: u64 },

    #[error("文件不是文本（包含二进制内容）：{path}")]
    BinaryFile { path: String },

    #[error("不支持的文件类型：{path}（支持 .md/.markdown/.mdown/.txt）")]
    UnsupportedType { path: String },

    #[error("不是普通文件：{path}")]
    NotAFile { path: String },

    #[error("无法读取文件：{path}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("无法打开文件选择对话框")]
    Dialog { detail: String },

    #[error("内部错误")]
    Internal { detail: String },
}

impl AppError {
    /// 稳定的错误码，前端可据此分支处理。
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "not_found",
            Self::TooLarge { .. } => "too_large",
            Self::BinaryFile { .. } => "binary_file",
            Self::UnsupportedType { .. } => "unsupported_type",
            Self::NotAFile { .. } => "not_a_file",
            Self::Io { .. } => "io_error",
            Self::Dialog { .. } => "dialog",
            Self::Internal { .. } => "internal",
        }
    }

    fn detail(&self) -> Option<String> {
        match self {
            Self::Io { source, .. } => Some(source.to_string()),
            Self::Dialog { detail } => Some(detail.clone()),
            Self::Internal { detail } => Some(detail.clone()),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 3)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("detail", &self.detail())?;
        state.end()
    }
}
