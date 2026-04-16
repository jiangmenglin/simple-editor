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
        };
        if let Some(ref fname) = filename {
            editor.open_file(fname)?;
            editor.filename = Some(fname.clone());
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
        self.set_status("New file created.");
    }

    // ---- Status ----

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

    fn paste_from_clipboard(&mut self) {
        if let Some(text) = get_clipboard_text() {
            self.insert_string(&text);
            self.set_status(&format!("Pasted {} chars", text.len()));
        } else {
            self.set_status("Paste failed (clipboard unavailable)");
        }
    }

    fn insert_string(&mut self, text: &str) {
        // Delete any active selection first
        if self.sel_start.is_some() && self.get_selection_range().is_some() {
            self.delete_selection();
        }

        let lines: Vec<&str> = text.split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                self.insert_newline_internal();
            }
            for c in line.chars() {
                self.insert_char_internal(c);
            }
        }
        self.dirty = true;
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

    fn delete_char_internal(&mut self) {
        if self.cursor_col > 0 {
            self.rows[self.cursor_row].delete(self.cursor_col - 1);
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            // Merge with previous row
            let prev_len = self.rows[self.cursor_row - 1].len();
            let current = self.rows.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.rows[self.cursor_row].append(&current);
            self.cursor_col = prev_len;
        }
    }

    fn delete_char_forward(&mut self) {
        let row = &mut self.rows[self.cursor_row];
        if self.cursor_col < row.len() {
            row.delete(self.cursor_col);
        } else if self.cursor_row + 1 < self.rows.len() {
            // Merge next row into current
            let next = self.rows.remove(self.cursor_row + 1);
            self.rows[self.cursor_row].append(&next);
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
        let term_cols = term_cols as usize;

        let cursor_display_col = self.rows[self.cursor_row].display_width_to(self.cursor_col);
        if cursor_display_col < self.offset_col {
            self.offset_col = cursor_display_col;
        }
        if cursor_display_col >= self.offset_col + term_cols {
            self.offset_col = cursor_display_col - term_cols + 1;
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

        // Do the replacement
        let mut count = 0;
        for row in &mut self.rows {
            count += row.replace_all(&query, &replacement);
        }

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

        for i in 0..term_rows.saturating_sub(1) {
            let file_row = self.offset_row + i;
            execute!(stdout, cursor::MoveTo(0, i as u16))?;

            if file_row >= self.rows.len() {
                // Empty area
                if self.rows.len() == 1 && self.rows[0].is_empty() && i == 0 {
                    execute!(stdout, crossterm::style::Print("~ "))?;
                    execute!(
                        stdout,
                        SetForegroundColor(Color::DarkCyan),
                        crossterm::style::Print("Simple Editor - Ctrl+Q:Quit Ctrl+S:Save Ctrl+N:New Ctrl+F:Find Ctrl+H:Replace"),
                        SetForegroundColor(Color::Reset)
                    )?;
                } else {
                    execute!(
                        stdout,
                        SetForegroundColor(Color::DarkBlue),
                        crossterm::style::Print("~"),
                        SetForegroundColor(Color::Reset)
                    )?;
                }
            } else {
                let row = &self.rows[file_row];
                let start_char = row.char_at_display_col(self.offset_col);
                let render_str = row.render(start_char, term_cols);

                // Character-by-character rendering for highlights
                let chars: Vec<char> = render_str.chars().collect();
                let mut in_match = false;
                let mut in_selection = false;

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

                    if is_match && !in_match {
                        execute!(stdout, SetBackgroundColor(Color::DarkYellow), SetForegroundColor(Color::Black))?;
                        in_match = true;
                    } else if !is_match && in_match {
                        execute!(stdout, SetBackgroundColor(Color::Reset), SetForegroundColor(Color::Reset))?;
                        in_match = false;
                    }

                    if is_selected && !in_selection {
                        execute!(stdout, SetBackgroundColor(Color::DarkCyan), SetForegroundColor(Color::White))?;
                        in_selection = true;
                    } else if !is_selected && in_selection {
                        execute!(stdout, SetBackgroundColor(Color::Reset), SetForegroundColor(Color::Reset))?;
                        in_selection = false;
                    }

                    execute!(stdout, crossterm::style::Print(ch))?;
                }

                if in_match || in_selection {
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

        execute!(stdout, terminal::Clear(ClearType::All))?;
        self.draw_rows(&mut stdout)?;
        self.draw_status_bar(&mut stdout)?;
        self.draw_message_bar(&mut stdout)?;

        // Position cursor
        let screen_row = (self.cursor_row - self.offset_row) as u16;
        let cursor_display_col = self.rows[self.cursor_row].display_width_to(self.cursor_col);
        let screen_col = (cursor_display_col - self.offset_col) as u16;
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
                        'a' => {
                            // Select all
                            self.sel_start = Some((0, 0));
                            self.cursor_row = self.rows.len() - 1;
                            self.cursor_col = self.rows[self.cursor_row].len();
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
                    // If there's a selection, delete it first
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        self.delete_selection();
                    }
                    self.insert_char_internal(_c);
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Enter => {
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        self.delete_selection();
                    }
                    self.insert_newline_internal();
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Backspace => {
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        self.delete_selection();
                    } else {
                        self.delete_char_internal();
                    }
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Delete => {
                    if self.sel_start.is_some() && self.get_selection_range().is_some() {
                        self.delete_selection();
                    } else {
                        self.delete_char_forward();
                    }
                    self.dirty = true;
                    self.sel_start = None;
                }
                KeyCode::Tab => {
                    for _ in 0..4 {
                        self.insert_char_internal(' ');
                    }
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
