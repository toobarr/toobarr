//! Async Rust wrapper for yt-dlp CLI.
//!
//! This library provides an async interface to the yt-dlp command-line tool
//! for downloading videos and extracting metadata from various video platforms.
//!
//! # Example
//!
//! ```no_run
//! use yt_dlp::{YtDlp, DownloadOptions};
//!
//! #[tokio::main]
//! async fn main() -> yt_dlp::Result<()> {
//!     let client = YtDlp::new();
//!
//!     // Check that yt-dlp is available
//!     let version = client.check_binary().await?;
//!     println!("yt-dlp version: {}", version);
//!
//!     // Get video info without downloading
//!     let info = client.get_video_info("https://www.youtube.com/watch?v=dQw4w9WgXcQ").await?;
//!     println!("Title: {}", info.title);
//!
//!     // Download a video
//!     client.download("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "video.mp4").await?;
//!
//!     Ok(())
//! }
//! ```

mod client;
mod command;
pub mod error;
pub mod types;

pub use client::{DownloadBuilder, YtDlp};
pub use error::{Error, Result};
pub use types::{
    Chapter, Container, DownloadEvent, DownloadOptions, DownloadProgress, Format, OutputFormat,
    PlaylistInfo, Thumbnail, VideoInfo
};
