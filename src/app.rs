use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use std::path::PathBuf;
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
        }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
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
            AppState::ExtractingAudio | AppState::GeneratingSubtitles | AppState::BurningSubtitles => {
                self.handle_progress_keys(key)
            }
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
                KeyCode::Char('q') => self.should_quit = true,
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
            AppState::GeneratingSubtitles => ui::progress::draw(frame, self, "Generating Subtitles"),
            AppState::Editing => ui::editor::draw(frame, self),
            AppState::BurningSubtitles => ui::progress::draw(frame, self, "Burning Subtitles"),
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

        std::thread::spawn(move || {
            let burner = SubtitleBurner::new();
            if let Err(e) = burner.burn(&video_path, &srt_path, &output_path, tx.clone()) {
                let _ = tx.send(ProgressMessage::Error(e.to_string()));
            }
        });
    }
}
