use core::ops::Range;
use std::env::current_dir;
use std::ffi::OsString;
use std::fs;

use crate::buffer::Buffer;
use crate::edit::EditKind;
use crate::mode::Mode;
use crate::position::Position;
use crate::selection::{Selection, SelectionKind};

use termion::terminal_size;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) enum SearchDirection {
    Forward,
    Reverse,
}

impl Default for SearchDirection {
    fn default() -> SearchDirection {
        SearchDirection::Forward
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub(super) struct State {
    pub(super) path: Option<OsString>,
    pub(super) row_offset: usize,
    pub(super) cursor: Position,
    pub(super) max_column: usize,
    pub(super) mode: Mode,
    pub(super) yanked: String,
    pub(super) size: (u16, u16),
    pub(super) buffer: Buffer,
    pub(super) prev_edit: Option<(EditKind, usize)>,
    pub(super) search_pattern: String,
    pub(super) search_direction: SearchDirection,
}

impl State {
    pub(super) fn new() -> Self {
        let size = terminal_size().unwrap();

        Self {
            size,
            buffer: "\n".into(),
            ..Default::default()
        }
    }

    pub(super) fn open_file(filename: &str) -> Self {
        let size = terminal_size().unwrap();
        let s = fs::read_to_string(filename).unwrap();
        let buffer = Buffer::from(s.as_str());
        let path = current_dir().unwrap().join(filename).into_os_string();

        Self {
            size,
            buffer,
            path: Some(path),
            ..Default::default()
        }
    }

    pub(super) fn get_cursor_offset(&self) -> usize {
        self.buffer.get_offset_by_position(self.cursor)
    }

    pub(super) fn count_word_end(&self) -> usize {
        self.buffer.count_word_end(self.cursor)
    }

    pub(super) fn count_word_back(&self) -> usize {
        self.buffer.count_back_word(self.cursor)
    }

    pub(super) fn count_word_forward(&self) -> usize {
        self.buffer.count_forward_word(self.cursor)
    }

    pub(super) fn count_word_start(&self) -> usize {
        self.buffer.count_word_start(self.cursor)
    }

    pub(super) fn current_line(&self) -> Range<usize> {
        self.buffer.line_range(self.cursor.row)
    }

    pub(super) fn current_line_remain(&self) -> Range<usize> {
        self.buffer.line_remain(self.cursor)
    }

    pub(super) fn measure_selection(&self, s: Selection) -> Range<usize> {
        let cursor_offset = self.get_cursor_offset();

        use SelectionKind::*;
        match s.kind {
            Left => {
                unimplemented!();
                // self.store.do_send(store::CursorLeft(cmd.count)).unwrap();
            }
            Down => {
                unimplemented!();
                // self.store.do_send(store::CursorDown(cmd.count)).unwrap();
            }
            Up => {
                unimplemented!();
                // self.store.do_send(store::CursorUp(cmd.count)).unwrap();
            }
            Right => {
                unimplemented!();
                // self.store.do_send(store::CursorRight(cmd.count)).unwrap();
            }
            WordEnd => {
                let count = self.count_word_end();
                cursor_offset..cursor_offset + count
            }
            ForwardWord => {
                let count = self.count_word_forward();
                cursor_offset..cursor_offset + count
            }
            BackWord => {
                let count = self.count_word_back();
                cursor_offset - count..cursor_offset
            }
            Word => {
                let end_count = self.count_word_end();
                let back_count = self.count_word_start();
                cursor_offset - back_count..cursor_offset + end_count
            }
            Line => self.current_line(),
            LineRemain => self.current_line_remain(),
        }
    }
}
