use anyhow::Result;
use crate::cli::args::ConfigArgs;
use std::path::PathBuf;

pub async fn execute(args: ConfigArgs) -> Result<()> {
    let config_path = get_config_path();

    if args.path {
        println!("Configuration file path:");
        println!("  {}", config_path.display());
        return Ok(());
    }

    if args.init {
        println!("Initializing configuration file...");
        create_default_config(&config_path)?;
        println!("✅ Configuration file created: {}", config_path.display());
        println!("\nYou can now edit this file to customize default settings.");
        return Ok(());
    }

    if args.show {
        println!("Current configuration:");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            println!("\n{}", content);
        } else {
            println!("  No configuration file found.");
            println!("  Run 'auto-subs-tui config --init' to create one.");
        }
        return Ok(());
    }

    // Default: show help
    println!("Configuration management");
    println!("\nOptions:");
    println!("  --show   Show current configuration");
    println!("  --init   Initialize default configuration file");
    println!("  --path   Show configuration file path");
    
    Ok(())
}

fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        let app_config_dir = config_dir.join("auto-subs-tui");
        app_config_dir.join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

fn create_default_config(path: &PathBuf) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let default_config = r#"# Auto-Subs TUI Configuration File
# This file contains default settings for the application

[whisper]
# Default Whisper model (tiny, base, small, medium, large)
model = "base"
# Default language (use "auto" for auto-detection)
language = "en"
# Model storage directory
model_dir = "~/.cache/auto-subs-tui/models"

[audio]
# Audio sample rate in Hz
sample_rate = 16000
# Number of audio channels (1 for mono, 2 for stereo)
channels = 1
# Audio format (wav, mp3, flac)
format = "wav"

[subtitles]
# Subtitle format (srt, vtt, ass)
format = "srt"
# Default font size
font_size = 24
# Default font color (hex without #)
font_color = "FFFFFF"
# Default outline color (hex without #)
outline_color = "000000"
# Default position (top, middle, bottom)
position = "bottom"

[video]
# Video codec (libx264, libx265, vp9, or "copy" to preserve original)
codec = "libx264"
# Constant Rate Factor for quality (18-28, lower = better quality)
crf = 23
# Encoding preset (ultrafast, superfast, veryfast, faster, fast, medium, slow, slower, veryslow)
preset = "medium"

[paths]
# Default output directory (use "." for current directory)
output_dir = "."
# Temporary files directory
temp_dir = "/tmp"

[behavior]
# Keep intermediate files (audio, SRT) after processing
keep_files = false
# Overwrite output files without asking
auto_overwrite = false
"#;

    std::fs::write(path, default_config)?;
    Ok(())
}
