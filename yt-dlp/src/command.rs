use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::types::DownloadOptions;

pub struct CommandBuilder {
    binary: PathBuf,
    args: Vec<String>
}

#[allow(dead_code)]
impl CommandBuilder {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            args: Vec::new()
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn url(self, url: impl Into<String>) -> Self {
        self.arg(url)
    }

    pub fn json_output(self) -> Self {
        self.arg("--dump-json")
    }

    pub fn skip_download(self) -> Self {
        self.arg("--skip-download")
    }

    pub fn list_formats(self) -> Self {
        self.arg("--list-formats")
    }

    pub fn output(self, path: impl AsRef<Path>) -> Self {
        self.arg("-o").arg(path.as_ref().to_string_lossy().to_string())
    }

    pub fn format(self, format: impl Into<String>) -> Self {
        self.arg("-f").arg(format)
    }

    pub fn extract_audio(self) -> Self {
        self.arg("-x")
    }

    pub fn audio_format(self, format: impl Into<String>) -> Self {
        self.arg("--audio-format").arg(format)
    }

    pub fn audio_quality(self, quality: impl Into<String>) -> Self {
        self.arg("--audio-quality").arg(quality)
    }

    pub fn embed_thumbnail(self) -> Self {
        self.arg("--embed-thumbnail")
    }

    pub fn embed_metadata(self) -> Self {
        self.arg("--embed-metadata")
    }

    pub fn embed_subtitles(self) -> Self {
        self.arg("--embed-subs")
    }

    pub fn write_subtitles(self) -> Self {
        self.arg("--write-subs")
    }

    pub fn subtitles_langs(self, langs: &[String]) -> Self {
        if langs.is_empty() {
            self
        } else {
            self.arg("--sub-langs").arg(langs.join(","))
        }
    }

    pub fn write_thumbnail(self) -> Self {
        self.arg("--write-thumbnail")
    }

    pub fn cookies_file(self, path: impl AsRef<Path>) -> Self {
        self.arg("--cookies").arg(path.as_ref().to_string_lossy().to_string())
    }

    pub fn cookies_file_opt(self, path: Option<&PathBuf>) -> Self {
        match path {
            Some(p) => self.cookies_file(p),
            None => self
        }
    }

    pub fn rate_limit(self, limit: impl Into<String>) -> Self {
        self.arg("-r").arg(limit)
    }

    pub fn concurrent_fragments(self, count: u32) -> Self {
        self.arg("--concurrent-fragments").arg(count.to_string())
    }

    pub fn merge_output_format(self, format: impl Into<String>) -> Self {
        self.arg("--merge-output-format").arg(format)
    }

    pub fn progress_template(self, template: impl Into<String>) -> Self {
        self.arg("--progress-template").arg(template)
    }

    pub fn newline_progress(self) -> Self {
        self.arg("--newline")
    }

    pub fn no_warnings(self) -> Self {
        self.arg("--no-warnings")
    }

    pub fn flat_playlist(self) -> Self {
        self.arg("--flat-playlist")
    }

    pub fn yes_playlist(self) -> Self {
        self.arg("--yes-playlist")
    }

    pub fn no_playlist(self) -> Self {
        self.arg("--no-playlist")
    }

    pub fn ffmpeg_location(self, path: impl AsRef<Path>) -> Self {
        self.arg("--ffmpeg-location").arg(path.as_ref().to_string_lossy().to_string())
    }

