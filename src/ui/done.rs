use ratatui::{
    layout::{Alignment, Constraint, Layout},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::style;
use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Percentage(30),
        Constraint::Length(10),
        Constraint::Percentage(30),
        Constraint::Length(3),
    ])
    .split(area);

    let output_path = app
        .output_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "╔════════════════════════════════════════════════════════════╗",
            style::success_style(),
        )]),
        Line::from(vec![Span::styled(
            "║                                                            ║",
            style::success_style(),
        )]),
        Line::from(vec![
            Span::styled("║   ", style::success_style()),
            Span::styled("✓  SUBTITLES BURNED SUCCESSFULLY!  ", style::title_style()),
            Span::styled("                  ║", style::success_style()),
        ]),
        Line::from(vec![Span::styled(
            "║                                                            ║",
            style::success_style(),
        )]),
        Line::from(vec![Span::styled(
            "╚════════════════════════════════════════════════════════════╝",
            style::success_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Output: ", style::muted_style()),
            Span::styled(&output_path, style::normal_style()),
        ]),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(content, chunks[1]);

    let help = Paragraph::new(vec![Line::from(vec![
        Span::styled("Press ", style::muted_style()),
        Span::styled("Enter", style::key_style()),
        Span::styled(" or ", style::muted_style()),
        Span::styled("Q", style::key_style()),
        Span::styled(" to exit  •  ", style::muted_style()),
        Span::styled("R", style::key_style()),
        Span::styled(" to start over", style::muted_style()),
    ])])
    .alignment(Alignment::Center);
    frame.render_widget(help, chunks[3]);
}
