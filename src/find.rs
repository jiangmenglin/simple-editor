use crate::row::Row;
use std::collections::HashSet;

/// Represents a search/replace dialog state
pub enum PromptMode {
    None,
    Find,
    ReplaceFind,
    ReplaceWith {
        find_query: String,
    },
}

pub struct FindState {
    pub mode: PromptMode,
    pub input: String,
    /// Positions of matches: (row_index, char_offset_in_row)
    pub matches: Vec<(usize, usize)>,
    /// Cached set for rendering lookups
    pub match_set: HashSet<(usize, usize)>,
    pub current_match: usize,
}

impl FindState {
    pub fn new() -> Self {
        FindState {
            mode: PromptMode::None,
            input: String::new(),
            matches: Vec::new(),
            match_set: HashSet::new(),
            current_match: 0,
        }
    }

    pub fn reset(&mut self) {
        self.mode = PromptMode::None;
        self.input.clear();
        self.matches.clear();
        self.match_set.clear();
        self.current_match = 0;
    }

    /// Search all rows for the query and store match positions
    pub fn search(&mut self, rows: &[Row]) {
        self.matches.clear();
        for (row_idx, row) in rows.iter().enumerate() {
            for pos in row.find_all(&self.input) {
                self.matches.push((row_idx, pos));
            }
        }
        self.match_set = self.matches.iter().copied().collect();
        self.current_match = 0;
    }

    /// Get the (row, col) of the current match, if any
    pub fn current_match_pos(&self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            None
        } else {
            Some(self.matches[self.current_match])
        }
    }

    /// Advance to next match
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    /// Go to previous match
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            if self.current_match == 0 {
                self.current_match = self.matches.len() - 1;
            } else {
                self.current_match -= 1;
            }
        }
    }
}
