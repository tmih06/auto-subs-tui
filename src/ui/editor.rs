use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use super::style;
use crate::app::App;
use crate::subtitle::srt::Subtitle;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(10),   // Content
        Constraint::Length(8), // Help (increased from 7 for preview status)
    ])
    .split(area);

    // Title
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("┌─", style::border_style()),
            Span::styled(" SUBTITLE EDITOR ", style::title_style()),
            Span::styled(
                "─".repeat((area.width as usize).saturating_sub(22)),
                style::border_style(),
            ),
            Span::styled("┐", style::border_style()),
        ]),
        Line::from(vec![
            Span::styled("│ ", style::border_style()),
            Span::styled(
                format!("{} subtitles", app.subtitles.len()),
                style::normal_style(),
            ),
            Span::styled(" │ ", style::muted_style()),
            Span::styled(
                app.video_path
                    .as_ref()
                    .map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    })
                    .unwrap_or_default(),
                style::muted_style(),
            ),
        ]),
    ]);
    frame.render_widget(title, chunks[0]);

    // Content area - split into list and edit panel
    let content_chunks = Layout::horizontal([
        Constraint::Percentage(60), // Subtitle list
        Constraint::Percentage(40), // Edit panel
    ])
    .split(chunks[1]);

    // Subtitle list
    draw_subtitle_list(frame, app, content_chunks[0]);

    // Edit panel
    draw_edit_panel(frame, app, content_chunks[1]);

    // Help bar
    draw_help(frame, app, chunks[2]);
}

fn draw_subtitle_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .subtitles
        .iter()
        .enumerate()
        .map(|(i, sub)| {
            let is_selected = i == app.selected_index;
            let time_str = format!(
                "{} → {}",
                Subtitle::format_time(sub.start_time),
                Subtitle::format_time(sub.end_time)
            );

            // Truncate text if too long
            let max_text_len = (area.width as usize).saturating_sub(35);
            let text_preview: String = sub.text.chars().take(max_text_len).collect();
            let text_preview = if sub.text.len() > max_text_len {
                format!("{}...", text_preview)
            } else {
                text_preview
            };

            let content = format!(
                " {:3} │ {} │ {}",
                sub.index,
                time_str,
                text_preview.replace('\n', " ")
            );

            let style = if is_selected {
                style::highlight_style()
            } else {
                style::normal_style()
            };

            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Subtitles ")
                .title_style(style::title_style())
                .borders(Borders::ALL)
                .border_style(style::border_style()),
        )
        .highlight_style(style::highlight_style());

    // Create a stateful list to enable scrolling
    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);

    // Scrollbar
    if !app.subtitles.is_empty() {
        let mut scrollbar_state =
            ScrollbarState::new(app.subtitles.len()).position(app.selected_index);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                horizontal: 0,
                vertical: 1,
            }),
            &mut scrollbar_state,
        );
    }
}

