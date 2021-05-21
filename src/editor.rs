use std::fs::File;
use std::io::{stdin, BufWriter, Write};

use termion::event::Key;
use termion::input::TermRead;

use xtra::prelude::*;

use crate::cmd;
use crate::cmdline;
use crate::store;
use crate::store::{Mode, State, Store};
use crate::motion;
use crate::operate;

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
                &Mode::Pending(_, _) => {
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
                    self.handle_pending_mode().await;
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

    async fn handle_motion(&mut self) {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = motion::parse(state.mode.get_cmd());

        if parsed.is_err() {
            return;
        }

        let (_, cmd) = parsed.unwrap();

        use motion::MotionKind::*;
        match cmd.kind {
            CursorLeft => {
                self.store.do_send(store::CursorLeft(cmd.count)).unwrap();
            }
            CursorDown => {
                self.store.do_send(store::CursorDown(cmd.count)).unwrap();
            }
            CursorUp => {
                self.store.do_send(store::CursorUp(cmd.count)).unwrap();
            }
            CursorRight => {
                self.store.do_send(store::CursorRight(cmd.count)).unwrap();
            }
            ForwardWord => {
                self.store.do_send(store::ForwardWord(cmd.count)).unwrap();
            }
            BackWord => {
                self.store.do_send(store::BackWord(cmd.count)).unwrap();
            }
            Line => ()
        }
        self.store
            .do_send(store::HandleState(move |state: &mut State| {
                if let Mode::Normal(ref mut cmd) = state.mode {
                    cmd.clear();
                }
            }))
            .unwrap();
    }

    async fn handle_pending_mode(&mut self) {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = motion::parse(state.mode.get_cmd());

        if parsed.is_err() {
            return;
        }

        let (_, cmd) = parsed.unwrap();

        use motion::MotionKind::*;
        match cmd.kind {
            CursorLeft => {
                // self.store.do_send(store::CursorLeft(cmd.count)).unwrap();
            }
            CursorDown => {
                // self.store.do_send(store::CursorDown(cmd.count)).unwrap();
            }
            CursorUp => {
                // self.store.do_send(store::CursorUp(cmd.count)).unwrap();
            }
            CursorRight => {
                // self.store.do_send(store::CursorRight(cmd.count)).unwrap();
            }
            ForwardWord => {
                let count = self.store.send(store::CountWordForward).await.unwrap();
                self.store.do_send(store::RemoveChars(count)).unwrap();
            }
            BackWord => {
                // let count = self.store.send(store::CountWordBack).await.unwrap();
                // self.store.do_send(store::RemoveChars(count)).unwrap();
            }
            Line => {
                self.store.do_send(store::RemoveLines(cmd.count)).unwrap();
            }
        }
        self.store.do_send(store::IntoNormalMode).unwrap();
    }

    async fn handle_operate(&mut self) {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = operate::parse(state.mode.get_cmd());

        if parsed.is_err() {
            return self.handle_motion().await;
        }

        let (_, operate) = parsed.unwrap();

        self.store.do_send(store::IntoPendingMode(operate)).unwrap();
    }

    async fn handle_normal_mode(&mut self) {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = cmd::parse(state.mode.get_cmd());
        if parsed.is_err() {
            return self.handle_operate().await;
        }
        let (_, cmd) = parsed.unwrap();

        use cmd::CmdKind::*;
        match cmd.kind {
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
                self.store.do_send(store::RemoveChars(cmd.count)).unwrap();
            }
            AppendYank => {
                self.store.do_send(store::AppendYank(cmd.count)).unwrap();
            }
            InsertYank => {
                self.store.do_send(store::InsertYank(cmd.count)).unwrap();
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
