# yt-dlp

Async Rust wrapper for the [yt-dlp](https://github.com/yt-dlp/yt-dlp) CLI.

## API

### `YtDlp`

| Method | Description |
|--------|-------------|
| `YtDlp::new()` | Create client using `yt-dlp` from `PATH` |
| `YtDlp::with_binary(path)` | Specify `yt-dlp` binary path |
| `set_binary(path)` | Change binary path |
| `set_cookies_file(path)` | Set Netscape cookies file |
| `set_extra_args(args)` | Set additional CLI arguments |
| `set_ffmpeg_location(path)` | Set ffmpeg binary path |
| `set_env(key, value)` | Set environment variable for subprocess |
| `check_binary()` | Verify `yt-dlp` is available, returns version string |
| `get_video_info(url)` | Fetch video metadata without downloading |
| `get_playlist_info(url)` | Fetch playlist metadata and entries |
| `list_formats(url)` | List available download formats |
| `download(url, output)` | Download to file |
| `download_with_options(url, output, options)` | Download with `DownloadOptions` |
| `download_with_progress(url, output, options)` | Returns a `Stream<DownloadEvent>` |
| `download_audio(url, output)` | Download and extract audio as MP3 |
| `build_download(url)` | Fluent `DownloadBuilder` |

### `DownloadBuilder`

Constructed via `YtDlp::build_download(url)`. Chainable methods: `format`, `container`, `output_template`, `embed_thumbnail`, `embed_metadata`, `embed_subtitles`, `extract_audio`, `audio_format`, `audio_quality`, `cookies_file`, `rate_limit`. Terminates with `download(output)` or `download_with_progress(output)`.

### Types

| Type | Description |
|------|-------------|
| `VideoInfo` | Video metadata (title, duration, formats, thumbnails, etc.) |
| `PlaylistInfo` | Playlist metadata with `entries: Vec<VideoInfo>` |
| `Format` | Format details (resolution, codecs, filesize) |
| `DownloadOptions` | Download configuration |
| `DownloadEvent` | Progress stream event (see variants below) |
| `DownloadProgress` | Download stats (bytes, speed, ETA, percent) |
| `OutputFormat` | Enum: `BestVideo`, `BestAudio`, `Custom(String)` |
| `Container` | Enum: `Mp4`, `Mkv`, `Webm` |

### `DownloadEvent` variants

`Extracting`, `DownloadStarted`, `Progress(DownloadProgress)`, `MergingFormats`, `EmbeddingThumbnail`, `EmbeddingMetadata`, `PostProcessing`, `Warning`, `Error`, `Finished`
