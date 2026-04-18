use std::io;
use std::collections::HashSet;

use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use clipboard::ClipboardProvider;

use crate::find::FindState;
use crate::row::Row;
use crate::terminal::Terminal;
use crate::syntax::{self, HighlightType};
use crate::undo::{UndoHistory, EditAction};

/// Helper: get text from system clipboard
fn get_clipboard_text() -> Option<String> {
    let mut ctx: clipboard::ClipboardContext = ClipboardProvider::new().ok()?;
    ctx.get_contents().ok()
}

/// Helper: set text to system clipboard
fn set_clipboard_text(text: &str) -> Option<()> {
    let mut ctx: clipboard::ClipboardContext = ClipboardProvider::new().ok()?;
    ctx.set_contents(text.to_string()).ok()
}

pub struct Editor {
    rows: Vec<Row>,
    cursor_row: usize,
    cursor_col: usize,
    offset_row: usize,
    offset_col: usize,
    filename: Option<String>,
    dirty: bool,
    find: FindState,
    status_msg: String,
    /// Selection start: (row, col). None = no selection
    sel_start: Option<(usize, usize)>,
    /// Whether the status message should persist (for search results etc.)
    status_persist: bool,
    show_line_numbers: bool,
    language: Option<&'static syntax::LanguageConfig>,
    undo: UndoHistory,
}

impl Editor {
    pub fn new(filename: Option<String>) -> io::Result<Self> {
        Terminal::init()?;
        let mut editor = Editor {
            rows: vec![Row::new()],
            cursor_row: 0,
            cursor_col: 0,
            offset_row: 0,
            offset_col: 0,
            filename: None,
            dirty: false,
            find: FindState::new(),
            status_msg: String::new(),
            sel_start: None,
            status_persist: false,
            show_line_numbers: false,
            language: None,
            undo: UndoHistory::new(),
        };
        if let Some(ref fname) = filename {
            editor.open_file(fname)?;
            editor.filename = Some(fname.clone());
            editor.language = syntax::detect_language(fname);
        }
        Ok(editor)
    }

    // ---- File I/O ----

