use anyhow::Result;
use std::sync::mpsc;
use crate::audio::extractor::AudioExtractor;
use crate::subtitle::generator::SubtitleGenerator;
use crate::subtitle::burner::SubtitleBurner;
use crate::app::ProgressMessage;
use crate::cli::args::ProcessArgs;

pub async fn execute(args: ProcessArgs) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         AUTO-SUBS TUI - PROCESS MODE                       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Validate input file
    if !args.input.exists() {
        anyhow::bail!("Video file not found: {}", args.input.display());
    }
    println!("📹 Input video: {}", args.input.display());
    println!("🎯 Model: {}", args.model.as_str());
    println!("🌍 Language: {}\n", args.language);

    // Determine output paths
    let audio_path = args.audio_output.clone().unwrap_or_else(|| {
        args.input.with_extension("wav")
    });
    
    let srt_path = args.srt_output.clone().unwrap_or_else(|| {
        args.input.with_extension("srt")
    });
    
    let output_path = args.output.clone().unwrap_or_else(|| {
        args.input.with_file_name(format!(
            "{}_subtitled.{}",
            args.input.file_stem().unwrap().to_string_lossy(),
            args.input.extension().unwrap().to_string_lossy()
        ))
    });

    // Step 1: Extract audio
    println!("[1/3] Extracting audio...");
    let (tx, rx) = mpsc::channel();
    let extractor = AudioExtractor::new();
    
    let video_clone = args.input.clone();
    let audio_clone = audio_path.clone();
    std::thread::spawn(move || {
        let _ = extractor.extract(&video_clone, &audio_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("      [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                anyhow::bail!("Audio extraction failed: {}", e);
            }
        }
    }
    println!("      ✅ Audio extracted: {}", audio_path.display());

    // Step 2: Generate subtitles
    println!("\n[2/3] Generating subtitles with Whisper ({})...", args.model.as_str());
    println!("      (This may download the model on first run)");
    let (tx, rx) = mpsc::channel();
    let generator = SubtitleGenerator::new();
    
    let audio_clone = audio_path.clone();
    let srt_clone = srt_path.clone();
    std::thread::spawn(move || {
        let _ = generator.generate(&audio_clone, &srt_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("      [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                anyhow::bail!("Subtitle generation failed: {}", e);
            }
        }
    }
    println!("      ✅ Subtitles generated: {}", srt_path.display());

    // Show preview of generated subtitles
    if let Ok(content) = std::fs::read_to_string(&srt_path) {
        let lines: Vec<&str> = content.lines().take(15).collect();
        println!("\n      Preview (first few entries):");
        for line in lines {
            println!("      │ {}", line);
        }
        println!("      │ ...");
    }

    // Step 3: Burn subtitles
    println!("\n[3/3] Burning subtitles into video...");
    if args.use_overlay {
        println!("      Using overlay method (creates separate subtitle layer)");
        if args.keep_overlay {
            println!("      Keeping overlay file for customization");
        }
    }
    let (tx, rx) = mpsc::channel();
    
    let mut burner = SubtitleBurner::new()
        .with_overlay(args.use_overlay)
        .keep_overlay_file(args.keep_overlay);
    
    if let Some(height) = args.overlay_height {
        burner = burner.with_overlay_height(height);
    }
    
    let video_clone = args.input.clone();
    let srt_clone = srt_path.clone();
    let output_clone = output_path.clone();
    std::thread::spawn(move || {
        let _ = burner.burn(&video_clone, &srt_clone, &output_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("      [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                anyhow::bail!("Subtitle burning failed: {}", e);
            }
        }
    }
    println!("      ✅ Output video: {}", output_path.display());

    // Cleanup if requested
    if !args.keep_files {
        println!("\n🧹 Cleaning up intermediate files...");
        let _ = std::fs::remove_file(&audio_path);
        let _ = std::fs::remove_file(&srt_path);
        println!("      ✅ Removed: {}", audio_path.display());
        println!("      ✅ Removed: {}", srt_path.display());
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                  PROCESSING COMPLETE!                      ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    
    if args.keep_files {
        println!("\nGenerated files:");
        println!("  📁 {}", audio_path.display());
        println!("  📄 {}", srt_path.display());
        println!("  🎬 {}", output_path.display());
    } else {
        println!("\nOutput file:");
        println!("  🎬 {}", output_path.display());
    }

    Ok(())
}
