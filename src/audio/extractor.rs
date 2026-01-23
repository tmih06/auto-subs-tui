use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::app::ProgressMessage;

pub struct AudioExtractor;

impl AudioExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract audio from video file to WAV format suitable for Whisper
    /// (16kHz, mono, 16-bit PCM)
    pub fn extract(
        &self,
        video_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            "Starting FFmpeg...".to_string(),
        ));

        // Check if ffmpeg is available
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("FFmpeg not found. Please install FFmpeg and ensure it's in your PATH.")?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Extracting audio...".to_string(),
        ));

        // Run ffmpeg to extract audio
        // -i input: input file
        // -vn: no video
        // -ar 16000: sample rate 16kHz (required by Whisper)
        // -ac 1: mono channel
        // -c:a pcm_s16le: 16-bit PCM
        // -y: overwrite output file
        let output = Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-vn",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-c:a",
                "pcm_s16le",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to run FFmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("FFmpeg failed: {}", stderr);
        }

        let _ = progress_tx.send(ProgressMessage::Progress(
            1.0,
            "Audio extraction complete!".to_string(),
        ));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(())
    }
}
