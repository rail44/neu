use std::io::stdin;

use termion::event::Key;
use termion::input::TermRead;

use crate::action::{Action, ActionKind, EditKind, MovementKind};
use crate::cmd;
use crate::cmdline;
use crate::mode::Mode;

use flume::Sender;

pub(crate) struct Editor {
    store: Sender<Action>,
}

impl Editor {
    pub(crate) fn new(store: Sender<Action>) -> Self {
        Editor { store }
    }

    async fn handle_normal_mode(&self) {
        let (tx, rx) = flume::unbounded();
        self.store.send(ActionKind::GetState(tx).once()).unwrap();
        let state = rx.recv_async().await.unwrap();
        let parsed = cmd::parse(state.mode.get_cmd());
        if parsed.is_err() {
            return;
        }
        let (_, action) = parsed.unwrap();

        self.store.send(action).unwrap();
        self.store.send(ActionKind::ClearCmd.once()).unwrap();
    }

    async fn handle_cmd_line_mode(&self) {
        let (tx, rx) = flume::unbounded();
        self.store.send(ActionKind::GetState(tx).once()).unwrap();
        let state = rx.recv_async().await.unwrap();
        let parsed = cmdline::parse(state.mode.get_cmdline());
        if parsed.is_err() {
            self.store.send(ActionKind::IntoNormalMode.once()).unwrap();
            return;
        }
        let (_, action) = parsed.unwrap();

        self.store.send(action).unwrap();
        self.store.send(ActionKind::IntoNormalMode.once()).unwrap();
    }

    fn handle_insert_mode(&self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.store.send(EditKind::LineBreak.once()).unwrap();
                    return;
                }
                self.store.send(EditKind::InsertChar(c).once()).unwrap();
            }
            Key::Esc | Key::Ctrl('c') => {
                self.store.send(ActionKind::IntoNormalMode.once()).unwrap();
            }
            _ => {}
        }
    }

    pub(crate) async fn run(&self) {
        let stdin = stdin();
        for k in stdin.keys() {
            let (tx, rx) = flume::unbounded();
            self.store.send(ActionKind::GetState(tx).once()).unwrap();
            let state = rx.recv_async().await.unwrap();
            match &state.mode {
                Mode::Normal(_) => {
                    match k.unwrap() {
                        Key::Char(c) => self.store.send(ActionKind::PushCmd(c).once()).unwrap(),
                        Key::Ctrl(c) => self
                            .store
                            .send(ActionKind::PushCmdStr(format!("<C-{}>", c)).once())
                            .unwrap(),
                        Key::Up => self
                            .store
                            .send(ActionKind::PushCmdStr("<Up>".to_string()).once())
                            .unwrap(),
                        Key::Down => self
                            .store
                            .send(ActionKind::PushCmdStr("<Down>".to_string()).once())
                            .unwrap(),
                        Key::Left => self
                            .store
                            .send(ActionKind::PushCmdStr("<Left>".to_string()).once())
                            .unwrap(),
                        Key::Right => self
                            .store
                            .send(ActionKind::PushCmdStr("<Right>".to_string()).once())
                            .unwrap(),
                        Key::Esc => self
                            .store
                            .send(ActionKind::PushCmdStr("<Esc>".to_string()).once())
                            .unwrap(),
                        _ => {}
                    };
                    self.handle_normal_mode().await;
                }
                Mode::Insert(_, _) => self.handle_insert_mode(k.unwrap()),
                Mode::CmdLine(_) => {
                    match k.unwrap() {
                        Key::Char('\n') => {
                            self.handle_cmd_line_mode().await;
                        }
                        Key::Char(c) => self.store.send(ActionKind::PushCmd(c).once()).unwrap(),
                        Key::Backspace => self.store.send(ActionKind::PopCmd.once()).unwrap(),
                        Key::Esc | Key::Ctrl('c') => {
                            self.store.send(ActionKind::IntoNormalMode.once()).unwrap()
                        }
                        _ => {}
                    };
                }
                Mode::Search => {
                    match k.unwrap() {
                        Key::Char('\n') => {
                            self.store
                                .send(MovementKind::MoveAsSeenOnView.once())
                                .unwrap();
                            self.store.send(ActionKind::IntoNormalMode.once()).unwrap()
                        }
                        Key::Char(c) => self.store.send(ActionKind::PushSearch(c).once()).unwrap(),
                        Key::Backspace => self.store.send(ActionKind::PopSearch.once()).unwrap(),
                        Key::Esc | Key::Ctrl('c') => {
                            self.store.send(ActionKind::ClearSearch.once()).unwrap();
                            self.store.send(ActionKind::IntoNormalMode.once()).unwrap()
                        }
                        _ => {}
                    };
                }
            }
        }
    }
}
