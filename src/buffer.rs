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

#[derive(Default, Clone, Debug, PartialEq)]
pub(super) struct Buffer(Rope);

impl Buffer {
    pub(super) fn get_offset_by_cursor(&self, col: usize, row: usize) -> usize {
        let offset = self.0.line_to_char(row);
        offset + col
    }

    pub(super) fn row_len(&self, row: usize) -> usize {
        let start = self.0.line_to_char(row);
        let end = self.0.line_to_char(row + 1);
        (end - start).saturating_sub(1)
    }

    pub(super) fn line(&self, i: usize) -> BufferSlice {
        self.0.line(i).into()
    }

    pub(super) fn lines_at(&self, i: usize) -> impl Iterator<Item = BufferSlice> {
        self.0.lines_at(i).map(BufferSlice::from)
    }

    pub(super) fn count_lines(&self) -> usize {
        self.0.len_lines().saturating_sub(1)
    }

    pub(super) fn insert(&mut self, col: usize, row: usize, s: &str) {
        let i = self.get_offset_by_cursor(col, row);
        self.0.insert(i, s);
    }

    pub(super) fn count_forward_word(&self, col: usize, row: usize) -> usize {
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

    pub(super) fn count_back_word(&self, col: usize, row: usize) -> usize {
        let start = self.get_offset_by_cursor(col, row);
        let mut chars = self.0.chars_at(start);

        let mut i = 0;
        while let Some(c) = chars.prev() {
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

    pub(super) fn current_line_remain(&self, col: usize, row: usize) -> (usize, usize) {
        let offset = self.get_offset_by_cursor(col, row);
        let end = self.0.line_to_char(row + 1);
        (offset, end - 1)
    }

    pub(super) fn current_line(&self, row: usize) -> (usize, usize) {
        let start = self.0.line_to_char(row);
        let end = self.0.line_to_char(row + 1);
        (start, end)
    }

    pub(super) fn current_line_indent_head(&self, row: usize) -> usize {
        let start = self.0.line_to_char(row);
        let chars = self.0.chars_at(start);

        let mut i = 0;
        for c in chars {
            if !c.is_ascii_whitespace() {
                break;
            }
            i += 1;
        }
        start + i
    }

    pub(super) fn remove<I: RangeBounds<usize> + Clone>(&mut self, range: I) -> String {
        let seq = self.0.slice(range.clone()).into();
        self.0.remove(range);
        seq
    }

    pub(super) fn slice<I: RangeBounds<usize> + Clone>(&self, range: I) -> BufferSlice {
        self.0.slice(range).into()
    }

    pub(super) fn as_str(&self) -> Cow<str> {
        (&self.0).into()
    }

    pub(super) fn get_cursor_by_offset(&self, offset: usize) -> (usize, usize) {
        let row = self.0.char_to_line(offset);
        let row_offset = self.0.line_to_char(row);
        (row, offset - row_offset)
    }

    pub(super) fn get_cursor_by_byte(&self, i: usize) -> (usize, usize) {
        let row = self.0.byte_to_line(i);
        let row_byte = self.0.line_to_byte(row);
        (row, i - row_byte)
    }

    pub(super) fn get_chunk_at_byte(&self, i: usize) -> Option<(&str, usize, usize, usize)> {
        self.0.get_chunk_at_byte(i)
    }

    pub(super) fn bytes_at(&self, i: usize) -> impl Iterator<Item = u8> + '_ {
        self.0.bytes_at(i)
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

#[derive(Clone, Debug)]
pub(super) struct BufferSlice<'a>(RopeSlice<'a>);

impl<'a> From<RopeSlice<'a>> for BufferSlice<'a> {
    fn from(rope_slice: RopeSlice<'a>) -> Self {
        BufferSlice(rope_slice)
    }
}

impl<'a> BufferSlice<'a> {
    pub(super) fn bytes(&self) -> impl Iterator<Item = u8> + '_ {
        self.0.bytes()
    }

    pub(super) fn as_str(&self) -> Cow<str> {
        (self.0).into()
    }

    pub(super) fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0.chars()
    }
}
