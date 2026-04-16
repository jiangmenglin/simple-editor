#[derive(Default, Clone)]
pub struct Row {
    pub chars: String,
}

impl Row {
    pub fn new() -> Self {
        Row { chars: String::new() }
    }

    pub fn from(s: &str) -> Self {
        Row { chars: s.to_string() }
    }

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
        self.chars.push_str(&other.chars);
    }

    /// Find all occurrences of query, returning byte-offset positions
    pub fn find_all(&self, query: &str) -> Vec<usize> {
        let mut results = Vec::new();
        if query.is_empty() {
            return results;
        }
        let mut start = 0;
        while let Some(pos) = self.chars[start..].find(query) {
            results.push(start + pos);
            start += pos + 1;
            if start >= self.chars.len() {
                break;
            }
        }
        results
    }

    /// Replace all occurrences of query with replacement
    pub fn replace_all(&mut self, query: &str, replacement: &str) -> usize {
        let count = self.find_all(query).len();
        if count > 0 {
            self.chars = self.chars.replace(query, replacement);
        }
        count
    }

    /// Render the row, returning the display string (may be truncated)
    pub fn render(&self, start: usize, width: usize) -> String {
        let end = start + width;
        if start >= self.chars.len() {
            return String::new();
        }
        let end = end.min(self.chars.len());
        self.chars[start..end].to_string()
    }
}
