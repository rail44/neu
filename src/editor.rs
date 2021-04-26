use core::cmp::min;
use std::fs::File;
use std::io::{stdin, BufWriter, Write};

use termion::event::Key;
use termion::input::TermRead;

use xtra::prelude::*;

use crate::actor;
use crate::actor::{Mode, State, StateActor};
use crate::cmd;
use crate::cmdline;

#[derive(PartialEq)]
enum Signal {
    Nope,
    Quit,
}

pub(crate) struct Editor {
    state: Address<StateActor>,
}

impl Actor for Editor {}

pub(crate) struct Run;
impl Message for Run {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<Run> for Editor {
    async fn handle(&mut self, _msg: Run, ctx: &mut Context<Self>) {
        let stdin = stdin();
        for k in stdin.keys() {
            let state = &self.state.send(actor::GetState).await.unwrap();
            match &state.mode {
                Mode::Normal(_) => {
                    match k.unwrap() {
                        Key::Char(c) => self.state.send(actor::PushCmd(c)).await.unwrap(),
                        Key::Ctrl(c) => self.state.send(actor::PushCmdStr(format!("<C-{}>", c))).await.unwrap(),
                        Key::Up => self.state.send(actor::PushCmdStr("<Up>".to_string())).await.unwrap(),
                        Key::Down => self.state.send(actor::PushCmdStr("<Down>".to_string())).await.unwrap(),
                        Key::Left => self.state.send(actor::PushCmdStr("<Left>".to_string())).await.unwrap(),
                        Key::Right => self.state.send(actor::PushCmdStr("<Right>".to_string())).await.unwrap(),
                        Key::Esc => self.state.send(actor::PushCmdStr("<Esc>".to_string())).await.unwrap(),
                        _ => {}
                    };
                    let state = &self.state.send(actor::GetState).await.unwrap();
                    self.handle_normal_mode(state, ctx).await;
                }
                Mode::Insert => self.handle_insert_mode(k.unwrap(), ctx).await,
                Mode::CmdLine(_) => {
                    match k.unwrap() {
                        Key::Char('\n') => {
                            let signal = self.handle_cmd_line_mode(state, ctx).await;
                            if Signal::Quit == signal {
                                break;
                            }
                        }
                        Key::Char(c) => self.state.send(actor::PushCmd(c)).await.unwrap(),
                        Key::Backspace => {
                            self.state.send(actor::PopCmd).await.unwrap();
                        }
                        Key::Esc | Key::Ctrl('c') => {
                            ctx.handle_while(self, self.state.send(actor::IntoNormalMode))
                                .await
                                .unwrap();
                        }
                        _ => {}
                    };
                }
            }
            self.coerce_cursor(ctx).await;
        }
    }
}

impl Editor {
    pub(crate) fn new(state: Address<StateActor>) -> Self {
        Editor { state }
    }

    async fn handle_normal_mode(&mut self, state: &State, ctx: &mut Context<Self>) {
        let parsed = cmd::parse(state.mode.get_cmd());
        if parsed.is_err() {
            return;
        }
        let (_, cmd) = parsed.unwrap();

        use cmd::CmdKind::*;
        match cmd.kind {
            CursorLeft => {
                self.state.send(actor::CursorLeft(cmd.count)).await.unwrap();
            }
            CursorDown => {
                ctx.handle_while(self, self.state.send(actor::CursorDown(cmd.count)))
                    .await
                    .unwrap();
            }
            CursorUp => {
                if state.cursor.row == 0 {
                    ctx.handle_while(self, self.state.send(actor::SubRowOffset(cmd.count)))
                        .await
                        .unwrap();
                    ctx.handle_while(
                        self,
                        self.state.send(actor::HandleState(|state: &mut State| {
                            state.mode.get_cmd_mut().clear();
                        })),
                    )
                    .await
                    .unwrap();
                    return;
                }
                ctx.handle_while(self, self.state.send(actor::CursorUp(cmd.count)))
                    .await
                    .unwrap();
            }
            CursorRight => {
                ctx.handle_while(self, self.state.send(actor::CursorRight(cmd.count)))
                    .await
                    .unwrap();
            }
            ForwardWord => {
                let count = state
                    .buffer
                    .count_forward_word(state.cursor.col, state.cursor.row + state.row_offset);
                ctx.handle_while(self, self.state.send(actor::CursorRight(count)))
                    .await
                    .unwrap();
            }
            BackWord => {
                let count = state
                    .buffer
                    .count_back_word(state.cursor.col, state.cursor.row + state.row_offset);
                ctx.handle_while(self, self.state.send(actor::CursorLeft(count)))
                    .await
                    .unwrap();
            }
            IntoInsertMode => {
                ctx.handle_while(self, self.state.send(actor::IntoInsertMode))
                    .await
                    .unwrap();
            }
            IntoAppendMode => {
                ctx.handle_while(self, self.state.send(actor::CursorRight(1)))
                    .await
                    .unwrap();
                ctx.handle_while(self, self.state.send(actor::IntoInsertMode))
                    .await
                    .unwrap();
            }
            IntoCmdLineMode => {
                ctx.handle_while(self, self.state.send(actor::IntoCmdLineMode))
                    .await
                    .unwrap();
            }
            RemoveChar => {
                let yank = self.state.send(actor::HandleState(move |state: &mut State| {
                    state.buffer.remove_chars(
                        state.cursor.col,
                        state.cursor.row + state.row_offset,
                        cmd.count,
                    )
                }))
                .await
                    .unwrap();
                ctx.handle_while(self, self.state.send(actor::SetYank(yank)))
                    .await
                    .unwrap();
            }
            RemoveLine => {
                let yank = self.state.send(actor::HandleState(move |state: &mut State| {
                state
                    .buffer
                    .remove_lines(state.cursor.row + state.row_offset, cmd.count)
                }))
                .await
                    .unwrap();
                ctx.handle_while(self, self.state.send(actor::SetYank(yank)))
                    .await
                    .unwrap();
            }
            YankLine => {
                let yank = state
                    .buffer
                    .subseq_lines(state.cursor.row + state.row_offset, cmd.count);
                ctx.handle_while(self, self.state.send(actor::SetYank(yank)))
                    .await
                    .unwrap();
            }
            AppendYank => {
                let col = if state.yanked.end_with_line_break() {
                    ctx.handle_while(self, self.state.send(actor::CursorDown(1)))
                        .await
                        .unwrap();
                    0
                } else {
                    ctx.handle_while(self, self.state.send(actor::CursorRight(1)))
                        .await
                        .unwrap();
                    state.cursor.col
                };
                for _ in 0..cmd.count {
                    ctx.handle_while(
                        self,
                        self.state.send(actor::HandleState(move |state: &mut State| {
                            state.buffer.insert(
                                col,
                                state.cursor.row + state.row_offset,
                                state.yanked.clone(),
                            );
                        })),
                    )
                    .await
                    .unwrap();
                }
            }
            InsertYank => {
                let col = if state.yanked.end_with_line_break() {
                    0
                } else {
                    state.cursor.col
                };
                for _ in 0..cmd.count {
                    ctx.handle_while(
                        self,
                        self.state.send(actor::HandleState(move |state: &mut State| {
                            state.buffer.insert(
                                col,
                                state.cursor.row + state.row_offset,
                                state.yanked.clone(),
                            );
                        })),
                    )
                        .await
                        .unwrap();
                }
            }
            Escape => {}
        }
        ctx.handle_while(
            self,
            self.state.send(actor::HandleState(move |state: &mut State| {
                if let Mode::Normal(ref mut cmd) = state.mode {
                    cmd.clear();
                }
            })),
        )
            .await
            .unwrap();
        }

