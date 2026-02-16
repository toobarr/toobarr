use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("yt-dlp binary not found: {0}")]
    BinaryNotFound(PathBuf),

    #[error("yt-dlp binary not executable: {0}")]
    BinaryNotExecutable(PathBuf),

    #[error("failed to execute yt-dlp: {0}")]
    ExecutionFailed(#[from] std::io::Error),

    #[error("yt-dlp command failed with exit code {code}: {stderr}")]
    CommandFailed { code: i32, stderr: String },

    #[error("failed to parse JSON output: {0}")]
    JsonParseFailed(#[from] serde_json::Error),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("download failed: {0}")]
    DownloadFailed(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("no formats available")]
    NoFormatsAvailable,

    #[error("video unavailable: {0}")]
    VideoUnavailable(String),

    #[error("playlist is empty")]
    EmptyPlaylist,

    #[error("operation cancelled")]
    Cancelled
}

pub type Result<T> = std::result::Result<T, Error>;
