# Agent Guidelines for auto-subs-tui

This document provides coding standards and guidelines for AI agents working on the auto-subs-tui codebase.

## Project Overview

**auto-subs-tui** is a Rust-based terminal application for automatic subtitle generation and video processing. It uses Whisper for transcription, FFmpeg for video/audio processing, and Ratatui for the TUI interface.

**Language:** Rust (Edition 2021)  
**Main Binary:** `auto-subs-tui`

## Build, Lint & Test Commands

### Building
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run in debug mode
cargo run

# Run with arguments
cargo run -- process video.mp4
```

### Linting & Formatting
```bash
# Format code (auto-fix)
cargo fmt

# Check formatting without modifying files
cargo fmt -- --check

# Run Clippy linter
cargo clippy

# Clippy with all warnings as errors
cargo clippy -- -D warnings
```

### Testing
```bash
# Run all tests
cargo test

# Run all tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Run doc tests
cargo test --doc
```

**Note:** This project currently has no test suite. When adding tests, follow Rust conventions:
- Unit tests: Add `#[cfg(test)]` modules in the same file
- Integration tests: Create files in `tests/` directory
- Test file naming: `tests/integration_test.rs`

### Running the Application
```bash
# Interactive TUI mode
./target/release/auto-subs-tui

# Process a video (full pipeline)
./target/release/auto-subs-tui process video.mp4

# Extract audio only
./target/release/auto-subs-tui extract video.mp4 -o audio.wav

# Transcribe audio
./target/release/auto-subs-tui transcribe audio.wav -o subtitles.srt

# Burn subtitles
./target/release/auto-subs-tui burn video.mp4 subtitles.srt -o output.mp4
```

## Code Style Guidelines

### Imports
- Group imports in this order:
  1. External crates (`use anyhow::...`)
  2. Standard library (`use std::...`)
  3. Internal modules (`use crate::...`)
- Use explicit imports rather than glob imports (avoid `use module::*`)
- Example:
```rust
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::mpsc::Sender;

use crate::app::ProgressMessage;
```

### Formatting
- **Indentation:** 4 spaces (enforced by rustfmt)
- **Line length:** 100 characters (rustfmt default)
- **Braces:** Same line for functions/structs, new line for match arms
- Use `rustfmt` to auto-format before committing
- Trailing commas in multi-line lists/function calls

### Types & Naming Conventions
- **Structs/Enums:** PascalCase (`AudioExtractor`, `WhisperModel`)
- **Functions/Variables:** snake_case (`extract_audio`, `video_path`)
- **Constants:** SCREAMING_SNAKE_CASE (`DEFAULT_SAMPLE_RATE`)
- **Type Aliases:** PascalCase (`type Result<T> = std::result::Result<T, Error>`)
- Prefer explicit types over inference when it improves clarity
- Use `PathBuf` for owned paths, `&Path` for borrowed paths
- Use `String` for owned strings, `&str` for borrowed strings

### Error Handling
- Use `anyhow::Result<T>` for functions that can fail
- Use `.context("message")` to add context to errors
- Use `anyhow::bail!("message")` for early returns with error
- Example:
```rust
pub fn extract(&self, video_path: &Path) -> Result<()> {
    Command::new("ffmpeg")
        .arg("-version")
        .status()
        .context("FFmpeg not found. Please install FFmpeg and ensure it's in your PATH.")?;
    
    if !output.status.success() {
        anyhow::bail!("FFmpeg failed: {}", stderr);
    }
    
    Ok(())
}
```

### Async/Await
- Use `#[tokio::main]` for async main function
- Prefer async/await syntax over raw futures
- Use `tokio::spawn` for background tasks

### Documentation
- Add doc comments (`///`) for public items
- Use `//` for inline comments
- Document parameters, return values, and errors for complex functions
- Example:
```rust
/// Extract audio from video file to WAV format suitable for Whisper
/// (16kHz, mono, 16-bit PCM)
pub fn extract(
    &self,
    video_path: &Path,
    output_path: &Path,
    progress_tx: Sender<ProgressMessage>,
) -> Result<()> {
```

### Module Organization
- Each module has a `mod.rs` or is a single file
- Re-export public items from `mod.rs`
- Module structure:
  - `src/main.rs` - Entry point
  - `src/app.rs` - Core application logic
  - `src/cli/` - CLI argument parsing and command execution
  - `src/audio/` - Audio extraction
  - `src/subtitle/` - Subtitle generation and burning
  - `src/ui/` - TUI components
  - `src/utils/` - Shared utilities

### Clap CLI Patterns
- Use `#[derive(Parser)]` for structs
- Use `#[command(...)]` for metadata
- Use `#[arg(...)]` for field attributes
- Use `ValueEnum` for enumerated arguments
- Provide sensible defaults with `default_value`
- Add helpful descriptions in doc comments

## Project-Specific Conventions

### Progress Reporting
- Use `ProgressMessage` enum for async progress updates
- Send progress via `mpsc::Sender<ProgressMessage>`
- Progress values: 0.0 to 1.0 (percentage: multiply by 100)

### FFmpeg Integration
- Check for FFmpeg availability before running commands
- Use explicit arguments array for clarity
- Capture stdout/stderr for error reporting
- Standard audio format: 16kHz mono WAV for Whisper

### File Paths
- Use `PathBuf` for owned paths, `&Path` for references
- Use `.with_extension()` and `.with_file_name()` for path manipulation
- Validate file existence before processing
- Default output naming: `<input>_subtitled.<ext>` for videos

### Configuration
- Support TOML configuration files
- Location: `~/.config/auto-subs-tui/config.toml`
- Use `serde` for serialization
- CLI args override config file values

## Common Patterns

### Command Execution
```rust
let output = Command::new("ffmpeg")
    .args(["-i", input, "-o", output])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .context("Failed to run FFmpeg")?;

if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("FFmpeg failed: {}", stderr);
}
```

### Threading with Progress
```rust
let (tx, rx) = mpsc::channel();
std::thread::spawn(move || {
    let _ = processor.process(input, output, tx);
});

while let Ok(msg) = rx.recv() {
    match msg {
        ProgressMessage::Progress(p, m) => println!("[{:.0}%] {}", p * 100.0, m),
        ProgressMessage::Complete => break,
        ProgressMessage::Error(e) => return Err(e.into()),
    }
}
```

## Git Workflow

- Write clear, descriptive commit messages
- Use conventional commits format: `type: description`
  - `feat:` new features
  - `fix:` bug fixes
  - `refactor:` code refactoring
  - `docs:` documentation updates
  - `chore:` maintenance tasks

## Dependencies

**Core:**
- `tokio` - async runtime
- `anyhow` - error handling
- `clap` - CLI parsing

**TUI:**
- `ratatui` - terminal UI framework
- `crossterm` - terminal manipulation

**Media Processing:**
- `ffmpeg-sidecar` - FFmpeg operations
- `whisper-rs` - speech-to-text
- `srtlib` - subtitle handling
- `hound` - WAV file reading

## External Dependencies

- **FFmpeg** must be installed and in PATH
- **Whisper models** downloaded automatically to `~/.cache/auto-subs-tui/models/`

## When Making Changes

1. Run `cargo fmt` before committing
2. Run `cargo clippy` and fix warnings
3. Test the application manually (no automated tests yet)
4. Update README.md if adding new features or changing CLI
5. Ensure backwards compatibility for CLI commands
