use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;

use crate::app::ProgressMessage;

pub struct SubtitleBurner {
    pub use_overlay: bool,
    pub keep_overlay: bool,
    pub overlay_height: Option<u32>,
    pub overlay_width: Option<u32>,
    pub overlay_x_offset: Option<i32>,
    pub overlay_y_offset: Option<i32>,
}

impl SubtitleBurner {
    pub fn new() -> Self {
        Self {
            use_overlay: true,
            keep_overlay: false,
            overlay_height: None,
            overlay_width: None,
            overlay_x_offset: None,
            overlay_y_offset: None,
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

    pub fn with_overlay_width(mut self, width: u32) -> Self {
        self.overlay_width = Some(width);
        self
    }

    pub fn with_overlay_x_offset(mut self, offset: i32) -> Self {
        self.overlay_x_offset = Some(offset);
        self
    }

    pub fn with_overlay_y_offset(mut self, offset: i32) -> Self {
        self.overlay_y_offset = Some(offset);
        self
    }

    /// Preview video with overlay positioned (launches external player)
    pub fn preview_with_overlay(
        &self,
        video_path: &Path,
        srt_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            "Preparing preview...".to_string(),
        ));

        // Get video dimensions
        let (video_width, video_height) = self.get_video_dimensions(video_path)?;

        // Calculate overlay dimensions and position
        let overlay_height = self.overlay_height.unwrap_or(200);
        let overlay_width = self.overlay_width.unwrap_or(video_width);

        let x_offset = self.overlay_x_offset.unwrap_or(0);
        let x_centered = ((video_width - overlay_width) / 2) as i32;
        let x_position = (x_centered + x_offset).max(0);

        let y_offset = self.overlay_y_offset.unwrap_or(0);
        let y_bottom = (video_height - overlay_height) as i32;
        let y_position = (y_bottom + y_offset).max(0);

        // Escape the SRT path for FFmpeg filter
        let srt_path_str = srt_path
            .to_str()
            .unwrap()
            .replace("\\", "/")
            .replace(":", "\\:");

        // Calculate font size based on overlay height
        let font_size = (overlay_height as f64 * 0.38).max(24.0) as u32;
        let margin_v = (overlay_height as f64 * 0.1) as u32;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.5,
            format!(
                "Launching preview (Overlay: {}x{} at {},{})",
                overlay_width, overlay_height, x_position, y_position
            ),
        ));

        // Create filter to overlay subtitles directly on video
        // This creates a transparent overlay and positions it
        let filter = format!(
            "subtitles='{}':force_style='FontSize={},MarginV={}'",
            srt_path_str, font_size, margin_v
        );

        // Try ffplay first, then mpv as fallback
        let player_result = self.try_launch_player(
            video_path,
            &filter,
            "ffplay",
            &[
                "-i",
                video_path.to_str().unwrap(),
                "-vf",
                &filter,
                "-window_title",
                "Subtitle Preview (Press Q to close)",
                "-autoexit",
            ],
        );

        if player_result.is_err() {
            // Try mpv as fallback
            let _ = progress_tx.send(ProgressMessage::Progress(
                0.6,
                "ffplay not found, trying mpv...".to_string(),
            ));

            self.try_launch_player(
                video_path,
                &filter,
                "mpv",
                &[
                    video_path.to_str().unwrap(),
                    &format!("--vf=lavfi=[{}]", filter),
                    "--title=Subtitle Preview (Press Q to close)",
                    "--keep-open=no",
                ],
            )?;
        }

        let _ = progress_tx.send(ProgressMessage::Progress(1.0, "Preview closed".to_string()));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(())
    }

    /// Launch preview process without blocking - returns the Child process
    /// Uses MPV with IPC for real-time overlay updates (no video restart needed)
    pub fn launch_preview_process_with_ipc(
        &self,
        video_path: &Path,
        srt_path: &Path,
        socket_path: &Path,
    ) -> Result<(Child, u32, u32)> {
        // Get video dimensions
        let (video_width, video_height) = self.get_video_dimensions(video_path)?;

        // Calculate overlay dimensions and position
        let overlay_height = self.overlay_height.unwrap_or(200);
        let overlay_width = self.overlay_width.unwrap_or(video_width);

        let x_offset = self.overlay_x_offset.unwrap_or(0);
        let x_centered = if overlay_width > video_width {
            0
        } else {
            ((video_width - overlay_width) / 2) as i32
        };
        let x_position = (x_centered + x_offset).max(0);

        let y_offset = self.overlay_y_offset.unwrap_or(0);
        let y_bottom = if overlay_height > video_height {
            0
        } else {
            (video_height - overlay_height) as i32
        };
        let y_position = (y_bottom + y_offset).max(0);

        // Calculate font size based on overlay height
        let font_size = (overlay_height as f64 * 0.38).max(24.0) as u32;

        // Create drawbox filter to show subtitle area
        let drawbox_filter = format!(
            "drawbox=x={}:y={}:w={}:h={}:color=yellow@0.3:t=3",
            x_position, y_position, overlay_width, overlay_height
        );

        // Launch MPV with IPC socket
        // Use simple approach: subtitles via --sub-file, overlay via --vf
        // DEBUG: Don't suppress stderr to see errors
        let child = Command::new("mpv")
            .arg(format!(
                "--input-ipc-server={}",
                socket_path.to_str().unwrap()
            ))
            .arg("--loop-file=inf")
            .arg("--keep-open=yes")
            .arg(format!(
                "--title=Preview - Adjust: h/H w/W x/X y/Y (p=stop)"
            ))
            .arg(format!("--sub-file={}", srt_path.to_str().unwrap()))
            .arg(format!("--sub-font-size={}", font_size))
            .arg(format!("--vf={}", drawbox_filter))
            .arg(video_path.to_str().unwrap())
            // Temporarily show errors for debugging
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to launch MPV. Please install mpv.")?;

        Ok((child, video_width, video_height))
    }

    /// Launch preview process without blocking - returns the Child process
    /// This allows the preview to run in background while UI remains responsive
    pub fn launch_preview_process(&self, video_path: &Path, srt_path: &Path) -> Result<Child> {
        // Get video dimensions
        let (video_width, video_height) = self.get_video_dimensions(video_path)?;

        // Calculate overlay dimensions and position
        let overlay_height = self.overlay_height.unwrap_or(200);
        let overlay_width = self.overlay_width.unwrap_or(video_width);

        let x_offset = self.overlay_x_offset.unwrap_or(0);
        // Prevent underflow if overlay_width > video_width
        let x_centered = if overlay_width > video_width {
            0
        } else {
            ((video_width - overlay_width) / 2) as i32
        };
        let _x_position = (x_centered + x_offset).max(0);

        let y_offset = self.overlay_y_offset.unwrap_or(0);
        // Prevent underflow if overlay_height > video_height
        let y_bottom = if overlay_height > video_height {
            0
        } else {
            (video_height - overlay_height) as i32
        };
        let _y_position = (y_bottom + y_offset).max(0);

        // Escape the SRT path for FFmpeg filter
        let srt_path_str = srt_path
            .to_str()
            .unwrap()
            .replace("\\", "/")
            .replace(":", "\\:");

        // Calculate font size based on overlay height
        let font_size = (overlay_height as f64 * 0.38).max(24.0) as u32;
        let margin_v = (overlay_height as f64 * 0.1) as u32;

        // Create filter to overlay subtitles directly on video
        let filter = format!(
            "subtitles='{}':force_style='FontSize={},MarginV={}'",
            srt_path_str, font_size, margin_v
        );

        // Try ffplay first
        let child = Command::new("ffplay")
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-vf",
                &filter,
                "-window_title",
                "Subtitle Preview (Press Q to close, or P in editor to stop)",
                "-autoexit",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(child) => Ok(child),
            Err(_) => {
                // Try mpv as fallback
                Ok(Command::new("mpv")
                    .args([
                        video_path.to_str().unwrap(),
                        &format!("--vf=lavfi=[{}]", filter),
                        "--title=Subtitle Preview (Press Q to close, or P in editor to stop)",
                        "--keep-open=no",
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .context("Failed to launch ffplay or mpv. Please install one of them.")?)
            }
        }
    }

    fn try_launch_player(
        &self,
        _video_path: &Path,
        _filter: &str,
        player: &str,
        args: &[&str],
    ) -> Result<()> {
        // Check if player exists
        Command::new(player)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context(format!("{} not found", player))?;

        // Launch player and wait for it to close
        let status = Command::new(player)
            .args(args)
            .status()
            .context(format!("Failed to launch {}", player))?;

        if !status.success() {
            anyhow::bail!("{} exited with error", player);
        }

        Ok(())
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

    /// Extract subtitle overlay only without burning into video
    pub fn extract_overlay(
        &self,
        video_path: &Path,
        srt_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<()> {
        let _ = progress_tx.send(ProgressMessage::Progress(
            0.05,
            "Creating subtitle overlay...".to_string(),
        ));

        // Get video dimensions
        let (width, height) = self.get_video_dimensions(video_path)?;

        // Calculate overlay dimensions
        let overlay_height = self.overlay_height.unwrap_or(200);
        let overlay_width = self.overlay_width.unwrap_or(width);

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            format!(
                "Video: {}x{}, Overlay: {}x{}",
                width, height, overlay_width, overlay_height
            ),
        ));

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Generating overlay with subtitles...".to_string(),
        ));

        // Create the subtitle overlay
        self.create_subtitle_overlay(
            video_path,
            srt_path,
            output_path,
            overlay_width,
            overlay_height,
            width,
        )?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            1.0,
            format!("Overlay saved to: {}", output_path.display()),
        ));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(())
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
        let overlay_width = self.overlay_width.unwrap_or(width); // Default: full video width

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            format!(
                "Video: {}x{}, Overlay: {}x{} (compact subtitle area)",
                width, height, overlay_width, overlay_height
            ),
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
        self.merge_overlay(video_path, &overlay_path, output_path, width, height)?;

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
        let srt_path_str = srt_path
            .to_str()
            .unwrap()
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
                "-f",
                "lavfi",
                "-i",
                &filter,
                "-r",
                &fps.to_string(),
                "-c:v",
                "libvpx-vp9",
                "-pix_fmt",
                "yuva420p",
                "-auto-alt-ref",
                "0",
                "-b:v",
                "1M",
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
        video_width: u32,
        video_height: u32,
    ) -> Result<()> {
        // Get overlay dimensions to calculate position
        let (overlay_width, overlay_height) = self.get_video_dimensions(overlay_path)?;

        // Calculate X position (centered by default, or with offset)
        let x_offset = self.overlay_x_offset.unwrap_or(0);
        let x_centered = ((video_width - overlay_width) / 2) as i32;
        let x_position = (x_centered + x_offset).max(0);

        // Calculate Y position (at bottom by default, or with offset)
        let y_offset = self.overlay_y_offset.unwrap_or(0);
        let y_bottom = (video_height - overlay_height) as i32;
        let y_position = (y_bottom + y_offset).max(0);

        // Use overlay filter to combine videos
        let output = Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-i",
                overlay_path.to_str().unwrap(),
                "-filter_complex",
                &format!("[0:v][1:v]overlay={}:{}", x_position, y_position),
                "-c:a",
                "copy",
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

        let srt_path_str = srt_path
            .to_str()
            .unwrap()
            .replace("\\", "/")
            .replace(":", "\\:");

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Running FFmpeg...".to_string(),
        ));

        let output = Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-vf",
                &format!("subtitles='{}'", srt_path_str),
                "-c:a",
                "copy",
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
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=r_frame_rate",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
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
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height",
                "-of",
                "csv=s=x:p=0",
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
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                video_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to get video duration")?;

        let duration_str = String::from_utf8_lossy(&output.stdout);
        duration_str
            .trim()
            .parse::<f64>()
            .context("Invalid duration")
    }
}
