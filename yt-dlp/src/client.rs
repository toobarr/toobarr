use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use futures_core::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt;

use crate::command::CommandBuilder;
use crate::error::{Error, Result};
use crate::types::{
    Container, DownloadEvent, DownloadOptions, DownloadProgress, Format, OutputFormat,
    PlaylistInfo, VideoInfo
};

#[derive(Debug, Clone)]
pub struct YtDlp {
    binary: PathBuf,
    cookies_file: Option<PathBuf>,
    extra_args: Vec<String>,
    ffmpeg_location: Option<PathBuf>,
    env_vars: HashMap<String, String>
}

impl Default for YtDlp {
    fn default() -> Self {
        Self::new()
    }
}

impl YtDlp {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("yt-dlp"),
            cookies_file: None,
            extra_args: Vec::new(),
            ffmpeg_location: None,
            env_vars: HashMap::new()
        }
    }

    pub fn with_binary(path: impl Into<PathBuf>) -> Self {
        Self {
            binary: path.into(),
            cookies_file: None,
            extra_args: Vec::new(),
            ffmpeg_location: None,
            env_vars: HashMap::new()
        }
    }

    pub fn set_binary(&mut self, path: PathBuf) {
        self.binary = path;
    }

    pub fn set_cookies_file(&mut self, path: Option<PathBuf>) {
        self.cookies_file = path;
    }

    pub fn set_extra_args(&mut self, args: Vec<String>) {
        self.extra_args = args;
    }

    pub fn set_ffmpeg_location(&mut self, path: Option<PathBuf>) {
        self.ffmpeg_location = path;
    }

    pub fn set_env(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    pub async fn check_binary(&self) -> Result<String> {
        let output = Command::new(&self.binary)
            .arg("--version")
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(Error::BinaryNotExecutable(self.binary.clone()))
        }
    }

    pub async fn get_video_info(&self, url: &str) -> Result<VideoInfo> {
        let output = self
            .command()
            .json_output()
            .skip_download()
            .no_playlist()
            .url(url)
            .build_with_env(&self.env_vars)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::CommandFailed {
                code: output.status.code().unwrap_or(-1),
                stderr
            });
        }

        let info: VideoInfo = serde_json::from_slice(&output.stdout)?;
        Ok(info)
    }

    pub async fn get_playlist_info(&self, url: &str) -> Result<PlaylistInfo> {
        let output = self
            .command()
            .json_output()
            .skip_download()
            .yes_playlist()
            .flat_playlist()
            .url(url)
            .build_with_env(&self.env_vars)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::CommandFailed {
                code: output.status.code().unwrap_or(-1),
                stderr
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        let mut playlist_info: Option<PlaylistInfo> = None;

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(info) = serde_json::from_str::<VideoInfo>(line) {
                if playlist_info.is_none() {
                    playlist_info = Some(PlaylistInfo {
                        id: info.playlist_id.clone().unwrap_or_default(),
                        title: info.playlist_title.clone(),
                        description: None,
                        uploader: info.uploader.clone(),
                        uploader_id: info.uploader_id.clone(),
                        uploader_url: info.uploader_url.clone(),
                        channel: info.channel.clone(),
                        channel_id: info.channel_id.clone(),
                        channel_url: info.channel_url.clone(),
                        webpage_url: None,
                        entries: Vec::new(),
                        playlist_count: info.playlist_count,
                        extractor: info.extractor.clone(),
                        extractor_key: info.extractor_key.clone()
                    });
                }
                entries.push(info);
            }
        }

        match playlist_info {
            Some(mut info) => {
                info.entries = entries;
                Ok(info)
            }
            None => Err(Error::EmptyPlaylist)
        }
    }

    pub async fn list_formats(&self, url: &str) -> Result<Vec<Format>> {
        let info = self.get_video_info(url).await?;
        if info.formats.is_empty() {
            Err(Error::NoFormatsAvailable)
        } else {
            Ok(info.formats)
        }
    }

    pub async fn download(&self, url: &str, output: impl AsRef<Path>) -> Result<PathBuf> {
        self.download_with_options(url, output, &DownloadOptions::default())
            .await
    }

    pub async fn download_with_options(
        &self,
        url: &str,
        output: impl AsRef<Path>,
        options: &DownloadOptions
    ) -> Result<PathBuf> {
        let output_path = output.as_ref().to_path_buf();

        let result = self
            .command()
            .with_options(options)
            .output(&output_path)
            .url(url)
            .build_with_env(&self.env_vars)
            .output()
            .await?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr).to_string();
            return Err(Error::CommandFailed {
                code: result.status.code().unwrap_or(-1),
                stderr
            });
        }

        Ok(output_path)
    }

    pub fn download_with_progress(
        &self,
        url: &str,
        output: impl AsRef<Path>,
        options: &DownloadOptions
    ) -> Pin<Box<dyn Stream<Item = Result<DownloadEvent>> + Send + '_>> {
        let output_path = output.as_ref().to_path_buf();
        let url = url.to_string();
        let options = options.clone();
        let binary = self.binary.clone();
        let cookies_file = self.cookies_file.clone();
        let extra_args = self.extra_args.clone();
        let ffmpeg_location = self.ffmpeg_location.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async_stream::try_stream! {
            yield DownloadEvent::Extracting { url: url.clone() };

            let mut builder = CommandBuilder::new(&binary)
                .cookies_file_opt(&cookies_file)
                .args(extra_args.iter().map(String::as_str))
                .with_options(&options)
                .output(&output_path)
                .newline_progress()
                .progress_template("download:%(progress._percent_str)s %(progress._total_bytes_str)s %(progress._speed_str)s %(progress._eta_str)s")
                .url(&url);

            if let Some(ref ffmpeg_path) = ffmpeg_location {
                builder = builder.ffmpeg_location(ffmpeg_path);
            }

            tracing::debug!(
                binary = %binary.display(),
                args = ?builder.get_args(),
                "spawning yt-dlp"
            );

            let mut cmd = builder.build_with_env(&env_vars);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd.spawn()?;

            let stderr = child.stderr.take().expect("stderr not captured");
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    tracing::trace!(line = %line, "yt-dlp stderr");
                }
            });

            let stdout = child.stdout.take().expect("stdout not captured");
            let mut reader = BufReader::new(stdout).lines();

            let mut current_filename: Option<String> = None;

            while let Some(line) = reader.next_line().await? {
                tracing::trace!(line = %line, "yt-dlp stdout");
                if let Some(event) = parse_progress_line(&line, &mut current_filename) {
                    yield event;
                }
            }

            let status = child.wait().await?;

            if !status.success() {
                yield DownloadEvent::Error {
                    message: format!("yt-dlp exited with code {}", status.code().unwrap_or(-1))
                };
            } else {
                let filename = current_filename
                    .unwrap_or_else(|| output_path.to_string_lossy().to_string());
                yield DownloadEvent::Finished { filename };
            }
        })
    }

    pub async fn download_audio(
        &self,
        url: &str,
        output: impl AsRef<Path>
    ) -> Result<PathBuf> {
        let options = DownloadOptions::new()
            .extract_audio(true)
            .audio_format("mp3")
            .audio_quality("0");

        self.download_with_options(url, output, &options).await
    }

    pub fn build_download(&self, url: &str) -> DownloadBuilder {
        DownloadBuilder::new(self.clone(), url.to_string())
    }

    fn command(&self) -> CommandBuilder {
        let mut builder = CommandBuilder::new(&self.binary)
            .cookies_file_opt(&self.cookies_file)
            .args(self.extra_args.iter().map(String::as_str));

        if let Some(ref ffmpeg_path) = self.ffmpeg_location {
            builder = builder.ffmpeg_location(ffmpeg_path);
        }

        builder
    }
}

