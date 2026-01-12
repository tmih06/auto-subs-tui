# Auto-Subs TUI

A terminal-based user interface application for creating and editing video subtitles, built in Rust.

## Features

- **Extract Audio**: Automatically extract audio from video files (MP4, MKV, AVI, MOV, WebM, etc.)
- **Generate Subtitles**: Transcribe speech to text using OpenAI's Whisper model (runs locally)
- **Edit Subtitles**: Review and edit generated subtitles with a full-featured TUI editor
- **Burn Subtitles**: Hardcode subtitles into the video file

## Requirements

- **Rust** (1.70+ recommended)
- **FFmpeg** installed and available in PATH
- ~500MB disk space for the Whisper model (downloaded automatically on first use)

### Installing FFmpeg

**Ubuntu/Debian:**
```bash
sudo apt update && sudo apt install ffmpeg
```

**Fedora:**
```bash
sudo dnf install ffmpeg
```

**macOS:**
```bash
brew install ffmpeg
```

**Arch Linux:**
```bash
sudo pacman -S ffmpeg
```

## Installation

```bash
# Clone the repository
cd auto-subs-tui

# Build the application
cargo build --release

# Run the application
cargo run --release
```

## Usage

### Workflow

1. **Start the app**: Run `cargo run --release`
2. **Select a video**: Press `S` or `Enter` to browse for a video file
3. **Wait for processing**: Audio extraction and transcription happen automatically
4. **Edit subtitles**: Review and modify the generated subtitles
5. **Burn to video**: Press `B` to hardcode subtitles into the video

### Keyboard Shortcuts

#### Home Screen
| Key | Action |
|-----|--------|
| `S` / `Enter` | Start - Select video file |
| `L` | Load existing SRT file |
| `Q` | Quit |

#### File Browser
| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | Select file / Enter directory |
| `.` | Toggle hidden files |
| `Esc` | Go back |

#### Subtitle Editor
| Key | Action |
|-----|--------|
| `↑` / `k` | Previous subtitle |
| `↓` / `j` | Next subtitle |
| `e` / `Enter` | Edit selected subtitle |
| `a` | Add new subtitle |
| `d` | Delete selected subtitle |
| `[` / `]` | Adjust start time (-/+ 100ms) |
| `{` / `}` | Adjust end time (-/+ 100ms) |
| `s` | Save SRT file |
| `b` | Burn subtitles into video |
| `Esc` | Back to home |
| `q` | Quit |

#### Edit Mode
| Key | Action |
|-----|--------|
| Type | Edit text |
| `Enter` | Save changes |
| `Esc` | Cancel editing |

## Output Files

When you process a video, the following files are created:

- `video.wav` - Extracted audio (16kHz mono WAV)
- `video.srt` - Generated subtitles in SRT format
- `video_subtitled.mp4` - Video with burned-in subtitles

## Technology Stack

- **[Ratatui](https://ratatui.rs/)** - Terminal UI framework
- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Cross-platform terminal manipulation
- **[Whisper-rs](https://github.com/tazz4843/whisper-rs)** - Rust bindings for whisper.cpp
- **[FFmpeg](https://ffmpeg.org/)** - Audio extraction and subtitle burning
- **[Tokio](https://tokio.rs/)** - Async runtime

## License

MIT License
