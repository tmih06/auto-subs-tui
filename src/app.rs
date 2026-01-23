use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use std::io::Write;
use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::audio::extractor::AudioExtractor;
use crate::subtitle::burner::SubtitleBurner;
use crate::subtitle::generator::SubtitleGenerator;
use crate::subtitle::srt::Subtitle;
use crate::ui;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Home,
    SelectingFile,
    ExtractingAudio,
    GeneratingSubtitles,
    Editing,
    BurningSubtitles,
    ExtractingOverlay,
    PreviewingOverlay,
    Done,
}

#[derive(Debug, Clone)]
pub enum ProgressMessage {
    Progress(f32, String),
    Complete,
    Error(String),
}

pub struct App {
    pub state: AppState,
    pub should_quit: bool,
    pub video_path: Option<PathBuf>,
    pub audio_path: Option<PathBuf>,
    pub srt_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub subtitles: Vec<Subtitle>,
    pub selected_index: usize,
    pub editing_subtitle: bool,
    pub edit_buffer: String,
    pub progress: f32,
    pub progress_message: String,
    pub file_browser: FileBrowser,
    pub error_message: Option<String>,
    progress_rx: Option<Receiver<ProgressMessage>>,
    // Overlay settings for burning
    pub overlay_height: u32,
    pub overlay_width: Option<u32>,
    pub overlay_x_offset: i32,
    pub overlay_y_offset: i32,
    // Preview state
    pub preview_active: bool,
    preview_process: Option<Child>,
    preview_socket_path: Option<PathBuf>,
    preview_video_width: u32,
    preview_video_height: u32,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub show_hidden: bool,
}

impl FileBrowser {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut browser = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected: 0,
            show_hidden: false,
        };
        browser.refresh();
        browser
    }

    pub fn refresh(&mut self) {
        self.entries.clear();

        // Add parent directory option
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(parent.to_path_buf());
        }

        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<PathBuf> = Vec::new();
            let mut files: Vec<PathBuf> = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();

                if !self.show_hidden && name.starts_with('.') {
                    continue;
                }

                if path.is_dir() {
                    dirs.push(path);
                } else if is_video_file(&path) {
                    files.push(path);
                }
            }

            dirs.sort();
            files.sort();

            self.entries.extend(dirs);
            self.entries.extend(files);
        }

        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    pub fn enter(&mut self) -> Option<PathBuf> {
        if let Some(path) = self.entries.get(self.selected) {
            if path.is_dir() {
                self.current_dir = path.clone();
                self.selected = 0;
                self.refresh();
                None
            } else {
                Some(path.clone())
            }
        } else {
            None
        }
    }

    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn down(&mut self) {
        if self.selected < self.entries.len().saturating_sub(1) {
            self.selected += 1;
        }
    }
}