    pub fn with_options(mut self, options: &DownloadOptions) -> Self {
        if let Some(format_arg) = options.format.as_arg() {
            self = self.format(format_arg);
        }

        if let Some(container) = options.container.as_str() {
            self = self.merge_output_format(container);
        }

        if let Some(ref template) = options.output_template {
            self = self.arg("-o").arg(template.clone());
        }

        if options.embed_thumbnail {
            self = self.embed_thumbnail();
        }

        if options.embed_metadata {
            self = self.embed_metadata();
        }

        if options.embed_subtitles {
            self = self.embed_subtitles();
        }

        if options.extract_audio {
            self = self.extract_audio();
        }

        if let Some(ref format) = options.audio_format {
            self = self.audio_format(format.clone());
        }

        if let Some(ref quality) = options.audio_quality {
            self = self.audio_quality(quality.clone());
        }

        if !options.subtitles_langs.is_empty() {
            self = self.subtitles_langs(&options.subtitles_langs);
        }

        if options.write_subtitles {
            self = self.write_subtitles();
        }

        if options.write_thumbnail {
            self = self.write_thumbnail();
        }

        if let Some(ref path) = options.cookies_file {
            self = self.cookies_file(path);
        }

        if let Some(ref limit) = options.rate_limit {
            self = self.rate_limit(limit.clone());
        }

        if let Some(count) = options.concurrent_fragments {
            self = self.concurrent_fragments(count);
        }

        for arg in &options.extra_args {
            self = self.arg(arg.clone());
        }

        self
    }

    pub fn build(&self) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.args(&self.args);
        cmd
    }

    pub fn build_with_env(&self, env_vars: &HashMap<String, String>) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.args(&self.args);

        if let Some(path_prepend) = env_vars.get("PATH_PREPEND") {
            let current_path = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("{path_prepend}:{current_path}"));
        }

        for (key, value) in env_vars {
            if key != "PATH_PREPEND" {
                cmd.env(key, value);
            }
        }

        cmd
    }

    pub fn get_args(&self) -> &[String] {
        &self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builder_basic() {
        let builder = CommandBuilder::new("yt-dlp")
            .arg("--version");
        assert_eq!(builder.get_args(), &["--version"]);
    }

    #[test]
    fn test_command_builder_download() {
        let builder = CommandBuilder::new("yt-dlp")
            .format("best")
            .output("/tmp/video.mp4")
            .url("https://example.com/video");
        assert_eq!(builder.get_args(), &[
            "-f", "best",
            "-o", "/tmp/video.mp4",
            "https://example.com/video"
        ]);
    }

    #[test]
    fn test_command_builder_cookies_file_opt() {
        let some_path = Some(PathBuf::from("/tmp/cookies.txt"));
        let builder = CommandBuilder::new("yt-dlp")
            .cookies_file_opt(some_path.as_ref());
        assert_eq!(builder.get_args(), &["--cookies", "/tmp/cookies.txt"]);

        let none_path: Option<PathBuf> = None;
        let builder = CommandBuilder::new("yt-dlp")
            .cookies_file_opt(none_path.as_ref());
        assert!(builder.get_args().is_empty());
    }

    #[test]
    fn test_command_builder_with_options() {
        let options = DownloadOptions::new()
            .embed_metadata(true)
            .embed_thumbnail(true);
        let builder = CommandBuilder::new("yt-dlp")
            .with_options(&options)
            .url("https://example.com/video");
        let args = builder.get_args();
        assert!(args.contains(&"--embed-thumbnail".to_string()));
        assert!(args.contains(&"--embed-metadata".to_string()));
    }

    #[test]
    fn test_command_builder_ffmpeg_location() {
        let builder = CommandBuilder::new("yt-dlp")
            .ffmpeg_location("/usr/local/bin/ffmpeg");
        assert_eq!(builder.get_args(), &["--ffmpeg-location", "/usr/local/bin/ffmpeg"]);
    }

    #[test]
    fn test_build_with_env_path_prepend() {
        let mut env_vars = HashMap::new();
        env_vars.insert("PATH_PREPEND".to_string(), "/opt/bin".to_string());
        let builder = CommandBuilder::new("echo")
            .arg("test");
        let cmd = builder.build_with_env(&env_vars);
        let cmd_ref = cmd.as_std();
        let envs: HashMap<_, _> = cmd_ref.get_envs()
            .filter_map(|(k, v)| Some((k.to_string_lossy().to_string(), v?.to_string_lossy().to_string())))
            .collect();
        assert!(envs.get("PATH").unwrap().starts_with("/opt/bin:"));
    }
}
