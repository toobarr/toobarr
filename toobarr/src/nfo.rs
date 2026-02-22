use std::path::Path;

use serde::Serialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "episodedetails")]
struct EpisodeDetails {
    plot: String,
    lockdata: bool,
    dateadded: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    art: Option<Art>,
    showtitle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    aired: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fileinfo: Option<FileInfo>,
    uniqueid: UniqueId,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<String>
}

#[derive(Debug, Clone, Serialize)]
struct Art {
    poster: String
}

#[derive(Debug, Clone, Serialize)]
struct FileInfo {
    streamdetails: StreamDetails
}

#[derive(Debug, Clone, Serialize)]
struct StreamDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    video: Option<VideoStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio: Option<AudioStream>
}

#[derive(Debug, Clone, Serialize)]
pub struct VideoStream {
    pub codec: String,
    pub width: i64,
    pub height: i64,
    pub aspect: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framerate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<i64>,
    pub duration: String,
    pub durationinseconds: i64
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioStream {
    pub codec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samplingrate: Option<i64>
}

#[derive(Debug, Clone, Serialize)]
struct UniqueId {
    #[serde(rename = "@type")]
    id_type: String,
    #[serde(rename = "@default")]
    default: String,
    #[serde(rename = "$text")]
    value: String
}

pub struct VideoNfo {
    pub title: String,
    pub description: Option<String>,
    pub youtube_id: String,
    pub channel_name: String,
    pub upload_date: Option<String>,
    pub duration_seconds: Option<i64>,
    pub thumb_filename: Option<String>,
    pub media_info: Option<MediaInfo>
}

pub struct MediaInfo {
    pub video: Option<VideoStream>,
    pub audio: Option<AudioStream>
}

impl VideoNfo {
    pub fn to_xml(&self) -> String {
        let plot = self.description.as_deref().unwrap_or("").to_string();
        let dateadded = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let year = self
            .upload_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .map(String::from);

        let runtime = self.duration_seconds.map(|d| (d + 59) / 60);

        let art = self.thumb_filename.as_ref().map(|t| Art {
            poster: t.clone()
        });

        let aired = self.upload_date.as_deref().map(format_upload_date);

        let fileinfo = self.media_info.as_ref().map(|mi| FileInfo {
            streamdetails: StreamDetails {
                video: mi.video.clone(),
                audio: mi.audio.clone()
            }
        });

        let details = EpisodeDetails {
            plot,
            lockdata: false,
            dateadded,
            title: self.title.clone(),
            year,
            runtime,
            art,
            showtitle: self.channel_name.clone(),
            aired,
            fileinfo,
            uniqueid: UniqueId {
                id_type: "youtube".to_string(),
                default: "true".to_string(),
                value: self.youtube_id.clone()
            },
            thumb: self.thumb_filename.as_ref().map(|_| String::new())
        };

        let body =
            quick_xml::se::to_string(&details).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize NFO XML: {}", e);
                String::new()
            });

        format!(
            "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n{body}\n"
        )
    }
}

pub async fn write_nfo(
    video_file_path: &str,
    nfo: &VideoNfo
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let video_path = Path::new(video_file_path);
    let nfo_path = video_path.with_extension("nfo");

    let xml = nfo.to_xml();
    let mut file = fs::File::create(&nfo_path).await?;
    file.write_all(xml.as_bytes()).await?;

    let nfo_path_str = nfo_path.to_string_lossy().to_string();
    tracing::debug!("Wrote NFO file: {}", nfo_path_str);

    Ok(nfo_path_str)
}

fn format_upload_date(date: &str) -> String {
    if date.len() == 8 {
        format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}

#[derive(serde::Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>
}

#[derive(serde::Deserialize)]
struct FfprobeFormat {
    duration: Option<String>
}

#[derive(serde::Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    display_aspect_ratio: Option<String>,
    r_frame_rate: Option<String>,
    bit_rate: Option<String>,
    duration: Option<String>,
    channels: Option<i64>,
    sample_rate: Option<String>
}

pub async fn probe_media(path: &str, ffprobe_bin: &str) -> Option<MediaInfo> {
    let output = tokio::process::Command::new(ffprobe_bin)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            "-show_format"
        ])
        .arg(path)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        tracing::warn!("ffprobe ({}) failed for {}", ffprobe_bin, path);
        return None;
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout).ok()?;
    let format_duration = parsed
        .format
        .as_ref()
        .and_then(|f| f.duration.as_deref())
        .and_then(|d| d.parse::<f64>().ok());
    let video = parse_video_stream(&parsed.streams, format_duration);
    let audio = parse_audio_stream(&parsed.streams);

    Some(MediaInfo { video, audio })
}

