use core::cmp::min;
use std::fs::File;
use std::io::{stdin, stdout, BufWriter, Stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::terminal_size;

use xtra::prelude::*;

use crate::actor;
use crate::actor::{State, StateActor};
use crate::buffer::Buffer;
use crate::cmd;
use crate::cmdline;

struct Cursor {
    row: usize,
    col: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor { row: 0, col: 0 }
    }
}

enum Mode {
    Normal(String),
    Insert,
    CmdLine(String),
}

impl Mode {
    fn get_cmd(&self) -> &String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }
        panic!();
    }

    fn get_cmd_mut(&mut self) -> &mut String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }
        panic!();
    }

    fn get_cmdline(&self) -> &String {
        if let Mode::CmdLine(cmd) = self {
            return cmd;
        }
        panic!();
    }
}

#[derive(PartialEq)]
enum Signal {
    Nope,
    Quit,
}

pub(crate) struct Editor {
    size: (u16, u16),
    state_actor: Address<StateActor>,
    state: State,
    mode: Mode,
    cursor: Cursor,
    buffer: Buffer,
    stdout: BufWriter<RawTerminal<Stdout>>,
    yanked: Buffer,
}

impl Actor for Editor {}

pub(crate) struct Run;
impl Message for Run {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<Run> for Editor {
    async fn handle(&mut self, _msg: Run, ctx: &mut Context<Self>) {
        self.draw();
        let stdin = stdin();
        for k in stdin.keys() {
            write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();

            match &mut self.mode {
                Mode::Normal(cmd) => {
                    match k.unwrap() {
                        Key::Char(c) => cmd.push(c),
                        Key::Ctrl(c) => cmd.push_str(&format!("<C-{}>", c)),
                        Key::Up => cmd.push_str("<Up>"),
                        Key::Down => cmd.push_str("<Down>"),
                        Key::Left => cmd.push_str("<Left>"),
                        Key::Right => cmd.push_str("<Right>"),
                        Key::Esc => cmd.push_str("<Esc>"),
                        _ => {}
                    };
                    self.handle_normal_mode();
                }
                Mode::Insert => self.handle_insert_mode(k.unwrap()),
                Mode::CmdLine(cmd) => {
                    match k.unwrap() {
                        Key::Char('\n') => {
                            let signal = self.handle_cmd_line_mode();
                            if Signal::Quit == signal {
                                break;
                            }
                        }
                        Key::Char(c) => cmd.push(c),
                        Key::Backspace => {
                            cmd.pop();
                        }
                        Key::Esc | Key::Ctrl('c') => {
                            self.mode = Mode::Normal(String::new());
                        }
                        _ => {}
                    };
                }
            }
            self.coerce_cursor(ctx).await;
            self.draw();
        }
    }
}

pub(crate) struct ChangeState(pub(crate) State);
impl Message for ChangeState {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<ChangeState> for Editor {
    async fn handle(&mut self, msg: ChangeState, _ctx: &mut Context<Self>) {
        self.state = msg.0;
    }
}

impl Editor {
    pub(crate) fn new(state_actor: Address<StateActor>) -> Self {
        let mode = Mode::Normal(String::new());

        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(
            stdout,
            "{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();
        let size = terminal_size().unwrap();

        let state = State::default();

        Editor {
            mode,
            size,
            stdout,
            state_actor,
            state,
            buffer: Buffer::new(),
            cursor: Cursor::default(),
            yanked: Buffer::default(),
        }
    }

    fn draw(&mut self) {
        write!(self.stdout, "{}", termion::clear::All).unwrap();
        let mut wraps = 0;
        let mut drawed_lines_count = 0;
        let textarea_row = self.size.1 - 2;
        for (i, line) in self.buffer.lines().skip(self.state.row_offset).enumerate() {
            let wrap = (line.len() as u16) / self.size.0;
            drawed_lines_count += 1;

            let line = if drawed_lines_count >= textarea_row && i != self.cursor.row {
                let s: String = line.chars().take(self.size.0 as usize).collect();
                s.into()
            } else {
                line
            };
            write!(self.stdout, "{}\r\n", line).unwrap();

            if i < self.cursor.row {
                wraps += wrap
            }
            drawed_lines_count += wrap;
            if drawed_lines_count >= textarea_row {
                break;
            }
        }
        write!(self.stdout, "{}", termion::cursor::Goto(0, self.size.1 - 1)).unwrap();
        match &self.mode {
            Mode::Normal(cmd) => {
                if cmd.is_empty() {
                    write!(self.stdout, "{}NORMAL", termion::cursor::SteadyBlock).unwrap();
                } else {
                    write!(self.stdout, "{}NORMAL", termion::cursor::SteadyUnderline).unwrap();
                }
            }
            Mode::Insert => {
                write!(self.stdout, "{}INSERT", termion::cursor::SteadyBar).unwrap();
            }
            Mode::CmdLine(cmd) => {
                write!(
                    self.stdout,
                    "{}COMMAND{}:{}",
                    termion::cursor::SteadyBlock,
                    termion::cursor::Goto(0, self.size.1),
                    cmd
                )
                .unwrap();
            }
        };
        let col = self.cursor.col as u16 % self.size.0;
        let row = self.cursor.row as u16 + self.cursor.col as u16 / self.size.0 + wraps;
        write!(self.stdout, "{}", termion::cursor::Goto(col + 1, row + 1)).unwrap();
        self.stdout.flush().unwrap();
    }

    fn handle_normal_mode(&mut self) {
        let parsed = cmd::parse(self.mode.get_cmd());
        if parsed.is_err() {
            return;
        }
        let (_, cmd) = parsed.unwrap();

        use cmd::CmdKind::*;
        match cmd.kind {
            CursorLeft => {
                self.cursor.col = self.cursor.col.saturating_sub(cmd.count);
            }
            CursorDown => {
                self.cursor.row += cmd.count;
            }
            CursorUp => {
                if self.cursor.row == 0 {
                    self.state_actor.send(actor::SubRowOffset(cmd.count));

                    self.mode.get_cmd_mut().clear();
                    return;
                }
                self.cursor.row = self.cursor.row.saturating_sub(cmd.count);
            }
            CursorRight => {
                self.cursor.col += cmd.count;
            }
            ForwardWord => {
                self.cursor.col += self
                    .buffer
                    .count_forward_word(self.cursor.col, self.cursor.row + self.state.row_offset);
            }
            BackWord => {
                self.cursor.col -= self
                    .buffer
                    .count_back_word(self.cursor.col, self.cursor.row + self.state.row_offset);
            }
            IntoInsertMode => {
                self.mode = Mode::Insert;
            }
            IntoAppendMode => {
                self.cursor.col += 1;
                self.mode = Mode::Insert;
            }
            IntoCmdLineMode => {
                self.mode = Mode::CmdLine(String::new());
            }
            RemoveChar => {
                self.yanked = self.buffer.remove_chars(
                    self.cursor.col,
                    self.cursor.row + self.state.row_offset,
                    cmd.count,
                );
            }
            RemoveLine => {
                self.yanked = self
                    .buffer
                    .remove_lines(self.cursor.row + self.state.row_offset, cmd.count);
            }
            YankLine => {
                self.yanked = self
                    .buffer
                    .subseq_lines(self.cursor.row + self.state.row_offset, cmd.count);
            }
            AppendYank => {
                let col = if self.yanked.end_with_line_break() {
                    self.cursor.row += 1;
                    0
                } else {
                    self.cursor.col += 1;
                    self.cursor.col
                };
                for _ in 0..cmd.count {
                    self.buffer.insert(
                        col,
                        self.cursor.row + self.state.row_offset,
                        self.yanked.clone(),
                    );
                }
            }
            InsertYank => {
                let col = if self.yanked.end_with_line_break() {
                    0
                } else {
                    self.cursor.col
                };
                for _ in 0..cmd.count {
                    self.buffer.insert(
                        col,
                        self.cursor.row + self.state.row_offset,
                        self.yanked.clone(),
                    );
                }
            }
            Escape => {}
        }
        if let Mode::Normal(ref mut cmd) = self.mode {
            cmd.clear();
        }
    }

    fn handle_cmd_line_mode(&mut self) -> Signal {
        let parsed = cmdline::parse(self.mode.get_cmdline());
        if parsed.is_err() {
            self.mode = Mode::Normal(String::new());
            return Signal::Nope;
        }
        let (_, cmd) = parsed.unwrap();

        use cmdline::Cmd::*;
        match cmd {
            Write(filename) => {
                let f = File::create(filename).unwrap();
                let mut w = BufWriter::new(f);
                write!(w, "{}", self.buffer.as_str()).unwrap();
            }
            Quit => return Signal::Quit,
        }
        self.mode = Mode::Normal(String::new());
        Signal::Nope
    }

    fn handle_insert_mode(&mut self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.buffer.insert_char(
                        self.cursor.col,
                        self.cursor.row + self.state.row_offset,
                        '\n',
                    );
                    self.cursor.row += 1;
                    self.cursor.col = 0;
                    // scroll();
                    return;
                }
                self.buffer.insert_char(
                    self.cursor.col,
                    self.cursor.row + self.state.row_offset,
                    c,
                );
                self.cursor.col += 1;
            }
            Key::Esc | Key::Ctrl('c') => {
                self.mode = Mode::Normal(String::new());
            }
            _ => {}
        }
    }

