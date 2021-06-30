use crate::action::{Action, ActionKind, EditKind, MovementKind};
use crate::buffer::Buffer;
use crate::compute::Reactor;
use crate::highlight::Highlighter;
use crate::language::Language;
use crate::mode::{InsertKind, Mode};
use crate::renderer::Renderer;
use crate::state::{Cursor, State};

use core::cmp::{max, min};
use flume::Receiver;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::mem;
use tree_sitter::{InputEdit, Point, Tree};

pub(crate) struct Store {
    state: State,
    renderer: Renderer,
    highlighter: Highlighter,
    rx: Receiver<Action>,
    reactor: Reactor,
    history: Vec<(Buffer, Cursor, Tree)>,
}

impl Store {
    pub(crate) fn new(rx: Receiver<Action>, renderer: Renderer) -> Self {
        let state = State::new();
        let highlighter = Highlighter::new(&state.buffer, &Language::Unknown);

        let mut store = Self {
            rx,
            renderer,
            highlighter,
            state,
            history: Vec::new(),
            reactor: Reactor::new(),
        };
        store.notify();
        store
    }

    pub(crate) fn open_file(filename: &str, rx: Receiver<Action>, renderer: Renderer) -> Self {
        let state = State::open_file(filename);
        let lang = Language::from_path(filename);
        let highlighter = Highlighter::new(&state.buffer, &lang);

        let mut store = Self {
            rx,
            renderer,
            highlighter,
            history: Vec::new(),
            reactor: Reactor::new(),
            state,
        };
        store.notify();
        store
    }

    pub(crate) async fn run(&mut self) {
        loop {
            let action = smol::block_on(async { self.rx.recv_async().await.unwrap() });
            if !self.action(action) {
                break;
            }
            self.notify();
        }
    }
}

impl Store {
    fn scroll(&mut self) {
        let state = &mut self.state;
        let textarea_row = (state.size.1 - 2) as usize;

        state.row_offset = max(
            min(state.cursor.row, state.row_offset),
            (state.cursor.row + 1).saturating_sub(textarea_row),
        );
    }

    fn coerce_col(&mut self) {
        let max_col = if self.state.mode.is_insert() {
            self.state.buffer.row_len(self.state.cursor.row)
        } else {
            self.state
                .buffer
                .row_len(self.state.cursor.row)
                .saturating_sub(1)
        };

        self.state.cursor.col = min(self.state.cursor.col, max_col);
    }

    fn notify(&mut self) {
        self.scroll();
        self.coerce_col();
        self.reactor.load_state(self.state.clone());
        let highlights = self.highlighter.update(&mut self.reactor);
        self.renderer.render(&mut self.reactor, highlights);
    }

    fn move_col(&mut self, col: usize) {
        self.state.cursor.col = col;
        self.state.max_column = col;
    }

