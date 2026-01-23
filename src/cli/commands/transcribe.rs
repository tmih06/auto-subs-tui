use crate::app::ProgressMessage;
use crate::cli::args::TranscribeArgs;
use crate::subtitle::generator::SubtitleGenerator;
use anyhow::Result;
use std::sync::mpsc;

pub async fn execute(args: TranscribeArgs) -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║       AUTO-SUBS TUI - TRANSCRIBE MODE                      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Validate input file
    if !args.input.exists() {
        anyhow::bail!("Audio file not found: {}", args.input.display());
    }

    // Determine output path
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| args.input.with_extension("srt"));

    println!("🎵 Input audio: {}", args.input.display());
    println!("📄 Output SRT: {}", output_path.display());
    println!("🎯 Model: {}", args.model.as_str());
    println!("🌍 Language: {}", args.language);
    println!("🔧 Provider: {}\n", args.provider);

    // Generate subtitles
    println!(
        "Generating subtitles with Whisper ({})...",
        args.model.as_str()
    );
    println!("(This may download the model on first run)");

    let (tx, rx) = mpsc::channel();
    let generator = SubtitleGenerator::new();

    let input_clone = args.input.clone();
    let output_clone = output_path.clone();
    std::thread::spawn(move || {
        let _ = generator.generate(&input_clone, &output_clone, tx);
    });

    while let Ok(msg) = rx.recv() {
        match msg {
            ProgressMessage::Progress(p, m) => println!("  [{:3.0}%] {}", p * 100.0, m),
            ProgressMessage::Complete => break,
            ProgressMessage::Error(e) => {
                anyhow::bail!("Subtitle generation failed: {}", e);
            }
        }
    }

    println!("\n✅ Subtitle generation complete!");
    println!("   Output: {}", output_path.display());

    // Show preview of generated subtitles
    if let Ok(content) = std::fs::read_to_string(&output_path) {
        let lines: Vec<&str> = content.lines().take(15).collect();
        println!("\n📋 Preview (first few entries):");
        for line in lines {
            println!("   │ {}", line);
        }
        if content.lines().count() > 15 {
            println!("   │ ...");
        }
    }

    Ok(())
}
