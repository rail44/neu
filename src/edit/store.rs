use super::action::EditKind;
use crate::action::ActionKind;
use crate::mode::Mode;
use crate::position::Position;
use crate::selection::Selection;
use crate::store::{RootStore, Store};
use core::ops::Range;
use tree_sitter::{InputEdit, Point};

pub(crate) struct EditStore<'a> {
    root: &'a mut RootStore,
}

impl<'a> Store for EditStore<'a> {
    fn root(&self) -> &RootStore {
        &self.root
    }

    fn root_mut(&mut self) -> &mut RootStore {
        &mut self.root
    }
}

impl<'a> EditStore<'a> {
    pub(crate) fn new(root: &'a mut RootStore) -> Self {
        Self { root }
    }

    fn insert(&mut self, to: usize, s: &str) {
        let pos = self.state().buffer.get_position_by_offset(to);
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
        self.state_mut().buffer.insert(pos, s);
    }

    fn remove(&mut self, range: Range<usize>) -> String {
        let start = range.start;
        let s = self.state_mut().buffer.remove(range);

        let byte_l = s.bytes().count();
        let edit = InputEdit {
            start_byte: start,
            old_end_byte: start + byte_l,
            new_end_byte: start,
            start_position: Point::default(),
            old_end_position: Point::default(),
            new_end_position: Point::default(),
        };
        self.highlighter_mut().edit_tree(&edit);
        s
    }

    pub(crate) fn remove_char(&mut self, count: usize) {
        let start = self.state().get_cursor_offset();
        let yank = self.remove(start..start + count);
        self.root_mut().action(ActionKind::SetYank(yank).once());
    }

    pub(crate) fn remove_selection(&mut self, selection: &Selection, count: usize) {
        for _ in 0..count {
            let range = self.state().measure_selection(selection.clone());
            let start = range.start;
            let yank = self.remove(range);
            self.root_mut().action(ActionKind::SetYank(yank).once());
            self.root_mut().movement().offset(start);
        }
    }

    pub(crate) fn append_yank(&mut self, count: usize) {
        let col = if self.state().yanked.ends_with('\n') {
            self.root_mut().movement().down(1);
            0
        } else {
            self.root_mut().movement().right(1);
            self.state().cursor.col
        };

        let to = self.state().buffer.get_offset_by_position(Position {
            col,
            row: self.state().cursor.row,
        });

        let s = self.state().yanked.clone();
        for _ in 0..count {
            self.insert(to, &s);
        }
    }

    pub(crate) fn insert_yank(&mut self, count: usize) {
        let col = if self.state().yanked.ends_with('\n') {
            0
        } else {
            self.state().cursor.col
        };

        let to = self.state().buffer.get_offset_by_position(Position {
            col,
            row: self.state().cursor.row,
        });
        let s = self.state().yanked.clone();
        for _ in 0..count {
            self.insert(to, &s);
        }
    }

    pub(crate) fn line_break(&mut self, count: usize) {
        let to = self.state().get_cursor_offset();

        for _ in 0..count {
            self.insert(to, "\n");
            if let Mode::Insert(_, s) = &mut self.state_mut().mode {
                s.push('\n');
            }
            self.root_mut().movement().down(1);
            self.root_mut().movement().line_head();
        }
    }

    pub(crate) fn insert_char(&mut self, c: char, count: usize) {
        let to = self.state().get_cursor_offset();
        for _ in 0..count {
            if let Mode::Insert(_, s) = &mut self.state_mut().mode {
                s.push(c);
            }
            self.insert(to, &c.to_string());
            self.root_mut().movement().right(1);
        }
        self.history_mut().pop();
    }

    pub(crate) fn insert_string(&mut self, s: &str, count: usize) {
        let to = self.state().get_cursor_offset();
        for _ in 0..count {
            self.insert(to, &s);
            self.root_mut().movement().right(s.chars().count());
        }
    }

    pub(crate) fn edit(&mut self, selection: &Selection, s: &str) {
        self.remove_selection(selection, 1);
        self.insert_string(s, 1);
        self.root_mut().movement().left(1);
    }

    pub(crate) fn action(&mut self, edit: EditKind, count: usize) {
        use EditKind::*;
        let record = self.root().create_record();
        self.history_mut().push(record);
        match &edit {
            RemoveChar => self.remove_char(count),
            RemoveSelection(selection) => self.remove_selection(&selection, count),
            AppendYank => self.append_yank(count),
            InsertYank => self.insert_yank(count),
            LineBreak => self.line_break(count),
            InsertChar(c) => self.insert_char(*c, count),
            InsertString(m, s) => {
                if let Some(m) = m {
                    self.root_mut().movement().action(m.clone(), 1);
                }
                self.insert_string(s, count)
            }
            Edit(selection, s) => self.edit(selection, s),
        };
        self.state_mut().prev_edit = Some((edit, count));
    }
}
