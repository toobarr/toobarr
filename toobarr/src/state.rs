use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use yt_dlp::YtDlp;

use crate::db::DbPool;
use crate::workers::download::DownloadCommand;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub yt_dlp: Arc<RwLock<YtDlp>>,
    pub download_tx: mpsc::Sender<DownloadCommand>,
    pub download_states: Arc<RwLock<HashMap<String, DownloadStateInfo>>>
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DownloadStateInfo {
    pub status: String,
    pub percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>
}
