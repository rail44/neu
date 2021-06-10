use core::ops::RangeBounds;
use std::borrow::Cow;

use ropey::{Rope, RopeSlice};

#[derive(PartialEq, Clone, Debug)]
enum CharKind {
    AlphaNumeric,
    Symbol,
    Other,
    WhiteSpace,
}

impl CharKind {
    fn from_char(c: &char) -> Self {
        use CharKind::*;
        if c.is_ascii_alphanumeric() {
            return AlphaNumeric;
        }

        if c.is_ascii_punctuation() {
            return Symbol;
        }

        if c.is_ascii_whitespace() {
            return WhiteSpace;
        }
        Other
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct Buffer(Rope);

impl Buffer {
    pub(crate) fn get_offset_by_cursor(&self, col: usize, row: usize) -> usize {
        let offset = self.0.line_to_char(row);
        offset + col
    }

    pub(crate) fn row_len(&self, row: usize) -> usize {
        let start = self.0.line_to_char(row);
        let end = self.0.line_to_char(row + 1);
        (end - start).saturating_sub(1)
    }

    pub(crate) fn lines_at(&self, i: usize) -> impl Iterator<Item = RopeSlice> {
        self.0.lines_at(i)
    }

    pub(crate) fn count_lines(&self) -> usize {
        self.0.len_lines()
    }

    pub(crate) fn count_chars(&self) -> usize {
        self.0.len_utf16_cu()
    }

    pub(crate) fn insert(&mut self, col: usize, row: usize, buffer: Buffer) {
        let start = self.get_offset_by_cursor(col, row);
        let right = self.0.split_off(start);
        self.0.append(buffer.0);
        self.0.append(right);
    }

    pub(crate) fn count_forward_word(&self, col: usize, row: usize) -> usize {
        let start = self.get_offset_by_cursor(col, row);
        let mut chars = self.0.chars_at(start);

        let c = chars.next();
        if c == None {
            return 0;
        }
        let c = c.unwrap();

        let k = CharKind::from_char(&c);

        let mut i = 0;
        while let Some(c) = chars.next() {
            i += 1;
            if c.is_ascii_whitespace() {
                break;
            }

            if CharKind::from_char(&c) != k {
                break;
            }
        }
        chars.prev();

        for c in chars {
            if !c.is_ascii_whitespace() {
                break;
            }
            i += 1;
        }

        i
    }

    pub(crate) fn count_back_word(&self, col: usize, row: usize) -> usize {
        let start = self.get_offset_by_cursor(col, row);
        let mut chars = self.0.chars_at(start);

        let mut i = 0;
        while let Some(c) = chars.next() {
            i += 1;
            if !c.is_ascii_whitespace() {
                break;
            }
        }
        chars.next();

        let c = chars.prev().unwrap();
        let k = CharKind::from_char(&c);

        while let Some(c) = chars.prev() {
            if c.is_ascii_whitespace() {
                break;
            }

            if CharKind::from_char(&c) != k {
                break;
            }
            i += 1;
        }

        i
    }

    pub(crate) fn current_line(&self, row: usize) -> (usize, usize) {
        let start = self.0.line_to_char(row);
        let end = self.0.line_to_char(row + 1);
        (start, end)
    }

    pub(crate) fn remove_chars(&mut self, col: usize, row: usize, count: usize) -> Buffer {
        let start = self.get_offset_by_cursor(col, row);
        let end = start + count;
        let range = start..end;
        let seq: Rope = self.0.slice(range.clone()).into();
        self.0.remove(range);
        Buffer(seq)
    }

    pub(crate) fn remove<I: RangeBounds<usize> + Clone>(&mut self, range: I) -> Buffer {
        let seq: Rope = self.0.slice(range.clone()).into();
        self.0.remove(range);
        Buffer(seq)
    }

    pub(crate) fn subseq<I: RangeBounds<usize> + Clone>(&mut self, range: I) -> Buffer {
        let seq = self.0.slice(range);
        Buffer(seq.into())
    }

    pub(crate) fn insert_char(&mut self, col: usize, row: usize, c: char) {
        let start = self.get_offset_by_cursor(col, row);
        self.0.insert_char(start, c);
    }

    pub(crate) fn slice_as_str<I: RangeBounds<usize>>(&self, range: I) -> Cow<str> {
        self.0.slice(range).into()
    }

    pub(crate) fn as_str(&self) -> Cow<str> {
        (&self.0).into()
    }

    pub(crate) fn last_char(&self) -> Option<char> {
        let offset = self.count_chars();
        let s = self.slice_as_str(offset - 1..offset);
        s.chars().last()
    }

    pub(crate) fn end_with_line_break(&self) -> bool {
        self.last_char().map(|c| c == '\n').unwrap_or(false)
    }

    pub(crate) fn get_cursor_by_offset(&self, offset: usize) -> (usize, usize) {
        let row = self.0.char_to_line(offset);
        let row_offset = self.0.line_to_char(row);
        (row, offset - row_offset)
    }
}

impl From<&str> for Buffer {
    fn from(s: &str) -> Self {
        Self(Rope::from(s))
    }
}

impl From<Rope> for Buffer {
    fn from(r: Rope) -> Self {
        Self(r)
    }
}

impl From<Buffer> for Rope {
    fn from(b: Buffer) -> Rope {
        b.0
    }
}