    async fn coerce_cursor(&mut self, ctx: &mut Context<Self>) {
        self.cursor.row = min(self.cursor.row, self.buffer.count_lines().saturating_sub(1));

        let textarea_row = (self.size.1 - 3) as usize;
        let actual_row = textarea_row - self.wrap_offset();
        if self.cursor.row > actual_row {
            let new_row_offset = min(
                self.state.row_offset + self.cursor.row - actual_row,
                self.buffer.count_lines().saturating_sub(actual_row),
            );
            ctx.handle_while(
                self,
                self.state_actor
                    .send(actor::AddRowOffset(new_row_offset - self.state.row_offset)),
            )
            .await
            .unwrap();
            self.cursor.row = actual_row;
        }
        self.cursor.col = min(
            self.cursor.col,
            self.buffer.row_len(self.cursor.row + self.state.row_offset),
        );
    }

    fn wrap_offset(&mut self) -> usize {
        let mut wraps = 0;
        let mut lines_count = 0;
        for line in self.buffer.lines().skip(self.state.row_offset) {
            let wrap = (line.len() as u16) / self.size.0;
            wraps += wrap;
            lines_count += 1 + wrap;
            if lines_count >= self.size.1 - 2 {
                break;
            }
        }
        wraps as usize
    }

    pub(crate) fn set_buffer(&mut self, b: Buffer) {
        self.buffer = b;
    }
}
