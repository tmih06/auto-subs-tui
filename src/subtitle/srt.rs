use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// A single subtitle entry
#[derive(Debug, Clone)]
pub struct Subtitle {
    pub index: usize,
    pub start_time: u64, // milliseconds
    pub end_time: u64,   // milliseconds
    pub text: String,
}

impl Subtitle {
    pub fn new(index: usize, start_time: u64, end_time: u64, text: String) -> Self {
        Self {
            index,
            start_time,
            end_time,
            text,
        }
    }

    /// Format time in SRT format: HH:MM:SS,mmm
    pub fn format_time(ms: u64) -> String {
        let hours = ms / 3_600_000;
        let minutes = (ms % 3_600_000) / 60_000;
        let seconds = (ms % 60_000) / 1_000;
        let millis = ms % 1_000;
        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
    }

    /// Parse time from SRT format: HH:MM:SS,mmm
    pub fn parse_time(s: &str) -> Result<u64> {
        let parts: Vec<&str> = s.split(|c| c == ':' || c == ',').collect();
        if parts.len() != 4 {
            anyhow::bail!("Invalid time format: {}", s);
        }
        
        let hours: u64 = parts[0].parse().context("Invalid hours")?;
        let minutes: u64 = parts[1].parse().context("Invalid minutes")?;
        let seconds: u64 = parts[2].parse().context("Invalid seconds")?;
        let millis: u64 = parts[3].parse().context("Invalid milliseconds")?;
        
        Ok(hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis)
    }

    /// Convert to SRT format string
    pub fn to_srt(&self) -> String {
        format!(
            "{}\n{} --> {}\n{}\n",
            self.index,
            Self::format_time(self.start_time),
            Self::format_time(self.end_time),
            self.text
        )
    }
}

/// Parse an SRT file into a list of subtitles
pub fn parse_srt(path: &Path) -> Result<Vec<Subtitle>> {
    let content = fs::read_to_string(path).context("Failed to read SRT file")?;
    parse_srt_string(&content)
}

/// Parse SRT content from a string
pub fn parse_srt_string(content: &str) -> Result<Vec<Subtitle>> {
    let mut subtitles = Vec::new();
    let mut lines = content.lines().peekable();

    while lines.peek().is_some() {
        // Skip empty lines
        while lines.peek().map(|l| l.trim().is_empty()).unwrap_or(false) {
            lines.next();
        }

        // Parse index
        let index_line = match lines.next() {
            Some(l) if !l.trim().is_empty() => l,
            _ => break,
        };
        
        let index: usize = index_line.trim().parse().context("Invalid subtitle index")?;

        // Parse time range
        let time_line = lines.next().context("Expected time range")?;
        let parts: Vec<&str> = time_line.split(" --> ").collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid time range: {}", time_line);
        }
        
        let start_time = Subtitle::parse_time(parts[0].trim())?;
        let end_time = Subtitle::parse_time(parts[1].trim())?;

        // Parse text (can be multiple lines)
        let mut text_lines = Vec::new();
        while let Some(line) = lines.peek() {
            if line.trim().is_empty() {
                break;
            }
            text_lines.push(lines.next().unwrap());
        }
        let text = text_lines.join("\n");

        subtitles.push(Subtitle::new(index, start_time, end_time, text));
    }

    Ok(subtitles)
}

/// Save subtitles to an SRT file
pub fn save_srt(path: &Path, subtitles: &[Subtitle]) -> Result<()> {
    let content: String = subtitles
        .iter()
        .map(|s| s.to_srt())
        .collect::<Vec<_>>()
        .join("\n");
    
    fs::write(path, content).context("Failed to write SRT file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(Subtitle::format_time(0), "00:00:00,000");
        assert_eq!(Subtitle::format_time(1500), "00:00:01,500");
        assert_eq!(Subtitle::format_time(65000), "00:01:05,000");
        assert_eq!(Subtitle::format_time(3661500), "01:01:01,500");
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(Subtitle::parse_time("00:00:00,000").unwrap(), 0);
        assert_eq!(Subtitle::parse_time("00:00:01,500").unwrap(), 1500);
        assert_eq!(Subtitle::parse_time("00:01:05,000").unwrap(), 65000);
    }
}