fn parse_progress_line(line: &str, current_filename: &mut Option<String>) -> Option<DownloadEvent> {
    let line = line.trim();

    if line.starts_with("[download] Destination:") {
        let filename = line.trim_start_matches("[download] Destination:").trim();
        *current_filename = Some(filename.to_string());
        return Some(DownloadEvent::DownloadStarted {
            filename: filename.to_string()
        });
    }

    if line.starts_with("[download]")
        && line.contains('%')
        && let Some(progress) = parse_download_progress(line)
    {
        return Some(DownloadEvent::Progress(progress));
    }

    if line.starts_with("download:")
        && let Some(progress) = parse_template_progress(line)
    {
        return Some(DownloadEvent::Progress(progress));
    }

    // Handle bare progress lines (e.g., " 14.6%  887.84MiB    7.61MiB/s 01:39")
    // These occur when using --newline without a progress template prefix
    if line.contains('%')
        && let Some(progress) = parse_download_progress(line)
    {
        return Some(DownloadEvent::Progress(progress));
    }

    if line.starts_with("[Merger]") || line.contains("Merging formats") {
        if let Some(start) = line.find('"')
            && let Some(end) = line.rfind('"')
            && end > start
        {
            *current_filename = Some(line[start + 1..end].to_string());
        }
        return Some(DownloadEvent::MergingFormats);
    }

    if line.starts_with("[EmbedThumbnail]") {
        return Some(DownloadEvent::EmbeddingThumbnail);
    }

    if line.starts_with("[Metadata]") {
        return Some(DownloadEvent::EmbeddingMetadata);
    }

    if line.starts_with("[ExtractAudio]") || line.starts_with("[ffmpeg]") {
        return Some(DownloadEvent::PostProcessing {
            status: line.to_string()
        });
    }

    if line.contains("has already been downloaded") {
        let filename = current_filename.clone().unwrap_or_default();
        return Some(DownloadEvent::Finished { filename });
    }

    if line.starts_with("WARNING:") {
        return Some(DownloadEvent::Warning {
            message: line.trim_start_matches("WARNING:").trim().to_string()
        });
    }

    if line.starts_with("ERROR:") {
        return Some(DownloadEvent::Error {
            message: line.trim_start_matches("ERROR:").trim().to_string()
        });
    }

    None
}

