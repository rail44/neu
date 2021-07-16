use crate::action::{Action, ActionKind};
use crate::compute::Reactor;
use crate::edit::{EditKind, EditStore};
use crate::highlight::Highlighter;
use crate::history::{History, Record};
use crate::language::Language;
use crate::mode::{InsertKind, Mode};
use crate::movement::MovementStore;
use crate::renderer::Renderer;
use crate::state::State;

use core::cmp::{max, min};
use flume::Receiver;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::mem;

pub(crate) trait Store {
    fn state(&self) -> &State {
        &self.root().state
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.root_mut().state
    }

    fn highlighter(&self) -> &Highlighter {
        &self.root().highlighter
    }

    fn highlighter_mut(&mut self) -> &mut Highlighter {
        &mut self.root_mut().highlighter
    }

    fn history_mut(&mut self) -> &mut History {
        &mut self.root_mut().history
    }

    fn reactor(&self) -> &Reactor {
        &self.root().reactor
    }

    fn reactor_mut(&mut self) -> &mut Reactor {
        &mut self.root_mut().reactor
    }

    fn root(&self) -> &RootStore;
    fn root_mut(&mut self) -> &mut RootStore;
}

pub(super) struct RootStore {
    pub(crate) state: State,
    renderer: Renderer,
    pub(crate) highlighter: Highlighter,
    rx: Receiver<Action>,
    pub(crate) reactor: Reactor,
    pub(crate) history: History,
}

impl RootStore {
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
        store.refresh();
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
        store.refresh();
        store
    }

    pub(super) async fn run(&mut self) {
        loop {
            let action = smol::block_on(async { self.rx.recv_async().await.unwrap() });
            if !self.action(action) {
                break;
            }
            self.refresh();
        }
    }

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

    pub(crate) fn create_record(&self) -> Record {
        Record {
            buffer: self.state.buffer.clone(),
            cursor: self.state.cursor,
            tree: self.highlighter.tree().cloned(),
        }
    }

    fn refresh(&mut self) {
        self.scroll();
        self.coerce_col();
        self.reactor.load_state(self.state.clone());
        let highlights = self.highlighter.update(&mut self.reactor);
        self.renderer.render(&mut self.reactor, highlights);
    }

    fn edit(&mut self) -> EditStore {
        EditStore::new(self)
    }

    pub(crate) fn movement(&mut self) -> MovementStore {
        MovementStore::new(self)
    }

    pub(crate) fn action(&mut self, action: Action) -> bool {
        use ActionKind::*;
        match action.kind {
            Movement(m) => self.movement().action(m, action.count),
            Edit(e) => self.edit().action(e, action.count),
            IntoNormalMode => {
                let mode = mem::replace(&mut self.state.mode, Mode::Normal(String::new()));

                if let Mode::Insert(k, s) = mode {
                    self.movement().left(1);
                    let edit = match k {
                        InsertKind::Insert(p) => EditKind::InsertString(p, s),
                        InsertKind::Edit(selection) => EditKind::Edit(selection, s),
                    };
                    self.state.prev_edit = Some((edit, 1));
                }
            }
            IntoInsertMode => {
                self.history.push(self.create_record());
                self.state.mode = Mode::Insert(InsertKind::Insert(None), String::new());
            }
            IntoAppendMode => {
                self.history.push(self.create_record());
                self.action(IntoInsertMode.once());
                self.movement().right(1);
            }
            IntoEditMode(selection) => {
                self.history.push(self.create_record());
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
                let range = self.state.measure_selection(selection);
                let yank = self.state.buffer.slice(range).as_str().to_string();
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
            Save => {
                if let Some(p) = &self.state.path {
                    let f = File::create(p).unwrap();
                    let mut w = BufWriter::new(f);
                    write!(w, "{}", self.state.buffer.as_str()).unwrap();
                }
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
