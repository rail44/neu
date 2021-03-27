use core::cmp::min;
use std::borrow::Cow;
use std::ops::Deref;

use xi_rope::{LinesMetric, Rope};
pub(crate) struct Buffer(Rope);

impl Buffer {
    pub(crate) fn new() -> Self {
        Buffer(Rope::from("Hello World\n"))
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

    pub(crate) fn lines(&self) -> impl Iterator<Item = Cow<'_, str>> + '_ {
        self.0.lines(..)
    }

    pub(crate) fn remove_lines(&mut self, row: usize, count: usize) -> Rope {
        let start = self.0.offset_of_line(row);
        let end_line = min(row + count, self.count_lines());
        let end = self.0.offset_of_line(end_line);
        let range = start..end;
        let seq = self.0.subseq(range.clone());
        self.0.edit(range, Rope::from(""));
        seq
    }

    pub(crate) fn subseq_lines(&self, row: usize, count: usize) -> Rope {
        let start = self.0.offset_of_line(row);
        let end_line = min(row + count, self.count_lines());
        let end = self.0.offset_of_line(end_line);
        let range = start..end;
        self.0.subseq(range)
    }

    pub(crate) fn insert(&mut self, col: usize, row: usize, rope: Rope) {
        let offset = self.0.offset_of_line(row);
        let start = offset + col;
        self.0.edit(start..start, rope);
    }

    pub(crate) fn insert_char(&mut self, col: usize, row: usize, c: char) {
        let offset = self.0.offset_of_line(row);
        let start = offset + col;
        self.0.edit(start..start, Rope::from(c.to_string()));
    }

    pub(crate) fn as_str(&self) -> Cow<str> {
        self.0.slice_to_cow(..)
    }
}

impl Deref for Buffer {
    type Target = Rope;

    fn deref(&self) -> &Rope {
        &self.0
    }
}
