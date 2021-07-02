use std::env::current_dir;
use std::ffi::OsString;
use std::fs;

use crate::action::EditKind;
use crate::buffer::Buffer;
use crate::mode::Mode;
use crate::selection::{Selection, SelectionKind};

use termion::terminal_size;

#[derive(Default, Clone, Debug, PartialEq)]
pub(super) struct Cursor {
    pub(super) row: usize,
    pub(super) col: usize,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub(super) struct State {
    pub(super) path: Option<OsString>,
    pub(super) row_offset: usize,
    pub(super) cursor: Cursor,
    pub(super) max_column: usize,
    pub(super) mode: Mode,
    pub(super) yanked: String,
    pub(super) size: (u16, u16),
    pub(super) buffer: Buffer,
    pub(super) prev_edit: Option<(EditKind, usize)>,
    pub(super) search_pattern: String,
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
        self.buffer
            .get_offset_by_cursor(self.cursor.col, self.cursor.row)
    }

    pub(super) fn count_word_back(&self) -> usize {
        self.buffer
            .count_back_word(self.cursor.col, self.cursor.row)
    }

    pub(super) fn count_word_forward(&self) -> usize {
        self.buffer
            .count_forward_word(self.cursor.col, self.cursor.row)
    }

    pub(super) fn current_line(&self) -> (usize, usize) {
        self.buffer.current_line(self.cursor.row)
    }

    pub(super) fn current_line_remain(&self) -> (usize, usize) {
        self.buffer
            .current_line_remain(self.cursor.col, self.cursor.row)
    }

    pub(super) fn measure_selection(&self, s: Selection) -> (usize, usize) {
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
            ForwardWord => {
                let count = self.count_word_forward();
                (cursor_offset, cursor_offset + count)
            }
            BackWord => {
                let count = self.count_word_back();
                (cursor_offset - count, cursor_offset)
            }
            Word => {
                let forward_count = self.count_word_forward();
                let back_count = self.count_word_back();
                (cursor_offset - back_count, cursor_offset + forward_count)
            }
            Line => self.current_line(),
            LineRemain => self.current_line_remain(),
        }
    }
}
