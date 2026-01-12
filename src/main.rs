mod app;
mod audio;
mod subtitle;
mod ui;
mod utils;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::panic;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // Check for --cli flag with optional video path
    if args.len() >= 2 && args[1] == "--cli" {
        let video_path = if args.len() >= 3 {
            &args[2]
        } else {
            eprintln!("Usage: auto-subs-tui --cli <video_path>");
            std::process::exit(1);
        };
        return run_cli(video_path).await;
    }

    // Normal TUI mode
    // Setup panic hook to restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the application
    let mut app = App::new();
    let result = app.run(&mut terminal).await;

    // Restore terminal
    restore_terminal()?;

    result
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

/// Run headless CLI mode for the transcription pipeline
async fn run_cli(video_path: &str) -> Result<()> {
    use std::sync::mpsc;
    use audio::extractor::AudioExtractor;
    use subtitle::generator::SubtitleGenerator;
    use subtitle::burner::SubtitleBurner;
    use app::ProgressMessage;

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
