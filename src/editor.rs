use std::fs::File;
use std::io::{stdin, BufWriter, Write};

use termion::event::Key;
use termion::input::TermRead;

use xtra::prelude::*;

use crate::action::{ActionKind, EditKind};
use crate::cmd;
use crate::cmdline;
use crate::state::Mode;
use crate::store;
use crate::store::Store;

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
                        Key::Char(c) => self
                            .store
                            .do_send(store::DispatchAction(ActionKind::PushCmd(c).once()))
                            .unwrap(),
                        Key::Ctrl(c) => self
                            .store
                            .do_send(store::DispatchAction(
                                ActionKind::PushCmdStr(format!("<C-{}>", c)).once(),
                            ))
                            .unwrap(),
                        Key::Up => self
                            .store
                            .do_send(store::DispatchAction(
                                ActionKind::PushCmdStr("<Up>".to_string()).once(),
                            ))
                            .unwrap(),
                        Key::Down => self
                            .store
                            .do_send(store::DispatchAction(
                                ActionKind::PushCmdStr("<Down>".to_string()).once(),
                            ))
                            .unwrap(),
                        Key::Left => self
                            .store
                            .do_send(store::DispatchAction(
                                ActionKind::PushCmdStr("<Left>".to_string()).once(),
                            ))
                            .unwrap(),
                        Key::Right => self
                            .store
                            .do_send(store::DispatchAction(
                                ActionKind::PushCmdStr("<Right>".to_string()).once(),
                            ))
                            .unwrap(),
                        Key::Esc => self
                            .store
                            .do_send(store::DispatchAction(
                                ActionKind::PushCmdStr("<Esc>".to_string()).once(),
                            ))
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
                        Key::Char(c) => self
                            .store
                            .do_send(store::DispatchAction(ActionKind::PushCmd(c).once()))
                            .unwrap(),
                        Key::Backspace => self
                            .store
                            .do_send(store::DispatchAction(ActionKind::PopCmd.once()))
                            .unwrap(),
                        Key::Esc | Key::Ctrl('c') => self
                            .store
                            .do_send(store::DispatchAction(ActionKind::IntoNormalMode.once()))
                            .unwrap(),
                        _ => {}
                    };
                }
            }
            self.store
                .send(store::DispatchAction(ActionKind::Notify.once()))
                .await
                .unwrap();
        }
    }
}

impl Editor {
    pub(crate) fn new(store: Address<Store>) -> Self {
        Editor { store }
    }

    async fn handle_normal_mode(&self) {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = cmd::parse(state.mode.get_cmd());
        if parsed.is_err() {
            return;
        }
        let (_, action) = parsed.unwrap();

        self.store.do_send(store::DispatchAction(action)).unwrap();
        self.store
            .do_send(store::DispatchAction(ActionKind::ClearCmd.once()))
            .unwrap();
    }

    async fn handle_cmd_line_mode(&self) -> Signal {
        let state = self.store.send(store::GetState).await.unwrap();
        let parsed = cmdline::parse(state.mode.get_cmdline());
        if parsed.is_err() {
            self.store
                .do_send(store::DispatchAction(ActionKind::IntoNormalMode.once()))
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
        self.store
            .do_send(store::DispatchAction(ActionKind::IntoNormalMode.once()))
            .unwrap();
        Signal::Nope
    }

    async fn handle_insert_mode(&self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.store
                        .do_send(store::DispatchAction(EditKind::LineBreak.once()))
                        .unwrap();
                    return;
                }
                self.store
                    .do_send(store::DispatchAction(EditKind::InsertChar(c).once()))
                    .unwrap();
            }
            Key::Esc | Key::Ctrl('c') => {
                self.store
                    .do_send(store::DispatchAction(ActionKind::IntoNormalMode.once()))
                    .unwrap();
            }
            _ => {}
        }
    }
}