fn draw_edit_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(if app.editing_subtitle {
            " Editing "
        } else {
            " Preview "
        })
        .title_style(if app.editing_subtitle {
            style::success_style().add_modifier(Modifier::BOLD)
        } else {
            style::title_style()
        })
        .borders(Borders::ALL)
        .border_style(if app.editing_subtitle {
            style::success_style()
        } else {
            style::border_style()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(sub) = app.subtitles.get(app.selected_index) {
        let content = if app.editing_subtitle {
            vec![
                Line::from(vec![Span::styled("Text:", style::key_style())]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(&app.edit_buffer, style::normal_style()),
                    Span::styled("█", style::key_style()), // Cursor
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", style::muted_style()),
                    Span::styled("Enter", style::key_style()),
                    Span::styled(" to save, ", style::muted_style()),
                    Span::styled("Esc", style::key_style()),
                    Span::styled(" to cancel", style::muted_style()),
                ]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("Index: ", style::muted_style()),
                    Span::styled(sub.index.to_string(), style::normal_style()),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Start: ", style::muted_style()),
                    Span::styled(Subtitle::format_time(sub.start_time), style::key_style()),
                ]),
                Line::from(vec![
                    Span::styled("End:   ", style::muted_style()),
                    Span::styled(Subtitle::format_time(sub.end_time), style::key_style()),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled("Text:", style::muted_style())]),
                Line::from(vec![Span::styled(&sub.text, style::normal_style())]),
            ]
        };

        let paragraph = Paragraph::new(content);
        frame.render_widget(paragraph, inner);
    } else {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No subtitles yet.",
                style::muted_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", style::muted_style()),
                Span::styled("a", style::key_style()),
                Span::styled(" to add one.", style::muted_style()),
            ]),
        ]);
        frame.render_widget(empty, inner);
    }
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    // Pre-format strings to avoid borrow issues
    let overlay_height_str = format!("H:{}px", app.overlay_height);
    let overlay_width_str = format!(
        "W:{}",
        app.overlay_width
            .map(|w| format!("{}px", w))
            .unwrap_or_else(|| "auto".to_string())
    );
    let overlay_x_str = format!("X:{}px", app.overlay_x_offset);
    let overlay_y_str = format!("Y:{}px", app.overlay_y_offset);

    let help_text = if app.editing_subtitle {
        vec![
            Line::from(vec![Span::styled(
                "─".repeat(area.width as usize),
                style::muted_style(),
            )]),
            Line::from(vec![
                Span::styled("  Type to edit  │  ", style::muted_style()),
                Span::styled("Enter ", style::key_style()),
                Span::styled("save  │  ", style::muted_style()),
                Span::styled("Esc ", style::key_style()),
                Span::styled("cancel", style::muted_style()),
            ]),
        ]
    } else {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                "─".repeat(area.width as usize),
                style::muted_style(),
            )]),
            Line::from(vec![
                Span::styled("  ↑/k ↓/j ", style::key_style()),
                Span::styled("navigate  ", style::muted_style()),
                Span::styled("e/Enter ", style::key_style()),
                Span::styled("edit  ", style::muted_style()),
                Span::styled("a ", style::key_style()),
                Span::styled("add  ", style::muted_style()),
                Span::styled("d ", style::key_style()),
                Span::styled("delete  ", style::muted_style()),
            ]),
            Line::from(vec![
                Span::styled("  [ ] ", style::key_style()),
                Span::styled("start time  ", style::muted_style()),
                Span::styled("{ } ", style::key_style()),
                Span::styled("end time  ", style::muted_style()),
                Span::styled("s ", style::key_style()),
                Span::styled("save  ", style::muted_style()),
                Span::styled("b ", style::key_style()),
                Span::styled("burn  ", style::muted_style()),
                Span::styled("o ", style::key_style()),
                Span::styled("overlay  ", style::muted_style()),
                Span::styled("p ", style::key_style()),
                Span::styled(
                    if app.preview_active {
                        "stop preview  "
                    } else {
                        "preview  "
                    },
                    if app.preview_active {
                        style::success_style()
                    } else {
                        style::muted_style()
                    },
                ),
                Span::styled("q ", style::key_style()),
                Span::styled("quit", style::muted_style()),
            ]),
            Line::from(vec![
                Span::styled("  Overlay: ", style::muted_style()),
                Span::styled("h/H ", style::key_style()),
                Span::styled("height  ", style::muted_style()),
                Span::styled("w/W ", style::key_style()),
                Span::styled("width  ", style::muted_style()),
                Span::styled("x/X ", style::key_style()),
                Span::styled("X pos  ", style::muted_style()),
                Span::styled("y/Y ", style::key_style()),
                Span::styled("Y pos  ", style::muted_style()),
                Span::styled("0 ", style::key_style()),
                Span::styled("reset", style::muted_style()),
            ]),
            Line::from(vec![
                Span::styled("  Overlay: ", style::muted_style()),
                Span::styled(&overlay_height_str, style::normal_style()),
                Span::styled(" │ ", style::muted_style()),
                Span::styled(&overlay_width_str, style::normal_style()),
                Span::styled(" │ ", style::muted_style()),
                Span::styled(&overlay_x_str, style::normal_style()),
                Span::styled(" │ ", style::muted_style()),
                Span::styled(&overlay_y_str, style::normal_style()),
            ]),
        ];

        // Show preview status indicator
        if app.preview_active {
            lines.push(Line::from(vec![
                Span::styled("  ▶ ", style::success_style()),
                Span::styled(
                    "LIVE PREVIEW ACTIVE - Changes update in real-time",
                    style::success_style(),
                ),
            ]));
        }

        lines
    };

    // Show error message if present
    let mut lines = help_text;
    if let Some(error) = &app.error_message {
        lines.push(Line::from(vec![
            Span::styled("  ⚠ ", style::error_style()),
            Span::styled(error, style::error_style()),
        ]));
    } else if !app.progress_message.is_empty() && !app.editing_subtitle {
        lines.push(Line::from(vec![
            Span::styled("  ✓ ", style::success_style()),
            Span::styled(&app.progress_message, style::success_style()),
        ]));
    }

    let help = Paragraph::new(lines);
    frame.render_widget(help, area);
}
