use crate::action::{EditKind, Selection, SelectionKind};
use crate::buffer::Buffer;
use crate::mode::Mode;

use termion::terminal_size;

#[derive(Default, Clone, Debug, PartialEq)]
pub(crate) struct Cursor {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub(crate) struct State {
    pub(crate) row_offset: usize,
    pub(crate) cursor: Cursor,
    pub(crate) mode: Mode,
    pub(crate) yanked: String,
    pub(crate) size: (u16, u16),
    pub(crate) buffer: Buffer,
    pub(crate) prev_edit: Option<(EditKind, usize)>,
}

impl State {
    pub(crate) fn new() -> Self {
        let size = terminal_size().unwrap();

        Self {
            size,
            ..Default::default()
        }
    }

    pub(crate) fn with_buffer(buffer: Buffer) -> Self {
        let size = terminal_size().unwrap();

        Self {
            size,
            buffer,
            ..Default::default()
        }
    }

    pub(crate) fn get_cursor_offset(&self) -> usize {
        self.buffer
            .get_offset_by_cursor(self.cursor.col, self.cursor.row)
    }

    pub(crate) fn count_word_back(&self) -> usize {
        self.buffer
            .count_back_word(self.cursor.col, self.cursor.row)
    }

    pub(crate) fn count_word_forward(&self) -> usize {
        self.buffer
            .count_forward_word(self.cursor.col, self.cursor.row)
    }

    pub(crate) fn current_line(&self) -> (usize, usize) {
        self.buffer.current_line(self.cursor.row)
    }

    pub(crate) fn current_line_remain(&self) -> (usize, usize) {
        self.buffer
            .current_line_remain(self.cursor.col, self.cursor.row)
    }

    pub(crate) fn measure_selection(&self, s: Selection) -> (usize, usize) {
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
