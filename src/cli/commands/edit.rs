use crate::app::App;
use crate::cli::args::EditArgs;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

pub async fn execute(args: EditArgs) -> Result<()> {
    // Validate input file
    if !args.input.exists() {
        anyhow::bail!("SRT file not found: {}", args.input.display());
    }

    println!("📄 Opening SRT file: {}", args.input.display());
    println!("Launching TUI editor...\n");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with pre-loaded SRT file
    let mut app = App::new();
    app.load_srt_file(&args.input)?;

    // Run the application
    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    result
}