    async fn handle_cmd_line_mode(&mut self, state: &State, ctx: &mut Context<Self>) -> Signal {
        let parsed = cmdline::parse(state.mode.get_cmdline());
        if parsed.is_err() {
            ctx.handle_while(self, self.state.send(actor::IntoNormalMode))
                .await
                .unwrap();
            return Signal::Nope;
        }
        let (_, cmd) = parsed.unwrap();

        use cmdline::Cmd::*;
        match cmd {
            Write(filename) => {
                let f = File::create(filename).unwrap();
                let mut w = BufWriter::new(f);
                write!(w, "{}", state.buffer.as_str()).unwrap();
            }
            Quit => return Signal::Quit,
        }
        ctx.handle_while(self, self.state.send(actor::IntoNormalMode))
            .await
            .unwrap();
        Signal::Nope
    }

    async fn handle_insert_mode(&mut self, k: Key, ctx: &mut Context<Self>) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    ctx.handle_while(
                        self,
                        self.state.send(actor::HandleState(|state: &mut State| {
                            state.buffer.insert_char(
                                state.cursor.col,
                                state.cursor.row + state.row_offset,
                                '\n',
                            );
                        })),
                    )
                    .await
                    .unwrap();
                    ctx.handle_while(self, self.state.send(actor::CursorDown(1)))
                        .await
                        .unwrap();
                    ctx.handle_while(self, self.state.send(actor::CursorLineHead))
                        .await
                        .unwrap();
                    // scroll();
                    return;
                }
                ctx.handle_while(
                    self,
                    self.state
                        .send(actor::HandleState(move |state: &mut State| {
                            state.buffer.insert_char(
                                state.cursor.col,
                                state.cursor.row + state.row_offset,
                                c,
                            );
                        })),
                )
                .await
                .unwrap();
                ctx.handle_while(self, self.state.send(actor::CursorRight(1)))
                    .await
                    .unwrap();
            }
            Key::Esc | Key::Ctrl('c') => {
                ctx.handle_while(self, self.state.send(actor::IntoNormalMode))
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }

    async fn coerce_cursor(&mut self, ctx: &mut Context<Self>) {
        let state = &self.state.send(actor::GetState).await.unwrap();
        let row = min(
            state.cursor.row,
            state.buffer.count_lines().saturating_sub(1),
        );
        ctx.handle_while(self, self.state.send(actor::CursorRow(row)))
            .await
            .unwrap();

        let textarea_row = (state.size.1 - 3) as usize;
        let actual_row = textarea_row - self.wrap_offset(state);
        if state.cursor.row > actual_row {
            let new_row_offset = min(
                state.row_offset + state.cursor.row - actual_row,
                state.buffer.count_lines().saturating_sub(actual_row),
            );
            ctx.handle_while(
                self,
                self.state
                    .send(actor::AddRowOffset(new_row_offset - state.row_offset)),
            )
            .await
            .unwrap();
            ctx.handle_while(self, self.state.send(actor::CursorRow(actual_row)))
                .await
                .unwrap();
        }
        let col = min(
            state.cursor.col,
            state.buffer.row_len(state.cursor.row + state.row_offset),
        );
        ctx.handle_while(self, self.state.send(actor::CursorCol(col)))
            .await
            .unwrap();
    }

    fn wrap_offset(&mut self, state: &State) -> usize {
        let mut wraps = 0;
        let mut lines_count = 0;
        for line in state.buffer.lines().skip(state.row_offset) {
            let wrap = (line.len() as u16) / state.size.0;
            wraps += wrap;
            lines_count += 1 + wrap;
            if lines_count >= state.size.1 - 2 {
                break;
            }
        }
        wraps as usize
    }
}
