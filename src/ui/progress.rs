use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::App;
use super::style;

pub fn draw(frame: &mut Frame, app: &App, title: &str) {
    let area = frame.area();

    // Center the progress display
    let center = centered_rect(60, 40, area);

    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Length(2), // Spacer
        Constraint::Length(3), // Progress bar
        Constraint::Length(2), // Spacer
        Constraint::Length(2), // Message
        Constraint::Min(5),    // Spacer
        Constraint::Length(2), // Help
    ])
    .split(center);

    // Title
    let title_widget = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("╔═══", style::border_style()),
            Span::styled(format!(" {} ", title), style::title_style()),
            Span::styled("═══╗", style::border_style()),
        ]),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(title_widget, chunks[0]);

    // Progress bar
    let progress_percent = (app.progress * 100.0) as u16;
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(style::border_style()))
        .gauge_style(style::progress_style())
        .percent(progress_percent)
        .label(format!("{}%", progress_percent));
    frame.render_widget(gauge, chunks[2]);

    // Message
    let message = Paragraph::new(vec![Line::from(vec![
        Span::styled(&app.progress_message, style::normal_style()),
    ])])
    .alignment(Alignment::Center);
    frame.render_widget(message, chunks[4]);

    // Spinner animation based on progress
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_idx = ((std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 100) as usize)
        % spinner_chars.len();
    let spinner = spinner_chars[spinner_idx];

    // If we have an error, show it
    if let Some(error) = &app.error_message {
        let error_widget = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("⚠ Error: ", style::error_style()),
                Span::styled(error, style::error_style()),
            ]),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(error_widget, chunks[5]);
    } else {
        let spinner_widget = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(spinner, style::key_style()),
                Span::styled(" Processing... ", style::muted_style()),
            ]),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(spinner_widget, chunks[5]);
    }

    // Help
    let help = Paragraph::new(vec![Line::from(vec![
        Span::styled("Press ", style::muted_style()),
        Span::styled("Esc", style::key_style()),
        Span::styled(" to cancel", style::muted_style()),
    ])])
    .alignment(Alignment::Center);
    frame.render_widget(help, chunks[6]);
}

/// Create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
