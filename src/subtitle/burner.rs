use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::app::ProgressMessage;

pub struct SubtitleBurner {
    pub use_overlay: bool,
    pub keep_overlay: bool,
    pub overlay_height: Option<u32>,
}

impl SubtitleBurner {
    pub fn new() -> Self {
        Self {
            use_overlay: true,
            keep_overlay: false,
            overlay_height: None,
        }
    }

    pub fn with_overlay(mut self, enabled: bool) -> Self {
        self.use_overlay = enabled;
        self
    }

    pub fn keep_overlay_file(mut self, keep: bool) -> Self {
        self.keep_overlay = keep;
        self
    }

    pub fn with_overlay_height(mut self, height: u32) -> Self {
        self.overlay_height = Some(height);
        self
    }

    /// Burn subtitles into video using FFmpeg
    /// Uses overlay method: creates transparent subtitle overlay, then merges with video
    pub fn burn(
        &self,
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        if self.use_overlay {
            self.burn_with_overlay(video_path, srt_path, output_path, progress_tx)
        } else {
            self.burn_direct(video_path, srt_path, output_path, progress_tx)
        }
    }

    /// Create subtitle overlay and merge with video
    fn burn_with_overlay(
        &self,
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.05,
            "Creating subtitle overlay workflow...".to_string(),
        ));

        // Get video dimensions
        let (width, height) = self.get_video_dimensions(video_path)?;
        
        // Calculate overlay dimensions
        // Keep full width, but use compact height for subtitles
        let overlay_height = self.overlay_height.unwrap_or(200); // Default: 200px for subtitle area
        let overlay_width = width;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            format!("Video: {}x{}, Overlay: {}x{} (compact subtitle area)", width, height, overlay_width, overlay_height),
        ));

        // Create temporary overlay file path
        let overlay_path = output_path.with_file_name(format!(
            "{}_overlay.mp4",
            output_path.file_stem().unwrap().to_string_lossy()
        ));

        // Step 1: Create compact overlay video with subtitles
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Step 1/2: Creating compact subtitle overlay...".to_string(),
        ));

        self.create_subtitle_overlay(
            video_path,
            srt_path,
            &overlay_path,
            overlay_width,
            overlay_height,
            width, // Pass full width for proper font scaling
        )?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.6,
            "Step 2/2: Merging overlay at bottom of video...".to_string(),
        ));

        // Step 2: Position overlay at bottom of video
        self.merge_overlay(
            video_path,
            &overlay_path,
            output_path,
            height,
        )?;

        // Cleanup temporary overlay file unless user wants to keep it
        if !self.keep_overlay {
            let _ = std::fs::remove_file(&overlay_path);
            let _ = progress_tx.send(ProgressMessage::Progress(
                0.95,
                "Cleaned up temporary overlay file".to_string(),
            ));
        } else {
            let _ = progress_tx.send(ProgressMessage::Progress(
                0.95,
                format!("Overlay saved to: {}", overlay_path.display()),
            ));
        }

        let _ = progress_tx.send(ProgressMessage::Progress(
            1.0,
            format!("Output saved to: {}", output_path.display()),
        ));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(())
    }

    /// Create a transparent overlay video with only subtitles
    fn create_subtitle_overlay(
        &self,
        video_path: &Path,
        srt_path: &Path,
        overlay_path: &Path,
        width: u32,
        height: u32,
        original_width: u32, // For font size calculation
    ) -> Result<()> {
        // Get video duration and framerate
        let duration = self.get_video_duration(video_path)?;
        let fps = self.get_video_fps(video_path)?;

        // Escape the SRT path for FFmpeg filter
        let srt_path_str = srt_path.to_str().unwrap()
            .replace("\\", "/")
            .replace(":", "\\:");

        // Calculate font size to fill the overlay space
        // Use 35-40% of overlay height for good readability
        // This ensures text uses the available space well
        let font_size = (height as f64 * 0.38).max(24.0) as u32;

        // Reduce margin to maximize space usage
        let margin_v = (height as f64 * 0.1) as u32; // 10% margin

        // Create transparent video with subtitles using VP9 codec with alpha
        // Force subtitles to render at proper size to fill the overlay area
        let filter = format!(
            "color=c=black@0.0:s={}x{}:d={},format=yuva420p,subtitles='{}':force_style='FontSize={},MarginV={}'",
            width, height, duration, srt_path_str, font_size, margin_v
        );

        let output = Command::new("ffmpeg")
            .args([
                "-f", "lavfi",
                "-i", &filter,
                "-r", &fps.to_string(),
                "-c:v", "libvpx-vp9",
                "-pix_fmt", "yuva420p",
                "-auto-alt-ref", "0",
                "-b:v", "1M",
                "-y",
                overlay_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to create subtitle overlay")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create overlay: {}", stderr);
        }

        Ok(())
    }

    /// Merge overlay video with original video
    fn merge_overlay(
        &self,
        video_path: &Path,
        overlay_path: &Path,
        output_path: &Path,
        video_height: u32,
    ) -> Result<()> {
        // Get overlay height to calculate position
        let overlay_height = self.get_overlay_height(overlay_path)?;
        
        // Position overlay at bottom of video
        let y_position = video_height - overlay_height;

        // Use overlay filter to combine videos at bottom
        let output = Command::new("ffmpeg")
            .args([
                "-i", video_path.to_str().unwrap(),
                "-i", overlay_path.to_str().unwrap(),
                "-filter_complex", &format!("[0:v][1:v]overlay=0:{}", y_position),
                "-c:a", "copy",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to merge overlay with video")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to merge overlay: {}", stderr);
        }

        Ok(())
    }

    /// Direct burn method (old approach) - kept for compatibility
    fn burn_direct(
        &self,
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            "Using direct burn method...".to_string(),
        ));

        let srt_path_str = srt_path.to_str().unwrap().replace("\\", "/").replace(":", "\\:");

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Running FFmpeg...".to_string(),
        ));

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
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("FFmpeg failed: {}", stderr);
        }

        let _ = progress_tx.send(ProgressMessage::Progress(
            1.0,
            format!("Output saved to: {}", output_path.display()),
        ));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(())
    }

    /// Get video framerate
    fn get_video_fps(&self, video_path: &Path) -> Result<u32> {
        let output = Command::new("ffprobe")
            .args([
                "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=r_frame_rate",
                "-of", "default=noprint_wrappers=1:nokey=1",
                video_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to get video fps")?;

        let fps_str = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = fps_str.trim().split('/').collect();
        
        if parts.len() == 2 {
            let num = parts[0].parse::<f64>().unwrap_or(30.0);
            let den = parts[1].parse::<f64>().unwrap_or(1.0);
            Ok((num / den).round() as u32)
        } else {
            Ok(30) // Default to 30fps
        }
    }

    /// Get overlay video height
    fn get_overlay_height(&self, overlay_path: &Path) -> Result<u32> {
        let (_, height) = self.get_video_dimensions(overlay_path)?;
        Ok(height)
    }

    /// Get video dimensions using ffprobe
    fn get_video_dimensions(&self, video_path: &Path) -> Result<(u32, u32)> {
        let output = Command::new("ffprobe")
            .args([
                "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=width,height",
                "-of", "csv=s=x:p=0",
                video_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to run ffprobe")?;

        let dimensions = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = dimensions.trim().split('x').collect();
        
        if parts.len() != 2 {
            anyhow::bail!("Failed to parse video dimensions");
        }

        let width = parts[0].parse::<u32>().context("Invalid width")?;
        let height = parts[1].parse::<u32>().context("Invalid height")?;

        Ok((width, height))
    }

    /// Get video duration in seconds
    fn get_video_duration(&self, video_path: &Path) -> Result<f64> {
        let output = Command::new("ffprobe")
            .args([
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                video_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to get video duration")?;

        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str.trim().parse::<f64>().context("Invalid duration")
    }
}
