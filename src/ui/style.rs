use ratatui::style::{Color, Modifier, Style};

// Color palette - cyberpunk/modern theme
pub const BG_PRIMARY: Color = Color::Rgb(15, 15, 25);
pub const BG_SECONDARY: Color = Color::Rgb(25, 25, 40);
pub const ACCENT_PRIMARY: Color = Color::Rgb(0, 200, 255); // Cyan
pub const ACCENT_SECONDARY: Color = Color::Rgb(255, 100, 150); // Pink
pub const TEXT_PRIMARY: Color = Color::Rgb(230, 230, 240);
pub const TEXT_SECONDARY: Color = Color::Rgb(150, 150, 170);
pub const TEXT_MUTED: Color = Color::Rgb(100, 100, 120);
pub const SUCCESS: Color = Color::Rgb(100, 255, 150);
pub const WARNING: Color = Color::Rgb(255, 200, 100);
pub const ERROR: Color = Color::Rgb(255, 100, 100);

// Styles
pub fn title_style() -> Style {
    Style::default()
        .fg(ACCENT_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn subtitle_style() -> Style {
    Style::default().fg(TEXT_SECONDARY)
}

pub fn highlight_style() -> Style {
    Style::default()
        .fg(BG_PRIMARY)
        .bg(ACCENT_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn normal_style() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

pub fn muted_style() -> Style {
    Style::default().fg(TEXT_MUTED)
}

pub fn key_style() -> Style {
    Style::default()
        .fg(ACCENT_SECONDARY)
        .add_modifier(Modifier::BOLD)
}

pub fn success_style() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn error_style() -> Style {
    Style::default().fg(ERROR)
}

pub fn border_style() -> Style {
    Style::default().fg(ACCENT_PRIMARY)
}

pub fn progress_style() -> Style {
    Style::default().fg(ACCENT_PRIMARY).bg(BG_SECONDARY)
}
