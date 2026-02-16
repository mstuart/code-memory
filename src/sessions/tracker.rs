use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::patterns::PatternLibrary;

/// A parsed entry from a Claude Code transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: String,
    pub entry_type: EntryType,
    pub content: String,
    pub tool_name: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntryType {
    ToolUse,
    FileEdit,
    FileCreate,
    FileRead,
    BashCommand,
    Decision,
    ErrorFix,
    UserMessage,
    AssistantMessage,
}

/// An event extracted from session analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub event_type: SessionEventType,
    pub description: String,
    pub files: Vec<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionEventType {
    ArchitecturalDecision,
    CodingPattern,
    ErrorAndFix,
    Refactoring,
    TestWritten,
    ConfigChange,
}

/// Tracks Claude Code sessions and extracts knowledge.
pub struct SessionTracker {
    history_dir: PathBuf,
    pattern_library: PatternLibrary,
}

impl SessionTracker {
    pub fn new(history_dir: PathBuf) -> Self {
        Self {
            history_dir,
            pattern_library: PatternLibrary::new(),
        }
    }

    /// Default history directory for Claude Code.
    pub fn with_default_dir() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::new(home.join(".claude").join("history"))
    }

    /// List available session transcript files.
    pub fn list_sessions(&self) -> Vec<PathBuf> {
        let mut sessions = Vec::new();

        // Check for main history.jsonl file (Claude Code stores history here)
        let history_file = self.history_dir.join("history.jsonl");
        if history_file.exists() {
            sessions.push(history_file);
        }

        // Also check if history_dir itself is a jsonl file
        if self.history_dir.is_file() {
            sessions.push(self.history_dir.clone());
            return sessions;
        }

        // Scan for any .json/.jsonl files in the directory
        if let Ok(entries) = fs::read_dir(&self.history_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json")
                    || path.extension().and_then(|e| e.to_str()) == Some("jsonl")
                {
                    if !sessions.contains(&path) {
                        sessions.push(path);
                    }
                }
            }
        }
        sessions.sort();
        sessions
    }

    /// Parse a single transcript file.
    pub fn parse_transcript(&self, path: &Path) -> Vec<TranscriptEntry> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut entries = Vec::new();

        // Try JSONL format
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(entry) = self.parse_json_entry(&value) {
                    entries.push(entry);
                }
            }
        }

        // If JSONL didn't work, try as a single JSON array
        if entries.is_empty() {
            if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                for value in &values {
                    if let Some(entry) = self.parse_json_entry(value) {
                        entries.push(entry);
                    }
                }
            }
        }

        entries
    }

    fn parse_json_entry(&self, value: &serde_json::Value) -> Option<TranscriptEntry> {
        // Support both structured transcripts ("type" field) and Claude Code history ("display" field)
        let entry_type_str = value.get("type").and_then(|v| v.as_str());

        let content = value
            .get("content")
            .or_else(|| value.get("message"))
            .or_else(|| value.get("display"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Handle timestamp as string or number
        let timestamp = match value.get("timestamp") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };

        // If no "type" field, treat as user message (Claude Code history format)
        let entry_type_str = match entry_type_str {
            Some(s) => s.to_string(),
            None if value.get("display").is_some() => "user".to_string(),
            None => return None,
        };
        let entry_type_str = entry_type_str.as_str();

        let tool_name = value
            .get("tool")
            .or_else(|| value.get("tool_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let file_path = value
            .get("file_path")
            .or_else(|| value.get("path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let entry_type = match entry_type_str {
            "tool_use" | "tool_call" => {
                match tool_name.as_deref() {
                    Some("Edit") | Some("Write") => EntryType::FileEdit,
                    Some("Read") => EntryType::FileRead,
                    Some("Bash") => EntryType::BashCommand,
                    _ => EntryType::ToolUse,
                }
            }
            "user" | "human" => EntryType::UserMessage,
            "assistant" => EntryType::AssistantMessage,
            _ => EntryType::ToolUse,
        };

        Some(TranscriptEntry {
            timestamp,
            entry_type,
            content,
            tool_name,
            file_path,
        })
    }

    /// Extract events from transcript entries.
    pub fn extract_events(&self, entries: &[TranscriptEntry]) -> Vec<SessionEvent> {
        let mut events = Vec::new();

        for (i, entry) in entries.iter().enumerate() {
            if entry.entry_type == EntryType::AssistantMessage {
                if let Some(event) = self.detect_decision(&entry.content, &entry.timestamp) {
                    events.push(event);
                }
            }

            if entry.entry_type == EntryType::BashCommand && self.looks_like_error(&entry.content) {
                let end = (i + 5).min(entries.len());
                if let Some(fix_entry) = entries.get(i + 1..end).and_then(|slice| {
                    slice.iter().find(|e| e.entry_type == EntryType::FileEdit)
                }) {
                    events.push(SessionEvent {
                        event_type: SessionEventType::ErrorAndFix,
                        description: format!(
                            "Error in command, fixed by editing {}",
                            fix_entry.file_path.as_deref().unwrap_or("unknown")
                        ),
                        files: fix_entry.file_path.iter().cloned().collect(),
                        timestamp: entry.timestamp.clone(),
                    });
                }
            }

            if entry.entry_type == EntryType::FileEdit || entry.entry_type == EntryType::FileCreate {
                if let Some(path) = &entry.file_path {
                    if path.contains("test") || path.contains("spec") {
                        events.push(SessionEvent {
                            event_type: SessionEventType::TestWritten,
                            description: format!("Test written: {}", path),
                            files: vec![path.clone()],
                            timestamp: entry.timestamp.clone(),
                        });
                    }
                }
            }

            if entry.entry_type == EntryType::AssistantMessage
                && self.looks_like_refactoring(&entry.content)
            {
                let files: Vec<String> = entries[i..]
                    .iter()
                    .take(10)
                    .filter_map(|e| e.file_path.clone())
                    .collect();

                events.push(SessionEvent {
                    event_type: SessionEventType::Refactoring,
                    description: self.summarize_refactoring(&entry.content),
                    files,
                    timestamp: entry.timestamp.clone(),
                });
            }
        }

        events
    }

    fn detect_decision(&self, text: &str, timestamp: &str) -> Option<SessionEvent> {
        let decision_indicators = [
            "I'll use", "let's use", "we should", "I recommend",
            "the best approach", "decided to", "choosing",
            "instead of", "rather than", "better to",
        ];

        let lower = text.to_lowercase();
        for indicator in &decision_indicators {
            if lower.contains(&indicator.to_lowercase()) {
                let description = text
                    .lines()
                    .find(|line| line.to_lowercase().contains(&indicator.to_lowercase()))
                    .unwrap_or(text)
                    .trim()
                    .to_string();

                if description.len() > 10 {
                    return Some(SessionEvent {
                        event_type: SessionEventType::ArchitecturalDecision,
                        description: if description.len() > 200 {
                            format!("{}...", &description[..200])
                        } else {
                            description
                        },
                        files: Vec::new(),
                        timestamp: timestamp.to_string(),
                    });
                }
            }
        }
        None
    }

    fn looks_like_error(&self, content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("error") || lower.contains("failed") || lower.contains("panic")
            || lower.contains("exception") || lower.contains("not found")
    }

    fn looks_like_refactoring(&self, content: &str) -> bool {
        let lower = content.to_lowercase();
        lower.contains("refactor") || lower.contains("restructure") || lower.contains("reorganize")
            || lower.contains("extract") || lower.contains("move to")
    }

    fn summarize_refactoring(&self, content: &str) -> String {
        content
            .lines()
            .find(|line| {
                let l = line.to_lowercase();
                l.contains("refactor") || l.contains("restructure") || l.contains("extract")
            })
            .unwrap_or("Refactoring detected")
            .trim()
            .to_string()
    }

    /// Analyze all sessions and build the pattern library.
    pub fn analyze_all_sessions(&mut self) -> Vec<SessionEvent> {
        let sessions = self.list_sessions();
        let mut all_events = Vec::new();

        for session_path in &sessions {
            let entries = self.parse_transcript(session_path);
            let events = self.extract_events(&entries);
            self.learn_from_entries(&entries);
            all_events.extend(events);
        }

        all_events
    }

    fn learn_from_entries(&mut self, entries: &[TranscriptEntry]) {
        for entry in entries {
            if let Some(path) = &entry.file_path {
                if entry.entry_type == EntryType::FileEdit {
                    let ext = Path::new(path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");

                    match ext {
                        "rs" => self.pattern_library.observe("naming:rust:snake_case"),
                        "ts" | "js" => self.pattern_library.observe("naming:ts:camelCase"),
                        "py" => self.pattern_library.observe("naming:python:snake_case"),
                        _ => {}
                    }
                }
            }
        }
    }

    /// Get the current pattern library.
    pub fn patterns(&self) -> &PatternLibrary {
        &self.pattern_library
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_decision() {
        let tracker = SessionTracker::new(PathBuf::from("/tmp"));
        let event = tracker.detect_decision(
            "I'll use SQLite for local storage because it's embedded",
            "2024-01-01",
        );
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.event_type, SessionEventType::ArchitecturalDecision);
    }

    #[test]
    fn test_no_decision_in_simple_text() {
        let tracker = SessionTracker::new(PathBuf::from("/tmp"));
        let event = tracker.detect_decision("Done.", "2024-01-01");
        assert!(event.is_none());
    }

    #[test]
    fn test_extract_events_error_fix() {
        let tracker = SessionTracker::new(PathBuf::from("/tmp"));
        let entries = vec![
            TranscriptEntry {
                timestamp: "t1".to_string(),
                entry_type: EntryType::BashCommand,
                content: "error: cannot find module".to_string(),
                tool_name: Some("Bash".to_string()),
                file_path: None,
            },
            TranscriptEntry {
                timestamp: "t2".to_string(),
                entry_type: EntryType::FileEdit,
                content: "fix import".to_string(),
                tool_name: Some("Edit".to_string()),
                file_path: Some("src/main.rs".to_string()),
            },
        ];
        let events = tracker.extract_events(&entries);
        let error_fix = events.iter().find(|e| e.event_type == SessionEventType::ErrorAndFix);
        assert!(error_fix.is_some());
    }

    #[test]
    fn test_parse_jsonl_transcript() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("session.jsonl");
        fs::write(
            &path,
            "{\"type\":\"user\",\"content\":\"hello\",\"timestamp\":\"t1\"}\n{\"type\":\"assistant\",\"content\":\"I'll use tokio for async runtime\",\"timestamp\":\"t2\"}\n{\"type\":\"tool_use\",\"tool\":\"Edit\",\"content\":\"edit\",\"file_path\":\"src/main.rs\",\"timestamp\":\"t3\"}\n",
        )
        .unwrap();

        let tracker = SessionTracker::new(dir.path().to_path_buf());
        let entries = tracker.parse_transcript(&path);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].entry_type, EntryType::UserMessage);
        assert_eq!(entries[1].entry_type, EntryType::AssistantMessage);
        assert_eq!(entries[2].entry_type, EntryType::FileEdit);
    }
}
