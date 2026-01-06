//! Query history management

use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A single history entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub query: String,
    pub timestamp: DateTime<Local>,
    pub execution_time_ms: u64,
    pub row_count: Option<usize>,
    pub database: String,
}

/// Query history manager
#[derive(Clone, Debug, Default)]
pub struct QueryHistory {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
    current_index: Option<usize>,
}

impl QueryHistory {
    pub fn new(max_entries: usize) -> Self {
        let mut history = Self {
            entries: Vec::new(),
            max_entries,
            current_index: None,
        };
        let _ = history.load();
        history
    }

    /// Add a new entry to history
    pub fn add(&mut self, query: String, execution_time_ms: u64, row_count: Option<usize>, database: String) {
        // Don't add duplicates of the last entry
        if let Some(last) = self.entries.last() {
            if last.query.trim() == query.trim() {
                return;
            }
        }

        let entry = HistoryEntry {
            query,
            timestamp: Local::now(),
            execution_time_ms,
            row_count,
            database,
        };

        self.entries.push(entry);

        // Limit history size
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        self.current_index = None;
        let _ = self.save();
    }

    /// Get previous entry (for up arrow)
    pub fn previous(&mut self) -> Option<&HistoryEntry> {
        if self.entries.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            Some(idx) if idx > 0 => idx - 1,
            Some(idx) => idx,
            None => self.entries.len() - 1,
        };

        self.current_index = Some(new_index);
        self.entries.get(new_index)
    }

    /// Get next entry (for down arrow)
    pub fn next(&mut self) -> Option<&HistoryEntry> {
        if self.entries.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            Some(idx) if idx < self.entries.len() - 1 => idx + 1,
            _ => return None,
        };

        self.current_index = Some(new_index);
        self.entries.get(new_index)
    }

    /// Reset navigation
    pub fn reset_navigation(&mut self) {
        self.current_index = None;
    }

    /// Get all entries
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Search history
    pub fn search(&self, term: &str) -> Vec<&HistoryEntry> {
        let term_lower = term.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.query.to_lowercase().contains(&term_lower))
            .collect()
    }

    /// Get history file path
    fn history_file() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("alrajhi-sql-tui")
            .join("history.json")
    }

    /// Load history from disk
    fn load(&mut self) -> Result<()> {
        let path = Self::history_file();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            self.entries = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    /// Save history to disk
    fn save(&self) -> Result<()> {
        let path = Self::history_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_index = None;
        let _ = self.save();
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