fn parse_download_progress(line: &str) -> Option<DownloadProgress> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    let mut percent: Option<f64> = None;
    let mut total_bytes: Option<u64> = None;
    let mut speed: Option<f64> = None;
    let mut eta: Option<f64> = None;

    for (i, part) in parts.iter().enumerate() {
        if part.ends_with('%') {
            percent = part.trim_end_matches('%').parse().ok();
        } else if part.contains("iB") || part.contains("B") {
            if i > 0 && parts.get(i - 1).is_some_and(|p| p.ends_with('%')) {
                total_bytes = parse_size(part);
            } else if part.contains("/s") {
                speed = parse_speed(part);
            }
        } else if part.starts_with("ETA") || (i > 0 && parts.get(i - 1) == Some(&"ETA")) {
            continue;
        } else if part.contains(':') && !part.starts_with('[') {
            eta = parse_eta(part);
        }
    }

    let downloaded_bytes = match (percent, total_bytes) {
        (Some(p), Some(t)) => ((p / 100.0) * t as f64) as u64,
        _ => 0
    };

    Some(DownloadProgress {
        downloaded_bytes,
        total_bytes,
        speed,
        eta,
        percent,
        fragment_index: None,
        fragment_count: None
    })
}

fn parse_template_progress(line: &str) -> Option<DownloadProgress> {
    let content = line.trim_start_matches("download:").trim();
    let parts: Vec<&str> = content.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    let percent = parts.first().and_then(|p| {
        p.trim_end_matches('%').trim().parse::<f64>().ok()
    });

    let total_bytes = parts.get(1).and_then(|s| parse_size(s));
    let speed = parts.get(2).and_then(|s| parse_speed(s));
    let eta = parts.get(3).and_then(|s| parse_eta(s));

    let downloaded_bytes = match (percent, total_bytes) {
        (Some(p), Some(t)) => ((p / 100.0) * t as f64) as u64,
        _ => 0
    };

    Some(DownloadProgress {
        downloaded_bytes,
        total_bytes,
        speed,
        eta,
        percent,
        fragment_index: None,
        fragment_count: None
    })
}

fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s == "N/A" || s == "~" || s.is_empty() {
        return None;
    }

    let multipliers = [
        ("GiB", 1024u64 * 1024 * 1024),
        ("MiB", 1024 * 1024),
        ("KiB", 1024),
        ("GB", 1000 * 1000 * 1000),
        ("MB", 1000 * 1000),
        ("KB", 1000),
        ("B", 1)
    ];

    for (suffix, mult) in multipliers {
        if s.ends_with(suffix) {
            let num_str = s.trim_end_matches(suffix).trim();
            if let Ok(num) = num_str.parse::<f64>() {
                return Some((num * mult as f64) as u64);
            }
        }
    }

    None
}

fn parse_speed(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches("/s");
    parse_size(s).map(|b| b as f64)
}

fn parse_eta(s: &str) -> Option<f64> {
    let s = s.trim();
    if s == "N/A" || s == "Unknown" || s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<f64>().ok(),
        2 => {
            let mins: f64 = parts[0].parse().ok()?;
            let secs: f64 = parts[1].parse().ok()?;
            Some(mins * 60.0 + secs)
        }
        3 => {
            let hours: f64 = parts[0].parse().ok()?;
            let mins: f64 = parts[1].parse().ok()?;
            let secs: f64 = parts[2].parse().ok()?;
            Some(hours * 3600.0 + mins * 60.0 + secs)
        }
        _ => None
    }
}

pub struct DownloadBuilder {
    client: YtDlp,
    url: String,
    options: DownloadOptions
}

impl DownloadBuilder {
    fn new(client: YtDlp, url: String) -> Self {
        Self {
            client,
            url,
            options: DownloadOptions::default()
        }
    }

    pub fn format(mut self, format: OutputFormat) -> Self {
        self.options.format = format;
        self
    }

    pub fn container(mut self, container: Container) -> Self {
        self.options.container = container;
        self
    }

    pub fn output_template(mut self, template: impl Into<String>) -> Self {
        self.options.output_template = Some(template.into());
        self
    }

    pub fn embed_thumbnail(mut self, embed: bool) -> Self {
        self.options.embed_thumbnail = embed;
        self
    }

