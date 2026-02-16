use std::path::PathBuf;

use axum::{
    extract::{Form, Multipart, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Redirect, Response}
};
use serde::Deserialize;

use crate::error::AppError;
use crate::models::{Channel, CreateChannel, Download, DownloadStatus, Settings, Video};
use crate::state::AppState;
use crate::thumbnail;
use crate::workers::download::{DownloadCommand, VideoMeta};

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    download_path: String,
    max_concurrent_downloads: String,
    extractor_args: Option<String>,
    ffmpeg_path: Option<String>,
    ffprobe_path: Option<String>,
    ytdlp_path: Option<String>,
    deno_path: Option<String>
}

#[tracing::instrument(skip(state))]
pub async fn create_channel(
    State(state): State<AppState>,
    Form(input): Form<CreateChannel>
) -> Result<Response, AppError> {
    tracing::info!("Fetching channel info for URL: {}", input.url);

    let yt_dlp = state.yt_dlp.read().await.clone();
    let playlist_info = yt_dlp
        .get_playlist_info(&input.url)
        .await
        .map_err(|e| AppError::bad_request(format!("Failed to fetch channel: {e}")))?;

    let channel_id = playlist_info.channel_id.clone().unwrap_or_else(|| playlist_info.id.clone());

    if let Some(existing) = Channel::find_by_youtube_id(&state.pool, &channel_id).await? {
        return Ok(Redirect::to(&format!("/channels/{}", existing.id)).into_response());
    }

    let id = uuid7::uuid7().to_string();
    let name = playlist_info
        .channel
        .clone()
        .or(playlist_info.title.clone())
        .unwrap_or_else(|| "Unknown Channel".to_string());

    let thumbnail_url = playlist_info
        .entries
        .first()
        .and_then(|v| v.best_thumbnail().map(String::from));

    Channel::insert(
        &state.pool,
        &id,
        &channel_id,
        &name,
        &input.url,
        None,
        playlist_info.description.as_deref()
    )
    .await?;

    if let Some(thumb_url) = thumbnail_url {
        match thumbnail::download_channel_thumbnail(&id, &thumb_url).await {
            Ok(local_path) => {
                if let Err(e) = Channel::update_thumbnail(&state.pool, &id, &local_path).await {
                    tracing::warn!("Failed to update channel thumbnail: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to download channel thumbnail: {}", e);
            }
        }
    }

    let video_count = sync_channel_videos(&state, &id, &playlist_info.entries).await?;

    let now = chrono::Utc::now().to_rfc3339();
    Channel::update_sync_info(&state.pool, &id, video_count, &now).await?;

    tracing::info!("Created channel {} with {} videos", name, video_count);

    Ok(Redirect::to(&format!("/channels/{id}")).into_response())
}

#[tracing::instrument(skip(state))]
pub async fn delete_channel(
    State(state): State<AppState>,
    Path(id): Path<String>
) -> Result<Response, AppError> {
    let deleted = Channel::delete(&state.pool, &id).await?;

    if deleted {
        Ok(Redirect::to("/channels").into_response())
    } else {
        Err(AppError::not_found("Channel not found"))
    }
}

#[tracing::instrument(skip(state))]
pub async fn sync_channel(
    State(state): State<AppState>,
    Path(id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let channel = Channel::find_by_id(&state.pool, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Channel not found"))?;

    tracing::info!("Syncing channel: {}", channel.name);

    let yt_dlp = state.yt_dlp.read().await.clone();
    let playlist_info = yt_dlp
        .get_playlist_info(&channel.url)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch channel: {e}")))?;

    let video_count = sync_channel_videos(&state, &id, &playlist_info.entries).await?;

    let now = chrono::Utc::now().to_rfc3339();
    Channel::update_sync_info(&state.pool, &id, video_count, &now).await?;

    tracing::info!("Synced {} videos for channel {}", video_count, channel.name);

    Ok((StatusCode::OK, Html("Sync complete")))
}

async fn sync_channel_videos(
    state: &AppState,
    channel_id: &str,
    entries: &[yt_dlp::VideoInfo]
) -> Result<i64, AppError> {
    let mut count = 0i64;

    for entry in entries {
        let video_id = uuid7::uuid7().to_string();

        #[allow(clippy::cast_possible_truncation)]
        let duration_seconds = entry.duration.map(|d| d as i64);
        #[allow(clippy::cast_possible_wrap)]
        let view_count = entry.view_count.map(|v| v as i64);

        let webpage_url = entry
            .webpage_url
            .clone()
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", entry.id));

        let local_thumbnail = if let Some(thumb_url) = entry.best_thumbnail() {
            match thumbnail::download_video_thumbnail(&entry.id, thumb_url).await {
                Ok(path) => Some(path),
                Err(e) => {
                    tracing::warn!("Failed to download thumbnail for {}: {}", entry.id, e);
                    None
                }
            }
        } else {
            None
        };

        Video::upsert(
            &state.pool,
            &video_id,
            channel_id,
            &entry.id,
            &entry.title,
            entry.description.as_deref(),
            local_thumbnail.as_deref(),
            duration_seconds,
            entry.upload_date.as_deref(),
            view_count,
            &webpage_url
        )
        .await?;

        count += 1;
    }

    Ok(count)
}

#[tracing::instrument(skip(state))]
pub async fn start_download(
    State(state): State<AppState>,
    Path(video_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let video = Video::find_by_id(&state.pool, &video_id)
        .await?
        .ok_or_else(|| AppError::not_found("Video not found"))?;

    let channel = Channel::find_by_id(&state.pool, &video.channel_id)
        .await?
        .ok_or_else(|| AppError::not_found("Channel not found"))?;

    if let Some(existing) = Download::find_by_video_id(&state.pool, &video_id).await? {
        match existing.status_enum() {
            DownloadStatus::Pending | DownloadStatus::Downloading => {
                return Ok((StatusCode::OK, Html("Download already in progress")));
            }
            DownloadStatus::Completed => {
                return Ok((StatusCode::OK, Html("Video already downloaded")));
            }
            DownloadStatus::Failed => {}
        }
    }

    let download_id = uuid7::uuid7().to_string();
    Download::insert(&state.pool, &download_id, &video_id).await?;

    let video_meta = VideoMeta {
        youtube_id: video.youtube_id,
        title: video.title.clone(),
        description: video.description,
        duration_seconds: video.duration_seconds,
        upload_date: video.upload_date
    };

    state
        .download_tx
        .send(DownloadCommand::Start {
            download_id: download_id.clone(),
            video_url: video.webpage_url,
            channel_name: channel.name,
            video_meta
        })
        .await
        .map_err(|e| AppError::internal(format!("Failed to queue download: {e}")))?;

    tracing::info!("Queued download {} for video {}", download_id, video.title);

    Ok((StatusCode::ACCEPTED, Html("Download queued")))
}

#[tracing::instrument(skip(state))]
pub async fn cancel_download(
    State(state): State<AppState>,
    Path(download_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let download = Download::find_by_id(&state.pool, &download_id)
        .await?
        .ok_or_else(|| AppError::not_found("Download not found"))?;

    if download.status_enum() != DownloadStatus::Downloading {
        return Err(AppError::bad_request("Download is not in progress"));
    }

    state
        .download_tx
        .send(DownloadCommand::Cancel {
            download_id: download_id.clone()
        })
        .await
        .map_err(|e| AppError::internal(format!("Failed to cancel download: {e}")))?;

    Download::update_status(&state.pool, &download_id, DownloadStatus::Failed).await?;
    Download::update_failed(&state.pool, &download_id, "Cancelled by user").await?;

    Ok((StatusCode::OK, Html("Download cancelled")))
}

#[tracing::instrument(skip(state))]
pub async fn retry_download(
    State(state): State<AppState>,
    Path(download_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let download = Download::find_by_id(&state.pool, &download_id)
        .await?
        .ok_or_else(|| AppError::not_found("Download not found"))?;

    if download.status_enum() != DownloadStatus::Failed {
        return Err(AppError::bad_request("Download has not failed"));
    }

    let video = Video::find_by_id(&state.pool, &download.video_id)
        .await?
        .ok_or_else(|| AppError::not_found("Video not found"))?;

    let channel = Channel::find_by_id(&state.pool, &video.channel_id)
        .await?
        .ok_or_else(|| AppError::not_found("Channel not found"))?;

    Download::update_status(&state.pool, &download_id, DownloadStatus::Pending).await?;

    let video_meta = VideoMeta {
        youtube_id: video.youtube_id,
        title: video.title,
        description: video.description,
        duration_seconds: video.duration_seconds,
        upload_date: video.upload_date
    };

    state
        .download_tx
        .send(DownloadCommand::Start {
            download_id: download_id.clone(),
            video_url: video.webpage_url,
            channel_name: channel.name,
            video_meta
        })
        .await
        .map_err(|e| AppError::internal(format!("Failed to retry download: {e}")))?;

    Ok((StatusCode::OK, Html("Download retrying")))
}

pub async fn active_downloads(
    State(state): State<AppState>
) -> Json<serde_json::Value> {
    let states = state.download_states.read().await;
    let active_count = states.values().filter(|s| {
        s.status == "started" || s.status == "progress" || s.status == "processing"
    }).count();
    Json(serde_json::json!({
        "downloads": *states,
        "active_count": active_count
    }))
}

pub async fn download_count(
    State(state): State<AppState>
) -> Html<String> {
    let states = state.download_states.read().await;
    let count = states.values().filter(|s| {
        s.status == "started" || s.status == "progress" || s.status == "processing"
    }).count();
    if count > 0 {
        Html(format!(r#"<span class="badge">{count}</span>"#))
    } else {
        Html(String::new())
    }
}

#[tracing::instrument(skip(state))]
pub async fn update_settings(
    State(state): State<AppState>,
    Form(input): Form<SettingsForm>
) -> Result<impl IntoResponse, AppError> {
    Settings::set(&state.pool, "download_path", &input.download_path).await?;
    Settings::set(
        &state.pool,
        "max_concurrent_downloads",
        &input.max_concurrent_downloads
    )
    .await?;

    if let Some(ref args_str) = input.extractor_args {
        Settings::set(&state.pool, "extractor_args", args_str).await?;
        let parsed = parse_extractor_args(args_str);
        let mut yt_dlp = state.yt_dlp.write().await;
        yt_dlp.set_extra_args(parsed);
    }

    if let Some(ref path) = input.ffmpeg_path {
        Settings::set(&state.pool, "ffmpeg_path", path).await?;
        let mut yt_dlp = state.yt_dlp.write().await;
        if path.is_empty() {
            yt_dlp.set_ffmpeg_location(None);
        } else {
            yt_dlp.set_ffmpeg_location(Some(PathBuf::from(path)));
        }
    }

    if let Some(ref path) = input.ffprobe_path {
        Settings::set(&state.pool, "ffprobe_path", path).await?;
    }

    if let Some(ref path) = input.ytdlp_path {
        Settings::set(&state.pool, "ytdlp_path", path).await?;
        let mut yt_dlp = state.yt_dlp.write().await;
        if path.is_empty() {
            yt_dlp.set_binary(PathBuf::from("yt-dlp"));
        } else {
            yt_dlp.set_binary(PathBuf::from(path));
        }
    }

    if let Some(ref path) = input.deno_path {
        Settings::set(&state.pool, "deno_path", path).await?;
        if !path.is_empty() {
            if let Some(parent) = std::path::Path::new(path).parent() {
                let mut yt_dlp = state.yt_dlp.write().await;
                yt_dlp.set_env("PATH_PREPEND".to_string(), parent.to_string_lossy().to_string());
            }
        }
    }

    tracing::info!("Updated settings");

    Ok((StatusCode::OK, Html("Settings saved")))
}

#[tracing::instrument(skip(state, multipart))]
pub async fn upload_cookies(
    State(state): State<AppState>,
    mut multipart: Multipart
) -> Result<impl IntoResponse, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("Invalid upload: {e}")))?
    {
        if field.name() == Some("cookies_file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("Failed to read file: {e}")))?;

            if data.is_empty() {
                return Err(AppError::bad_request("Empty file"));
            }

            let cookies_dir = PathBuf::from("./data");
            tokio::fs::create_dir_all(&cookies_dir)
                .await
                .map_err(|e| AppError::internal(format!("Failed to create data dir: {e}")))?;

            let cookies_path = cookies_dir.join("cookies.txt");
            tokio::fs::write(&cookies_path, &data)
                .await
                .map_err(|e| AppError::internal(format!("Failed to save cookies: {e}")))?;

            let path_str = cookies_path.to_string_lossy().to_string();
            Settings::set(&state.pool, "cookies_file", &path_str).await?;

            let mut yt_dlp = state.yt_dlp.write().await;
            yt_dlp.set_cookies_file(Some(cookies_path));

            tracing::info!("Cookies file uploaded");
            return Ok((StatusCode::OK, Html("Cookies uploaded")));
        }
    }

    Err(AppError::bad_request("No cookies file in upload"))
}

#[tracing::instrument(skip(state))]
pub async fn delete_cookies(
    State(state): State<AppState>
) -> Result<impl IntoResponse, AppError> {
    let cookies_path = PathBuf::from("./data/cookies.txt");
    if cookies_path.exists() {
        tokio::fs::remove_file(&cookies_path)
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete cookies: {e}")))?;
    }

    Settings::set(&state.pool, "cookies_file", "").await?;

    let mut yt_dlp = state.yt_dlp.write().await;
    yt_dlp.set_cookies_file(None);

    tracing::info!("Cookies file deleted");

    Ok((StatusCode::OK, Html("Cookies deleted")))
}

pub fn parse_extractor_args(input: &str) -> Vec<String> {
    let joined: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if joined.is_empty() {
        return Vec::new();
    }
    vec![
        "--extractor-args".to_string(),
        joined.join(";")
    ]
}

pub async fn check_binary_version(binary: &str) -> Option<String> {
    let output = tokio::process::Command::new(binary)
        .arg("--version")
        .output()
        .await
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extractor_args_basic() {
        let input = "youtube:player-client=default,mweb\nyoutubepot-bgutilhttp:base_url=http://bgutil:4416";
        let result = parse_extractor_args(input);
        assert_eq!(result, vec![
            "--extractor-args",
            "youtube:player-client=default,mweb;youtubepot-bgutilhttp:base_url=http://bgutil:4416"
        ]);
    }

    #[test]
    fn test_parse_extractor_args_empty() {
        assert!(parse_extractor_args("").is_empty());
        assert!(parse_extractor_args("  \n  \n  ").is_empty());
    }

    #[test]
    fn test_parse_extractor_args_whitespace() {
        let input = "  youtube:player-client=mweb  \n\n  youtube:po_token=abc  ";
        let result = parse_extractor_args(input);
        assert_eq!(result, vec![
            "--extractor-args",
            "youtube:player-client=mweb;youtube:po_token=abc"
        ]);
    }
}
