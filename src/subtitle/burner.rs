use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::app::ProgressMessage;

pub struct SubtitleBurner;

impl SubtitleBurner {
    pub fn new() -> Self {
        Self
    }

    /// Burn subtitles into video using FFmpeg
    pub fn burn(
        &self,
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            "Starting subtitle burning...".to_string(),
        ));

        // Escape the SRT path for FFmpeg filter (handle special characters)
        let srt_path_str = srt_path.to_str().unwrap().replace("\\", "/").replace(":", "\\:");

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Running FFmpeg...".to_string(),
        ));

        // Run ffmpeg with subtitles filter
        // The subtitles filter burns the SRT into the video
        let output = Command::new("ffmpeg")
            .args([
                "-i", video_path.to_str().unwrap(),
                "-vf", &format!("subtitles='{}'", srt_path_str),
                "-c:a", "copy",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to run FFmpeg")?;

        if !output.status.success() {
            let _stderr = String::from_utf8_lossy(&output.stderr);
            
            // Try alternative method using ASS filter if subtitles filter fails
            let _ = progress_tx.send(ProgressMessage::Progress(
                0.3,
                "Trying alternative encoding method...".to_string(),
            ));

            let output = Command::new("ffmpeg")
                .args([
                    "-i", video_path.to_str().unwrap(),
                    "-vf", &format!(
                        "subtitles={}:force_style='FontSize=24,PrimaryColour=&HFFFFFF,OutlineColour=&H000000,Outline=2'",
                        srt_path.to_str().unwrap()
                    ),
                    "-c:a", "copy",
                    "-y",
                    output_path.to_str().unwrap(),
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .context("Failed to run FFmpeg (alternative method)")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("FFmpeg failed: {}", stderr);
            }
        }

        let _ = progress_tx.send(ProgressMessage::Progress(
            1.0,
            format!("Output saved to: {}", output_path.display()),
        ));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(())
    }
}
