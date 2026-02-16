mod db;
mod error;
mod handlers;
mod models;
mod nfo;
mod state;
mod thumbnail;
mod workers;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post}
};
use tokio::sync::{RwLock, mpsc};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use yt_dlp::YtDlp;

use handlers::{api, pages};
use models::Settings;
use state::AppState;
use workers::download::DownloadWorker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "toobarr=info,tower_http=debug".into())
        )
        .init();

    let database_path =
        std::env::var("DATABASE_PATH").unwrap_or_else(|_| "./toobarr.db".to_string());

    let pool = db::init_pool(&database_path).await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database initialized at {}", database_path);

    let mut yt_dlp = YtDlp::new();

    if let Ok(Some(ytdlp_path)) = Settings::get(&pool, "ytdlp_path").await {
        if !ytdlp_path.is_empty() {
            yt_dlp = YtDlp::with_binary(&ytdlp_path);
            tracing::info!("Using custom yt-dlp path: {}", ytdlp_path);
        }
    }

    if let Ok(args_str) = Settings::get_extractor_args(&pool).await {
        let parsed = api::parse_extractor_args(&args_str);
        if !parsed.is_empty() {
            yt_dlp.set_extra_args(parsed);
        }
    }

    if let Ok(Some(cookies_path)) = Settings::get_cookies_file(&pool).await {
        if !cookies_path.is_empty() {
            let path = PathBuf::from(&cookies_path);
            if path.exists() {
                yt_dlp.set_cookies_file(Some(path));
                tracing::info!("Using cookies file: {}", cookies_path);
            }
        }
    }

    if let Ok(Some(ffmpeg_path)) = Settings::get(&pool, "ffmpeg_path").await {
        if !ffmpeg_path.is_empty() {
            yt_dlp.set_ffmpeg_location(Some(PathBuf::from(&ffmpeg_path)));
            tracing::info!("Using custom ffmpeg path: {}", ffmpeg_path);
        }
    }

    if let Ok(Some(deno_path)) = Settings::get(&pool, "deno_path").await {
        if !deno_path.is_empty() {
            if let Some(parent) = std::path::Path::new(&deno_path).parent() {
                yt_dlp.set_env("PATH_PREPEND".to_string(), parent.to_string_lossy().to_string());
                tracing::info!("Using custom deno path: {}", deno_path);
            }
        }
    }

    if let Err(e) = yt_dlp.check_binary().await {
        tracing::warn!("yt-dlp not found or not executable: {}", e);
    } else {
        let version = yt_dlp.check_binary().await.unwrap_or_default();
        tracing::info!("yt-dlp version: {}", version);
    }

    let yt_dlp = Arc::new(RwLock::new(yt_dlp));

    let (download_tx, download_rx) = mpsc::channel(100);
    let download_states = Arc::new(RwLock::new(HashMap::new()));

    let worker = DownloadWorker::new(pool.clone(), yt_dlp.clone(), download_rx, download_states.clone());

    tokio::spawn(async move {
        worker.run().await;
    });

    let state = AppState {
        pool,
        yt_dlp,
        download_tx,
        download_states
    };

    let app = Router::new()
        .route("/", get(pages::home_page))
        .route("/channels", get(pages::channels_page))
        .route("/channels/new", get(pages::new_channel_page))
        .route("/channels/{id}", get(pages::channel_detail_page))
        .route("/downloads", get(pages::downloads_page))
        .route("/settings", get(pages::settings_page))
        .route("/api/channels", post(api::create_channel))
        .route("/api/channels/{id}", delete(api::delete_channel))
        .route("/api/channels/{id}/sync", post(api::sync_channel))
        .route("/api/videos/{id}/download", post(api::start_download))
        .route("/api/downloads/{id}/cancel", post(api::cancel_download))
        .route("/api/downloads/{id}/retry", post(api::retry_download))
        .route("/api/downloads/active", get(api::active_downloads))
        .route("/api/downloads/count", get(api::download_count))
        .route("/api/settings", post(api::update_settings))
        .route("/api/settings/cookies", post(api::upload_cookies))
        .route("/api/settings/cookies", delete(api::delete_cookies))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
