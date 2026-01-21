use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "auto-subs-tui")]
#[command(author = "tmih06")]
#[command(version = "0.1.0")]
#[command(about = "Automatic subtitle generation and video processing tool", long_about = None)]
#[command(arg_required_else_help = false)]
pub struct Cli {
    /// Subcommand to execute (if none provided, launches TUI)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Increase output verbosity
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Overwrite output files without asking
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    /// Never overwrite output files
    #[arg(short = 'n', long, global = true)]
    pub no_overwrite: bool,

    /// Use custom config file
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Process video: extract audio, transcribe, and burn subtitles (full pipeline)
    Process(ProcessArgs),

    /// Extract audio from video file
    Extract(ExtractArgs),

    /// Transcribe audio to subtitles
    Transcribe(TranscribeArgs),

    /// Burn subtitles into video
    Burn(BurnArgs),

    /// Launch TUI editor for existing SRT file
    Edit(EditArgs),

    /// Manage configuration
    Config(ConfigArgs),
}

#[derive(Parser, Debug)]
pub struct ProcessArgs {
    /// Input video file path
    #[arg(value_name = "VIDEO")]
    pub input: PathBuf,

    /// Output video file path (default: <input>_subtitled.<ext>)
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Whisper model to use
    #[arg(short = 'm', long, default_value = "base")]
    pub model: WhisperModel,

    /// Language code (e.g., en, fr, es) or 'auto' for auto-detection
    #[arg(short = 'l', long, default_value = "auto")]
    pub language: String,

    /// SRT file output path (default: <input>.srt)
    #[arg(long, value_name = "FILE")]
    pub srt_output: Option<PathBuf>,

    /// Audio file output path (default: <input>.wav)
    #[arg(long, value_name = "FILE")]
    pub audio_output: Option<PathBuf>,

    /// Keep intermediate files (audio, SRT)
    #[arg(short = 'k', long)]
    pub keep_files: bool,

    /// Translate to English (if source is not English)
    #[arg(long)]
    pub translate: bool,

    /// Subtitle font size
    #[arg(long, default_value = "24")]
    pub font_size: u32,

    /// Subtitle font color in hex (e.g., FFFFFF for white)
    #[arg(long, default_value = "FFFFFF")]
    pub font_color: String,

    /// Subtitle outline color in hex
    #[arg(long, default_value = "000000")]
    pub outline_color: String,

    /// Custom FFmpeg subtitle style string
    #[arg(long)]
    pub style: Option<String>,

    /// Use overlay method (creates separate subtitle overlay video)
    #[arg(long, default_value = "true")]
    pub use_overlay: bool,

    /// Keep overlay file for reuse or customization
    #[arg(long)]
    pub keep_overlay: bool,

    /// Overlay video height in pixels (default: 1/4 of video height)
    #[arg(long)]
    pub overlay_height: Option<u32>,
}

#[derive(Parser, Debug)]
pub struct ExtractArgs {
    /// Input video file path
    #[arg(value_name = "VIDEO")]
    pub input: PathBuf,

    /// Output audio file path (default: <input>.wav)
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Audio sample rate in Hz
    #[arg(long, default_value = "16000")]
    pub sample_rate: u32,

    /// Number of audio channels (1 for mono, 2 for stereo)
    #[arg(long, default_value = "1")]
    pub channels: u32,

    /// Audio format
    #[arg(long, default_value = "wav")]
    pub format: AudioFormat,
}

#[derive(Parser, Debug)]
pub struct TranscribeArgs {
    /// Input audio file path
    #[arg(value_name = "AUDIO")]
    pub input: PathBuf,

    /// Output SRT file path (default: <input>.srt)
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Whisper model to use
    #[arg(short = 'm', long, default_value = "base")]
    pub model: WhisperModel,

    /// Language code (e.g., en, fr, es) or 'auto' for auto-detection
    #[arg(short = 'l', long, default_value = "auto")]
    pub language: String,

    /// Translate to English (if source is not English)
    #[arg(long)]
    pub translate: bool,

    /// Transcription provider to use
    #[arg(long, default_value = "whisper")]
    pub provider: String,
}

#[derive(Parser, Debug)]
pub struct BurnArgs {
    /// Input video file path
    #[arg(value_name = "VIDEO")]
    pub video: PathBuf,

    /// Input SRT subtitle file path
    #[arg(value_name = "SUBTITLES")]
    pub subtitles: PathBuf,

    /// Output video file path (default: <video>_subtitled.<ext>)
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Subtitle font size
    #[arg(long, default_value = "24")]
    pub font_size: u32,

    /// Subtitle font color in hex (e.g., FFFFFF for white)
    #[arg(long, default_value = "FFFFFF")]
    pub font_color: String,

    /// Subtitle outline color in hex
    #[arg(long, default_value = "000000")]
    pub outline_color: String,

    /// Subtitle position
    #[arg(long, default_value = "bottom")]
    pub position: SubtitlePosition,

    /// Custom FFmpeg subtitle style string (overrides individual style options)
    #[arg(long)]
    pub style: Option<String>,

    /// Video codec (use 'copy' to preserve original)
    #[arg(long, default_value = "libx264")]
    pub video_codec: String,

    /// Constant Rate Factor for quality (lower = better quality, 18-28 typical)
    #[arg(long, default_value = "23")]
    pub crf: u32,

    /// Encoding preset (ultrafast, fast, medium, slow, veryslow)
    #[arg(long, default_value = "medium")]
    pub preset: String,

    /// Use overlay method (creates separate subtitle overlay video)
    #[arg(long, default_value = "true")]
    pub use_overlay: bool,

    /// Keep overlay file for reuse or customization
    #[arg(long)]
    pub keep_overlay: bool,

    /// Overlay video height in pixels (default: 1/4 of video height)
    #[arg(long)]
    pub overlay_height: Option<u32>,
}

#[derive(Parser, Debug)]
pub struct EditArgs {
    /// SRT file to edit
    #[arg(value_name = "SRT_FILE")]
    pub input: PathBuf,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Show current configuration
    #[arg(long)]
    pub show: bool,

    /// Initialize default configuration file
    #[arg(long)]
    pub init: bool,

    /// Show configuration file path
    #[arg(long)]
    pub path: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum WhisperModel {
    /// Tiny model (~75MB, fastest, least accurate)
    Tiny,
    /// Base model (~150MB, balanced speed/accuracy)
    Base,
    /// Small model (~500MB, better accuracy)
    Small,
    /// Medium model (~1.5GB, high accuracy)
    Medium,
    /// Large model (~3GB, best accuracy, slowest)
    Large,
}

impl WhisperModel {
    pub fn filename(&self) -> &str {
        match self {
            WhisperModel::Tiny => "ggml-tiny.en.bin",
            WhisperModel::Base => "ggml-base.en.bin",
            WhisperModel::Small => "ggml-small.en.bin",
            WhisperModel::Medium => "ggml-medium.en.bin",
            WhisperModel::Large => "ggml-large.bin",
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
            WhisperModel::Medium => "medium",
            WhisperModel::Large => "large",
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
}

impl AudioFormat {
    pub fn as_str(&self) -> &str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SubtitlePosition {
    Top,
    Middle,
    Bottom,
}

impl SubtitlePosition {
    pub fn as_str(&self) -> &str {
        match self {
            SubtitlePosition::Top => "top",
            SubtitlePosition::Middle => "middle",
            SubtitlePosition::Bottom => "bottom",
        }
    }
}
