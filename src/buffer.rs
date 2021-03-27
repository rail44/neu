use core::cmp::min;
use std::borrow::Cow;
use std::ops::Deref;

use xi_rope::interval::IntervalBounds;
use xi_rope::rope::BaseMetric;
use xi_rope::{LinesMetric, Rope};

#[derive(Default, Clone)]
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

    pub(crate) fn count_chars(&self) -> usize {
        self.0.measure::<BaseMetric>()
    }

    pub(crate) fn lines(&self) -> impl Iterator<Item = Cow<'_, str>> + '_ {
        self.0.lines(..)
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
        let offset = self.0.offset_of_line(row);
        let start = offset + col;
        self.0.edit(start..start, buffer);
    }

    pub(crate) fn remove_chars(&mut self, col: usize, row: usize, count: usize) -> Buffer {
        let offset = self.0.offset_of_line(row);
        let start = offset + col;
        let end = start + count;
        let range = start..end;
        let seq = self.0.subseq(range.clone());
        self.0.edit(range, Rope::default());
        seq.into()
    }

    pub(crate) fn insert_char(&mut self, col: usize, row: usize, c: char) {
        let offset = self.0.offset_of_line(row);
        let start = offset + col;
        self.0.edit(start..start, Rope::from(c.to_string()));
    }

    pub(crate) fn slice_as_str<I: IntervalBounds>(&self, range: I) -> Cow<str> {
        self.0.slice_to_cow(range)
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
