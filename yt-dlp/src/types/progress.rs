#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub percent: Option<f64>,
    pub fragment_index: Option<u32>,
    pub fragment_count: Option<u32>
}

impl DownloadProgress {
    pub fn format_speed(&self) -> Option<String> {
        self.speed.map(|s| {
            if s >= 1_000_000.0 {
                format!("{:.2} MB/s", s / 1_000_000.0)
            } else if s >= 1_000.0 {
                format!("{:.2} KB/s", s / 1_000.0)
            } else {
                format!("{:.0} B/s", s)
            }
        })
    }

    pub fn format_eta(&self) -> Option<String> {
        self.eta.map(|e| {
            let secs = e as u64;
            let mins = secs / 60;
            let hours = mins / 60;
            if hours > 0 {
                format!("{}:{:02}:{:02}", hours, mins % 60, secs % 60)
            } else {
                format!("{}:{:02}", mins, secs % 60)
            }
        })
    }

    pub fn format_size(&self) -> String {
        format_bytes(self.downloaded_bytes)
    }

    pub fn format_total(&self) -> Option<String> {
        self.total_bytes.map(format_bytes)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.2} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Extracting { url: String },
    DownloadStarted { filename: String },
    Progress(DownloadProgress),
    PostProcessing { status: String },
    MergingFormats,
    EmbeddingThumbnail,
    EmbeddingMetadata,
    Finished { filename: String },
    Error { message: String },
    Warning { message: String }
}

impl DownloadEvent {
    pub fn is_error(&self) -> bool {
        matches!(self, DownloadEvent::Error { .. })
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, DownloadEvent::Finished { .. })
    }
}