fn parse_video_stream(
    streams: &[FfprobeStream],
    format_duration: Option<f64>
) -> Option<VideoStream> {
    let s = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))?;

    let width = s.width?;
    let height = s.height?;
    let codec = s.codec_name.clone()?;
    let aspect = s
        .display_aspect_ratio
        .clone()
        .unwrap_or_else(|| format!("{width}:{height}"));

    let framerate = s.r_frame_rate.as_deref().and_then(parse_frame_rate);
    let bitrate = s.bit_rate.as_deref().and_then(|b| b.parse::<i64>().ok());

    let duration_secs = s
        .duration
        .as_deref()
        .and_then(|d| d.parse::<f64>().ok())
        .or(format_duration)
        .unwrap_or(0.0);

    #[allow(clippy::cast_possible_truncation)]
    let duration_int = duration_secs as i64;
    let minutes = duration_int / 60;
    let seconds = duration_int % 60;
    let duration = format!("{minutes}:{seconds:02}");

    Some(VideoStream {
        codec,
        width,
        height,
        aspect,
        framerate,
        bitrate,
        duration,
        durationinseconds: duration_int
    })
}

fn parse_frame_rate(rate: &str) -> Option<String> {
    let parts: Vec<&str> = rate.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().ok()?;
        let den: f64 = parts[1].parse().ok()?;
        if den > 0.0 {
            return Some(format!("{:.3}", num / den));
        }
    }
    None
}

