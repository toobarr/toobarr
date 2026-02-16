# Tubarr

A self-hosted YouTube video downloader and media organizer. Manages channels, syncs video metadata, downloads videos via yt-dlp, and generates Jellyfin/Kodi-compatible NFO files with thumbnails.

Built with Rust (Axum + SQLx/SQLite), HTMX, and Pico CSS.

## Prerequisites

### Runtime dependencies

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) -- must be on `PATH`
- [ffmpeg/ffprobe](https://ffmpeg.org/) -- used by yt-dlp for merging formats and by Tubarr for media info in NFO files

### Build dependencies

- Rust 1.70+ (stable)
- SQLite3 development libraries (`libsqlite3-dev` on Debian/Ubuntu, `sqlite` on macOS via Homebrew)

## Building

The project depends on a local `yt-dlp` Rust crate located at `../yt-dlp` relative to this directory. Clone or place it there before building.

```
cargo build --release
```

The binary is at `target/release/toobarr`.

## Running

```
./target/release/toobarr
```

The web UI is served at `http://localhost:8000`.

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8000` | HTTP listen port |
| `DATABASE_PATH` | `./toobarr.db` | Path to the SQLite database file |

The database and migrations are applied automatically on startup.

### Runtime settings

These are configured through the Settings page in the web UI and stored in the SQLite database:

| Setting | Default | Description |
|---------|---------|-------------|
| Download path | `./downloads` | Directory where videos are saved (organized by channel) |
| Max concurrent downloads | `2` | Number of simultaneous downloads |
| Extractor args | (empty) | Extra arguments passed to yt-dlp (e.g. PO token provider config) |
| Cookies file | (empty) | Path to a Netscape-format cookies file for authenticated downloads |

## Docker

The Dockerfile builds from the parent directory (to include the `yt-dlp` crate dependency). A `docker-compose.yml` is provided:

```
docker compose up -d
```

This starts Tubarr on port 8000 with persistent volumes for the database (`./data`) and downloaded videos (`./downloads`). It also runs a [bgutil-ytdlp-pot-provider](https://github.com/Brainicism/bgutil-ytdlp-pot-provider) sidecar for PO token generation.

To build and run manually:

```
docker build -t toobarr -f toobarr/Dockerfile ..
docker run -p 8000:8000 -v ./data:/app/data -v ./downloads:/app/downloads -e DATABASE_PATH=/app/data/toobarr.db toobarr
```

## Testing

```
cargo test
```

## Project structure

```
src/
  main.rs             -- server setup and routing
  state.rs            -- shared application state
  db.rs               -- database pool initialization
  nfo.rs              -- NFO file generation and ffprobe integration
  thumbnail.rs        -- thumbnail downloading and caching
  handlers/
    api.rs            -- JSON/HTML fragment API handlers
    pages.rs          -- full page renders
    sse.rs            -- server-sent events for download progress
  models/             -- database models (channels, videos, downloads, settings)
  workers/
    download.rs       -- background download worker with cancel support
templates/            -- Askama HTML templates
migrations/           -- SQLite schema migrations
static/               -- CSS and static assets
```
