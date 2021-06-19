use crate::action::{Action, ActionKind, EditKind, MovementKind};
use crate::buffer::Buffer;
use crate::mode::Mode;
use crate::renderer::Renderer;
use crate::state::State;

use core::cmp::min;
use flume::Receiver;
use std::fs::File;
use std::io::{BufWriter, Write};

pub(crate) struct Store {
    state: State,
    renderer: Renderer,
    rx: Receiver<Action>,
}

impl Store {
    pub(crate) fn new(rx: Receiver<Action>, renderer: Renderer) -> Self {
        let mut store = Self {
            rx,
            renderer,
            state: State::new(),
        };
        store.notify();
        store
    }

    pub(crate) fn with_buffer(rx: Receiver<Action>, renderer: Renderer, buffer: Buffer) -> Self {
        let mut store = Self {
            rx,
            renderer,
            state: State::with_buffer(buffer),
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
    fn coerce_cursor(&mut self) {
        let state = &mut self.state;
        let line_count = state.buffer.count_lines();
        if state.cursor.row + state.row_offset > line_count.saturating_sub(2) {
            state.cursor.row = line_count - state.row_offset - 1;
        }

        let textarea_row = (state.size.1 - 3) as usize;
        if state.cursor.row > textarea_row {
            let row_offset = min(
                state.row_offset + state.cursor.row - textarea_row,
                state.buffer.count_lines().saturating_sub(textarea_row + 1),
            );
            state.row_offset = row_offset;
            state.cursor.row = textarea_row;
        }
        let col = min(
            state.cursor.col,
            state
                .buffer
                .row_len(state.cursor.row + state.row_offset)
                .saturating_sub(1),
        );
        state.cursor.col = col;
    }

    fn notify(&mut self) {
        self.coerce_cursor();
        self.renderer.render(&self.state);
    }

    fn movement(&mut self, movement: MovementKind, count: usize) {
        use MovementKind::*;
        match movement {
            CursorLeft => {
                self.state.cursor.col = self.state.cursor.col.saturating_sub(count);
            }
            CursorDown => {
                self.state.cursor.row += count;
            }
            CursorUp => {
                if self.state.cursor.row == 0 {
                    self.state.row_offset = self.state.row_offset.saturating_sub(count);
                    return;
                }
                self.state.cursor.row = self.state.cursor.row.saturating_sub(count);
            }
            CursorRight => {
                self.state.cursor.col += count;
            }
            CursorLineHead => {
                self.state.cursor.col = 0;
            }
            MoveTo(pos) => {
                let result = self.state.buffer.get_cursor_by_offset(pos);
                self.state.cursor.row = result.0 - self.state.row_offset;
                self.state.cursor.col = result.1;
            }
            ForwardWord => {
                let word_offset = self.state.buffer.count_forward_word(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                );
                self.movement(MovementKind::CursorRight, word_offset * count);
            }
            BackWord => {
                let word_offset = self.state.buffer.count_back_word(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                );
                self.movement(MovementKind::CursorLeft, word_offset * count);
            }
            MoveToHead => {
                self.state.cursor.row = 0;
                self.state.row_offset = 0;
            }
            MoveToTail => {
                self.state.cursor.row = self.state.buffer.count_lines();
            }
        }
    }

    fn edit(&mut self, edit: EditKind, count: usize) {
        use EditKind::*;
        self.state.prev_edit = Some((edit.clone(), count));
        match edit {
            RemoveChar => {
                let yank = self.state.buffer.remove_chars(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                    count,
                );
                self.action(ActionKind::SetYank(yank).once());
            }
            Remove(selection) => {
                let (from, to) = self.state.measure_selection(selection);
                let yank = self.state.buffer.remove(from..to);
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
                for _ in 0..count {
                    self.state.buffer.insert(
                        col,
                        self.state.cursor.row + self.state.row_offset,
                        &self.state.yanked,
                    );
                }
            }
            InsertYank => {
                let col = if self.state.yanked.ends_with('\n') {
                    0
                } else {
                    self.state.cursor.col
                };
                for _ in 0..count {
                    self.state.buffer.insert(
                        col,
                        self.state.cursor.row + self.state.row_offset,
                        &self.state.yanked,
                    );
                }
            }
            LineBreak => {
                self.state.buffer.insert_char(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                    '\n',
                );
                self.movement(MovementKind::CursorDown, 1);
                self.movement(MovementKind::CursorLineHead, 1);
            }
            InsertChar(c) => {
                self.state.buffer.insert_char(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                    c,
                );
                self.movement(MovementKind::CursorRight, 1);
            }
        }
    }

    fn action(&mut self, action: Action) -> bool {
        use ActionKind::*;
        match action.kind {
            Movement(m) => self.movement(m, action.count),
            Edit(e) => self.edit(e, action.count),
            IntoNormalMode => {
                self.state.mode = Mode::Normal(String::new());
            }
            IntoInsertMode => {
                self.state.mode = Mode::Insert;
            }
            IntoAppendMode => {
                self.movement(MovementKind::CursorRight, 1);
                self.action(IntoInsertMode.once());
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
                Mode::Insert => (),
                Mode::CmdLine(cmd) => {
                    cmd.clear();
                }
            },
            PushCmd(c) => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.push(c);
                }
                Mode::Insert => (),
                Mode::CmdLine(cmd) => {
                    cmd.push(c);
                }
            },
            PushCmdStr(s) => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.push_str(&s);
                }
                Mode::Insert => (),
                Mode::CmdLine(cmd) => {
                    cmd.push_str(&s);
                }
            },
            PopCmd => match &mut self.state.mode {
                Mode::Normal(cmd) => {
                    cmd.pop();
                }
                Mode::Insert => (),
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
        };
        true
    }
}
