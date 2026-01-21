use anyhow::Result;
use std::sync::mpsc;
use crate::audio::extractor::AudioExtractor;
use crate::app::ProgressMessage;
use crate::cli::args::ExtractArgs;

pub async fn execute(args: ExtractArgs) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         AUTO-SUBS TUI - EXTRACT MODE                       ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Validate input file
    if !args.input.exists() {
        anyhow::bail!("Video file not found: {}", args.input.display());
    }

    // Determine output path
    let output_path = args.output.clone().unwrap_or_else(|| {
        args.input.with_extension(args.format.as_str())
    });

    println!("📹 Input video: {}", args.input.display());
    println!("🎵 Output audio: {}", output_path.display());
    println!("⚙️  Sample rate: {}Hz", args.sample_rate);
    println!("⚙️  Channels: {}", args.channels);
    println!("⚙️  Format: {}\n", args.format.as_str());

    // Extract audio
    println!("Extracting audio...");
    let (tx, rx) = mpsc::channel();
    let extractor = AudioExtractor::new();
    
    let input_clone = args.input.clone();
    let output_clone = output_path.clone();
    std::thread::spawn(move || {
        let _ = extractor.extract(&input_clone, &output_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("  [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                anyhow::bail!("Audio extraction failed: {}", e);
            }
        }
    }

    println!("\n✅ Audio extraction complete!");
    println!("   Output: {}", output_path.display());

    Ok(())
}
