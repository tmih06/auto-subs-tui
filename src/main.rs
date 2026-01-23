mod app;
mod audio;
mod cli;
mod subtitle;
mod ui;
mod utils;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Check for legacy --cli flag for backward compatibility
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "--cli" {
        eprintln!("⚠️  WARNING: The --cli flag is deprecated.");
        eprintln!("   Please use: auto-subs-tui process <video_path>");
        eprintln!("   Running in compatibility mode...\n");

        let video_path = if args.len() >= 3 {
            &args[2]
        } else {
            eprintln!("Usage: auto-subs-tui --cli <video_path>");
            std::process::exit(1);
        };
        return run_legacy_cli(video_path).await;
    }

    // Use new CLI structure
    cli::run().await
}

/// Run headless CLI mode for the transcription pipeline (legacy support)
async fn run_legacy_cli(video_path: &str) -> Result<()> {
    use app::ProgressMessage;
    use audio::extractor::AudioExtractor;
    use std::path::Path;
    use std::sync::mpsc;
    use subtitle::burner::SubtitleBurner;
    use subtitle::generator::SubtitleGenerator;

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║            AUTO-SUBS TUI - CLI MODE                        ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let video_path = Path::new(video_path);
    if !video_path.exists() {
        println!("❌ Error: Video file not found: {}", video_path.display());
        return Ok(());
    }
    println!("📹 Input video: {}", video_path.display());

    let audio_path = video_path.with_extension("wav");
    let srt_path = video_path.with_extension("srt");
    let output_path = video_path.with_file_name(format!(
        "{}_subtitled.{}",
        video_path.file_stem().unwrap().to_string_lossy(),
        video_path.extension().unwrap().to_string_lossy()
    ));

    // Step 1: Extract audio
    println!("\n[1/3] Extracting audio...");
    let (tx, rx) = mpsc::channel();
    let extractor = AudioExtractor::new();

    let video_clone = video_path.to_path_buf();
    let audio_clone = audio_path.clone();
    std::thread::spawn(move || {
        let _ = extractor.extract(&video_clone, &audio_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("      [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                println!("      ❌ Error: {}", e);
                return Ok(());
            }
        }
    }
    println!("      ✅ Audio extracted: {}", audio_path.display());

    // Step 2: Generate subtitles
    println!("\n[2/3] Generating subtitles with Whisper...");
    println!("      (This may download the model on first run, ~150MB)");
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
                println!("      ❌ Error: {}", e);
                return Ok(());
            }
        }
    }
    println!("      ✅ Subtitles generated: {}", srt_path.display());

    // Show some generated subtitles
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
    let (tx, rx) = mpsc::channel();
    let burner = SubtitleBurner::new();

    let video_clone = video_path.to_path_buf();
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
                println!("      ❌ Error: {}", e);
                return Ok(());
            }
        }
    }
    println!("      ✅ Output video: {}", output_path.display());

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    TEST COMPLETE!                          ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\nGenerated files:");
    println!("  📁 {}", audio_path.display());
    println!("  📄 {}", srt_path.display());
    println!("  🎬 {}", output_path.display());

    Ok(())
}
