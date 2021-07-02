use tree_sitter::{InputEdit, Point};
use crate::state::State;
use crate::highlight::Highlighter;

pub(super) struct EditStore<'a> {
    state: &'a mut State,
    highlighter: &'a mut Highlighter,
}

impl<'a> EditStore<'a> {
    fn insert(&mut self, to: usize, s: &str) {
        let (row, col) = self.state.buffer.get_cursor_by_offset(to);
        let byte_l = s.bytes().count();
        let edit = InputEdit {
            start_byte: to,
            old_end_byte: to,
            new_end_byte: to + byte_l,
            start_position: Point::default(),
            old_end_position: Point::default(),
            new_end_position: Point::default(),
        };
        self.highlighter.edit_tree(&edit);
        self.state.buffer.insert(col, row, s);
    }

    fn remove(&mut self, from: usize, count: usize) -> String {
        let to = from + count;
        let s = self.state.buffer.remove(from..to);

        let byte_l = s.bytes().count();
        let edit = InputEdit {
            start_byte: from,
            old_end_byte: from + byte_l,
            new_end_byte: from,
            start_position: Point::default(),
            old_end_position: Point::default(),
            new_end_position: Point::default(),
        };
        self.highlighter.edit_tree(&edit);
        s
    }

    fn edit(&mut self, edit: EditKind, count: usize) {
        use EditKind::*;
        self.history.push(self.create_record());
        match &edit {
            RemoveChar => {
                let cursor = &self.state.cursor;
                let start = self
                    .state
                    .buffer
                    .get_offset_by_cursor(cursor.col, cursor.row);
                let yank = self.remove(start, count);
                self.action(ActionKind::SetYank(yank).once());
            }
            Remove(selection) => {
                let (from, to) = self.state.measure_selection(selection.clone());
                let yank = self.remove(from, to - from);
                self.action(ActionKind::SetYank(yank).once());
                self.movement(MovementKind::MoveTo(from), 1);
            }
            AppendYank => {
                let col = if self.state.yanked.ends_with('\n') {
                    self.movement(MovementKind::CursorDown, 1);
                    0
                } else {
                    self.movement(MovementKind::CursorRight, 1);
                    self.state.cursor.col
                };

                let to = self
                    .state
                    .buffer
                    .get_offset_by_cursor(col, self.state.cursor.row);

                let s = self.state.yanked.clone();
                for _ in 0..count {
                    self.insert(to, &s);
                }
            }
            InsertYank => {
                let col = if self.state.yanked.ends_with('\n') {
                    0
                } else {
                    self.state.cursor.col
                };

                let to = self
                    .state
                    .buffer
                    .get_offset_by_cursor(col, self.state.cursor.row);
                let s = self.state.yanked.clone();
                for _ in 0..count {
                    self.insert(to, &s);
                }
            }
            LineBreak => {
                let to = self
                    .state
                    .buffer
                    .get_offset_by_cursor(self.state.cursor.col, self.state.cursor.row);
                self.insert(to, "\n");
                if let Mode::Insert(_, s) = &mut self.state.mode {
                    s.push('\n');
                }
                self.movement(MovementKind::CursorDown, 1);
                self.movement(MovementKind::CursorLineHead, 1);
            }
            InsertChar(c) => {
                let to = self
                    .state
                    .buffer
                    .get_offset_by_cursor(self.state.cursor.col, self.state.cursor.row);
                if let Mode::Insert(_, s) = &mut self.state.mode {
                    s.push(*c);
                }
                self.insert(to, &c.to_string());
                self.movement(MovementKind::CursorRight, 1);
                self.history.pop();
            }
            Insert(s) => {
                let to = self
                    .state
                    .buffer
                    .get_offset_by_cursor(self.state.cursor.col, self.state.cursor.row);
                self.insert(to, &s);
                self.movement(MovementKind::CursorRight, s.chars().count());
                self.history.pop();
            }
            Edit(selection, s) => {
                self.edit(EditKind::Remove(selection.clone()), 1);
                self.history.pop();
                self.edit(EditKind::Insert(s.clone()), 1);
            }
        };
        self.state.prev_edit = Some((edit, count));
    }
}
