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
    async fn handle(&mut self, _msg: Run, _ctx: &mut Context<Self>) {
        let stdin = stdin();
        for k in stdin.keys() {
            let state = self.state.send(actor::GetState).await.unwrap();
            match &state.mode {
                Mode::Normal(_) => {
                    match k.unwrap() {
                        Key::Char(c) => self.state.do_send(actor::PushCmd(c)).unwrap(),
                        Key::Ctrl(c) => self
                            .state
                            .do_send(actor::PushCmdStr(format!("<C-{}>", c)))
                            .unwrap(),
                        Key::Up => self
                            .state
                            .do_send(actor::PushCmdStr("<Up>".to_string()))
                            .unwrap(),
                        Key::Down => self
                            .state
                            .do_send(actor::PushCmdStr("<Down>".to_string()))
                            .unwrap(),
                        Key::Left => self
                            .state
                            .do_send(actor::PushCmdStr("<Left>".to_string()))
                            .unwrap(),
                        Key::Right => self
                            .state
                            .do_send(actor::PushCmdStr("<Right>".to_string()))
                            .unwrap(),
                        Key::Esc => self
                            .state
                            .do_send(actor::PushCmdStr("<Esc>".to_string()))
                            .unwrap(),
                        _ => {}
                    };
                    self.handle_normal_mode().await;
                }
                Mode::Insert => self.handle_insert_mode(k.unwrap()).await,
                Mode::CmdLine(_) => {
                    match k.unwrap() {
                        Key::Char('\n') => {
                            let signal = self.handle_cmd_line_mode().await;
                            if Signal::Quit == signal {
                                break;
                            }
                        }
                        Key::Char(c) => self.state.do_send(actor::PushCmd(c)).unwrap(),
                        Key::Backspace => {
                            self.state.do_send(actor::PopCmd).unwrap();
                        }
                        Key::Esc | Key::Ctrl('c') => {
                            self.state.do_send(actor::IntoNormalMode).unwrap();
                        }
                        _ => {}
                    };
                }
            }
        }
    }
}

impl Editor {
    pub(crate) fn new(state: Address<StateActor>) -> Self {
        Editor { state }
    }

    async fn handle_normal_mode(&mut self) {
        let state = self.state.send(actor::GetState).await.unwrap();
        let parsed = cmd::parse(state.mode.get_cmd());
        if parsed.is_err() {
            return;
        }
        let (_, cmd) = parsed.unwrap();

        use cmd::CmdKind::*;
        match cmd.kind {
            CursorLeft => {
                self.state.do_send(actor::CursorLeft(cmd.count)).unwrap();
            }
            CursorDown => {
                self.state.do_send(actor::CursorDown(cmd.count)).unwrap();
            }
            CursorUp => {
                if state.cursor.row == 0 {
                    self.state
                        .do_send(actor::SubRowOffset(cmd.count))
                        .unwrap();
                    self.state
                        .do_send(actor::HandleState(|state: &mut State| {
                            state.mode.get_cmd_mut().clear();
                        }))
                        .unwrap();
                    return;
                }
                self.state.do_send(actor::CursorUp(cmd.count)).unwrap();
            }
            CursorRight => {
                self.state
                    .do_send(actor::CursorRight(cmd.count))
                    .unwrap();
            }
            ForwardWord => {
                let count = state
                    .buffer
                    .count_forward_word(state.cursor.col, state.cursor.row + state.row_offset);
                self.state.do_send(actor::CursorRight(count)).unwrap();
            }
            BackWord => {
                let count = state
                    .buffer
                    .count_back_word(state.cursor.col, state.cursor.row + state.row_offset);
                self.state.do_send(actor::CursorLeft(count)).unwrap();
            }
            IntoInsertMode => {
                self.state.do_send(actor::IntoInsertMode).unwrap();
            }
            IntoAppendMode => {
                self.state.do_send(actor::CursorRight(1)).unwrap();
                self.state.do_send(actor::IntoInsertMode).unwrap();
            }
            IntoCmdLineMode => {
                self.state.do_send(actor::IntoCmdLineMode).unwrap();
            }
            RemoveChar => {
                let yank = self
                    .state
                    .send(actor::HandleState(move |state: &mut State| {
                        state.buffer.remove_chars(
                            state.cursor.col,
                            state.cursor.row + state.row_offset,
                            cmd.count,
                        )
                    }))
                    .await
                    .unwrap();
                self.state.do_send(actor::SetYank(yank)).unwrap();
            }
            RemoveLine => {
                let yank = self
                    .state
                    .send(actor::HandleState(move |state: &mut State| {
                        state
                            .buffer
                            .remove_lines(state.cursor.row + state.row_offset, cmd.count)
                    }))
                    .await
                    .unwrap();
                self.state.do_send(actor::SetYank(yank)).unwrap();
            }
            YankLine => {
                let yank = state
                    .buffer
                    .subseq_lines(state.cursor.row + state.row_offset, cmd.count);
                self.state.do_send(actor::SetYank(yank)).unwrap();
            }
            AppendYank => {
                let col = if state.yanked.end_with_line_break() {
                    self.state.do_send(actor::CursorDown(1)).unwrap();
                    0
                } else {
                    self.state.do_send(actor::CursorRight(1)).unwrap();
                    state.cursor.col
                };
                for _ in 0..cmd.count {
                    self.state
                        .do_send(actor::HandleState(move |state: &mut State| {
                            state.buffer.insert(
                                col,
                                state.cursor.row + state.row_offset,
                                state.yanked.clone(),
                            );
                        }))
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
                    self.state
                        .do_send(actor::HandleState(move |state: &mut State| {
                            state.buffer.insert(
                                col,
                                state.cursor.row + state.row_offset,
                                state.yanked.clone(),
                            );
                        }))
                        .unwrap();
                }
            }
            Escape => {}
        }
        self.state
            .do_send(actor::HandleState(move |state: &mut State| {
                if let Mode::Normal(ref mut cmd) = state.mode {
                    cmd.clear();
                }
            }))
            .unwrap();
    }

    async fn handle_cmd_line_mode(&mut self) -> Signal {
        let state = self.state.send(actor::GetState).await.unwrap();
        let parsed = cmdline::parse(state.mode.get_cmdline());
        if parsed.is_err() {
            self.state.do_send(actor::IntoNormalMode).unwrap();
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
        self.state.do_send(actor::IntoNormalMode).unwrap();
        Signal::Nope
    }

    async fn handle_insert_mode(&mut self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.state
                        .do_send(actor::HandleState(|state: &mut State| {
                            state.buffer.insert_char(
                                state.cursor.col,
                                state.cursor.row + state.row_offset,
                                '\n',
                            );
                        }))
                        .unwrap();
                    self.state.do_send(actor::CursorDown(1)).unwrap();
                    self.state.do_send(actor::CursorLineHead).unwrap();
                    // scroll();
                    return;
                }
                self.state
                    .do_send(actor::HandleState(move |state: &mut State| {
                        state.buffer.insert_char(
                            state.cursor.col,
                            state.cursor.row + state.row_offset,
                            c,
                        );
                    }))
                    .unwrap();
                self.state.do_send(actor::CursorRight(1)).unwrap();
            }
            Key::Esc | Key::Ctrl('c') => {
                self.state.do_send(actor::IntoNormalMode).unwrap();
            }
            _ => {}
        }
    }
}
