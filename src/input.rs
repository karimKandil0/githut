use std::path::PathBuf;

/// A text input field with cursor support.
#[derive(Default, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize, // byte index into value
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // find the char boundary before cursor
        let mut pos = self.cursor - 1;
        while !self.value.is_char_boundary(pos) {
            pos -= 1;
        }
        self.value.remove(pos);
        self.cursor = pos;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        self.value.remove(self.cursor);
        // cursor stays, but clamp in case value shrank
        self.cursor = self.cursor.min(self.value.len());
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut pos = self.cursor - 1;
        while !self.value.is_char_boundary(pos) {
            pos -= 1;
        }
        self.cursor = pos;
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let mut pos = self.cursor + 1;
        while pos <= self.value.len() && !self.value.is_char_boundary(pos) {
            pos += 1;
        }
        self.cursor = pos;
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Returns a string with a `_` cursor marker inserted at the cursor position,
    /// for display in overlays / search bar.
    pub fn display_with_cursor(&self) -> String {
        let mut s = self.value.clone();
        s.insert(self.cursor, '_');
        s
    }

    /// Trim and return the value, expanding path shortcuts.
    pub fn to_path(&self) -> String {
        expand_path(self.value.trim())
    }
}

/// Expand `~`, `./`, `../` in a path string.
pub fn expand_path(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    let home = std::env::var("HOME").unwrap_or_default();

    let expanded = if s == "~" {
        home.clone()
    } else if let Some(rest) = s.strip_prefix("~/") {
        format!("{}/{}", home, rest)
    } else {
        s.to_string()
    };

    // resolve ./ and ../ by canonicalizing relative to cwd
    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        expanded
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let joined = cwd.join(&path);
        // normalize without requiring path to exist
        normalize_path(&joined).to_string_lossy().to_string()
    }
}

/// Normalize a path (resolve . and ..) without requiring it to exist.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}