    fn open_file(&mut self, path: &str) -> io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.rows = content.lines().map(Row::from).collect();
        if self.rows.is_empty() {
            self.rows.push(Row::new());
        }
        self.dirty = false;
        self.undo.clear();
        Ok(())
    }

    fn save_file(&mut self) -> io::Result<()> {
        let filename = match &self.filename {
            Some(f) => f.clone(),
            None => {
                // Prompt for filename via status bar
                let name = self.prompt("Save as: ")?;
                if name.is_empty() {
                    self.set_status("Save cancelled.");
                    return Ok(());
                }
                self.filename = Some(name);
                self.filename.as_ref().unwrap().clone()
            }
        };

        let content: String = self.rows.iter().map(|r| r.as_str()).collect::<Vec<_>>().join("\n");
        std::fs::write(&filename, content)?;
        self.dirty = false;
        self.set_status(&format!("Saved to {}", filename));
        Ok(())
    }

    fn new_file(&mut self) {
        self.rows = vec![Row::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.offset_row = 0;
        self.offset_col = 0;
        self.filename = None;
        self.dirty = false;
        self.sel_start = None;
        self.undo.clear();
        self.set_status("New file created.");
    }

    // ---- Status ----

    fn gutter_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        let num_lines = self.rows.len();
        let digits = if num_lines < 10 { 1 }
            else if num_lines < 100 { 2 }
            else if num_lines < 1000 { 3 }
            else if num_lines < 10000 { 4 }
            else { 5 };
        digits + 2
    }

    fn set_status(&mut self, msg: &str) {
        self.status_msg = msg.to_string();
        self.status_persist = false;
    }

    // ---- Selection helpers ----

    fn get_selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let start = self.sel_start?;
        let end = (self.cursor_row, self.cursor_col);
        if start == end {
            return None;
        }
        // Normalize so start <= end
        if start.0 < end.0 || (start.0 == end.0 && start.1 <= end.1) {
            Some((start, end))
        } else {
            Some((end, start))
        }
    }

    fn copy_selection(&mut self) {
        if let Some(((sr, sc), (er, ec))) = self.get_selection_range() {
            let text = if sr == er {
                self.rows[sr].substring(sc, ec)
            } else {
                let mut parts = Vec::new();
                parts.push(self.rows[sr].substring_from(sc));
                for i in (sr + 1)..er {
                    parts.push(self.rows[i].as_str());
                }
                parts.push(self.rows[er].substring_to(ec));
                parts.join("\n")
            };

            if !text.is_empty() {
                if set_clipboard_text(&text).is_some() {
                    self.set_status(&format!("Copied {} chars", text.len()));
                } else {
                    self.set_status("Copy failed (clipboard unavailable)");
                }
            }
        }
    }

    fn delete_selection(&mut self) {
        if let Some(((sr, sc), (er, ec))) = self.get_selection_range() {
            if sr == er {
                self.rows[sr].delete_range(sc, ec);
            } else {
                // Keep prefix of first row + suffix of last row
                let first_prefix = self.rows[sr].substring_to(sc);
                let last_suffix = self.rows[er].substring_from(ec);
                // Remove rows sr..=er, then set row sr to combined
                self.rows.drain(sr..=er);
                let combined = format!("{}{}", first_prefix, last_suffix);
                self.rows.insert(sr, Row::from(&combined));
            }
            self.cursor_row = sr;
            self.cursor_col = sc;
            self.sel_start = None;
            self.dirty = true;
        }
    }

    fn get_selected_text(&self) -> Option<String> {
        let ((sr, sc), (er, ec)) = self.get_selection_range()?;
        let text = if sr == er {
            self.rows[sr].substring(sc, ec)
        } else {
            let mut parts = Vec::new();
            parts.push(self.rows[sr].substring_from(sc));
            for i in (sr + 1)..er {
                parts.push(self.rows[i].as_str());
            }
            parts.push(self.rows[er].substring_to(ec));
            parts.join("\n")
        };
        Some(text)
    }

    fn paste_from_clipboard(&mut self) {
        if let Some(text) = get_clipboard_text() {
            // Track selection deletion for undo
            if self.sel_start.is_some() && self.get_selection_range().is_some() {
                if let Some(deleted) = self.get_selected_text() {
                    let ((sr, sc), (er, ec)) = self.get_selection_range().unwrap();
                    self.delete_selection();
                    self.undo.push(EditAction::DeleteSelection {
                        start_row: sr, start_col: sc,
                        end_row: er, end_col: ec,
                        deleted_text: deleted,
                    });
                }
            }
            let cr = self.cursor_row;
            let cc = self.cursor_col;
            self.insert_string_no_sel(&text);
            self.undo.push(EditAction::InsertString { row: cr, col: cc, text: text.clone() });
            self.dirty = true;
            self.set_status(&format!("Pasted {} chars", text.len()));
        } else {
            self.set_status("Paste failed (clipboard unavailable)");
        }
    }

    fn insert_string_no_sel(&mut self, text: &str) {
        let lines: Vec<&str> = text.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                self.insert_newline_internal();
            }
            for c in line.chars() {
                self.insert_char_internal(c);
            }
        }
    }

    // ---- Core editing ----

    fn insert_char_internal(&mut self, c: char) {
        let row = &mut self.rows[self.cursor_row];
        row.insert(self.cursor_col, c);
        self.cursor_col += 1;
    }

    fn insert_newline_internal(&mut self) {
        let new_row = self.rows[self.cursor_row].split(self.cursor_col);
        self.cursor_row += 1;
        self.rows.insert(self.cursor_row, new_row);
        self.cursor_col = 0;
    }

    // ---- Tracked editing (for undo) ----

    fn delete_char_back_tracked(&mut self) -> Option<EditAction> {
        if self.cursor_col > 0 {
            let ch = self.rows[self.cursor_row].chars()[self.cursor_col - 1];
            let row = self.cursor_row;
            let col = self.cursor_col;
            self.rows[self.cursor_row].delete(self.cursor_col - 1);
            self.cursor_col -= 1;
            Some(EditAction::DeleteCharBack { row, col, ch })
        } else if self.cursor_row > 0 {
            let prev_len = self.rows[self.cursor_row - 1].len();
            let moved_text = self.rows[self.cursor_row].as_str();
            let row = self.cursor_row;
            let current = self.rows.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.rows[self.cursor_row].append(&current);
            self.cursor_col = prev_len;
            Some(EditAction::MergeRowsBack { row, prev_len, moved_text })
        } else {
            None
        }
    }

    fn delete_char_forward_tracked(&mut self) -> Option<EditAction> {
        if self.cursor_col < self.rows[self.cursor_row].len() {
            let ch = self.rows[self.cursor_row].chars()[self.cursor_col];
            let row = self.cursor_row;
            let col = self.cursor_col;
            self.rows[self.cursor_row].delete(self.cursor_col);
            Some(EditAction::DeleteCharForward { row, col, ch })
        } else if self.cursor_row + 1 < self.rows.len() {
            let original_len = self.rows[self.cursor_row].len();
            let next_text = self.rows[self.cursor_row + 1].as_str();
            let row = self.cursor_row;
            self.rows.remove(self.cursor_row + 1);
            // Reconstruct: need to get the combined text and re-split
            // Actually we just append, so rows[row] now has original content + next_text
            let next_row = Row::from(next_text.as_str());
            self.rows[self.cursor_row].append(&next_row);
            Some(EditAction::MergeRowsForward { row, original_len, next_text: next_text.to_string() })
        } else {
            None
        }
    }

    // ---- Undo/Redo ----

    fn do_undo(&mut self) {
        let action = match self.undo.undo_action() {
            Some(a) => a.clone(),
            None => return,
        };
        match action {
            EditAction::InsertChar { row, col, .. } => {
                self.cursor_row = row;
                self.cursor_col = col + 1;
                self.rows[row].delete(col);
                self.cursor_col = col;
            }
            EditAction::DeleteCharBack { row, col, ch } => {
                self.rows[row].insert(col - 1, ch);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            EditAction::DeleteCharForward { row, col, ch } => {
                self.rows[row].insert(col, ch);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            EditAction::MergeRowsBack { row, prev_len, moved_text: _ } => {
                // Undo: row was merged into row-1 at position prev_len.
                // Split row-1 at prev_len to recreate the original row.
                let new_row = self.rows[row - 1].split(prev_len);
                self.rows.insert(row, new_row);
                self.cursor_row = row;
                self.cursor_col = 0;
            }
            EditAction::MergeRowsForward { row, original_len, next_text: _ } => {
                // Undo: next row was merged into current row at original_len.
                // Split current row at original_len to recreate next row.
                let new_row = self.rows[row].split(original_len);
                self.rows.insert(row + 1, new_row);
                self.cursor_row = row;
                self.cursor_col = original_len;
            }
            EditAction::InsertNewline { row, col } => {
                // Undo: merge row+1 back into row at col
                let next = self.rows.remove(row + 1);
                self.rows[row].append(&next);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            EditAction::InsertTab { row, col } => {
                self.rows[row].delete_range(col, col + 4);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            EditAction::DeleteSelection { start_row, start_col, deleted_text, .. } => {
                self.insert_string_at(start_row, start_col, &deleted_text);
                self.cursor_row = start_row;
                self.cursor_col = start_col;
            }
            EditAction::InsertString { row, col, text } => {
                self.delete_string_at(row, col, &text);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            EditAction::ReplaceAll { old_rows } => {
                self.rows = old_rows.iter().map(|s| Row::from(s.as_str())).collect();
                if self.rows.is_empty() {
                    self.rows.push(Row::new());
                }
                self.cursor_row = self.cursor_row.min(self.rows.len() - 1);
                self.cursor_col = self.cursor_col.min(self.rows[self.cursor_row].len());
            }
        }
        self.dirty = true;
        self.sel_start = None;
    }

    fn do_redo(&mut self) {
        let action = match self.undo.redo_action() {
            Some(a) => a.clone(),
            None => return,
        };
        match action {
            EditAction::InsertChar { row, col, ch } => {
                self.rows[row].insert(col, ch);
                self.cursor_row = row;
                self.cursor_col = col + 1;
            }
            EditAction::DeleteCharBack { row, col, ch: _ } => {
                self.rows[row].delete(col - 1);
                self.cursor_row = row;
                self.cursor_col = col - 1;
            }
            EditAction::DeleteCharForward { row, col, ch: _ } => {
                self.rows[row].delete(col);
                self.cursor_row = row;
                self.cursor_col = col;
            }
            EditAction::MergeRowsBack { row, prev_len, moved_text: _ } => {
                let current = self.rows.remove(row);
                self.cursor_row = row - 1;
                self.rows[self.cursor_row].append(&current);
                self.cursor_col = prev_len;
            }
            EditAction::MergeRowsForward { row, original_len: _, next_text: _ } => {
                let next = self.rows.remove(row + 1);
                self.rows[row].append(&next);
                self.cursor_row = row;
                self.cursor_col = self.rows[row].len() - next.len();
            }
            EditAction::InsertNewline { row, col } => {
                let new_row = self.rows[row].split(col);
                self.rows.insert(row + 1, new_row);
                self.cursor_row = row + 1;
                self.cursor_col = 0;
            }
            EditAction::InsertTab { row, col } => {
                for _ in 0..4 {
                    self.rows[row].insert(col, ' ');
                }
                self.cursor_row = row;
                self.cursor_col = col + 4;
            }
            EditAction::DeleteSelection { start_row, start_col, end_row, end_col, .. } => {
                // Re-perform the deletion
                if start_row == end_row {
                    self.rows[start_row].delete_range(start_col, end_col);
                } else {
                    let first_prefix = self.rows[start_row].substring_to(start_col);
                    let last_suffix = self.rows[end_row].substring_from(end_col);
                    self.rows.drain(start_row..=end_row);
                    let combined = format!("{}{}", first_prefix, last_suffix);
                    self.rows.insert(start_row, Row::from(&combined));
                }
                self.cursor_row = start_row;
                self.cursor_col = start_col;
            }
            EditAction::InsertString { row, col, text } => {
                self.insert_string_at(row, col, &text);
                // Calculate final position
                let lines: Vec<&str> = text.split('\n').collect();
                if lines.len() == 1 {
                    self.cursor_row = row;
                    self.cursor_col = col + text.chars().count();
                } else {
                    self.cursor_row = row + lines.len() - 1;
                    self.cursor_col = lines.last().map(|l| l.chars().count()).unwrap_or(0);
                }
            }
            EditAction::ReplaceAll { old_rows: _ } => {
                // Can't redo ReplaceAll without storing the replacement info.
                // For now, this is a no-op since we don't store the replacement details.
            }
        }
        self.dirty = true;
        self.sel_start = None;
    }

    /// Insert a string at a specific position without using cursor
    fn insert_string_at(&mut self, row: usize, col: usize, text: &str) {
        let lines: Vec<&str> = text.split('\n').collect();
        if lines.len() == 1 {
            for (j, c) in text.chars().enumerate() {
                self.rows[row].insert(col + j, c);
            }
        } else {
            // Split current row at col
            let suffix = self.rows[row].substring_from(col);
            let row_len = self.rows[row].len();
            self.rows[row].delete_range(col, row_len);

            // Add first line content to current row
            for c in lines[0].chars() {
                self.rows[row].insert(col, c);
            }

            // Insert middle rows
            let mut insert_pos = row + 1;
            for line in &lines[1..lines.len() - 1] {
                self.rows.insert(insert_pos, Row::from(line));
                insert_pos += 1;
            }

            // Last line + suffix
            let mut last_row = Row::from(lines[lines.len() - 1]);
            let suffix_row = Row::from(&suffix);
            last_row.append(&suffix_row);
            self.rows.insert(insert_pos, last_row);
        }
    }

    /// Delete a string that was inserted at (row, col)
    fn delete_string_at(&mut self, row: usize, col: usize, text: &str) {
        let lines: Vec<&str> = text.split('\n').collect();
        if lines.len() == 1 {
            self.rows[row].delete_range(col, col + text.chars().count());
        } else {
            let prefix = self.rows[row].substring_to(col);
            let last_line_len = lines.last().map(|l| l.chars().count()).unwrap_or(0);
            let end_row = row + lines.len() - 1;
            let suffix = self.rows[end_row].substring_from(last_line_len);
            self.rows.drain(row..=end_row);
            let combined = format!("{}{}", prefix, suffix);
            self.rows.insert(row, Row::from(&combined));
        }
    }

    // ---- Cursor movement ----

    fn move_cursor(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.rows[self.cursor_row].len();
                }
            }
            KeyCode::Right => {
                let row_len = self.rows[self.cursor_row].len();
                if self.cursor_col < row_len {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.rows.len() {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
            KeyCode::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.cursor_col.min(self.rows[self.cursor_row].len());
                }
            }
            KeyCode::Down => {
                if self.cursor_row + 1 < self.rows.len() {
                    self.cursor_row += 1;
                    self.cursor_col = self.cursor_col.min(self.rows[self.cursor_row].len());
                }
            }
            KeyCode::Home => {
                self.cursor_col = 0;
            }
            KeyCode::End => {
                self.cursor_col = self.rows[self.cursor_row].len();
            }
            KeyCode::PageUp => {
                self.cursor_row = self.cursor_row.saturating_sub(20);
                self.cursor_col = self.cursor_col.min(self.rows[self.cursor_row].len());
            }
            KeyCode::PageDown => {
                self.cursor_row = (self.cursor_row + 20).min(self.rows.len() - 1);
                self.cursor_col = self.cursor_col.min(self.rows[self.cursor_row].len());
            }
            _ => {}
        }
    }

    // ---- Scrolling ----

    fn scroll(&mut self) {
        let (_, term_rows) = Terminal::size().unwrap_or((80, 24));
        let term_rows = term_rows as usize;

        if self.cursor_row < self.offset_row {
            self.offset_row = self.cursor_row;
        }
        if self.cursor_row >= self.offset_row + term_rows.saturating_sub(2) {
            self.offset_row = self.cursor_row - term_rows.saturating_sub(2) + 1;
        }

        let (term_cols, _) = Terminal::size().unwrap_or((80, 24));
        let text_area_width = term_cols as usize - self.gutter_width();

        let cursor_display_col = self.rows[self.cursor_row].display_width_to(self.cursor_col);
        if cursor_display_col < self.offset_col {
            self.offset_col = cursor_display_col;
        }
        if cursor_display_col >= self.offset_col + text_area_width {
            self.offset_col = cursor_display_col - text_area_width + 1;
        }
    }

    // ---- Prompt (for save-as, find, replace) ----

    fn prompt(&mut self, prompt_str: &str) -> io::Result<String> {
        let mut input = String::new();
        loop {
            self.set_status(&format!("{}{}", prompt_str, input));
            self.status_persist = true;
            self.refresh_screen()?;

            let key = Terminal::read_key()?;
            match key.code {
                KeyCode::Enter => {
                    self.status_persist = false;
                    return Ok(input);
                }
                KeyCode::Esc => {
                    self.status_persist = false;
                    self.set_status("");
                    return Ok(String::new());
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    input.push(c);
                }
                _ => {}
            }
        }
    }

    // ---- Find & Replace ----

    fn do_find(&mut self) -> io::Result<()> {
        let query = self.prompt("Find: ")?;
        if query.is_empty() {
            return Ok(());
        }
        self.find.input = query;
        self.find.search(&self.rows);

        if self.find.matches.is_empty() {
            self.set_status("No matches found.");
        } else {
            self.set_status(&format!("Found {} matches. Press F3/Shift+F3 to cycle.", self.find.matches.len()));
            self.status_persist = true;
            if let Some((row, col)) = self.find.current_match_pos() {
                self.cursor_row = row;
                self.cursor_col = col;
            }
        }
        Ok(())
    }

    fn find_next(&mut self) {
        if self.find.matches.is_empty() {
            self.set_status("No search active. Press Ctrl+F to search.");
            return;
        }
        self.find.next_match();
        if let Some((row, col)) = self.find.current_match_pos() {
            self.cursor_row = row;
            self.cursor_col = col;
        }
    }

    fn find_prev(&mut self) {
        if self.find.matches.is_empty() {
            self.set_status("No search active. Press Ctrl+F to search.");
            return;
        }
        self.find.prev_match();
        if let Some((row, col)) = self.find.current_match_pos() {
            self.cursor_row = row;
            self.cursor_col = col;
        }
    }

    fn do_replace(&mut self) -> io::Result<()> {
        let query = self.prompt("Replace - Find: ")?;
        if query.is_empty() {
            return Ok(());
        }

        // Count occurrences
        let mut total = 0;
        for row in &self.rows {
            total += row.find_all(&query).len();
        }
        if total == 0 {
            self.set_status("No matches found.");
            return Ok(());
        }

        let replacement = self.prompt(&format!("Replace '{}' with: ", query))?;

        // Snapshot rows for undo
        let old_rows: Vec<String> = self.rows.iter().map(|r| r.as_str()).collect();

        // Do the replacement
        let mut count = 0;
        for row in &mut self.rows {
            count += row.replace_all(&query, &replacement);
        }

        self.undo.push(EditAction::ReplaceAll { old_rows });

        // Fix cursor if it's now out of bounds
        if self.cursor_row >= self.rows.len() {
            self.cursor_row = self.rows.len() - 1;
        }
        self.cursor_col = self.cursor_col.min(self.rows[self.cursor_row].len());

        self.dirty = true;
        self.set_status(&format!("Replaced {} occurrence(s).", count));
        Ok(())
    }

    // ---- Rendering ----

    fn draw_rows(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        let (term_cols, term_rows) = Terminal::size().unwrap_or((80, 24));
        let term_cols = term_cols as usize;
        let term_rows = term_rows as usize;
        let gutter_w = self.gutter_width();
        let text_area_width = term_cols - gutter_w;

        // Collect match positions for highlighting
        let match_positions: HashSet<(usize, usize)> = {
            let mut set = HashSet::new();
            for &(r, c) in &self.find.matches {
                set.insert((r, c));
            }
            set
        };

        // Get selection range for highlighting
        let sel_range = self.get_selection_range();

        // Pre-compute block comment state up to the first visible row
        let mut in_block_comment = false;
        if let Some(lang) = self.language {
            if lang.block_comment.is_some() {
                for r in 0..self.offset_row.min(self.rows.len()) {
                    let row_chars: Vec<char> = self.rows[r].as_str().chars().collect();
                    let (_, state) = syntax::highlight_row(&row_chars, lang, in_block_comment);
                    in_block_comment = state;
                }
            }
        }

        for i in 0..term_rows.saturating_sub(1) {
            let file_row = self.offset_row + i;
            execute!(stdout, cursor::MoveTo(0, i as u16))?;

            // Render gutter (line numbers or ~)
            if file_row >= self.rows.len() {
                // Empty area
                if self.show_line_numbers {
                    execute!(stdout, SetForegroundColor(Color::DarkBlue))?;
                    execute!(stdout, crossterm::style::Print(format!("{:>width$} ", "~", width = gutter_w - 1)))?;
                    execute!(stdout, SetForegroundColor(Color::Reset))?;
                }
                if self.rows.len() == 1 && self.rows[0].is_empty() && i == 0 {
                    if !self.show_line_numbers {
                        execute!(stdout, crossterm::style::Print("~ "))?;
                    }
                    execute!(
                        stdout,
                        SetForegroundColor(Color::DarkCyan),
                        crossterm::style::Print("Simple Editor - Ctrl+Q:Quit Ctrl+S:Save Ctrl+N:New Ctrl+F:Find Ctrl+H:Replace"),
                        SetForegroundColor(Color::Reset)
                    )?;
                } else if !self.show_line_numbers {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::DarkBlue),
                        crossterm::style::Print("~"),
                        SetForegroundColor(Color::Reset)
                    )?;
                }
            } else {
                // Render line number
                if self.show_line_numbers {
                    let line_num = file_row + 1;
                    execute!(
                        stdout,
                        SetForegroundColor(Color::DarkGrey),
                        crossterm::style::Print(format!("{:>width$} ", line_num, width = gutter_w - 1)),
                        SetForegroundColor(Color::Reset)
                    )?;
                }
                let row = &self.rows[file_row];
                let start_char = row.char_at_display_col(self.offset_col);
                let render_str = row.render(start_char, text_area_width);

                // Compute syntax highlights for this row
                let syntax_highlights: Vec<HighlightType> = if let Some(lang) = self.language {
                    let row_chars: Vec<char> = row.as_str().chars().collect();
                    let (h, new_block) = syntax::highlight_row(&row_chars, lang, in_block_comment);
                    in_block_comment = new_block;
                    h
                } else {
                    in_block_comment = false;
                    Vec::new()
                };

                // Character-by-character rendering for highlights
                let chars: Vec<char> = render_str.chars().collect();
                let mut in_match = false;
                let mut in_selection = false;
                let mut current_syntax_color: Option<Color> = None;

                for (ci, ch) in chars.iter().enumerate() {
                    let actual_col = start_char + ci;

                    // Check if this char is part of a search match
                    let is_match = if !self.find.input.is_empty() {
                        let qlen = self.find.input.chars().count();
                        match_positions.iter().any(|&(r, c)| {
                            r == file_row && actual_col >= c && actual_col < c + qlen
                        })
                    } else {
                        false
                    };

                    // Check if this char is part of selection
                    let is_selected = if let Some(((sr, sc), (er, ec))) = sel_range {
                        (file_row > sr || (file_row == sr && actual_col >= sc))
                            && (file_row < er || (file_row == er && actual_col < ec))
                    } else {
                        false
                    };

                    // Syntax highlight color for this char
                    let syn_color = if actual_col < syntax_highlights.len() {
                        syntax_highlights[actual_col].foreground_color()
                    } else {
                        None
                    };

                    // Apply search match highlight (highest priority)
                    if is_match && !in_match {
                        execute!(stdout, SetBackgroundColor(Color::DarkYellow), SetForegroundColor(Color::Black))?;
                        in_match = true;
                    } else if !is_match && in_match {
                        execute!(stdout, SetBackgroundColor(Color::Reset), SetForegroundColor(Color::Reset))?;
                        in_match = false;
                    }

                    // Apply selection highlight
                    if is_selected && !in_selection {
                        execute!(stdout, SetBackgroundColor(Color::DarkCyan), SetForegroundColor(Color::White))?;
                        in_selection = true;
                    } else if !is_selected && in_selection {
                        execute!(stdout, SetBackgroundColor(Color::Reset), SetForegroundColor(Color::Reset))?;
                        in_selection = false;
                    }

                    // Apply syntax highlight (only when no search/selection override)
                    if !is_match && !is_selected {
                        if syn_color != current_syntax_color {
                            if let Some(c) = syn_color {
                                execute!(stdout, SetForegroundColor(c))?;
                            } else {
                                execute!(stdout, SetForegroundColor(Color::Reset))?;
                            }
                            current_syntax_color = syn_color;
                        }
                    }

                    execute!(stdout, crossterm::style::Print(ch))?;
                }

                if in_match || in_selection || current_syntax_color.is_some() {
                    execute!(stdout, SetBackgroundColor(Color::Reset), SetForegroundColor(Color::Reset))?;
                }
            }
        }
        Ok(())
    }

    fn draw_status_bar(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        let (term_cols, term_rows) = Terminal::size().unwrap_or((80, 24));
        let term_cols = term_cols as usize;
        let y = term_rows.saturating_sub(2) as u16;

        execute!(stdout, cursor::MoveTo(0, y))?;
        execute!(
            stdout,
            SetBackgroundColor(Color::DarkGrey),
            SetForegroundColor(Color::White)
        )?;

        let filename = self.filename.as_deref().unwrap_or("[No Name]");
        let dirty_flag = if self.dirty { " (modified)" } else { "" };
        let left = format!(" {}{} ", filename, dirty_flag);
        let right = format!(" Ln {}, Col {} ", self.cursor_row + 1, self.cursor_col + 1);

        let left_padded = format!("{:<width$}", left, width = term_cols);
        let truncated: String = left_padded.chars().take(term_cols).collect();
        execute!(stdout, crossterm::style::Print(&truncated))?;

        // Draw right-aligned part
        if right.len() < term_cols {
            execute!(
                stdout,
                cursor::MoveTo((term_cols - right.len()) as u16, y)
            )?;
            execute!(stdout, crossterm::style::Print(&right))?;
        }

        execute!(
            stdout,
            SetBackgroundColor(Color::Reset),
            SetForegroundColor(Color::Reset)
        )?;
        Ok(())
    }

    fn draw_message_bar(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        let (term_cols, term_rows) = Terminal::size().unwrap_or((80, 24));
        let term_cols = term_cols as usize;
        let y = term_rows.saturating_sub(1) as u16;

        execute!(stdout, cursor::MoveTo(0, y))?;
        execute!(stdout, terminal::Clear(ClearType::CurrentLine))?;

        let truncated: String = self.status_msg.chars().take(term_cols).collect();
        execute!(stdout, crossterm::style::Print(&truncated))?;
        Ok(())
    }

    fn refresh_screen(&mut self) -> io::Result<()> {
        self.scroll();
        let mut stdout = io::stdout();
        let gutter_w = self.gutter_width();

        execute!(stdout, terminal::Clear(ClearType::All))?;
        self.draw_rows(&mut stdout)?;
        self.draw_status_bar(&mut stdout)?;
        self.draw_message_bar(&mut stdout)?;

        // Position cursor
        let screen_row = (self.cursor_row - self.offset_row) as u16;
        let cursor_display_col = self.rows[self.cursor_row].display_width_to(self.cursor_col);
        let screen_col = (cursor_display_col - self.offset_col + gutter_w) as u16;
        Terminal::move_cursor(screen_row, screen_col)?;
        Terminal::flush()?;
        Ok(())
    }

    // ---- Main event loop ----

    fn handle_selection_key(&mut self, key: &KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            // Start selection if not already started
            if self.sel_start.is_none() {
                self.sel_start = Some((self.cursor_row, self.cursor_col));
            }
            match key.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
                | KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                    self.move_cursor(key.code);
                    return true;
                }
                _ => {}
            }
        }
        // If shift is NOT held and we move, clear selection
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            | KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                if !key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.sel_start = None;
                }
                self.move_cursor(key.code);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.set_status("Ctrl+Q: Quit | Ctrl+S: Save | Ctrl+N: New | Ctrl+F: Find | Ctrl+H: Replace | Ctrl+C/V: Copy/Paste");
        self.status_persist = true;

        loop {
            self.refresh_screen()?;
            let key = Terminal::read_key()?;

            // Clear persist status after any key
            if self.status_persist {
                // keep it
            } else {
                self.status_msg.clear();
            }

            match key.code {
                KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match c {
                        'q' => {
                            if self.dirty {
                                self.set_status("Unsaved changes! Press Ctrl+Q again to force quit, or Ctrl+S to save.");
                                self.refresh_screen()?;
                                let key2 = Terminal::read_key()?;
                                if let KeyCode::Char('q') = key2.code {
                                    if key2.modifiers.contains(KeyModifiers::CONTROL) {
                                        break;
                                    }
                                }
                                continue;
                            }
                            break;
                        }
                        's' => {
                            self.save_file()?;
                        }
                        'n' => {
                            if self.dirty {
                                self.set_status("Unsaved changes! Press Ctrl+N again to discard, or Ctrl+S to save.");
                                self.refresh_screen()?;
                                let key2 = Terminal::read_key()?;
                                if let KeyCode::Char('n') = key2.code {
                                    if key2.modifiers.contains(KeyModifiers::CONTROL) {
                                        self.new_file();
                                    }
                                }
                                continue;
                            }
                            self.new_file();
                        }
                        'f' => {
                            self.do_find()?;
                        }
                        'h' => {
                            self.do_replace()?;
                        }
                        'c' => {
                            self.copy_selection();
                        }
                        'v' => {
                            self.paste_from_clipboard();
                        }
                        'z' => {
                            self.do_undo();
                        }
                        'y' => {
                            self.do_redo();
                        }
                        'a' => {
                            // Select all
                            self.sel_start = Some((0, 0));
                            self.cursor_row = self.rows.len() - 1;
                            self.cursor_col = self.rows[self.cursor_row].len();
                        }
                        'l' => {
                            self.show_line_numbers = !self.show_line_numbers;
                            self.set_status(if self.show_line_numbers { "Line numbers ON" } else { "Line numbers OFF" });
                        }
                        _ => {}
                    }
                }
                KeyCode::F(3) => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        self.find_prev();
                    } else {
                        self.find_next();
                    }
                }
                KeyCode::Char(_c) => {
                    // If there's a selection, delete it first (tracked)
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        if let Some(deleted) = self.get_selected_text() {
                            let ((sr, sc), (er, ec)) = self.get_selection_range().unwrap();
                            self.delete_selection();
                            self.undo.push(EditAction::DeleteSelection {
                                start_row: sr, start_col: sc,
                                end_row: er, end_col: ec,
                                deleted_text: deleted,
                            });
                        }
                    }
                    let cr = self.cursor_row;
                    let cc = self.cursor_col;
                    self.insert_char_internal(_c);
                    self.undo.push(EditAction::InsertChar { row: cr, col: cc, ch: _c });
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Enter => {
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        if let Some(deleted) = self.get_selected_text() {
                            let ((sr, sc), (er, ec)) = self.get_selection_range().unwrap();
                            self.delete_selection();
                            self.undo.push(EditAction::DeleteSelection {
                                start_row: sr, start_col: sc,
                                end_row: er, end_col: ec,
                                deleted_text: deleted,
                            });
                        }
                    }
                    let cr = self.cursor_row;
                    let cc = self.cursor_col;
                    self.insert_newline_internal();
                    self.undo.push(EditAction::InsertNewline { row: cr, col: cc });
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Backspace => {
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        if let Some(deleted) = self.get_selected_text() {
                            let ((sr, sc), (er, ec)) = self.get_selection_range().unwrap();
                            self.delete_selection();
                            self.undo.push(EditAction::DeleteSelection {
                                start_row: sr, start_col: sc,
                                end_row: er, end_col: ec,
                                deleted_text: deleted,
                            });
                        }
                    } else if let Some(action) = self.delete_char_back_tracked() {
                        self.undo.push(action);
                    }
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Delete => {
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        if let Some(deleted) = self.get_selected_text() {
                            let ((sr, sc), (er, ec)) = self.get_selection_range().unwrap();
                            self.delete_selection();
                            self.undo.push(EditAction::DeleteSelection {
                                start_row: sr, start_col: sc,
                                end_row: er, end_col: ec,
                                deleted_text: deleted,
                            });
                        }
                    } else if let Some(action) = self.delete_char_forward_tracked() {
                        self.undo.push(action);
                    }
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Tab => {
                    let cr = self.cursor_row;
                    let cc = self.cursor_col;
                    for _ in 0..4 {
                        self.insert_char_internal(' ');
                    }
                    self.undo.push(EditAction::InsertTab { row: cr, col: cc });
                    self.dirty = true;
                }
                KeyCode::Esc => {
                    self.sel_start = None;
                    self.find.reset();
                    self.status_persist = false;
                }
                _ => {
                    self.handle_selection_key(&key);
                }
            }

            // After any action, clear persist flag
            if !matches!(
                key.code,
                KeyCode::F(3) | KeyCode::Esc
            ) {
                self.status_persist = false;
            }
        }

        Terminal::restore()?;
        Ok(())
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::restore();
    }
}
