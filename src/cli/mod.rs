pub mod args;
pub mod commands;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;

/// Execute the CLI with parsed arguments
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on verbosity
    setup_logging(cli.verbose, cli.quiet);

    // Execute subcommand or launch TUI
    match cli.command {
        Some(Commands::Process(args)) => {
            commands::process::execute(args).await
        }
        Some(Commands::Extract(args)) => {
            commands::extract::execute(args).await
        }
        Some(Commands::Transcribe(args)) => {
            commands::transcribe::execute(args).await
        }
        Some(Commands::Burn(args)) => {
            commands::burn::execute(args).await
        }
        Some(Commands::Edit(args)) => {
            commands::edit::execute(args).await
        }
        Some(Commands::Config(args)) => {
            commands::config::execute(args).await
        }
        None => {
            // No subcommand provided - launch TUI mode
            launch_tui().await
        }
    }
}

/// Launch the TUI interface
async fn launch_tui() -> Result<()> {
    use crate::app::App;
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::prelude::*;
    use std::io;
    use std::panic;

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
    use crossterm::{
        event::DisableMouseCapture,
        execute,
        terminal::{disable_raw_mode, LeaveAlternateScreen},
    };

    disable_raw_mode()?;
    execute!(
        std::io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn setup_logging(verbose: u8, quiet: bool) {
    use tracing_subscriber::{EnvFilter, fmt};

    if quiet {
        // Suppress all output except errors
        return;
    }

    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