    fn movement(&mut self, movement: MovementKind, count: usize) {
        let state = &mut self.state;
        use MovementKind::*;
        match movement {
            CursorLeft => {
                self.move_col(self.state.cursor.col.saturating_sub(count));
            }
            CursorDown => {
                state.cursor.row += count;
                state.cursor.row = min(
                    state.buffer.count_lines().saturating_sub(1),
                    state.cursor.row,
                );
                state.cursor.col = state.max_column;
            }
            CursorUp => {
                state.cursor.row = state.cursor.row.saturating_sub(count);
                state.cursor.col = state.max_column;
            }
            CursorRight => {
                self.move_col(self.state.cursor.col + count);
            }
            CursorLineHead => {
                self.move_col(0);
            }
            MoveTo(pos) => {
                let result = state.buffer.get_cursor_by_offset(pos);
                state.cursor.row = result.0;
                self.move_col(result.1);
            }
            ForwardWord => {
                let word_offset = state
                    .buffer
                    .count_forward_word(state.cursor.col, state.cursor.row);
                self.movement(MovementKind::CursorRight, word_offset * count);
            }
            BackWord => {
                let word_offset = state
                    .buffer
                    .count_back_word(state.cursor.col, state.cursor.row);
                self.movement(MovementKind::CursorLeft, word_offset * count);
            }
            MoveLine => {
                state.cursor.row = min(count, state.buffer.count_lines()) - 1;
            }
            MoveToTail => {
                state.cursor.row = state.buffer.count_lines() - 1;
            }
            ScollScreenUp => {
                let textarea_row = (state.size.1 - 2) as usize;
                state.row_offset = state.row_offset.saturating_sub(textarea_row);
                state.cursor.row = min(state.cursor.row, state.row_offset + textarea_row - 1);
            }
            ScollScreenDown => {
                let textarea_row = (state.size.1 - 2) as usize;
                state.row_offset += textarea_row;
                state.row_offset = min(
                    state.buffer.count_lines().saturating_sub(1),
                    state.row_offset,
                );
                state.cursor.row = state.row_offset;
            }
            MoveToLineTail => {
                self.movement(
                    MovementKind::MoveTo(self.state.current_line().1.saturating_sub(2)),
                    count,
                );
            }
            MoveToLineIndentHead => {
                self.movement(
                    MovementKind::MoveTo(
                        self.state
                            .buffer
                            .current_line_indent_head(self.state.cursor.row),
                    ),
                    count,
                );
            }
        }
    }

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
        self.history.push((
            self.state.buffer.clone(),
            self.state.cursor.clone(),
            self.highlighter.tree().clone(),
        ));
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
            }
            Insert(s) => {
                let to = self
                    .state
                    .buffer
                    .get_offset_by_cursor(self.state.cursor.col, self.state.cursor.row);
                self.insert(to, &s);
                self.movement(MovementKind::CursorRight, s.chars().count());
            }
            Edit(selection, s) => {
                self.edit(EditKind::Remove(selection.clone()), 1);
                self.edit(EditKind::Insert(s.clone()), 1);
            }
        };
        self.state.prev_edit = Some((edit, count));
    }

    fn action(&mut self, action: Action) -> bool {
        use ActionKind::*;
        match action.kind {
            Movement(m) => self.movement(m, action.count),
            Edit(e) => self.edit(e, action.count),
            IntoNormalMode => {
                let mode = mem::replace(&mut self.state.mode, Mode::Normal(String::new()));

                if let Mode::Insert(k, s) = mode {
                    let edit = match k {
                        InsertKind::Insert => EditKind::Insert(s),
                        InsertKind::Edit(selection) => EditKind::Edit(selection, s),
                    };
                    self.state.prev_edit = Some((edit, 1));
                }
            }
            IntoInsertMode => {
                self.state.mode = Mode::Insert(InsertKind::Insert, String::new());
            }
            IntoAppendMode => {
                self.action(IntoInsertMode.once());
                self.movement(MovementKind::CursorRight, 1);
            }
            IntoEditMode(selection) => {
                self.edit(EditKind::Remove(selection.clone()), 1);
                self.state.mode = Mode::Insert(InsertKind::Edit(selection), String::new());
            }
            IntoCmdLineMode => {
                self.state.mode = Mode::CmdLine(String::new());
            }
            SetYank(b) => {
                self.state.yanked = b;
            }
            ClearCmd => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.clear();
                }
                Mode::Insert(_, _) => (),
                Mode::CmdLine(cmd) => {
                    cmd.clear();
                }
            },
            PushCmd(c) => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.push(c);
                }
                Mode::Insert(_, _) => (),
                Mode::CmdLine(cmd) => {
                    cmd.push(c);
                }
            },
            PushCmdStr(s) => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.push_str(&s);
                }
                Mode::Insert(_, _) => (),
                Mode::CmdLine(cmd) => {
                    cmd.push_str(&s);
                }
            },
            PopCmd => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.pop();
                }
                Mode::Insert(_, _) => (),
                Mode::CmdLine(cmd) => {
                    cmd.pop();
                }
            },
            Yank(selection) => {
                let (from, to) = self.state.measure_selection(selection);
                let yank = self.state.buffer.slice(from..to).as_str().to_string();
                self.action(SetYank(yank).once());
            }
            Repeat => {
                if let Some((edit, count)) = self.state.prev_edit.clone() {
                    self.edit(edit, count);
                }
            }
            Quit => {
                return false;
            }
            WriteOut(filename) => {
                let f = File::create(filename).unwrap();
                let mut w = BufWriter::new(f);
                write!(w, "{}", self.state.buffer.as_str()).unwrap();
            }
            GetState(tx) => {
                tx.send(self.state.clone()).unwrap();
            }
            Undo => {
                if let Some((b, c, t)) = self.history.pop() {
                    self.state.cursor = c;
                    self.state.buffer = b;
                    self.highlighter.set_tree(t);
                }
            }
        };
        true
    }
}
