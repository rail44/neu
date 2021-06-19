use crate::action::{Action, ActionKind, EditKind, MovementKind};
use crate::buffer::Buffer;
use crate::mode::Mode;
use crate::renderer::Renderer;
use crate::state::State;

use core::cmp::{max, min};
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
    fn scroll(&mut self) {
        let state = &mut self.state;
        let textarea_row = (state.size.1 - 2) as usize;

        state.row_offset = max(
            min(state.cursor.row, state.row_offset),
            (state.cursor.row + 1).saturating_sub(textarea_row),
        );
    }

    fn notify(&mut self) {
        self.scroll();
        self.renderer.render(&self.state);
    }

    fn movement(&mut self, movement: MovementKind, count: usize) {
        let state = &mut self.state;
        use MovementKind::*;
        match movement {
            CursorLeft => {
                state.cursor.col = state.cursor.col.saturating_sub(count);
            }
            CursorDown => {
                state.cursor.row += count;
                state.cursor.row = min(
                    state.buffer.count_lines().saturating_sub(1),
                    state.cursor.row,
                );
            }
            CursorUp => {
                state.cursor.row = state.cursor.row.saturating_sub(count);
            }
            CursorRight => {
                state.cursor.col += count;

                state.cursor.col = min(
                    state.cursor.col,
                    state.buffer.row_len(state.cursor.row).saturating_sub(1),
                );
            }
            CursorLineHead => {
                state.cursor.col = 0;
            }
            MoveTo(pos) => {
                let result = state.buffer.get_cursor_by_offset(pos);
                state.cursor.row = result.0;
                state.cursor.col = result.1;
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
                state.cursor.row = state.row_offset + textarea_row;
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
                    MovementKind::MoveTo(self.state.current_line().1.saturating_sub(1)),
                    count,
                );
            }
            MoveToLineIndentHead => {
                self.movement(
                        MovementKind::MoveTo(self.state.buffer.current_line_indent_head(self.state.cursor.row)),
                    count,
                );
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
                    self.state.cursor.row,
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
                    self.state
                        .buffer
                        .insert(col, self.state.cursor.row, &self.state.yanked);
                }
            }
            InsertYank => {
                let col = if self.state.yanked.ends_with('\n') {
                    0
                } else {
                    self.state.cursor.col
                };
                for _ in 0..count {
                    self.state
                        .buffer
                        .insert(col, self.state.cursor.row, &self.state.yanked);
                }
            }
            LineBreak => {
                self.state
                    .buffer
                    .insert_char(self.state.cursor.col, self.state.cursor.row, '\n');
                self.movement(MovementKind::CursorDown, 1);
                self.movement(MovementKind::CursorLineHead, 1);
            }
            InsertChar(c) => {
                self.state
                    .buffer
                    .insert_char(self.state.cursor.col, self.state.cursor.row, c);
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
