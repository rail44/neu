use std::fs::File;
use std::io::{stdin, BufWriter, Write};

use termion::event::Key;
use termion::input::TermRead;

use xtra::prelude::*;

use crate::cmd;
use crate::cmdline;
use crate::store;
use crate::store::{Mode, State, Store};

#[derive(PartialEq)]
enum Signal {
    Nope,
    Quit,
}

pub(crate) struct Editor {
    store: Address<Store>,
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
            let state = self.store.send(store::GetState).await.unwrap();
            match &state.mode {
                Mode::Normal(_) => {
                    match k.unwrap() {
                        Key::Char(c) => self.store.do_send(store::PushCmd(c)).unwrap(),
                        Key::Ctrl(c) => self
                            .store
                            .do_send(store::PushCmdStr(format!("<C-{}>", c)))
                            .unwrap(),
                        Key::Up => self
                            .store
                            .do_send(store::PushCmdStr("<Up>".to_string()))
                            .unwrap(),
                        Key::Down => self
                            .store
                            .do_send(store::PushCmdStr("<Down>".to_string()))
                            .unwrap(),
                        Key::Left => self
                            .store
                            .do_send(store::PushCmdStr("<Left>".to_string()))
                            .unwrap(),
                        Key::Right => self
                            .store
                            .do_send(store::PushCmdStr("<Right>".to_string()))
                            .unwrap(),
                        Key::Esc => self
                            .store
                            .do_send(store::PushCmdStr("<Esc>".to_string()))
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
                        Key::Char(c) => self.store.do_send(store::PushCmd(c)).unwrap(),
                        Key::Backspace => {
                            self.store.do_send(store::PopCmd).unwrap();
                        }
                        Key::Esc | Key::Ctrl('c') => {
                            self.store.do_send(store::IntoNormalMode).unwrap();
                        }
                        _ => {}
                    };
                }
            }
            self.store.send(store::Notify).await.unwrap();
        }
    }
}

impl Editor {
    pub(crate) fn new(store: Address<Store>) -> Self {
        Editor { store }
    }

    async fn handle_normal_mode(&mut self) {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = cmd::parse(state.mode.get_cmd());
        if parsed.is_err() {
            return;
        }
        let (_, cmd) = parsed.unwrap();

        use cmd::CmdKind::*;
        match cmd.kind {
            CursorLeft => {
                self.store.do_send(store::CursorLeft(cmd.count)).unwrap();
            }
            CursorDown => {
                self.store.do_send(store::CursorDown(cmd.count)).unwrap();
            }
            CursorUp => {
                if state.cursor.row == 0 {
                    self.store.do_send(store::SubRowOffset(cmd.count)).unwrap();
                    self.store
                        .do_send(store::HandleState(|store: &mut State| {
                            store.mode.get_cmd_mut().clear();
                        }))
                        .unwrap();
                    return;
                }
                self.store.do_send(store::CursorUp(cmd.count)).unwrap();
            }
            CursorRight => {
                self.store.do_send(store::CursorRight(cmd.count)).unwrap();
            }
            ForwardWord => {
                let count = state
                    .buffer
                    .count_forward_word(state.cursor.col, state.cursor.row + state.row_offset);
                self.store.do_send(store::CursorRight(count)).unwrap();
            }
            BackWord => {
                let count = state
                    .buffer
                    .count_back_word(state.cursor.col, state.cursor.row + state.row_offset);
                self.store.do_send(store::CursorLeft(count)).unwrap();
            }
            IntoInsertMode => {
                self.store.do_send(store::IntoInsertMode).unwrap();
            }
            IntoAppendMode => {
                self.store.do_send(store::CursorRight(1)).unwrap();
                self.store.do_send(store::IntoInsertMode).unwrap();
            }
            IntoCmdLineMode => {
                self.store.do_send(store::IntoCmdLineMode).unwrap();
            }
            RemoveChar => {
                let yank = self
                    .store
                    .send(store::HandleState(move |state: &mut State| {
                        state.buffer.remove_chars(
                            state.cursor.col,
                            state.cursor.row + state.row_offset,
                            cmd.count,
                        )
                    }))
                    .await
                    .unwrap();
                self.store.do_send(store::SetYank(yank)).unwrap();
            }
            RemoveLine => {
                let yank = self
                    .store
                    .send(store::HandleState(move |state: &mut State| {
                        state
                            .buffer
                            .remove_lines(state.cursor.row + state.row_offset, cmd.count)
                    }))
                    .await
                    .unwrap();
                self.store.do_send(store::SetYank(yank)).unwrap();
            }
            YankLine => {
                let yank = state
                    .buffer
                    .subseq_lines(state.cursor.row + state.row_offset, cmd.count);
                self.store.do_send(store::SetYank(yank)).unwrap();
            }
            AppendYank => {
                let col = if state.yanked.end_with_line_break() {
                    self.store.do_send(store::CursorDown(1)).unwrap();
                    0
                } else {
                    self.store.do_send(store::CursorRight(1)).unwrap();
                    state.cursor.col
                };
                for _ in 0..cmd.count {
                    self.store
                        .do_send(store::HandleState(move |state: &mut State| {
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
                    self.store
                        .do_send(store::HandleState(move |state: &mut State| {
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
        self.store
            .do_send(store::HandleState(move |state: &mut State| {
                if let Mode::Normal(ref mut cmd) = state.mode {
                    cmd.clear();
                }
            }))
            .unwrap();
    }

    async fn handle_cmd_line_mode(&mut self) -> Signal {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = cmdline::parse(state.mode.get_cmdline());
        if parsed.is_err() {
            self.store.do_send(store::IntoNormalMode).unwrap();
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
        self.store.do_send(store::IntoNormalMode).unwrap();
        Signal::Nope
    }

    async fn handle_insert_mode(&mut self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.store
                        .do_send(store::HandleState(|state: &mut State| {
                            state.buffer.insert_char(
                                state.cursor.col,
                                state.cursor.row + state.row_offset,
                                '\n',
                            );
                        }))
                        .unwrap();
                    self.store.do_send(store::CursorDown(1)).unwrap();
                    self.store.do_send(store::CursorLineHead).unwrap();
                    return;
                }
                self.store
                    .do_send(store::HandleState(move |state: &mut State| {
                        state.buffer.insert_char(
                            state.cursor.col,
                            state.cursor.row + state.row_offset,
                            c,
                        );
                    }))
                    .unwrap();
                self.store.do_send(store::CursorRight(1)).unwrap();
            }
            Key::Esc | Key::Ctrl('c') => {
                self.store.do_send(store::IntoNormalMode).unwrap();
            }
            _ => {}
        }
    }
}