    pub fn embed_metadata(mut self, embed: bool) -> Self {
        self.options.embed_metadata = embed;
        self
    }

    pub fn embed_subtitles(mut self, embed: bool) -> Self {
        self.options.embed_subtitles = embed;
        self
    }

    pub fn extract_audio(mut self, extract: bool) -> Self {
        self.options.extract_audio = extract;
        self
    }

    pub fn audio_format(mut self, format: impl Into<String>) -> Self {
        self.options.audio_format = Some(format.into());
        self
    }

    pub fn audio_quality(mut self, quality: impl Into<String>) -> Self {
        self.options.audio_quality = Some(quality.into());
        self
    }

    pub fn cookies_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cookies_file = Some(path.into());
        self
    }

    pub fn rate_limit(mut self, limit: impl Into<String>) -> Self {
        self.options.rate_limit = Some(limit.into());
        self
    }

    pub async fn download(self, output: impl AsRef<Path>) -> Result<PathBuf> {
        self.client
            .download_with_options(&self.url, output, &self.options)
            .await
    }

    pub fn download_with_progress(
        self,
        output: impl AsRef<Path>
    ) -> Pin<Box<dyn Stream<Item = Result<DownloadEvent>> + Send + 'static>> {
        let output = output.as_ref().to_path_buf();

        Box::pin(async_stream::try_stream! {
            let stream = self.client.download_with_progress(&self.url, &output, &self.options);
            tokio::pin!(stream);

            while let Some(event) = stream.next().await {
                yield event?;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100MiB"), Some(104857600));
        assert_eq!(parse_size("1GiB"), Some(1073741824));
        assert_eq!(parse_size("500KiB"), Some(512000));
        assert_eq!(parse_size("1000B"), Some(1000));
        assert_eq!(parse_size("N/A"), None);
    }

    #[test]
    fn test_parse_speed() {
        assert_eq!(parse_speed("1MiB/s"), Some(1048576.0));
        assert_eq!(parse_speed("500KiB/s"), Some(512000.0));
    }

    #[test]
    fn test_parse_eta() {
        assert_eq!(parse_eta("1:30"), Some(90.0));
        assert_eq!(parse_eta("1:00:00"), Some(3600.0));
        assert_eq!(parse_eta("N/A"), None);
    }

    #[test]
    fn test_parse_progress_line_destination() {
        let mut filename = None;
        let event = parse_progress_line(
            "[download] Destination: video.mp4",
            &mut filename
        );
        assert!(matches!(event, Some(DownloadEvent::DownloadStarted { .. })));
        assert_eq!(filename, Some("video.mp4".to_string()));
    }

    #[test]
    fn test_parse_progress_line_error() {
        let mut filename = None;
        let event = parse_progress_line("ERROR: Video unavailable", &mut filename);
        assert!(matches!(event, Some(DownloadEvent::Error { .. })));
    }

    #[test]
    fn test_ytdlp_default() {
        let client = YtDlp::default();
        assert_eq!(client.binary, PathBuf::from("yt-dlp"));
        assert!(client.cookies_file.is_none());
        assert!(client.extra_args.is_empty());
    }

    #[test]
    fn test_ytdlp_with_binary() {
        let client = YtDlp::with_binary("/usr/local/bin/yt-dlp");
        assert_eq!(client.binary, PathBuf::from("/usr/local/bin/yt-dlp"));
    }

    #[test]
    fn test_ytdlp_set_cookies_and_extra_args() {
        let mut client = YtDlp::new();
        client.set_cookies_file(Some(PathBuf::from("/tmp/cookies.txt")));
        client.set_extra_args(vec![
            "--extractor-args".to_string(),
            "youtube:player-client=mweb".to_string()
        ]);
        assert_eq!(client.cookies_file, Some(PathBuf::from("/tmp/cookies.txt")));
        assert_eq!(client.extra_args.len(), 2);
    }

    #[test]
    fn test_ytdlp_set_binary() {
        let mut client = YtDlp::new();
        client.set_binary(PathBuf::from("/opt/yt-dlp"));
        assert_eq!(client.binary, PathBuf::from("/opt/yt-dlp"));
    }

    #[test]
    fn test_ytdlp_ffmpeg_location() {
        let mut client = YtDlp::new();
        client.set_ffmpeg_location(Some(PathBuf::from("/usr/local/bin/ffmpeg")));
        assert_eq!(client.ffmpeg_location, Some(PathBuf::from("/usr/local/bin/ffmpeg")));
    }

    #[test]
    fn test_ytdlp_env_vars() {
        let mut client = YtDlp::new();
        client.set_env("PATH_PREPEND".to_string(), "/opt/bin".to_string());
        assert_eq!(client.env_vars.get("PATH_PREPEND"), Some(&"/opt/bin".to_string()));
    }
}
