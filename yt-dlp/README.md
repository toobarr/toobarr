# yt-dlp

Async Rust wrapper for the [yt-dlp](https://github.com/yt-dlp/yt-dlp) CLI.

## Requirements

- yt-dlp must be installed and available in PATH (or specify a custom path)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
yt-dlp = { git = "https://forgejo.nw8.xyz/todd/yt-dlp" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

### Basic Example

```rust
use yt_dlp::{YtDlp, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = YtDlp::new();

    // Check yt-dlp is available
    let version = client.check_binary().await?;
    println!("yt-dlp version: {}", version);

    // Get video metadata
    let info = client.get_video_info("https://www.youtube.com/watch?v=dQw4w9WgXcQ").await?;
    println!("Title: {}", info.title);
    println!("Duration: {:?}s", info.duration);

    // Download video
    client.download("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "video.mp4").await?;

    Ok(())
}
```

### Download with Progress

```rust
use yt_dlp::{YtDlp, DownloadOptions, DownloadEvent};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> yt_dlp::Result<()> {
    let client = YtDlp::new();
    let options = DownloadOptions::new()
        .embed_metadata(true)
        .embed_thumbnail(true);

    let mut stream = client.download_with_progress(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "video.mp4",
        &options
    );

    while let Some(event) = stream.next().await {
        match event? {
            DownloadEvent::Progress(p) => {
                println!("{:.1}% - {}", p.percent.unwrap_or(0.0), p.format_speed().unwrap_or_default());
            }
            DownloadEvent::Finished { filename } => {
                println!("Downloaded: {}", filename);
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Audio Extraction

```rust
let client = YtDlp::new();
client.download_audio("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "audio.mp3").await?;
```

### Builder Pattern

```rust
let client = YtDlp::new();
client
    .build_download("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
    .embed_metadata(true)
    .embed_thumbnail(true)
    .download("video.mp4")
    .await?;
```

## API

### Client

- `YtDlp::new()` - Create client using yt-dlp from PATH
- `YtDlp::with_binary(path)` - Specify yt-dlp binary location
- `check_binary()` - Verify yt-dlp is available, returns version
- `get_video_info(url)` - Extract metadata without downloading
- `get_playlist_info(url)` - Extract playlist metadata
- `list_formats(url)` - List available formats
- `download(url, output)` - Simple download
- `download_with_options(url, output, options)` - Download with options
- `download_with_progress(url, output, options)` - Returns async stream of events
- `download_audio(url, output)` - Extract audio as MP3
- `build_download(url)` - Fluent builder pattern

### Types

- `VideoInfo` - Video metadata (title, duration, formats, thumbnails, etc.)
- `PlaylistInfo` - Playlist metadata with entries
- `Format` - Available format details (resolution, codecs, filesize)
- `DownloadOptions` - Configuration (format, container, embed options, etc.)
- `DownloadEvent` - Progress stream events
- `DownloadProgress` - Download stats (bytes, speed, ETA, percent)

## License

MIT
