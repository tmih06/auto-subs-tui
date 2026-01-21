use anyhow::{Context, Result};
use std::path::Path;
use std::sync::mpsc::Sender;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::app::ProgressMessage;
use crate::subtitle::srt::Subtitle;

pub struct SubtitleGenerator {
    model_path: std::path::PathBuf,
}

impl SubtitleGenerator {
    pub fn new() -> Self {
        // Use cache directory for model storage
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("auto-subs-tui")
            .join("models");
        
        Self {
            model_path: cache_dir.join("ggml-base.en.bin"),
        }
    }

    /// Download the Whisper model if not present
    fn ensure_model(&self, progress_tx: &Sender<ProgressMessage>) -> Result<()> {
        if self.model_path.exists() {
            let _ = progress_tx.send(ProgressMessage::Progress(
                0.1,
                "Model found, loading...".to_string(),
            ));
            return Ok(());
        }

        // Create model directory
        if let Some(parent) = self.model_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create model directory")?;
        }

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.05,
            "Downloading Whisper model (~150MB)...".to_string(),
        ));

        // Download the base.en model from Hugging Face
        let model_url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
        
        let response = ureq::get(model_url)
            .call()
            .context("Failed to download model")?;
        
        let mut file = std::fs::File::create(&self.model_path)
            .context("Failed to create model file")?;
        
        std::io::copy(&mut response.into_reader(), &mut file)
            .context("Failed to save model")?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.1,
            "Model downloaded successfully!".to_string(),
        ));

        Ok(())
    }

    /// Generate subtitles from audio file and save to SRT
    pub fn generate(
        &self,
        audio_path: &Path,
        output_path: &Path,
        progress_tx: Sender<ProgressMessage>,
    ) -> Result<Vec<Subtitle>> {
        // Ensure model is available
        self.ensure_model(&progress_tx)?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.15,
            "Loading Whisper model...".to_string(),
        ));

        // Create Whisper context
        let ctx = WhisperContext::new_with_params(
            self.model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .context("Failed to load Whisper model")?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.2,
            "Loading audio file...".to_string(),
        ));

        // Read audio file
        let audio_data = self.read_audio(audio_path)?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.25,
            "Transcribing audio...".to_string(),
        ));

        // Configure transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_language(Some("en"));
        params.set_token_timestamps(true);

        // Create state and run transcription
        let mut state = ctx.create_state().context("Failed to create Whisper state")?;
        state.full(params, &audio_data).context("Transcription failed")?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            0.9,
            "Processing segments...".to_string(),
        ));

        // Extract segments and split into sentences
        let num_segments = state.full_n_segments().context("Failed to get segment count")?;
        let mut subtitles = Vec::new();

        for i in 0..num_segments {
            let start = state.full_get_segment_t0(i).context("Failed to get start time")? as u64 * 10; // Convert to ms
            let end = state.full_get_segment_t1(i).context("Failed to get end time")? as u64 * 10;
            let text = state.full_get_segment_text(i).context("Failed to get text")?;

            let text = text.trim().to_string();
            if !text.is_empty() {
                // Split text into sentences for more detailed subtitles
                let sentences = self.split_into_sentences(&text);
                
                if sentences.len() == 1 {
                    // Single sentence or short text - keep as is
                    subtitles.push(Subtitle::new(subtitles.len() + 1, start, end, text));
                } else {
                    // Multiple sentences - distribute time proportionally
                    let total_duration = end - start;
                    let total_chars: usize = sentences.iter().map(|s| s.len()).sum();
                    
                    let mut current_time = start;
                    for sentence in sentences {
                        if sentence.is_empty() {
                            continue;
                        }
                        
                        // Calculate duration based on sentence length
                        let sentence_duration = (total_duration as f64 * sentence.len() as f64 / total_chars as f64) as u64;
                        let sentence_end = (current_time + sentence_duration).min(end);
                        
                        subtitles.push(Subtitle::new(
                            subtitles.len() + 1,
                            current_time,
                            sentence_end,
                            sentence.to_string(),
                        ));
                        
                        current_time = sentence_end;
                    }
                }
            }
        }

        // Save to file
        crate::subtitle::srt::save_srt(output_path, &subtitles)?;

        let _ = progress_tx.send(ProgressMessage::Progress(
            1.0,
            format!("Generated {} subtitles!", subtitles.len()),
        ));
        let _ = progress_tx.send(ProgressMessage::Complete);

        Ok(subtitles)
    }

    /// Read and convert audio file to f32 samples
    fn read_audio(&self, path: &Path) -> Result<Vec<f32>> {
        let reader = hound::WavReader::open(path).context("Failed to open WAV file")?;
        let spec = reader.spec();

        // Whisper expects 16kHz mono audio
        if spec.sample_rate != 16000 {
            anyhow::bail!(
                "Audio must be 16kHz, got {}Hz. FFmpeg should have converted this.",
                spec.sample_rate
            );
        }

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max_value = (1 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .into_samples::<i32>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / max_value)
                    .collect()
            }
            hound::SampleFormat::Float => {
                reader
                    .into_samples::<f32>()
                    .filter_map(|s| s.ok())
                    .collect()
            }
        };

        Ok(samples)
    }

    /// Split text into sentences for more detailed subtitles
    fn split_into_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut sentences = Vec::new();
        let mut start = 0;
        let chars: Vec<char> = text.chars().collect();
        
        for (i, ch) in chars.iter().enumerate() {
            // Check for sentence endings: . ! ?
            if matches!(ch, '.' | '!' | '?') {
                // Look ahead to see if there's a space or end of string
                let is_sentence_end = if i + 1 < chars.len() {
                    // Next char should be space, quote, or another punctuation
                    matches!(chars[i + 1], ' ' | '"' | '\'' | ')' | ']')
                } else {
                    true // End of string
                };
                
                if is_sentence_end {
                    let end = text.char_indices().nth(i + 1).map(|(pos, _)| pos).unwrap_or(text.len());
                    let sentence = text[start..end].trim();
                    if !sentence.is_empty() {
                        sentences.push(sentence);
                    }
                    start = end;
                }
            }
        }
        
        // Add remaining text if any
        if start < text.len() {
            let sentence = text[start..].trim();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }
        
        // If no sentences were found, return the whole text
        if sentences.is_empty() {
            sentences.push(text.trim());
        }
        
        sentences
    }
}
