#[derive(Clone, Debug)]
pub enum EditAction {
    InsertChar {
        row: usize,
        col: usize,
        ch: char,
    },
    DeleteCharBack {
        row: usize,
        col: usize,
        ch: char,
    },
    DeleteCharForward {
        row: usize,
        col: usize,
        ch: char,
    },
    MergeRowsBack {
        row: usize,
        prev_len: usize,
        moved_text: String,
    },
    MergeRowsForward {
        row: usize,
        original_len: usize,
        next_text: String,
    },
    InsertNewline {
        row: usize,
        col: usize,
    },
    InsertTab {
        row: usize,
        col: usize,
    },
    DeleteSelection {
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
        deleted_text: String,
    },
    InsertString {
        row: usize,
        col: usize,
        text: String,
    },
    ReplaceAll {
        old_rows: Vec<String>,
        find_query: String,
        replacement: String,
    },
}

pub struct UndoHistory {
    actions: Vec<EditAction>,
    index: usize,
    max_size: usize,
}

impl UndoHistory {
    pub fn new() -> Self {
        UndoHistory {
            actions: Vec::new(),
            index: 0,
            max_size: 1000,
        }
    }

    pub fn push(&mut self, action: EditAction) {
        self.actions.truncate(self.index);
        if self.actions.len() >= self.max_size {
            self.actions.remove(0);
        }
        self.actions.push(action);
        self.index = self.actions.len();
    }

    pub fn can_undo(&self) -> bool {
        self.index > 0
    }

    pub fn can_redo(&self) -> bool {
        self.index < self.actions.len()
    }

    pub fn undo_action(&mut self) -> Option<&EditAction> {
        if self.index > 0 {
            self.index -= 1;
            Some(&self.actions[self.index])
        } else {
            None
        }
    }

    pub fn redo_action(&mut self) -> Option<&EditAction> {
        if self.index < self.actions.len() {
            self.index += 1;
            Some(&self.actions[self.index - 1])
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.index = 0;
    }
}
