use crate::action::{Action, ActionKind, EditKind, MovementKind};
use crate::compute::{CursorView, MatchPositions, Reactor};
use crate::edit::Store as EditStore;
use crate::highlight::Highlighter;
use crate::history::{History, Record};
use crate::language::Language;
use crate::mode::{InsertKind, Mode};
use crate::renderer::Renderer;
use crate::state::State;

use core::cmp::{max, min};
use flume::Receiver;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::mem;

pub(super) struct Store {
    pub(crate) state: State,
    renderer: Renderer,
    pub(crate) highlighter: Highlighter,
    rx: Receiver<Action>,
    pub(crate) reactor: Reactor,
    pub(crate) history: History,
}

impl Store {
    pub(super) fn new(rx: Receiver<Action>, renderer: Renderer) -> Self {
        let state = State::new();
        let highlighter = Highlighter::new(&state.buffer, &Language::Unknown);

        let mut store = Self {
            rx,
            renderer,
            highlighter,
            state,
            history: History::default(),
            reactor: Reactor::new(),
        };
        store.notify();
        store
    }

    pub(super) fn open_file(filename: &str, rx: Receiver<Action>, renderer: Renderer) -> Self {
        let state = State::open_file(filename);
        let lang = Language::from_path(filename);
        let highlighter = Highlighter::new(&state.buffer, &lang);

        let mut store = Self {
            rx,
            renderer,
            highlighter,
            history: History::default(),
            reactor: Reactor::new(),
            state,
        };
        store.notify();
        store
    }

    pub(super) async fn run(&mut self) {
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

    fn create_record(&self) -> Record {
        Record {
            buffer: self.state.buffer.clone(),
            cursor: self.state.cursor.clone(),
            tree: self.highlighter.tree().cloned(),
        }
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

    fn edit(&mut self) -> EditStore {
        EditStore::new(self)
    }

    pub(crate) fn movement(&mut self, movement: MovementKind, count: usize) {
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
            MoveAsSeenOnView => {
                let pos = self.reactor.compute::<CursorView>().0;
                self.state.cursor.row = pos.0;
                self.state.cursor.col = pos.1;
            }
            GoToNextMatch => {
                let matches = self.reactor.compute::<MatchPositions>().0;
                let cursor = &mut self.state.cursor;

                if matches.is_empty() {
                    return;
                }

                for (pos, _) in &matches {
                    if pos.0 == cursor.row && pos.1 > cursor.col {
                        cursor.row = pos.0;
                        cursor.col = pos.1;
                        return;
                    }

                    if pos.0 > cursor.row {
                        cursor.row = pos.0;
                        cursor.col = pos.1;
                        return;
                    }
                }
                let pos = matches.first().unwrap().0;
                cursor.row = pos.0;
                cursor.col = pos.1;
            }
            GoToPrevMatch => {
                let matches = self.reactor.compute::<MatchPositions>().0;
                let cursor = &mut self.state.cursor;

                if matches.is_empty() {
                    return;
                }

                for (pos, _) in matches.iter().rev() {
                    if pos.0 == cursor.row && pos.1 < cursor.col {
                        cursor.row = pos.0;
                        cursor.col = pos.1;
                        return;
                    }

                    if pos.0 < cursor.row {
                        cursor.row = pos.0;
                        cursor.col = pos.1;
                        return;
                    }
                }
                let pos = matches.last().unwrap().0;
                cursor.row = pos.0;
                cursor.col = pos.1;
            }
        }
    }

    pub(crate) fn action(&mut self, action: Action) -> bool {
        use ActionKind::*;
        match action.kind {
            Movement(m) => self.movement(m, action.count),
            Edit(e) => self.edit().action(e, action.count),
            IntoNormalMode => {
                let mode = mem::replace(&mut self.state.mode, Mode::Normal(String::new()));

                if let Mode::Insert(k, s) = mode {
                    let edit = match k {
                        InsertKind::Insert => EditKind::InsertString(s),
                        InsertKind::Edit(selection) => EditKind::Edit(selection, s),
                    };
                    self.state.prev_edit = Some((edit, 1));
                }
            }
            IntoInsertMode => {
                self.history.push(self.create_record());
                self.state.mode = Mode::Insert(InsertKind::Insert, String::new());
            }
            IntoAppendMode => {
                self.history.push(self.create_record());
                self.action(IntoInsertMode.once());
                self.movement(MovementKind::CursorRight, 1);
            }
            IntoEditMode(selection) => {
                self.edit().remove_selection(&selection, 1);
                self.state.mode = Mode::Insert(InsertKind::Edit(selection), String::new());
            }
            IntoCmdLineMode => {
                self.state.mode = Mode::CmdLine(String::new());
            }
            IntoSearchMode => {
                self.action(ClearSearch.once());
                self.state.mode = Mode::Search;
            }
            SetYank(b) => {
                self.state.yanked = b;
            }
            ClearCmd => match &mut self.state.mode {
                Mode::Normal(cmd) | Mode::CmdLine(cmd) => {
                    cmd.clear();
                }
                _ => (),
            },
            PushCmd(c) => match &mut self.state.mode {
                Mode::Normal(cmd) | Mode::CmdLine(cmd) => {
                    cmd.push(c);
                }
                _ => (),
            },
            PushCmdStr(s) => match &mut self.state.mode {
                Mode::Normal(cmd) | Mode::CmdLine(cmd) => {
                    cmd.push_str(&s);
                }
                _ => (),
            },
            PopCmd => match &mut self.state.mode {
                Mode::Normal(cmd) | Mode::CmdLine(cmd) => {
                    cmd.pop();
                }
                _ => (),
            },
            Yank(selection) => {
                let (from, to) = self.state.measure_selection(selection);
                let yank = self.state.buffer.slice(from..to).as_str().to_string();
                self.action(SetYank(yank).once());
            }
            Repeat => {
                if let Some((edit, count)) = self.state.prev_edit.clone() {
                    self.edit().action(edit, count);
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
                if let Some(record) = self.history.undo(self.create_record(), action.count) {
                    self.state.cursor = record.cursor;
                    self.state.buffer = record.buffer;
                    if let Some(tree) = record.tree {
                        self.highlighter.set_tree(tree);
                    }
                }
            }
            Redo => {
                if let Some(record) = self.history.redo(self.create_record(), action.count) {
                    self.state.cursor = record.cursor;
                    self.state.buffer = record.buffer;
                    if let Some(tree) = record.tree {
                        self.highlighter.set_tree(tree);
                    }
                }
            }
            PushSearch(c) => {
                self.state.search_pattern.push(c);
            }
            PopSearch => {
                self.state.search_pattern.pop();
            }
            ClearSearch => {
                self.state.search_pattern.clear();
            }
        };
        true
    }
}
