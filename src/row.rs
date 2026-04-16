use unicode_width::UnicodeWidthChar;

#[derive(Default, Clone)]
pub struct Row {
    chars: Vec<char>,
}

impl Row {
    pub fn new() -> Self {
        Row { chars: Vec::new() }
    }

    pub fn from(s: &str) -> Self {
        Row { chars: s.chars().collect() }
    }

    /// Number of characters (not bytes)
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn insert(&mut self, at: usize, c: char) {
        if at >= self.chars.len() {
            self.chars.push(c);
        } else {
            self.chars.insert(at, c);
        }
    }

    pub fn delete(&mut self, at: usize) -> Option<char> {
        if at < self.chars.len() {
            Some(self.chars.remove(at))
        } else {
            None
        }
    }

    pub fn split(&mut self, at: usize) -> Self {
        let right = self.chars.split_off(at);
        Row { chars: right }
    }

    pub fn append(&mut self, other: &Row) {
        self.chars.extend(other.chars.iter().cloned());
    }

    /// Convert to String
    pub fn as_str(&self) -> String {
        self.chars.iter().collect()
    }

    /// Display width of characters from start to char_idx
    pub fn display_width_to(&self, char_idx: usize) -> usize {
        let end = char_idx.min(self.chars.len());
        self.chars[..end]
            .iter()
            .map(|c| c.width().unwrap_or(1))
            .sum()
    }

    /// Find the char index corresponding to a display column
    pub fn char_at_display_col(&self, display_col: usize) -> usize {
        let mut width = 0;
        for (i, c) in self.chars.iter().enumerate() {
            if width >= display_col {
                return i;
            }
            width += c.width().unwrap_or(1);
        }
        self.chars.len()
    }

    /// Find all occurrences of query, returning char-offset positions
    pub fn find_all(&self, query: &str) -> Vec<usize> {
        let s: String = self.chars.iter().collect();
        let mut results = Vec::new();
        if query.is_empty() {
            return results;
        }
        let mut byte_start = 0;
        while let Some(byte_pos) = s[byte_start..].find(query) {
            let abs_byte_pos = byte_start + byte_pos;
            let char_pos = s[..abs_byte_pos].chars().count();
            results.push(char_pos);
            byte_start = abs_byte_pos + query.len();
            if byte_start >= s.len() {
                break;
            }
        }
        results
    }

    /// Replace all occurrences of query with replacement
    pub fn replace_all(&mut self, query: &str, replacement: &str) -> usize {
        let count = self.find_all(query).len();
        if count > 0 {
            let s: String = self.chars.iter().collect();
            self.chars = s.replace(query, replacement).chars().collect();
        }
        count
    }

    /// Render the row from a given char index, fitting within max display width
    pub fn render(&self, start: usize, max_width: usize) -> String {
        if start >= self.chars.len() {
            return String::new();
        }
        let mut result = String::new();
        let mut width = 0;
        for c in &self.chars[start..] {
            let cw = c.width().unwrap_or(1);
            if width + cw > max_width {
                break;
            }
            result.push(*c);
            width += cw;
        }
        result
    }

    /// Get substring by char range [start, end)
    pub fn substring(&self, start: usize, end: usize) -> String {
        let end = end.min(self.chars.len());
        if start >= end {
            return String::new();
        }
        self.chars[start..end].iter().collect()
    }

    /// Get substring from start to end of row
    pub fn substring_from(&self, start: usize) -> String {
        self.substring(start, self.chars.len())
    }

    /// Get substring from beginning to end
    pub fn substring_to(&self, end: usize) -> String {
        self.substring(0, end)
    }

    /// Delete a range of characters
    pub fn delete_range(&mut self, start: usize, end: usize) {
        let end = end.min(self.chars.len());
        if start < end {
            self.chars.drain(start..end);
        }
    }
}
