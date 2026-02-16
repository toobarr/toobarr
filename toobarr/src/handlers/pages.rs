use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html
};
use sqlx::Row;

use crate::error::AppError;
use crate::handlers::api::check_binary_version;
use crate::models::{Channel, Download, DownloadWithVideo, Settings, Video};
use crate::state::AppState;

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    channel_count: i64,
    video_count: i64,
    active_downloads: i64,
    completed_downloads: i64,
    recent_downloads: Vec<DownloadWithVideo>
}

#[derive(Template)]
#[template(path = "channels/index.html")]
struct ChannelsTemplate {
    channels: Vec<Channel>
}

#[derive(Template)]
#[template(path = "channels/new.html")]
struct NewChannelTemplate;

#[derive(Template)]
#[template(path = "channels/detail.html")]
struct ChannelDetailTemplate {
    channel: Channel,
    videos: Vec<Video>,
    download_statuses: HashMap<String, String>
}

#[derive(Template)]
#[template(path = "downloads.html")]
struct DownloadsTemplate {
    downloads: Vec<DownloadWithVideo>
}

pub struct BinaryStatus {
    pub name: String,
    pub setting_key: String,
    pub path: String,
    pub version: Option<String>,
    pub available: bool
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
    download_path: String,
    max_concurrent_downloads: usize,
    extractor_args: String,
    has_cookies: bool,
    binaries: Vec<BinaryStatus>
}

#[tracing::instrument(skip(state))]
pub async fn home_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let channel_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM channels")
        .fetch_one(&state.pool)
        .await?
        .get("count");

    let video_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM videos")
        .fetch_one(&state.pool)
        .await?
        .get("count");

    let active_downloads: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM downloads WHERE status IN ('pending', 'downloading')")
            .fetch_one(&state.pool)
            .await?
            .get("count");

    let completed_downloads: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM downloads WHERE status = 'completed'")
            .fetch_one(&state.pool)
            .await?
            .get("count");

    let all_downloads = Download::find_all_with_video(&state.pool).await?;
    let recent_downloads: Vec<_> = all_downloads.into_iter().take(5).collect();

    let template = HomeTemplate {
        channel_count,
        video_count,
        active_downloads,
        completed_downloads,
        recent_downloads
    };
    Ok(Html(template.render()?))
}

#[tracing::instrument(skip(state))]
pub async fn channels_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let channels = Channel::find_all(&state.pool).await?;
    let template = ChannelsTemplate { channels };
    Ok(Html(template.render()?))
}

#[tracing::instrument]
pub async fn new_channel_page() -> Result<Html<String>, AppError> {
    let template = NewChannelTemplate;
    Ok(Html(template.render()?))
}

#[tracing::instrument(skip(state))]
pub async fn channel_detail_page(
    State(state): State<AppState>,
    Path(id): Path<String>
) -> Result<Html<String>, AppError> {
    let channel = Channel::find_by_id(&state.pool, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Channel not found"))?;

    let videos = Video::find_by_channel(&state.pool, &id).await?;

    let rows = sqlx::query(
        r"SELECT d.video_id, d.status FROM downloads d
          WHERE d.video_id IN (SELECT v.id FROM videos v WHERE v.channel_id = ?)
          AND d.id = (SELECT d2.id FROM downloads d2 WHERE d2.video_id = d.video_id ORDER BY d2.created_at DESC LIMIT 1)"
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    let mut download_statuses = HashMap::new();
    for row in rows {
        let video_id: String = row.get("video_id");
        let status: String = row.get("status");
        download_statuses.insert(video_id, status);
    }

    let template = ChannelDetailTemplate { channel, videos, download_statuses };
    Ok(Html(template.render()?))
}

#[tracing::instrument(skip(state))]
pub async fn downloads_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let downloads = Download::find_all_with_video(&state.pool).await?;
    let template = DownloadsTemplate { downloads };
    Ok(Html(template.render()?))
}

#[tracing::instrument(skip(state))]
pub async fn settings_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let download_path = Settings::get_download_path(&state.pool).await?;
    let max_concurrent_downloads = Settings::get_max_concurrent_downloads(&state.pool).await?;
    let extractor_args = Settings::get_extractor_args(&state.pool).await?;
    let cookies_file = Settings::get_cookies_file(&state.pool).await?.unwrap_or_default();
    let has_cookies = !cookies_file.is_empty()
        && std::path::Path::new(&cookies_file).exists();

    let binary_configs = [
        ("yt-dlp", "ytdlp_path", "yt-dlp"),
        ("ffmpeg", "ffmpeg_path", "ffmpeg"),
        ("ffprobe", "ffprobe_path", "ffprobe"),
        ("deno", "deno_path", "deno")
    ];

    let mut binaries = Vec::new();
    for (name, setting_key, default_bin) in binary_configs {
        let custom_path = Settings::get(&state.pool, setting_key)
            .await
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let bin_path = custom_path.unwrap_or_else(|| default_bin.to_string());
        let version = check_binary_version(&bin_path).await;
        let available = version.is_some();
        binaries.push(BinaryStatus {
            name: name.to_string(),
            setting_key: setting_key.to_string(),
            path: if bin_path == default_bin { String::new() } else { bin_path },
            version,
            available
        });
    }

    let template = SettingsTemplate {
        download_path,
        max_concurrent_downloads,
        extractor_args,
        has_cookies,
        binaries
    };
    Ok(Html(template.render()?))
}
