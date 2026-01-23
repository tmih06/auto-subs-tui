use ratatui::{
    layout::{Constraint, Layout},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::style;
use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Length(2), // Current path
        Constraint::Min(10),   // File list
        Constraint::Length(3), // Help
    ])
    .split(area);

    // Title
    let title = Paragraph::new(vec![Line::from(vec![
        Span::styled("┌─", style::border_style()),
        Span::styled(" SELECT VIDEO FILE ", style::title_style()),
        Span::styled(
            "─".repeat((area.width as usize).saturating_sub(24)),
            style::border_style(),
        ),
        Span::styled("┐", style::border_style()),
    ])]);
    frame.render_widget(title, chunks[0]);

    // Current directory
    let path_display = Paragraph::new(vec![Line::from(vec![
        Span::styled("  📁 ", style::key_style()),
        Span::styled(
            app.file_browser.current_dir.display().to_string(),
            style::normal_style(),
        ),
    ])]);
    frame.render_widget(path_display, chunks[1]);

    // File list
    let items: Vec<ListItem> = app
        .file_browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let is_selected = i == app.file_browser.selected;
            let is_dir = path.is_dir();
            let is_parent = path == &app.file_browser.current_dir.parent().unwrap_or(path);

            let name = if is_parent && i == 0 {
                "..".to_string()
            } else {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            };

            let icon = if is_dir { "📁" } else { "🎬" };
            let display = format!("  {} {}", icon, name);

            let style = if is_selected {
                style::highlight_style()
            } else if is_dir {
                style::normal_style().add_modifier(Modifier::BOLD)
            } else {
                style::normal_style()
            };

            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(style::border_style()),
    );
    frame.render_widget(list, chunks[2]);

    // Help
    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("└", style::border_style()),
            Span::styled(
                "─".repeat((area.width as usize).saturating_sub(2)),
                style::border_style(),
            ),
            Span::styled("┘", style::border_style()),
        ]),
        Line::from(vec![
            Span::styled("  ↑/k ", style::key_style()),
            Span::styled("up  ", style::muted_style()),
            Span::styled("↓/j ", style::key_style()),
            Span::styled("down  ", style::muted_style()),
            Span::styled("Enter ", style::key_style()),
            Span::styled("select  ", style::muted_style()),
            Span::styled(". ", style::key_style()),
            Span::styled("toggle hidden  ", style::muted_style()),
            Span::styled("Esc ", style::key_style()),
            Span::styled("back", style::muted_style()),
        ]),
    ]);
    frame.render_widget(help, chunks[3]);
}
