use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Default,
    Best,
    Worst,
    BestVideo,
    BestAudio,
    Custom(String)
}

impl OutputFormat {
    pub fn as_arg(&self) -> Option<String> {
        match self {
            OutputFormat::Default => None,
            OutputFormat::Best => Some("best".to_string()),
            OutputFormat::Worst => Some("worst".to_string()),
            OutputFormat::BestVideo => Some("bestvideo".to_string()),
            OutputFormat::BestAudio => Some("bestaudio".to_string()),
            OutputFormat::Custom(s) => Some(s.clone())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum Container {
    #[default]
    Default,
    Mp4,
    Mkv,
    Webm,
    Mp3,
    M4a,
    Opus,
    Flac,
    Custom(String)
}

impl Container {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Container::Default => None,
            Container::Mp4 => Some("mp4"),
            Container::Mkv => Some("mkv"),
            Container::Webm => Some("webm"),
            Container::Mp3 => Some("mp3"),
            Container::M4a => Some("m4a"),
            Container::Opus => Some("opus"),
            Container::Flac => Some("flac"),
            Container::Custom(s) => Some(s.as_str())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DownloadOptions {
    pub format: OutputFormat,
    pub container: Container,
    pub output_template: Option<String>,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_subtitles: bool,
    pub extract_audio: bool,
    pub audio_format: Option<String>,
    pub audio_quality: Option<String>,
    pub subtitles_langs: Vec<String>,
    pub write_subtitles: bool,
    pub write_thumbnail: bool,
    pub cookies_file: Option<PathBuf>,
    pub rate_limit: Option<String>,
    pub concurrent_fragments: Option<u32>,
    pub extra_args: Vec<String>
}

impl DownloadOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    pub fn container(mut self, container: Container) -> Self {
        self.container = container;
        self
    }

    pub fn output_template(mut self, template: impl Into<String>) -> Self {
        self.output_template = Some(template.into());
        self
    }

    pub fn embed_thumbnail(mut self, embed: bool) -> Self {
        self.embed_thumbnail = embed;
        self
    }

    pub fn embed_metadata(mut self, embed: bool) -> Self {
        self.embed_metadata = embed;
        self
    }

    pub fn embed_subtitles(mut self, embed: bool) -> Self {
        self.embed_subtitles = embed;
        self
    }

    pub fn extract_audio(mut self, extract: bool) -> Self {
        self.extract_audio = extract;
        self
    }

    pub fn audio_format(mut self, format: impl Into<String>) -> Self {
        self.audio_format = Some(format.into());
        self
    }

    pub fn audio_quality(mut self, quality: impl Into<String>) -> Self {
        self.audio_quality = Some(quality.into());
        self
    }

    pub fn subtitles_langs(mut self, langs: Vec<String>) -> Self {
        self.subtitles_langs = langs;
        self
    }

    pub fn write_subtitles(mut self, write: bool) -> Self {
        self.write_subtitles = write;
        self
    }

    pub fn write_thumbnail(mut self, write: bool) -> Self {
        self.write_thumbnail = write;
        self
    }

    pub fn cookies_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.cookies_file = Some(path.into());
        self
    }

    pub fn rate_limit(mut self, limit: impl Into<String>) -> Self {
        self.rate_limit = Some(limit.into());
        self
    }

    pub fn concurrent_fragments(mut self, count: u32) -> Self {
        self.concurrent_fragments = Some(count);
        self
    }

    pub fn extra_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    pub fn extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args.extend(args);
        self
    }
}
