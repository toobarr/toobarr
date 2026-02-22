# toobarr

Web application crate. Built with [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx) (SQLite), [Askama](https://github.com/djc/askama) templates, [HTMX](https://htmx.org/), and [Pico CSS](https://picocss.com/).

## Routes

### Pages

| Route | Handler |
|-------|---------|
| `GET /` | Home — recent downloads |
| `GET /channels` | Channel list |
| `GET /channels/new` | New channel form |
| `GET /channels/{id}` | Channel detail and video list |
| `GET /downloads` | Active and recent downloads |
| `GET /settings` | Settings form |

### API

| Route | Handler |
|-------|---------|
| `POST /api/channels` | Create channel |
| `DELETE /api/channels/{id}` | Delete channel |
| `POST /api/channels/{id}/sync` | Sync channel metadata from YouTube |
| `POST /api/videos/{id}/download` | Queue video for download |
| `POST /api/downloads/{id}/cancel` | Cancel a download |
| `POST /api/downloads/{id}/retry` | Retry a failed download |
| `GET /api/downloads/active` | Active download list (HTMX fragment) |
| `GET /api/downloads/count` | Active download count (HTMX fragment) |
| `POST /api/settings` | Update settings |
| `POST /api/settings/cookies` | Upload cookies file |
| `DELETE /api/settings/cookies` | Delete cookies file |

## Templating

Templates use [Askama](https://github.com/djc/askama) (Jinja2-like syntax, compiled at build time). Template files are in `templates/` and extend `base.html`. HTMX handles dynamic fragments — partial responses are rendered as standalone templates or inline HTML returned from API handlers.

## Project structure

```
src/
  main.rs        -- server setup and routing
  state.rs       -- shared application state (pool, yt-dlp client, download channel)
  db.rs          -- database pool initialization
  nfo.rs         -- NFO file generation and ffprobe integration
  thumbnail.rs   -- thumbnail fetching
  handlers/
    pages.rs     -- full page renders
    api.rs       -- API and HTMX fragment handlers
  models/        -- SQLx models (channels, videos, downloads, settings)
  workers/
    download.rs  -- background download worker
templates/       -- Askama HTML templates
migrations/      -- SQLite schema migrations
static/          -- CSS and static assets
```