fn parse_audio_stream(streams: &[FfprobeStream]) -> Option<AudioStream> {
    let s = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"))?;

    let codec = s.codec_name.clone()?;
    let channels = s.channels;
    let samplingrate = s
        .sample_rate
        .as_deref()
        .and_then(|r| r.parse::<i64>().ok());

    Some(AudioStream {
        codec,
        channels,
        samplingrate
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_xml_full() {
        let nfo = VideoNfo {
            title: "Test Video".to_string(),
            description: Some("A test description".to_string()),
            youtube_id: "abc123".to_string(),
            channel_name: "Test Channel".to_string(),
            upload_date: Some("20230415".to_string()),
            duration_seconds: Some(300),
            thumb_filename: Some("thumb.jpg".to_string()),
            media_info: Some(MediaInfo {
                video: Some(VideoStream {
                    codec: "h264".to_string(),
                    width: 1920,
                    height: 1080,
                    aspect: "16:9".to_string(),
                    framerate: Some("29.970".to_string()),
                    bitrate: Some(5_000_000),
                    duration: "5:00".to_string(),
                    durationinseconds: 300
                }),
                audio: Some(AudioStream {
                    codec: "aac".to_string(),
                    channels: Some(2),
                    samplingrate: Some(48000)
                })
            })
        };

        let xml = nfo.to_xml();
        assert!(xml.starts_with("\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>"));
        assert!(xml.contains("<episodedetails>"));
        assert!(xml.contains("<plot>A test description</plot>"));
        assert!(xml.contains("<lockdata>false</lockdata>"));
        assert!(xml.contains("<dateadded>"));
        assert!(xml.contains("<title>Test Video</title>"));
        assert!(xml.contains("<year>2023</year>"));
        assert!(xml.contains("<runtime>5</runtime>"));
        assert!(xml.contains("<poster>thumb.jpg</poster>"));
        assert!(xml.contains("<showtitle>Test Channel</showtitle>"));
        assert!(xml.contains("<aired>2023-04-15</aired>"));
        assert!(xml.contains("<codec>h264</codec>"));
        assert!(xml.contains("<width>1920</width>"));
        assert!(xml.contains("<height>1080</height>"));
        assert!(xml.contains("<aspect>16:9</aspect>"));
        assert!(xml.contains("<framerate>29.970</framerate>"));
        assert!(xml.contains("<bitrate>5000000</bitrate>"));
        assert!(xml.contains("<codec>aac</codec>"));
        assert!(xml.contains("<channels>2</channels>"));
        assert!(xml.contains("<samplingrate>48000</samplingrate>"));
        assert!(xml.contains(r#"<uniqueid type="youtube" default="true">abc123</uniqueid>"#));
        assert!(xml.contains("<thumb/>"));
        assert!(xml.contains("</episodedetails>"));
    }

    #[test]
    fn test_to_xml_minimal() {
        let nfo = VideoNfo {
            title: "Minimal".to_string(),
            description: None,
            youtube_id: "xyz789".to_string(),
            channel_name: "Chan".to_string(),
            upload_date: None,
            duration_seconds: None,
            thumb_filename: None,
            media_info: None
        };

        let xml = nfo.to_xml();
        assert!(xml.contains("<title>Minimal</title>"));
        assert!(xml.contains("<plot></plot>") || xml.contains("<plot/>"));
        assert!(xml.contains("<showtitle>Chan</showtitle>"));
        assert!(!xml.contains("<year>"));
        assert!(!xml.contains("<runtime>"));
        assert!(!xml.contains("<art>"));
        assert!(!xml.contains("<aired>"));
        assert!(!xml.contains("<fileinfo>"));
        assert!(!xml.contains("<thumb"));
    }

    #[test]
    fn test_to_xml_escapes_special_chars() {
        let nfo = VideoNfo {
            title: "Tom & Jerry <3 \"Quotes\" 'Apos'".to_string(),
            description: Some("A & B < C > D".to_string()),
            youtube_id: "id&1".to_string(),
            channel_name: "Chan <&>".to_string(),
            upload_date: None,
            duration_seconds: None,
            thumb_filename: None,
            media_info: None
        };

        let xml = nfo.to_xml();
        assert!(!xml.contains("<title>Tom & Jerry"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
    }

    #[test]
    fn test_to_xml_date_formatting() {
        let nfo = VideoNfo {
            title: "Date Test".to_string(),
            description: None,
            youtube_id: "dt1".to_string(),
            channel_name: "Chan".to_string(),
            upload_date: Some("20180102".to_string()),
            duration_seconds: None,
            thumb_filename: None,
            media_info: None
        };

        let xml = nfo.to_xml();
        assert!(xml.contains("<aired>2018-01-02</aired>"));
        assert!(xml.contains("<year>2018</year>"));
    }

    #[test]
    fn test_parse_ffprobe_output() {
        let json = r#"{
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "vp9",
                    "width": 3840,
                    "height": 2160,
                    "display_aspect_ratio": "16:9",
                    "r_frame_rate": "30/1",
                    "bit_rate": "10000000",
                    "duration": "600.5"
                },
                {
                    "codec_type": "audio",
                    "codec_name": "opus",
                    "channels": 2,
                    "sample_rate": "48000"
                }
            ]
        }"#;

        let parsed: FfprobeOutput = serde_json::from_str(json).unwrap();
        let video = parse_video_stream(&parsed.streams, None).unwrap();
        assert_eq!(video.codec, "vp9");
        assert_eq!(video.width, 3840);
        assert_eq!(video.height, 2160);
        assert_eq!(video.aspect, "16:9");
        assert_eq!(video.framerate.as_deref(), Some("30.000"));
        assert_eq!(video.bitrate, Some(10_000_000));
        assert_eq!(video.durationinseconds, 600);
        assert_eq!(video.duration, "10:00");

        let audio = parse_audio_stream(&parsed.streams).unwrap();
        assert_eq!(audio.codec, "opus");
        assert_eq!(audio.channels, Some(2));
        assert_eq!(audio.samplingrate, Some(48000));
    }

    #[test]
    fn test_parse_ffprobe_format_duration_fallback() {
        let json = r#"{
            "streams": [
                {
                    "codec_type": "video",
                    "codec_name": "av1",
                    "width": 3840,
                    "height": 2160,
                    "display_aspect_ratio": "16:9",
                    "r_frame_rate": "24000/1001"
                },
                {
                    "codec_type": "audio",
                    "codec_name": "opus",
                    "channels": 2,
                    "sample_rate": "48000"
                }
            ],
            "format": {
                "duration": "1320.5"
            }
        }"#;

        let parsed: FfprobeOutput = serde_json::from_str(json).unwrap();
        let format_duration = parsed
            .format
            .as_ref()
            .and_then(|f| f.duration.as_deref())
            .and_then(|d| d.parse::<f64>().ok());
        let video = parse_video_stream(&parsed.streams, format_duration).unwrap();
        assert_eq!(video.durationinseconds, 1320);
        assert_eq!(video.duration, "22:00");
    }
}
