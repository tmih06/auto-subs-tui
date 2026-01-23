use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::style;
use crate::app::App;

pub fn draw(frame: &mut Frame, _app: &App) {
    let area = frame.area();

    // Create main layout
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(10),   // Content
        Constraint::Length(3), // Footer
    ])
    .split(area);

    // Title
    let title = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "╔═══════════════════════════════════════════════════╗",
            style::title_style(),
        )]),
        Line::from(vec![
            Span::styled("║          ", style::title_style()),
            Span::styled(
                "AUTO-SUBS TUI",
                Style::default()
                    .fg(style::ACCENT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  •  ", style::muted_style()),
            Span::styled("Subtitle Creator", style::subtitle_style()),
            Span::styled("          ║", style::title_style()),
        ]),
        Line::from(vec![Span::styled(
            "╚═══════════════════════════════════════════════════╝",
            style::title_style(),
        )]),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Main content
    let content_chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

    // Left panel - Workflow
    let workflow = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ┌─", style::muted_style()),
            Span::styled(" WORKFLOW ", style::title_style()),
            Span::styled("─────────────────┐", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    1. ", style::key_style()),
            Span::styled("Select Video File", style::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("       └─ ", style::muted_style()),
            Span::styled("MP4, MKV, AVI, MOV...", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    2. ", style::key_style()),
            Span::styled("Extract Audio", style::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("       └─ ", style::muted_style()),
            Span::styled("Auto-converts to 16kHz WAV", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    3. ", style::key_style()),
            Span::styled("Generate Subtitles", style::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("       └─ ", style::muted_style()),
            Span::styled("Using Whisper AI", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    4. ", style::key_style()),
            Span::styled("Review & Edit", style::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("       └─ ", style::muted_style()),
            Span::styled("Adjust timing and text", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    5. ", style::key_style()),
            Span::styled("Burn Subtitles", style::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("       └─ ", style::muted_style()),
            Span::styled("Hardcode into video", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  └──────────────────────────────┘",
            style::muted_style(),
        )]),
    ])
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(workflow, content_chunks[0]);

    // Right panel - Controls
    let controls = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ┌─", style::muted_style()),
            Span::styled(" CONTROLS ", style::title_style()),
            Span::styled("──────────────────┐", style::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    [S] ", style::key_style()),
            Span::styled("Start / Select Video", style::normal_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    [L] ", style::key_style()),
            Span::styled("Load Existing SRT", style::normal_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    [Q] ", style::key_style()),
            Span::styled("Quit", style::normal_style()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  └───────────────────────────────┘",
            style::muted_style(),
        )]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ⚡ ", style::key_style()),
            Span::styled("Powered by ", style::muted_style()),
            Span::styled("Whisper AI", style::success_style()),
            Span::styled(" + ", style::muted_style()),
            Span::styled("FFmpeg", style::success_style()),
        ]),
    ])
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(controls, content_chunks[1]);

    // Footer
    let footer = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "─".repeat(area.width as usize),
            style::muted_style(),
        )]),
        Line::from(vec![
            Span::styled("  Press ", style::muted_style()),
            Span::styled("Enter", style::key_style()),
            Span::styled(" or ", style::muted_style()),
            Span::styled("S", style::key_style()),
            Span::styled(" to start  •  ", style::muted_style()),
            Span::styled("Q", style::key_style()),
            Span::styled(" to quit", style::muted_style()),
        ]),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}
