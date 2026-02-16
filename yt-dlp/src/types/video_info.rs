use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub uploader_id: Option<String>,
    #[serde(default)]
    pub uploader_url: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub channel_url: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub view_count: Option<u64>,
    #[serde(default)]
    pub like_count: Option<u64>,
    #[serde(default)]
    pub dislike_count: Option<u64>,
    #[serde(default)]
    pub comment_count: Option<u64>,
    #[serde(default)]
    pub upload_date: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub webpage_url: Option<String>,
    #[serde(default)]
    pub original_url: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub thumbnails: Vec<Thumbnail>,
    #[serde(default)]
    pub formats: Vec<Format>,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub age_limit: Option<u32>,
    #[serde(default)]
    pub is_live: Option<bool>,
    #[serde(default)]
    pub was_live: Option<bool>,
    #[serde(default)]
    pub live_status: Option<String>,
    #[serde(default)]
    pub extractor: Option<String>,
    #[serde(default)]
    pub extractor_key: Option<String>,
    #[serde(default)]
    pub playlist: Option<String>,
    #[serde(default)]
    pub playlist_index: Option<u32>,
    #[serde(default)]
    pub playlist_id: Option<String>,
    #[serde(default)]
    pub playlist_title: Option<String>,
    #[serde(default)]
    pub playlist_count: Option<u32>,
    #[serde(default)]
    pub availability: Option<String>,
    #[serde(default)]
    pub filesize: Option<u64>,
    #[serde(default)]
    pub filesize_approx: Option<u64>
}

impl VideoInfo {
    pub fn best_thumbnail(&self) -> Option<&str> {
        if let Some(ref url) = self.thumbnail {
            return Some(url);
        }
        self.thumbnails
            .iter()
            .max_by_key(|t| t.width.unwrap_or(0))
            .map(|t| t.url.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Format {
    pub format_id: String,
    #[serde(default)]
    pub format_note: Option<String>,
    #[serde(default)]
    pub ext: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub vcodec: Option<String>,
    #[serde(default)]
    pub acodec: Option<String>,
    #[serde(default)]
    pub abr: Option<f64>,
    #[serde(default)]
    pub vbr: Option<f64>,
    #[serde(default)]
    pub tbr: Option<f64>,
    #[serde(default)]
    pub filesize: Option<u64>,
    #[serde(default)]
    pub filesize_approx: Option<u64>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub quality: Option<f64>,
    #[serde(default)]
    pub source_preference: Option<i32>,
    #[serde(default)]
    pub audio_channels: Option<u32>,
    #[serde(default)]
    pub asr: Option<u32>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub language_preference: Option<i32>,
    #[serde(rename = "dynamic_range", default)]
    pub dynamic_range: Option<String>,
    #[serde(default)]
    pub container: Option<String>
}

impl Format {
    pub fn has_video(&self) -> bool {
        self.vcodec.as_ref().is_some_and(|v| v != "none")
    }

    pub fn has_audio(&self) -> bool {
        self.acodec.as_ref().is_some_and(|a| a != "none")
    }

    pub fn display_size(&self) -> Option<String> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some(format!("{}x{}", w, h)),
            _ => self.resolution.clone()
        }
    }

    pub fn estimated_size(&self) -> Option<u64> {
        self.filesize.or(self.filesize_approx)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub url: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub preference: Option<i32>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub start_time: f64,
    pub end_time: f64,
    pub title: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistInfo {
    pub id: String,
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub uploader_id: Option<String>,
    #[serde(default)]
    pub uploader_url: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub channel_url: Option<String>,
    #[serde(default)]
    pub webpage_url: Option<String>,
    #[serde(default)]
    pub entries: Vec<VideoInfo>,
    #[serde(default)]
    pub playlist_count: Option<u32>,
    #[serde(default)]
    pub extractor: Option<String>,
    #[serde(default)]
    pub extractor_key: Option<String>
}