fn is_video_file(path: &PathBuf) -> bool {
    let extensions = ["mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v"];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::Home,
            should_quit: false,
            video_path: None,
            audio_path: None,
            srt_path: None,
            output_path: None,
            subtitles: Vec::new(),
            selected_index: 0,
            editing_subtitle: false,
            edit_buffer: String::new(),
            progress: 0.0,
            progress_message: String::new(),
            file_browser: FileBrowser::new(),
            error_message: None,
            progress_rx: None,
            overlay_height: 200,
            overlay_width: None,
            overlay_x_offset: 0,
            overlay_y_offset: 0,
            preview_active: false,
            preview_process: None,
            preview_socket_path: None,
            preview_video_width: 0,
            preview_video_height: 0,
        }
    }

    pub fn load_srt_file(&mut self, path: &PathBuf) -> Result<()> {
        use crate::subtitle::srt;

        self.subtitles = srt::parse_srt(path)?;
        self.srt_path = Some(path.clone());
        self.state = AppState::Editing;
        self.selected_index = 0;

        Ok(())
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        loop {
            // Check for progress updates
            self.check_progress();

            // Draw UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle events with timeout for async operations
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code).await?;
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn check_progress(&mut self) {
        // Check if preview process has died
        if self.preview_active {
            if let Some(child) = &mut self.preview_process {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Preview closed
                        self.preview_active = false;
                        self.preview_process = None;
                        if status.success() {
                            self.progress_message = "Preview closed".to_string();
                        } else {
                            self.error_message = Some(format!(
                                "Preview exited with error (code: {:?}). Check if MPV is installed.",
                                status.code()
                            ));
                        }
                    }
                    Ok(None) => {
                        // Still running
                    }
                    Err(_) => {
                        // Error checking status, assume dead
                        self.preview_active = false;
                        self.preview_process = None;
                        self.error_message = Some("Preview process error".to_string());
                    }
                }
            }
        }

        // Take the receiver out to avoid borrow issues
        let rx = match self.progress_rx.take() {
            Some(rx) => rx,
            None => return,
        };

        // Collect all pending messages
        let mut messages = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            messages.push(msg);
        }

        // Determine if we should keep the receiver
        let mut should_drop_rx = false;
        let mut should_start_generation = false;

        for msg in messages {
            match msg {
                ProgressMessage::Progress(progress, message) => {
                    self.progress = progress;
                    self.progress_message = message;
                }
                ProgressMessage::Complete => {
                    match self.state {
                        AppState::ExtractingAudio => {
                            self.state = AppState::GeneratingSubtitles;
                            self.progress = 0.0;
                            should_start_generation = true;
                        }
                        AppState::GeneratingSubtitles => {
                            // Load the generated subtitles from file
                            if let Some(srt_path) = &self.srt_path {
                                if let Ok(subs) = crate::subtitle::srt::parse_srt(srt_path) {
                                    self.subtitles = subs;
                                }
                            }
                            self.state = AppState::Editing;
                            should_drop_rx = true;
                        }
                        AppState::BurningSubtitles => {
                            self.state = AppState::Done;
                            should_drop_rx = true;
                        }
                        AppState::ExtractingOverlay | AppState::PreviewingOverlay => {
                            // Return to editor after overlay extraction or preview
                            self.state = AppState::Editing;
                            should_drop_rx = true;
                        }
                        _ => {}
                    }
                }
                ProgressMessage::Error(err) => {
                    self.error_message = Some(err);
                    should_drop_rx = true;
                }
            }
        }

        // Put the receiver back if we should keep it
        if !should_drop_rx {
            self.progress_rx = Some(rx);
        }

        // Start subtitle generation after we've released the borrow
        if should_start_generation {
            self.start_subtitle_generation();
        }
    }

    async fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        // Clear error on any key press
        if self.error_message.is_some() && key != KeyCode::Esc {
            self.error_message = None;
        }

        match &self.state {
            AppState::Home => self.handle_home_keys(key),
            AppState::SelectingFile => self.handle_file_browser_keys(key),
            AppState::ExtractingAudio
            | AppState::GeneratingSubtitles
            | AppState::BurningSubtitles
            | AppState::ExtractingOverlay
            | AppState::PreviewingOverlay => self.handle_progress_keys(key),
            AppState::Editing => self.handle_editor_keys(key),
            AppState::Done => self.handle_done_keys(key),
        }
        Ok(())
    }

    fn handle_home_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Enter | KeyCode::Char('s') => {
                self.state = AppState::SelectingFile;
            }
            KeyCode::Char('l') => {
                // Load existing SRT file
                self.state = AppState::SelectingFile;
            }
            _ => {}
        }
    }

    fn handle_file_browser_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = AppState::Home;
            }
            KeyCode::Up | KeyCode::Char('k') => self.file_browser.up(),
            KeyCode::Down | KeyCode::Char('j') => self.file_browser.down(),
            KeyCode::Enter => {
                if let Some(path) = self.file_browser.enter() {
                    self.video_path = Some(path.clone());
                    self.start_audio_extraction();
                }
            }
            KeyCode::Char('.') => {
                self.file_browser.show_hidden = !self.file_browser.show_hidden;
                self.file_browser.refresh();
            }
            _ => {}
        }
    }

    fn handle_progress_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                // TODO: Cancel current operation
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_editor_keys(&mut self, key: KeyCode) {
        if self.editing_subtitle {
            match key {
                KeyCode::Esc => {
                    self.editing_subtitle = false;
                    self.edit_buffer.clear();
                }
                KeyCode::Enter => {
                    if let Some(sub) = self.subtitles.get_mut(self.selected_index) {
                        sub.text = self.edit_buffer.clone();
                    }
                    self.editing_subtitle = false;
                    self.edit_buffer.clear();
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                _ => {}
            }
        } else {
            match key {
                KeyCode::Char('q') => {
                    // Stop preview before quitting
                    if self.preview_active {
                        self.stop_preview();
                    }
                    self.should_quit = true;
                }
                KeyCode::Esc => self.state = AppState::Home,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_index < self.subtitles.len().saturating_sub(1) {
                        self.selected_index += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    if let Some(sub) = self.subtitles.get(self.selected_index) {
                        self.edit_buffer = sub.text.clone();
                        self.editing_subtitle = true;
                    }
                }
                KeyCode::Char('a') => {
                    // Add new subtitle
                    let new_sub = if let Some(last) = self.subtitles.last() {
                        Subtitle {
                            index: self.subtitles.len() + 1,
                            start_time: last.end_time,
                            end_time: last.end_time + 2000, // 2 seconds
                            text: String::from("New subtitle"),
                        }
                    } else {
                        Subtitle {
                            index: 1,
                            start_time: 0,
                            end_time: 2000,
                            text: String::from("New subtitle"),
                        }
                    };
                    self.subtitles.push(new_sub);
                    self.selected_index = self.subtitles.len() - 1;
                }
                KeyCode::Char('d') => {
                    if !self.subtitles.is_empty() {
                        self.subtitles.remove(self.selected_index);
                        if self.selected_index >= self.subtitles.len() && self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                        // Re-index subtitles
                        for (i, sub) in self.subtitles.iter_mut().enumerate() {
                            sub.index = i + 1;
                        }
                    }
                }
                KeyCode::Char('[') => {
                    // Decrease start time by 100ms
                    if let Some(sub) = self.subtitles.get_mut(self.selected_index) {
                        sub.start_time = sub.start_time.saturating_sub(100);
                    }
                }
                KeyCode::Char(']') => {
                    // Increase start time by 100ms
                    if let Some(sub) = self.subtitles.get_mut(self.selected_index) {
                        sub.start_time += 100;
                        if sub.start_time >= sub.end_time {
                            sub.start_time = sub.end_time - 100;
                        }
                    }
                }
                KeyCode::Char('{') => {
                    // Decrease end time by 100ms
                    if let Some(sub) = self.subtitles.get_mut(self.selected_index) {
                        if sub.end_time > sub.start_time + 100 {
                            sub.end_time -= 100;
                        }
                    }
                }
                KeyCode::Char('}') => {
                    // Increase end time by 100ms
                    if let Some(sub) = self.subtitles.get_mut(self.selected_index) {
                        sub.end_time += 100;
                    }
                }
                KeyCode::Char('s') => {
                    // Save SRT file
                    self.save_subtitles();
                }
                KeyCode::Char('b') => {
                    // Burn subtitles
                    self.save_subtitles();
                    self.start_burning();
                }
                KeyCode::Char('o') => {
                    // Extract overlay only
                    self.save_subtitles();
                    self.start_overlay_extraction();
                }
                KeyCode::Char('p') => {
                    // Toggle preview overlay position
                    self.save_subtitles();
                    self.toggle_preview();
                }
                KeyCode::Char('h') => {
                    // Decrease overlay height
                    self.overlay_height = self.overlay_height.saturating_sub(10);
                    if self.preview_active {
                        self.progress_message =
                            format!("Updating preview... Height: {}px", self.overlay_height);
                        self.update_preview_overlay();
                    } else {
                        self.progress_message =
                            format!("Overlay height: {}px", self.overlay_height);
                    }
                }
                KeyCode::Char('H') => {
                    // Increase overlay height
                    self.overlay_height = self.overlay_height.saturating_add(10);
                    if self.preview_active {
                        self.progress_message =
                            format!("Updating preview... Height: {}px", self.overlay_height);
                        self.update_preview_overlay();
                    } else {
                        self.progress_message =
                            format!("Overlay height: {}px", self.overlay_height);
                    }
                }
                KeyCode::Char('w') => {
                    // Decrease overlay width
                    let current = self.overlay_width.unwrap_or(1920);
                    self.overlay_width = Some(current.saturating_sub(10));
                    self.progress_message =
                        format!("Overlay width: {}px", self.overlay_width.unwrap());
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                KeyCode::Char('W') => {
                    // Increase overlay width (or set to None for auto)
                    if let Some(current) = self.overlay_width {
                        self.overlay_width = Some(current.saturating_add(10));
                        self.progress_message = format!("Overlay width: {}px", current + 10);
                    } else {
                        self.overlay_width = Some(1920);
                        self.progress_message = "Overlay width: 1920px".to_string();
                    }
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                KeyCode::Char('x') => {
                    // Decrease X offset (move left)
                    self.overlay_x_offset = self.overlay_x_offset.saturating_sub(10);
                    self.progress_message =
                        format!("Overlay X offset: {}px", self.overlay_x_offset);
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                KeyCode::Char('X') => {
                    // Increase X offset (move right)
                    self.overlay_x_offset = self.overlay_x_offset.saturating_add(10);
                    self.progress_message =
                        format!("Overlay X offset: {}px", self.overlay_x_offset);
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                KeyCode::Char('y') => {
                    // Decrease Y offset (move up)
                    self.overlay_y_offset = self.overlay_y_offset.saturating_sub(10);
                    self.progress_message =
                        format!("Overlay Y offset: {}px", self.overlay_y_offset);
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                KeyCode::Char('Y') => {
                    // Increase Y offset (move down)
                    self.overlay_y_offset = self.overlay_y_offset.saturating_add(10);
                    self.progress_message =
                        format!("Overlay Y offset: {}px", self.overlay_y_offset);
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                KeyCode::Char('0') => {
                    // Reset overlay settings to defaults
                    self.overlay_height = 200;
                    self.overlay_width = None;
                    self.overlay_x_offset = 0;
                    self.overlay_y_offset = 0;
                    self.progress_message = "Overlay settings reset to defaults".to_string();
                    if self.preview_active {
                        self.update_preview_overlay();
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_done_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                self.should_quit = true;
            }
            KeyCode::Char('r') => {
                // Reset and start over
                *self = App::new();
            }
            _ => {}
        }
    }

    fn draw(&self, frame: &mut Frame) {
        match &self.state {
            AppState::Home => ui::home::draw(frame, self),
            AppState::SelectingFile => ui::file_picker::draw(frame, self),
            AppState::ExtractingAudio => ui::progress::draw(frame, self, "Extracting Audio"),
            AppState::GeneratingSubtitles => {
                ui::progress::draw(frame, self, "Generating Subtitles")
            }
            AppState::Editing => ui::editor::draw(frame, self),
            AppState::BurningSubtitles => ui::progress::draw(frame, self, "Burning Subtitles"),
            AppState::ExtractingOverlay => ui::progress::draw(frame, self, "Extracting Overlay"),
            AppState::PreviewingOverlay => ui::progress::draw(frame, self, "Preview"),
            AppState::Done => ui::done::draw(frame, self),
        }
    }

    fn start_audio_extraction(&mut self) {
        self.state = AppState::ExtractingAudio;
        self.progress = 0.0;
        self.progress_message = "Starting audio extraction...".to_string();

        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);

        let video_path = self.video_path.clone().unwrap();
        let audio_path = video_path.with_extension("wav");
        self.audio_path = Some(audio_path.clone());

        std::thread::spawn(move || {
            let extractor = AudioExtractor::new();
            if let Err(e) = extractor.extract(&video_path, &audio_path, tx.clone()) {
                let _ = tx.send(ProgressMessage::Error(e.to_string()));
            }
        });
    }

    fn start_subtitle_generation(&mut self) {
        self.progress = 0.0;
        self.progress_message = "Initializing Whisper model...".to_string();

        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);

        let audio_path = self.audio_path.clone().unwrap();
        let srt_path = audio_path.with_extension("srt");
        self.srt_path = Some(srt_path.clone());

        std::thread::spawn(move || {
            let generator = SubtitleGenerator::new();
            match generator.generate(&audio_path, &srt_path, tx.clone()) {
                Ok(_) => {
                    // Subtitles are saved to file, will be loaded when Complete is received
                }
                Err(e) => {
                    let _ = tx.send(ProgressMessage::Error(e.to_string()));
                }
            }
        });
    }

    fn save_subtitles(&mut self) {
        if let Some(srt_path) = &self.srt_path {
            if let Err(e) = crate::subtitle::srt::save_srt(srt_path, &self.subtitles) {
                self.error_message = Some(format!("Failed to save SRT: {}", e));
            } else {
                self.progress_message = format!("Saved to {}", srt_path.display());
            }
        }
    }

    fn start_burning(&mut self) {
        if self.subtitles.is_empty() {
            self.error_message = Some("No subtitles to burn".to_string());
            return;
        }

        self.state = AppState::BurningSubtitles;
        self.progress = 0.0;
        self.progress_message = "Starting subtitle burning...".to_string();

        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);

        let video_path = self.video_path.clone().unwrap();
        let srt_path = self.srt_path.clone().unwrap();
        let output_path = video_path.with_file_name(format!(
            "{}_subtitled.{}",
            video_path.file_stem().unwrap().to_string_lossy(),
            video_path.extension().unwrap().to_string_lossy()
        ));
        self.output_path = Some(output_path.clone());

        // Get overlay settings from app state
        let overlay_height = self.overlay_height;
        let overlay_width = self.overlay_width;
        let overlay_x_offset = self.overlay_x_offset;
        let overlay_y_offset = self.overlay_y_offset;

        std::thread::spawn(move || {
            let mut burner = SubtitleBurner::new().with_overlay_height(overlay_height);

            if let Some(width) = overlay_width {
                burner = burner.with_overlay_width(width);
            }
            burner = burner.with_overlay_x_offset(overlay_x_offset);
            burner = burner.with_overlay_y_offset(overlay_y_offset);

            if let Err(e) = burner.burn(&video_path, &srt_path, &output_path, tx.clone()) {
                let _ = tx.send(ProgressMessage::Error(e.to_string()));
            }
        });
    }

    fn start_overlay_extraction(&mut self) {
        if self.subtitles.is_empty() {
            self.error_message = Some("No subtitles to extract overlay from".to_string());
            return;
        }

        self.state = AppState::ExtractingOverlay;
        self.progress = 0.0;
        self.progress_message = "Extracting subtitle overlay...".to_string();

        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);

        let video_path = self.video_path.clone().unwrap();
        let srt_path = self.srt_path.clone().unwrap();
        let overlay_output = video_path.with_file_name(format!(
            "{}_overlay.mp4",
            video_path.file_stem().unwrap().to_string_lossy()
        ));
        self.output_path = Some(overlay_output.clone());

        // Get overlay settings from app state
        let overlay_height = self.overlay_height;
        let overlay_width = self.overlay_width;

        std::thread::spawn(move || {
            let mut burner = SubtitleBurner::new().with_overlay_height(overlay_height);

            if let Some(width) = overlay_width {
                burner = burner.with_overlay_width(width);
            }

            if let Err(e) =
                burner.extract_overlay(&video_path, &srt_path, &overlay_output, tx.clone())
            {
                let _ = tx.send(ProgressMessage::Error(e.to_string()));
            }
        });
    }

    fn toggle_preview(&mut self) {
        if self.preview_active {
            // Stop preview
            self.stop_preview();
        } else {
            // Start preview
            self.start_preview();
        }
    }

    fn stop_preview(&mut self) {
        if let Some(mut child) = self.preview_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // Clean up socket file - retry a few times if needed
        if let Some(socket_path) = self.preview_socket_path.take() {
            for _ in 0..5 {
                if std::fs::remove_file(&socket_path).is_ok() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        self.preview_active = false;
        self.progress_message = "Preview stopped".to_string();
    }

    fn restart_preview(&mut self) {
        if self.preview_active {
            self.stop_preview();
            self.start_preview();
        }
    }

    fn update_preview_overlay(&mut self) {
        if !self.preview_active {
            return;
        }

        // Simple approach: stop and restart preview with new settings
        self.stop_preview();

        // Wait for full cleanup
        std::thread::sleep(Duration::from_millis(200));

        self.start_preview();
    }

    fn start_preview(&mut self) {
        if self.subtitles.is_empty() {
            self.error_message = Some("No subtitles to preview".to_string());
            return;
        }

        let video_path = match &self.video_path {
            Some(p) => p.clone(),
            None => {
                self.error_message = Some("No video loaded".to_string());
                return;
            }
        };

        let srt_path = match &self.srt_path {
            Some(p) => p.clone(),
            None => {
                self.error_message = Some("No subtitles loaded".to_string());
                return;
            }
        };

        // Create socket path for IPC
        let socket_path = std::env::temp_dir().join("auto-subs-preview.sock");
        // Remove old socket if it exists
        let _ = std::fs::remove_file(&socket_path);

        // Get overlay settings
        let overlay_height = self.overlay_height;
        let overlay_width = self.overlay_width;
        let overlay_x_offset = self.overlay_x_offset;
        let overlay_y_offset = self.overlay_y_offset;

        // Create burner with current settings
        let mut burner = SubtitleBurner::new().with_overlay_height(overlay_height);
        if let Some(width) = overlay_width {
            burner = burner.with_overlay_width(width);
        }
        burner = burner.with_overlay_x_offset(overlay_x_offset);
        burner = burner.with_overlay_y_offset(overlay_y_offset);

        // Launch preview process with IPC
        match burner.launch_preview_process_with_ipc(&video_path, &srt_path, &socket_path) {
            Ok((child, video_width, video_height)) => {
                self.preview_process = Some(child);
                self.preview_socket_path = Some(socket_path);
                self.preview_video_width = video_width;
                self.preview_video_height = video_height;
                self.preview_active = true;
                self.progress_message =
                    "Live preview active - adjust with h/H/w/W/x/X/y/Y (Press p to stop)"
                        .to_string();
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to start preview: {}", e));
            }
        }
    }
}
