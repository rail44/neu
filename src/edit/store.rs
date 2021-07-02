use crate::action::{ActionKind, EditKind, MovementKind};
use crate::highlight::Highlighter;
use crate::history::{History, Record};
use crate::mode::Mode;
use crate::state::State;
use crate::store::Store as RootStore;
use tree_sitter::{InputEdit, Point};

pub(crate) struct Store<'a> {
    root: &'a mut RootStore,
}

impl<'a> Store<'a> {
    pub(crate) fn new(root: &'a mut RootStore) -> Self {
        Self { root }
    }

    fn state(&self) -> &State {
        &self.root.state
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.root.state
    }

    fn highlighter(&self) -> &Highlighter {
        &self.root.highlighter
    }

    fn highlighter_mut(&mut self) -> &mut Highlighter {
        &mut self.root.highlighter
    }

    fn history(&self) -> &History {
        &self.root.history
    }

    fn history_mut(&mut self) -> &mut History {
        &mut self.root.history
    }

    fn root(&self) -> &RootStore {
        &self.root
    }

    fn root_mut(&mut self) -> &mut RootStore {
        &mut self.root
    }

    fn create_record(&self) -> Record {
        Record {
            buffer: self.state().buffer.clone(),
            cursor: self.state().cursor.clone(),
            tree: self.highlighter().tree().cloned(),
        }
    }

    fn insert(&mut self, to: usize, s: &str) {
        let (row, col) = self.state().buffer.get_cursor_by_offset(to);
        let byte_l = s.bytes().count();
        let edit = InputEdit {
            start_byte: to,
            old_end_byte: to,
            new_end_byte: to + byte_l,
            start_position: Point::default(),
            old_end_position: Point::default(),
            new_end_position: Point::default(),
        };
        self.highlighter_mut().edit_tree(&edit);
        self.state_mut().buffer.insert(col, row, s);
    }

    fn remove(&mut self, from: usize, count: usize) -> String {
        let to = from + count;
        let s = self.state_mut().buffer.remove(from..to);

        let byte_l = s.bytes().count();
        let edit = InputEdit {
            start_byte: from,
            old_end_byte: from + byte_l,
            new_end_byte: from,
            start_position: Point::default(),
            old_end_position: Point::default(),
            new_end_position: Point::default(),
        };
        self.highlighter_mut().edit_tree(&edit);
        s
    }

    pub(crate) fn action(&mut self, edit: EditKind, count: usize) {
        use EditKind::*;
        let record = self.create_record();
        self.history_mut().push(record);
        match &edit {
            RemoveChar => {
                let cursor = &self.state().cursor;
                let start = self
                    .state()
                    .buffer
                    .get_offset_by_cursor(cursor.col, cursor.row);
                let yank = self.remove(start, count);
                self.root_mut().action(ActionKind::SetYank(yank).once());
            }
            Remove(selection) => {
                let (from, to) = self.state().measure_selection(selection.clone());
                let yank = self.remove(from, to - from);
                self.root_mut().action(ActionKind::SetYank(yank).once());
                self.root_mut().movement(MovementKind::MoveTo(from), 1);
            }
            AppendYank => {
                let col = if self.state().yanked.ends_with('\n') {
                    self.root_mut().movement(MovementKind::CursorDown, 1);
                    0
                } else {
                    self.root_mut().movement(MovementKind::CursorRight, 1);
                    self.state().cursor.col
                };

                let to = self
                    .state()
                    .buffer
                    .get_offset_by_cursor(col, self.state().cursor.row);

                let s = self.state().yanked.clone();
                for _ in 0..count {
                    self.insert(to, &s);
                }
            }
            InsertYank => {
                let col = if self.state().yanked.ends_with('\n') {
                    0
                } else {
                    self.state().cursor.col
                };

                let to = self
                    .state()
                    .buffer
                    .get_offset_by_cursor(col, self.state().cursor.row);
                let s = self.state().yanked.clone();
                for _ in 0..count {
                    self.insert(to, &s);
                }
            }
            LineBreak => {
                let to = self
                    .state()
                    .buffer
                    .get_offset_by_cursor(self.state().cursor.col, self.state().cursor.row);
                self.insert(to, "\n");
                if let Mode::Insert(_, s) = &mut self.state_mut().mode {
                    s.push('\n');
                }
                self.root_mut().movement(MovementKind::CursorDown, 1);
                self.root_mut().movement(MovementKind::CursorLineHead, 1);
            }
            InsertChar(c) => {
                let to = self
                    .state()
                    .buffer
                    .get_offset_by_cursor(self.state().cursor.col, self.state().cursor.row);
                if let Mode::Insert(_, s) = &mut self.state_mut().mode {
                    s.push(*c);
                }
                self.insert(to, &c.to_string());
                self.root_mut().movement(MovementKind::CursorRight, 1);
                self.history_mut().pop();
            }
            Insert(s) => {
                let to = self
                    .state()
                    .buffer
                    .get_offset_by_cursor(self.state().cursor.col, self.state().cursor.row);
                self.insert(to, &s);
                self.root_mut()
                    .movement(MovementKind::CursorRight, s.chars().count());
                self.history_mut().pop();
            }
            Edit(selection, s) => {
                self.action(EditKind::Remove(selection.clone()), 1);
                self.history_mut().pop();
                self.action(EditKind::Insert(s.clone()), 1);
            }
        };
        self.state_mut().prev_edit = Some((edit, count));
    }
}
