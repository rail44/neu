use crate::action::{Action, ActionKind};
use crate::buffer::Buffer;
use crate::renderer;
use crate::renderer::Renderer;
use crate::state::{Mode, State};

use core::cmp::min;
use xtra::prelude::*;

pub(crate) struct Store {
    state: State,
    renderer: Address<Renderer>,
}

impl Store {
    pub(crate) async fn new(renderer: Address<Renderer>) -> Self {
        let mut actor = Self {
            renderer,
            state: State::new(),
        };
        actor.notify();
        actor
    }

    pub(crate) async fn set_buffer(&mut self, buffer: Buffer) {
        self.state.buffer = buffer;
        self.notify();
    }
}

impl Actor for Store {}

impl Store {
    fn coerce_cursor(&mut self) {
        let row = min(
            self.state.cursor.row,
            self.state.buffer.count_lines()
        );
        self.state.cursor.row = row;

        let textarea_row = (self.state.size.1 - 3) as usize;
        let actual_row = textarea_row - self.wrap_offset();
        if self.state.cursor.row > actual_row {
            let row_offset = min(
                self.state.row_offset + self.state.cursor.row - actual_row,
                self.state.buffer.count_lines().saturating_sub(actual_row),
            );
            self.state.row_offset = row_offset;
            self.state.cursor.row = actual_row;
        }
        let col = min(
            self.state.cursor.col,
            self.state
                .buffer
                .row_len(self.state.cursor.row + self.state.row_offset),
        );
        self.state.cursor.col = col;
    }

    fn wrap_offset(&self) -> usize {
        let mut wraps = 0;
        let mut lines_count = 0;
        for line in self.state.buffer.lines().skip(self.state.row_offset) {
            let wrap = (line.len() as u16) / self.state.size.0;
            wraps += wrap;
            lines_count += 1 + wrap;
            if lines_count >= self.state.size.1 - 2 {
                break;
            }
        }
        wraps as usize
    }

    fn notify(&mut self) {
        self.coerce_cursor();
        self.renderer
            .do_send(renderer::Render(self.state.clone()))
            .unwrap()
    }

    fn action(&mut self, action: Action) {
        use ActionKind::*;
        match action.kind {
            CursorLeft => {
                self.state.cursor.col = self.state.cursor.col.saturating_sub(action.count);
            }
            CursorDown => {
                self.state.cursor.row += action.count;
            }
            CursorUp => {
                if self.state.cursor.row == 0 {
                    self.state.row_offset = self.state.row_offset.saturating_sub(action.count);
                    return;
                }
                self.state.cursor.row = self.state.cursor.row.saturating_sub(action.count);
            }
            CursorRight => {
                self.state.cursor.col += action.count;
            }
            CursorLineHead => {
                self.state.cursor.col = 0;
            }
            IntoNormalMode => {
                self.state.mode = Mode::Normal(String::new());
            }
            IntoInsertMode => {
                self.state.mode = Mode::Insert;
            }
            IntoAppendMode => {
                self.action(CursorRight.once());
                self.action(IntoInsertMode.once());
            }
            IntoCmdLineMode => {
                self.state.mode = Mode::CmdLine(String::new());
            }
            SetYank(b) => {
                self.state.yanked = b;
            }
            LineBreak => {
                self.state.buffer.insert_char(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                    '\n',
                );
                self.action(CursorDown.once());
                self.action(CursorLineHead.once());
            }
            InsertChar(c) => {
                self.state.buffer.insert_char(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                    c,
                );
                self.action(CursorRight.once());
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
            Notify => {
                self.notify();
            }
            ForwardWord => {
                let count = self.state.buffer.count_forward_word(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                );
                self.action(CursorRight.nth(count * action.count))
            }
            BackWord => {
                let count = self.state.buffer.count_back_word(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                );
                self.action(CursorLeft.nth(count * action.count));
            }
            RemoveChar => {
                let yank = self.state.buffer.remove_chars(
                    self.state.cursor.col,
                    self.state.cursor.row + self.state.row_offset,
                    action.count,
                );
                self.action(SetYank(yank).once());
            }
            RemoveLine(n) => {
                let yank = self
                    .state
                    .buffer
                    .remove_lines(self.state.cursor.row + self.state.row_offset, n);
                self.action(SetYank(yank).once());
            }
            YankLine(n) => {
                let yank = self
                    .state
                    .buffer
                    .subseq_lines(self.state.cursor.row + self.state.row_offset, n);
                self.action(SetYank(yank).once());
            }
            Remove(selection) => {
                let (from, to) = self.state.measure_selection(selection);
                let yank = self.state.buffer.remove(from..to);
                self.action(SetYank(yank).once());
                self.action(MoveTo(from).once());
            }
            Yank(selection) => {
                let (from, to) = self.state.measure_selection(selection);
                let yank = self.state.buffer.subseq(from..to);
                self.action(SetYank(yank).once());
            }
            AppendYank => {
                let col = if self.state.yanked.end_with_line_break() {
                    self.action(CursorDown.once());
                    0
                } else {
                    self.action(CursorRight.once());
                    self.state.cursor.col
                };
                for _ in 0..action.count {
                    self.state.buffer.insert(
                        col,
                        self.state.cursor.row + self.state.row_offset,
                        self.state.yanked.clone(),
                    );
                }
            }
            InsertYank => {
                let col = if self.state.yanked.end_with_line_break() {
                    0
                } else {
                    self.state.cursor.col
                };
                for _ in 0..action.count {
                    self.state.buffer.insert(
                        col,
                        self.state.cursor.row + self.state.row_offset,
                        self.state.yanked.clone(),
                    );
                }
            }
            MoveTo(pos) => {
                let result = self.state.buffer.get_cursor_by_offset(pos);
                self.state.cursor.row = result.0 - self.state.row_offset;
                self.state.cursor.col = result.1;
            }
        }
    }
}

pub(crate) struct DispatchAction(pub(crate) Action);
impl Message for DispatchAction {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<DispatchAction> for Store {
    async fn handle(&mut self, msg: DispatchAction, _ctx: &mut Context<Self>) {
        self.action(msg.0);
    }
}

pub(crate) struct GetState;
impl Message for GetState {
    type Result = State;
}

#[async_trait::async_trait]
impl Handler<GetState> for Store {
    async fn handle(&mut self, _msg: GetState, _ctx: &mut Context<Self>) -> State {
        self.state.clone()
    }
}
