use crate::app::ProgressMessage;
use crate::cli::args::BurnArgs;
use crate::subtitle::burner::SubtitleBurner;
use anyhow::Result;
use std::sync::mpsc;

pub async fn execute(args: BurnArgs) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║          AUTO-SUBS TUI - BURN MODE                         ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Validate input files
    if !args.video.exists() {
        anyhow::bail!("Video file not found: {}", args.video.display());
    }
    if !args.subtitles.exists() {
        anyhow::bail!("Subtitle file not found: {}", args.subtitles.display());
    }

    // Determine output path
    let output_path = args.output.clone().unwrap_or_else(|| {
        args.video.with_file_name(format!(
            "{}_subtitled.{}",
            args.video.file_stem().unwrap().to_string_lossy(),
            args.video.extension().unwrap().to_string_lossy()
        ))
    });

    println!("📹 Input video: {}", args.video.display());
    println!("📄 Input subtitles: {}", args.subtitles.display());
    println!("🎬 Output video: {}", output_path.display());
    println!("⚙️  Font size: {}", args.font_size);
    println!("⚙️  Font color: #{}", args.font_color);
    println!("⚙️  Outline color: #{}", args.outline_color);
    println!("⚙️  Position: {}", args.position.as_str());

    if args.use_overlay {
        println!("🎨 Method: Overlay (separate subtitle layer)");
        if let Some(height) = args.overlay_height {
            println!("⚙️  Overlay height: {}px", height);
        }
        if let Some(width) = args.overlay_width {
            println!("⚙️  Overlay width: {}px", width);
        }
        if let Some(x_offset) = args.overlay_x_offset {
            println!("⚙️  Overlay X offset: {}px", x_offset);
        }
        if let Some(y_offset) = args.overlay_y_offset {
            println!("⚙️  Overlay Y offset: {}px", y_offset);
        }
        if args.keep_overlay {
            println!("💾 Keeping overlay file for customization");
        }
    } else {
        println!("🎨 Method: Direct burn");
    }

    println!("⚙️  Video codec: {}", args.video_codec);
    if args.video_codec != "copy" {
        println!("⚙️  CRF: {}", args.crf);
        println!("⚙️  Preset: {}\n", args.preset);
    } else {
        println!();
    }

    // Burn subtitles with overlay method
    println!("Burning subtitles into video...");
    let (tx, rx) = mpsc::channel();

    let mut burner = SubtitleBurner::new()
        .with_overlay(args.use_overlay)
        .keep_overlay_file(args.keep_overlay);

    if let Some(height) = args.overlay_height {
        burner = burner.with_overlay_height(height);
    }
    if let Some(width) = args.overlay_width {
        burner = burner.with_overlay_width(width);
    }
    if let Some(x_offset) = args.overlay_x_offset {
        burner = burner.with_overlay_x_offset(x_offset);
    }
    if let Some(y_offset) = args.overlay_y_offset {
        burner = burner.with_overlay_y_offset(y_offset);
    }

    let video_clone = args.video.clone();
    let srt_clone = args.subtitles.clone();
    let output_clone = output_path.clone();
    std::thread::spawn(move || {
        let _ = burner.burn(&video_clone, &srt_clone, &output_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("  [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                anyhow::bail!("Subtitle burning failed: {}", e);
            }
        }
    }

    println!("\n✅ Subtitle burning complete!");
    println!("   Output: {}", output_path.display());

    if args.keep_overlay {
        let overlay_path = output_path.with_file_name(format!(
            "{}_overlay.mp4",
            output_path.file_stem().unwrap().to_string_lossy()
        ));
        println!("   Overlay: {}", overlay_path.display());
    }

    Ok(())
}
