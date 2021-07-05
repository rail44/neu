use super::action::MovementKind;
use crate::compute::{CursorView, MatchPositions};
use crate::store::{RootStore, Store};

use core::cmp::min;

pub(crate) struct MovementStore<'a> {
    root: &'a mut RootStore,
}

impl<'a> Store for MovementStore<'a> {
    fn root(&self) -> &RootStore {
        &self.root
    }

    fn root_mut(&mut self) -> &mut RootStore {
        &mut self.root
    }
}

impl<'a> MovementStore<'a> {
    pub(crate) fn new(root: &'a mut RootStore) -> Self {
        Self { root }
    }

    fn move_col(&mut self, col: usize) {
        self.state_mut().cursor.col = col;
        self.state_mut().max_column = col;
    }

    fn cursor_left(&mut self, count: usize) {
        self.move_col(self.state().cursor.col.saturating_sub(count));
    }

    pub(crate) fn cursor_down(&mut self, count: usize) {
        let state = self.state_mut();
        state.cursor.row += count;
        state.cursor.row = min(
            state.buffer.count_lines().saturating_sub(1),
            state.cursor.row,
        );
        state.cursor.col = state.max_column;
    }

    fn cursor_up(&mut self, count: usize) {
        let state = self.state_mut();
        state.cursor.row = state.cursor.row.saturating_sub(count);
        state.cursor.col = state.max_column;
    }

    pub(crate) fn cursor_right(&mut self, count: usize) {
        self.move_col(self.state().cursor.col + count);
    }

    pub(crate) fn cursor_line_head(&mut self) {
        self.move_col(0);
    }

    pub(crate) fn move_to(&mut self, offset: usize) {
        let result = self.state().buffer.get_position_by_offset(offset);
        self.state_mut().cursor.row = result.row;
        self.move_col(result.col);
    }

    fn forward_word(&mut self, count: usize) {
        let word_offset = self.state().count_word_forward();
        self.cursor_right(word_offset * count);
    }

    fn back_word(&mut self, count: usize) {
        let word_offset = self.state().count_word_back();
        self.cursor_left(word_offset * count);
    }

    fn move_line(&mut self, count: usize) {
        self.state_mut().cursor.row = min(count, self.state().buffer.count_lines()) - 1;
    }

    fn move_to_tail(&mut self) {
        self.state_mut().cursor.row = self.state().buffer.count_lines() - 1;
    }

    fn scroll_screen_up(&mut self) {
        let textarea_row = (self.state().size.1 - 2) as usize;
        self.state_mut().row_offset = self.state().row_offset.saturating_sub(textarea_row);
        self.state_mut().cursor.row = min(
            self.state().cursor.row,
            self.state().row_offset + textarea_row - 1,
        );
    }

    fn scroll_screen_down(&mut self) {
        let textarea_row = (self.state().size.1 - 2) as usize;
        self.state_mut().row_offset += textarea_row;
        self.state_mut().row_offset = min(
            self.state().buffer.count_lines().saturating_sub(1),
            self.state().row_offset,
        );
        self.state_mut().cursor.row = self.state().row_offset;
    }

    fn move_to_line_tail(&mut self) {
        self.move_to(self.state().current_line().1.saturating_sub(2))
    }

    fn move_to_line_indent_head(&mut self) {
        self.move_to(
            self.state()
                .buffer
                .current_line_indent_head(self.state().cursor.row),
        )
    }

    fn move_as_seen_on_view(&mut self) {
        let pos = self.reactor_mut().compute::<CursorView>().0;
        self.state_mut().cursor.row = pos.row;
        self.state_mut().cursor.col = pos.col;
    }

    fn go_to_next_match(&mut self) {
        let matches = self.reactor_mut().compute::<MatchPositions>().0;
        let cursor = &mut self.state_mut().cursor;

        if matches.is_empty() {
            return;
        }

        for (pos, _) in &matches {
            if pos.row == cursor.row && pos.col > cursor.col {
                cursor.row = pos.row;
                cursor.col = pos.col;
                return;
            }

            if pos.row > cursor.row {
                cursor.row = pos.row;
                cursor.col = pos.col;
                return;
            }
        }
        let pos = matches.first().unwrap().0;
        cursor.row = pos.row;
        cursor.col = pos.col;
    }

    fn go_to_prev_match(&mut self) {
        let matches = self.reactor_mut().compute::<MatchPositions>().0;
        let cursor = &mut self.state_mut().cursor;

        if matches.is_empty() {
            return;
        }

        for (pos, _) in matches.iter().rev() {
            if pos.row == cursor.row && pos.col < cursor.col {
                cursor.row = pos.row;
                cursor.col = pos.col;
                return;
            }

            if pos.row < cursor.row {
                cursor.row = pos.row;
                cursor.col = pos.col;
                return;
            }
        }
        let pos = matches.last().unwrap().0;
        cursor.row = pos.row;
        cursor.col = pos.col;
    }

    pub(crate) fn action(&mut self, movement: MovementKind, count: usize) {
        use MovementKind::*;
        match movement {
            CursorLeft => self.cursor_left(count),
            CursorDown => self.cursor_down(count),
            CursorUp => self.cursor_up(count),
            CursorRight => self.cursor_right(count),
            ForwardWord => self.forward_word(count),
            BackWord => self.back_word(count),
            MoveLine => self.move_line(count),
            MoveToTail => self.move_to_tail(),
            ScollScreenUp => self.scroll_screen_up(),
            ScollScreenDown => self.scroll_screen_down(),
            MoveToLineTail => self.move_to_line_tail(),
            MoveToLineIndentHead => self.move_to_line_indent_head(),
            MoveAsSeenOnView => self.move_as_seen_on_view(),
            GoToNextMatch => self.go_to_next_match(),
            GoToPrevMatch => self.go_to_prev_match(),
        }
    }
}
