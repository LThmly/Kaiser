use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Select,
}
#[derive(Clone)]
struct Snapshot {
    text: String,
    cursor: usize,
}
#[derive(Default)]
pub struct Editor {
    pub cursor: usize,
    pub anchor: usize,
    pub mode: Mode,
    pub preview: bool,
    pending: Option<char>,
    register: String,
    linewise: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}
impl Editor {
    pub fn normal(&mut self) {
        self.mode = Mode::Normal;
        self.pending = None;
    }
    pub fn new() -> Self {
        Self {
            preview: true,
            ..Self::default()
        }
    }
    fn checkpoint(&mut self, text: &str) {
        if self.undo.last().is_none_or(|entry| entry.text != text) {
            self.undo.push(Snapshot {
                text: text.into(),
                cursor: self.cursor,
            });
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }
    pub fn selection(&self, text: &str) -> Range<usize> {
        self.anchor.min(self.cursor)..(self.anchor.max(self.cursor) + 1).min(text.chars().count())
    }
    pub fn position(&self, text: &str) -> (usize, usize) {
        let mut row = 0;
        let mut column = 0;
        for c in text.chars().take(self.cursor) {
            if c == '\n' {
                row += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
        (row, column)
    }
    fn bounds(&self, text: &str) -> (usize, usize) {
        let chars: Vec<char> = text.chars().collect();
        let cursor = self.cursor.min(chars.len());
        let start = chars[..cursor]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |i| i + 1);
        let end = chars[cursor..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(chars.len(), |i| cursor + i);
        (start, end)
    }
    fn move_to(&mut self, target: usize, len: usize) {
        self.cursor = target.min(len);
        if self.mode != Mode::Select {
            self.anchor = self.cursor;
        }
    }
    fn vertical(&mut self, text: &str, delta: isize) {
        let (row, col) = self.position(text);
        let rows: Vec<&str> = text.split('\n').collect();
        let target = row.saturating_add_signed(delta).min(rows.len() - 1);
        let start = rows[..target]
            .iter()
            .map(|line| line.chars().count() + 1)
            .sum::<usize>();
        self.move_to(
            start + col.min(rows[target].chars().count()),
            text.chars().count(),
        );
    }
    fn replace(text: &mut String, range: Range<usize>, value: &str) {
        let byte = |i| {
            text.char_indices()
                .nth(i)
                .map_or(text.len(), |(offset, _)| offset)
        };
        let range = byte(range.start)..byte(range.end);
        text.replace_range(range, value);
    }
    pub fn insert(&mut self, text: &mut String, value: &str) {
        self.cursor = self.cursor.min(text.chars().count());
        Self::replace(text, self.cursor..self.cursor, value);
        self.cursor += value.chars().count();
        self.anchor = self.cursor;
    }
    pub fn key(&mut self, text: &mut String, key: KeyEvent) -> bool {
        let previous = text.clone();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        self.cursor = self.cursor.min(len);
        self.anchor = self.anchor.min(len);
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        if self.mode == Mode::Insert {
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    self.cursor = self.cursor.saturating_sub(1);
                    self.anchor = self.cursor;
                }
                KeyCode::Left => self.move_to(self.cursor.saturating_sub(1), len),
                KeyCode::Right => self.move_to(self.cursor + 1, len),
                KeyCode::Up => self.vertical(text, -1),
                KeyCode::Down => self.vertical(text, 1),
                KeyCode::Home => self.move_to(self.bounds(text).0, len),
                KeyCode::End => self.move_to(self.bounds(text).1, len),
                KeyCode::Backspace if self.cursor > 0 => {
                    Self::replace(text, self.cursor - 1..self.cursor, "");
                    self.cursor -= 1;
                    self.anchor = self.cursor;
                }
                KeyCode::Delete if self.cursor < len => {
                    Self::replace(text, self.cursor..self.cursor + 1, "")
                }
                KeyCode::Enter => self.insert(text, "\n"),
                KeyCode::Tab => self.insert(text, "    "),
                KeyCode::Char('s') if control => self.checkpoint(text),
                KeyCode::Char('w') if control => {
                    let end = self.cursor;
                    let mut start = end;
                    while start > 0 && chars[start - 1].is_whitespace() {
                        start -= 1;
                    }
                    while start > 0 && !chars[start - 1].is_whitespace() {
                        start -= 1;
                    }
                    Self::replace(text, start..end, "");
                    self.move_to(start, text.chars().count());
                }
                KeyCode::Char('u') if control => {
                    let start = self.bounds(text).0;
                    Self::replace(text, start..self.cursor, "");
                    self.move_to(start, text.chars().count());
                }
                KeyCode::Char('k') if control => {
                    Self::replace(text, self.cursor..self.bounds(text).1, "")
                }
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) =>
                {
                    self.insert(text, &c.to_string())
                }
                _ => {}
            }
            return *text != previous;
        }
        if let Some(pending) = self.pending.take() {
            if let KeyCode::Char(c) = key.code {
                if pending == 'g' {
                    let (start, end) = self.bounds(text);
                    let target = match c {
                        'g' => 0,
                        'e' => len.saturating_sub(1),
                        'h' => start,
                        'l' => end.saturating_sub(usize::from(end > start)),
                        's' => {
                            start
                                + chars[start..end]
                                    .iter()
                                    .take_while(|c| c.is_whitespace())
                                    .count()
                        }
                        _ => self.cursor,
                    };
                    self.move_to(target, len);
                } else if pending == 'r' {
                    self.checkpoint(text);
                    let range = self.selection(text);
                    let value = c.to_string().repeat(range.len());
                    Self::replace(text, range, &value);
                }
            }
            return *text != previous;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return false;
        }
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => self.move_to(self.cursor.saturating_sub(1), len),
            KeyCode::Char('l') | KeyCode::Right => self.move_to(self.cursor + 1, len),
            KeyCode::Char('j') | KeyCode::Down => self.vertical(text, 1),
            KeyCode::Char('k') | KeyCode::Up => self.vertical(text, -1),
            KeyCode::PageUp => self.vertical(text, -10),
            KeyCode::PageDown => self.vertical(text, 10),
            KeyCode::Home => self.move_to(self.bounds(text).0, len),
            KeyCode::End => {
                let (start, end) = self.bounds(text);
                self.move_to(end.saturating_sub(usize::from(end > start)), len);
            }
            KeyCode::Char('w' | 'W' | 'b' | 'B' | 'e' | 'E') => {
                let KeyCode::Char(motion) = key.code else {
                    unreachable!()
                };
                let class = |c: char| {
                    if c.is_whitespace() {
                        0
                    } else if motion.is_uppercase() || c.is_alphanumeric() || c == '_' {
                        1
                    } else {
                        2
                    }
                };
                let extended = self.selection(text).len() > 1;
                let origin = self.cursor;
                let (anchor, target) = match motion.to_ascii_lowercase() {
                    'w' => {
                        let start = if extended && self.cursor >= self.anchor {
                            (origin + 1).min(len)
                        } else {
                            origin
                        };
                        let mut target = start;
                        if target < len {
                            let kind = class(chars[target]);
                            while target < len && class(chars[target]) == kind {
                                target += 1;
                            }
                            if kind != 0 {
                                while target < len && chars[target].is_whitespace() {
                                    target += 1;
                                }
                            }
                        }
                        (start, target.saturating_sub(1).max(start).min(len))
                    }
                    'b' => {
                        let mut target = origin.saturating_sub(1);
                        while target > 0 && chars[target].is_whitespace() {
                            target -= 1;
                        }
                        if target < len {
                            let kind = class(chars[target]);
                            while target > 0 && class(chars[target - 1]) == kind {
                                target -= 1;
                            }
                        }
                        (
                            if extended && self.cursor >= self.anchor {
                                origin
                            } else {
                                origin.saturating_sub(1)
                            },
                            target,
                        )
                    }
                    _ => {
                        let mut target = origin;
                        if target + 1 < len
                            && (extended || class(chars[target + 1]) != class(chars[target]))
                        {
                            target += 1;
                        }
                        let anchor = target;
                        while target < len && chars[target].is_whitespace() {
                            target += 1;
                        }
                        if target < len {
                            let kind = class(chars[target]);
                            while target + 1 < len && class(chars[target + 1]) == kind {
                                target += 1;
                            }
                        }
                        (anchor, target)
                    }
                };
                if self.mode != Mode::Select {
                    self.anchor = anchor;
                }
                self.cursor = target;
            }
            KeyCode::Char('g' | 'r') => {
                if let KeyCode::Char(c) = key.code {
                    self.pending = Some(c);
                }
            }
            KeyCode::Char('v') => {
                self.mode = if self.mode == Mode::Select {
                    Mode::Normal
                } else {
                    Mode::Select
                };
            }
            KeyCode::Esc | KeyCode::Char(';') => {
                self.mode = Mode::Normal;
                self.anchor = self.cursor;
            }
            KeyCode::Char('%') => {
                self.anchor = 0;
                self.cursor = len.saturating_sub(1);
            }
            KeyCode::Char('x') => {
                let (start, end) = self.bounds(text);
                let range = self.selection(text);
                if range.start == start && range.end == (end + 1).min(len) && end < len {
                    self.cursor = end + 1;
                    let next_end = self.bounds(text).1;
                    self.cursor = next_end.min(len.saturating_sub(1));
                } else {
                    self.anchor = start;
                    self.cursor = end.min(len.saturating_sub(1));
                }
            }
            KeyCode::Char('i' | 'a' | 'I' | 'A' | 'o' | 'O') => {
                self.checkpoint(text);
                let (start, end) = self.bounds(text);
                self.cursor = match key.code {
                    KeyCode::Char('a') => self.selection(text).end,
                    KeyCode::Char('I') => {
                        start
                            + chars[start..end]
                                .iter()
                                .take_while(|c| c.is_whitespace())
                                .count()
                    }
                    KeyCode::Char('A') => end,
                    KeyCode::Char('o') => {
                        if end < len {
                            Self::replace(text, end + 1..end + 1, "\n");
                            end + 1
                        } else {
                            Self::replace(text, end..end, "\n");
                            end + 1
                        }
                    }
                    KeyCode::Char('O') => {
                        Self::replace(text, start..start, "\n");
                        start
                    }
                    _ => self.selection(text).start,
                };
                self.anchor = self.cursor;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('y' | 'd' | 'c') => {
                let range = self.selection(text);
                self.register = chars[range.clone()].iter().collect();
                self.linewise =
                    range.start == 0 || chars.get(range.start.saturating_sub(1)) == Some(&'\n');
                self.linewise &= self.register.ends_with('\n');
                if key.code != KeyCode::Char('y') {
                    self.checkpoint(text);
                    Self::replace(text, range.clone(), "");
                    self.cursor = range.start;
                    self.anchor = self.cursor;
                    self.mode = if key.code == KeyCode::Char('c') {
                        Mode::Insert
                    } else {
                        Mode::Normal
                    };
                } else {
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char('p' | 'P') => {
                self.checkpoint(text);
                self.cursor = if self.linewise {
                    let (start, end) = self.bounds(text);
                    if key.code == KeyCode::Char('P') {
                        start
                    } else if end < len {
                        end + 1
                    } else {
                        Self::replace(text, end..end, "\n");
                        end + 1
                    }
                } else if key.code == KeyCode::Char('p') {
                    self.selection(text).end
                } else {
                    self.selection(text).start
                };
                self.insert(text, &self.register.clone());
                self.mode = Mode::Normal;
            }
            KeyCode::Char('u' | 'U') => {
                let redo = key.code == KeyCode::Char('U');
                let source = if redo { &mut self.redo } else { &mut self.undo };
                if let Some(snapshot) = source.pop() {
                    let current = Snapshot {
                        text: text.clone(),
                        cursor: self.cursor,
                    };
                    if redo {
                        self.undo.push(current);
                    } else {
                        self.redo.push(current);
                    }
                    *text = snapshot.text;
                    self.cursor = snapshot.cursor;
                    self.anchor = self.cursor;
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
        self.cursor = self.cursor.min(text.chars().count());
        self.anchor = self.anchor.min(text.chars().count());
        *text != previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn press(editor: &mut Editor, text: &mut String, c: char) {
        editor.key(text, KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    #[test]
    fn unicode_selections_change_and_insert_undo_as_one_group() {
        let mut editor = Editor::new();
        let mut text = "héllo 世界\nnext".to_string();
        press(&mut editor, &mut text, 'w');
        assert_eq!(editor.cursor, 5);
        press(&mut editor, &mut text, 'l');
        press(&mut editor, &mut text, 'v');
        press(&mut editor, &mut text, 'l');
        press(&mut editor, &mut text, 'c');
        press(&mut editor, &mut text, '猫');
        editor.key(&mut text, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(text, "héllo 猫\nnext");
        press(&mut editor, &mut text, 'u');
        assert_eq!(text, "héllo 世界\nnext");
        press(&mut editor, &mut text, 'U');
        assert_eq!(text, "héllo 猫\nnext");
    }
    #[test]
    fn word_motions_select_text_before_delete_or_change() {
        let mut editor = Editor::new();
        let mut text = "one two.of three".to_string();
        press(&mut editor, &mut text, 'w');
        assert_eq!(editor.selection(&text), 0..4);
        press(&mut editor, &mut text, 'd');
        assert_eq!(text, "two.of three");
        press(&mut editor, &mut text, 'e');
        assert_eq!(editor.selection(&text), 0..3);
        press(&mut editor, &mut text, 'c');
        press(&mut editor, &mut text, '2');
        assert_eq!(text, "2.of three");
    }
    #[test]
    fn line_selection_yank_paste_and_goto_work_without_vim_operators() {
        let mut editor = Editor::new();
        let mut text = "one\ntwo\nthree".into();
        press(&mut editor, &mut text, 'j');
        press(&mut editor, &mut text, 'x');
        press(&mut editor, &mut text, 'd');
        assert_eq!(text, "one\nthree");
        press(&mut editor, &mut text, 'P');
        assert_eq!(text, "one\ntwo\nthree");
        press(&mut editor, &mut text, 'g');
        press(&mut editor, &mut text, 'e');
        assert_eq!(editor.cursor, 12);
        press(&mut editor, &mut text, 'g');
        press(&mut editor, &mut text, 'h');
        assert_eq!(editor.cursor, 8);
        press(&mut editor, &mut text, 'o');
        press(&mut editor, &mut text, '!');
        assert_eq!(text, "one\ntwo\nthree\n!");
    }
}
