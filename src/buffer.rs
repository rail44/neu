use core::cmp::min;
use std::borrow::Cow;
use std::ops::Deref;

use xi_rope::interval::IntervalBounds;
use xi_rope::rope::BaseMetric;
use xi_rope::{Cursor, LinesMetric, Rope};

#[derive(Default, Clone)]
pub(crate) struct Buffer(Rope);

fn is_alpha(c: &char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '_')
}

impl Buffer {
    pub(crate) fn new() -> Self {
        Buffer(Rope::default())
    }

    pub(crate) fn get_offset_by_cursor(&self, col: usize, row: usize) -> usize {
        let offset = self.0.offset_of_line(row);
        offset + col
    }

    pub(crate) fn row_len(&self, row: usize) -> usize {
        let start = self.0.offset_of_line(row);
        let end = self.0.offset_of_line(row + 1);
        if row == self.count_lines() {
            return end - start;
        }
        end - start - 1
    }

    pub(crate) fn count_lines(&self) -> usize {
        self.0.measure::<LinesMetric>()
    }

    pub(crate) fn count_chars(&self) -> usize {
        self.0.measure::<BaseMetric>()
    }

    pub(crate) fn lines(&self) -> impl Iterator<Item = Cow<'_, str>> + '_ {
        self.0.lines(..)
    }

    pub(crate) fn line(&self, n: usize) -> Option<Cow<'_, str>> {
        self.lines().nth(n)
    }

    pub(crate) fn remove_lines(&mut self, row: usize, count: usize) -> Buffer {
        let start = self.0.offset_of_line(row);
        let end_line = min(row + count, self.count_lines());
        let end = self.0.offset_of_line(end_line);
        let range = start..end;
        let seq = self.0.subseq(range.clone());
        self.0.edit(range, Rope::from(""));
        seq.into()
    }

    pub(crate) fn subseq_lines(&self, row: usize, count: usize) -> Buffer {
        let start = self.0.offset_of_line(row);
        let end_line = min(row + count, self.count_lines());
        let end = self.0.offset_of_line(end_line);
        let range = start..end;
        self.0.subseq(range).into()
    }

    pub(crate) fn insert(&mut self, col: usize, row: usize, buffer: Buffer) {
        let start = self.get_offset_by_cursor(col, row);
        self.0.edit(start..start, buffer);
    }

    pub(crate) fn count_forward_word(&mut self, col: usize, row: usize) -> usize {
        let start = self.get_offset_by_cursor(col, row);
        let mut cursor = Cursor::new(&self.0, start);

        let mut i = 0;
        while let Some(c) = cursor.next_codepoint() {
            if !is_alpha(&c) {
                break;
            }
            i += 1;
        }

        while let Some(c) = cursor.next_codepoint() {
            i += 1;
            if is_alpha(&c) {
                break;
            }
        }

        i
    }

    pub(crate) fn count_back_word(&mut self, col: usize, row: usize) -> usize {
        let start = self.get_offset_by_cursor(col, row);
        let mut cursor = Cursor::new(&self.0, start);

        let mut i = 0;
        while let Some(c) = cursor.prev_codepoint() {
            i += 1;
            if is_alpha(&c) {
                break;
            }
        }

        while let Some(c) = cursor.prev_codepoint() {
            if !is_alpha(&c) {
                break;
            }
            i += 1;
        }

        i
    }

    pub(crate) fn remove_chars(&mut self, col: usize, row: usize, count: usize) -> Buffer {
        let start = self.get_offset_by_cursor(col, row);
        let end = start + count;
        let range = start..end;
        let seq = self.0.subseq(range.clone());
        self.0.edit(range, Rope::default());
        seq.into()
    }

    pub(crate) fn insert_char(&mut self, col: usize, row: usize, c: char) {
        let start = self.get_offset_by_cursor(col, row);
        self.0.edit(start..start, Rope::from(c.to_string()));
    }

    pub(crate) fn slice_as_str<I: IntervalBounds>(&self, range: I) -> Cow<str> {
        self.0.slice_to_cow(range)
    }

    pub(crate) fn as_str(&self) -> Cow<str> {
        self.slice_as_str(..)
    }

    pub(crate) fn last_char(&self) -> Option<char> {
        let offset = self.count_chars();
        let s = self.slice_as_str(offset - 1..offset);
        s.chars().last()
    }

    pub(crate) fn end_with_line_break(&self) -> bool {
        self.last_char().map(|c| c == '\n').unwrap_or(false)
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

impl Deref for Buffer {
    type Target = Rope;

    fn deref(&self) -> &Rope {
        &self.0
    }
}
