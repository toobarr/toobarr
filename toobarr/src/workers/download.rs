use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tokio_stream::StreamExt;
use yt_dlp::{DownloadEvent, DownloadOptions, YtDlp};

use crate::db::DbPool;
use crate::models::{Download, DownloadStatus, Settings};
use crate::nfo::{self, VideoNfo};
use crate::state::DownloadStateInfo;
use crate::thumbnail;

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[derive(Debug, Clone)]
pub struct VideoMeta {
    pub youtube_id: String,
    pub title: String,
    pub description: Option<String>,
    pub duration_seconds: Option<i64>,
    pub upload_date: Option<String>
}

#[derive(Debug, Clone)]
pub enum DownloadCommand {
    Start {
        download_id: String,
        video_url: String,
        channel_name: String,
        video_meta: VideoMeta
    },
    Cancel { download_id: String }
}

pub struct DownloadWorker {
    pool: DbPool,
    yt_dlp: Arc<RwLock<YtDlp>>,
    rx: mpsc::Receiver<DownloadCommand>,
    download_states: Arc<RwLock<HashMap<String, DownloadStateInfo>>>,
    active_downloads: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<()>>>>
}

impl DownloadWorker {
    pub fn new(
        pool: DbPool,
        yt_dlp: Arc<RwLock<YtDlp>>,
        rx: mpsc::Receiver<DownloadCommand>,
        download_states: Arc<RwLock<HashMap<String, DownloadStateInfo>>>
    ) -> Self {
        Self {
            pool,
            yt_dlp,
            rx,
            download_states,
            active_downloads: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub async fn run(mut self) {
        tracing::info!("Download worker started");

        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                DownloadCommand::Start {
                    download_id,
                    video_url,
                    channel_name,
                    video_meta
                } => {
                    let pool = self.pool.clone();
                    let yt_dlp = self.yt_dlp.read().await.clone();
                    let download_states = self.download_states.clone();
                    let active_downloads = self.active_downloads.clone();

                    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
                    {
                        let mut downloads = active_downloads.write().await;
                        downloads.insert(download_id.clone(), cancel_tx);
                    }

                    tokio::spawn(async move {
                        process_download(
                            pool,
                            yt_dlp,
                            download_states.clone(),
                            download_id.clone(),
                            video_url,
                            channel_name,
                            video_meta,
                            cancel_rx
                        )
                        .await;

                        let mut downloads = active_downloads.write().await;
                        downloads.remove(&download_id);
                    });
                }
                DownloadCommand::Cancel { download_id } => {
                    let mut downloads = self.active_downloads.write().await;
                    if let Some(cancel_tx) = downloads.remove(&download_id) {
                        let _ = cancel_tx.send(());
                        tracing::info!("Sent cancel signal for download {}", download_id);
                    }
                }
            }
        }

        tracing::info!("Download worker stopped");
    }
}

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
async fn process_download(
    pool: DbPool,
    yt_dlp: YtDlp,
    download_states: Arc<RwLock<HashMap<String, DownloadStateInfo>>>,
    download_id: String,
    video_url: String,
    channel_name: String,
    video_meta: VideoMeta,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>
) {
    tracing::info!("Starting download {} for {} (channel: {})", download_id, video_url, channel_name);

    if let Err(e) = Download::update_status(&pool, &download_id, DownloadStatus::Downloading).await
    {
        tracing::error!("Failed to update download status: {}", e);
        return;
    }

    {
        let mut states = download_states.write().await;
        states.insert(download_id.clone(), DownloadStateInfo {
            status: "started".to_string(),
            percent: 0.0,
            speed: None,
            eta: None,
            error: None
        });
    }

    let base_download_path = match Settings::get_download_path(&pool).await {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Failed to get download path: {}", e);
            let _ =
                Download::update_failed(&pool, &download_id, &format!("Config error: {e}")).await;
            return;
        }
    };

    let safe_channel_name = sanitize_filename(&channel_name);
    let download_path = format!("{base_download_path}/{safe_channel_name}");

    if let Err(e) = std::fs::create_dir_all(&download_path) {
        tracing::error!("Failed to create download directory: {}", e);
        let _ = Download::update_failed(
            &pool,
            &download_id,
            &format!("Failed to create directory: {e}")
        )
        .await;
        return;
    }

    let output_template = format!("{download_path}/%(title)s.%(ext)s");
    let output_path = PathBuf::from(&output_template);

    let options = DownloadOptions::default();

    let stream = yt_dlp.download_with_progress(&video_url, &output_path, &options);
    tokio::pin!(stream);
    tracing::info!("Download {} stream created, waiting for events", download_id);

    let mut final_filename: Option<String> = None;
    let mut had_error = false;
    let mut error_message: Option<String> = None;
    let mut max_percent: f64 = 0.0;

    loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                tracing::info!("Download {} cancelled", download_id);
                had_error = true;
                error_message = Some("Cancelled by user".to_string());
                break;
            }
            event = stream.next() => {
                match event {
                    Some(Ok(event)) => {
                        tracing::debug!("Download {} event: {:?}", download_id, event);
                        match &event {
                            DownloadEvent::Progress(progress) => {
                                let percent = progress.percent.unwrap_or(0.0);
                                // Track max progress to prevent pulsing when yt-dlp downloads
                                // multiple formats/fragments (each reports 0-100%)
                                if percent > max_percent {
                                    max_percent = percent;
                                }
                                let display_percent = max_percent;
                                tracing::info!("Download {} progress: {:.1}% (max: {:.1}%)", download_id, percent, display_percent);
                                let _ = Download::update_progress(&pool, &download_id, display_percent).await;

                                let mut states = download_states.write().await;
                                states.insert(download_id.clone(), DownloadStateInfo {
                                    status: "progress".to_string(),
                                    percent: display_percent,
                                    speed: progress.format_speed(),
                                    eta: progress.format_eta(),
                                    error: None
                                });
                            }
                            DownloadEvent::DownloadStarted { filename } => {
                                final_filename = Some(filename.clone());
                                tracing::info!("Download {} started: {}", download_id, filename);
                            }
                            DownloadEvent::PostProcessing { status } => {
                                tracing::info!("Download {} post-processing: {}", download_id, status);
                                let mut states = download_states.write().await;
                                states.insert(download_id.clone(), DownloadStateInfo {
                                    status: "processing".to_string(),
                                    percent: 100.0,
                                    speed: None,
                                    eta: None,
                                    error: Some(status.clone())
                                });
                            }
                            DownloadEvent::Finished { filename } => {
                                final_filename = Some(filename.clone());
                                tracing::info!("Download {} finished: {}", download_id, filename);
                            }
                            DownloadEvent::Error { message } => {
                                tracing::error!("Download {} error: {}", download_id, message);
                                had_error = true;
                                error_message = Some(message.clone());
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("Stream error for download {}: {}", download_id, e);
                        had_error = true;
                        error_message = Some(e.to_string());
                        break;
                    }
                    None => break
                }
            }
        }
    }

    if had_error {
        let msg = error_message.unwrap_or_else(|| "Unknown error".to_string());
        let _ = Download::update_failed(&pool, &download_id, &msg).await;
        {
            let mut states = download_states.write().await;
            states.insert(download_id.clone(), DownloadStateInfo {
                status: "failed".to_string(),
                percent: 0.0,
                speed: None,
                eta: None,
                error: Some(msg)
            });
        }
        schedule_state_cleanup(download_states, download_id);
    } else if let Some(filename) = final_filename {
        #[allow(clippy::cast_possible_wrap)]
        let file_size = std::fs::metadata(&filename).map(|m| m.len() as i64).ok();
        let _ = Download::update_completed(&pool, &download_id, &filename, file_size).await;

        let thumb_filename = save_thumb_alongside(&filename, &video_meta).await;

        let ffprobe_bin = Settings::get(&pool, "ffprobe_path")
            .await
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "ffprobe".to_string());
        let media_info = nfo::probe_media(&filename, &ffprobe_bin).await;

        let nfo_data = VideoNfo {
            title: video_meta.title,
            description: video_meta.description,
            youtube_id: video_meta.youtube_id,
            channel_name,
            upload_date: video_meta.upload_date,
            duration_seconds: video_meta.duration_seconds,
            thumb_filename,
            media_info
        };
        if let Err(e) = nfo::write_nfo(&filename, &nfo_data).await {
            tracing::warn!("Failed to write NFO for {}: {}", download_id, e);
        }

        {
            let mut states = download_states.write().await;
            states.insert(download_id.clone(), DownloadStateInfo {
                status: "completed".to_string(),
                percent: 100.0,
                speed: None,
                eta: None,
                error: None
            });
        }
        schedule_state_cleanup(download_states, download_id);
    } else {
        let _ = Download::update_failed(&pool, &download_id, "Download completed but no file found")
            .await;
        {
            let mut states = download_states.write().await;
            states.insert(download_id.clone(), DownloadStateInfo {
                status: "failed".to_string(),
                percent: 0.0,
                speed: None,
                eta: None,
                error: Some("No file found".to_string())
            });
        }
        schedule_state_cleanup(download_states, download_id);
    }
}

fn schedule_state_cleanup(
    download_states: Arc<RwLock<HashMap<String, DownloadStateInfo>>>,
    download_id: String
) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let mut states = download_states.write().await;
        states.remove(&download_id);
    });
}

async fn save_thumb_alongside(video_file_path: &str, meta: &VideoMeta) -> Option<String> {
    let thumb_url = format!(
        "https://i.ytimg.com/vi/{}/maxresdefault.jpg",
        meta.youtube_id
    );
    let video_path = std::path::Path::new(video_file_path);
    let stem = video_path.file_stem()?.to_string_lossy();
    let parent = video_path.parent()?;
    let thumb_name = format!("{stem}-thumb.jpg");
    let thumb_path = parent.join(&thumb_name);
    let thumb_path_str = thumb_path.to_string_lossy().to_string();

    match thumbnail::download_image(&thumb_url, &thumb_path_str).await {
        Ok(()) => {
            tracing::debug!("Saved thumbnail alongside video: {}", thumb_path_str);
            Some(thumb_path_str)
        }
        Err(e) => {
            tracing::warn!("Failed to save thumbnail alongside video: {}", e);
            None
        }
    }
}
