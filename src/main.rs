use core::cmp::min;
use std::fs::File;
use std::io::{stdin, stdout, BufWriter, Stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::terminal_size;

mod buffer;
mod cmd;
use crate::buffer::Buffer;

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
}

impl Mode {
    fn get_mut_cmd(&mut self) -> &mut String {
        if let Mode::Normal(ref mut cmd) = self {
            return cmd;
        }
        panic!();
    }

    fn get_cmd(&self) -> &String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }
        panic!();
    }
}

struct Editor {
    size: (u16, u16),
    mode: Mode,
    cursor: Cursor,
    buffer: Buffer,
    stdout: RawTerminal<Stdout>,
    yanked: Buffer,
}

impl Default for Editor {
    fn default() -> Self {
        let mode = Mode::Normal(String::new());

        let mut stdout = stdout().into_raw_mode().unwrap();
        write!(
            stdout,
            "{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();
        let size = terminal_size().unwrap();

        Editor {
            mode,
            size,
            stdout,
            buffer: Buffer::new(),
            cursor: Cursor::default(),
            yanked: Buffer::default(),
        }
    }
}

#[derive(PartialEq)]
enum Signal {
    Nope,
    Quit,
}

impl Editor {
    fn draw(&mut self) {
        write!(self.stdout, "{}", termion::clear::All).unwrap();
        for line in self.buffer.lines() {
            write!(self.stdout, "{}", line).unwrap();
            write!(self.stdout, "\r\n").unwrap();
        }
        write!(self.stdout, "{}", termion::cursor::Goto(0, self.size.1)).unwrap();
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
        };
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto(self.cursor.col as u16 + 1, self.cursor.row as u16 + 1)
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }

    fn handle_normal_mode(&mut self) -> Signal {
        let parsed = cmd::parse(self.mode.get_cmd());
        if parsed.is_err() {
            return Signal::Nope;
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
                self.cursor.row = self.cursor.row.saturating_sub(cmd.count);
            }
            CursorRight => {
                self.cursor.col += cmd.count;
            }
            IntoInsertMode => {
                self.mode = Mode::Insert;
            }
            IntoAppendMode => {
                self.cursor.col += 1;
                self.mode = Mode::Insert;
            }
            RemoveChar => {
                self.yanked = self
                    .buffer
                    .remove_chars(self.cursor.col, self.cursor.row, cmd.count);
            }
            Quit => return Signal::Quit,
            RemoveLine => {
                self.yanked = self.buffer.remove_lines(self.cursor.row, cmd.count);
            }
            YankLine => {
                self.yanked = self.buffer.subseq_lines(self.cursor.row, cmd.count);
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
                    self.buffer
                        .insert(col, self.cursor.row, self.yanked.clone());
                }
            }
            InsertYank => {
                let col = if self.yanked.end_with_line_break() {
                    0
                } else {
                    self.cursor.col
                };
                for _ in 0..cmd.count {
                    self.buffer
                        .insert(col, self.cursor.row, self.yanked.clone());
                }
            }
            Escape => {}
        }
        self.mode.get_mut_cmd().clear();
        Signal::Nope
    }

    fn handle_insert_mode(&mut self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.buffer
                        .insert_char(self.cursor.col, self.cursor.row, '\n');
                    self.cursor.row += 1;
                    self.cursor.col = 0;
                    // scroll();
                    return;
                }
                self.buffer.insert_char(self.cursor.col, self.cursor.row, c);
                self.cursor.col += 1;
            }
            Key::Esc | Key::Ctrl('c') => {
                self.mode = Mode::Normal(String::new());
            }
            _ => {}
        }
    }

    fn run(&mut self) {
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
                        Key::Esc => cmd.push_str("<Esc>"),
                        _ => {}
                    };
                    let signal = self.handle_normal_mode();
                    if Signal::Quit == signal {
                        break;
                    }
                }
                Mode::Insert => self.handle_insert_mode(k.unwrap()),
            }
            self.cursor.row = min(
                self.cursor.row,
                self.buffer.lines().count().saturating_sub(1),
            );
            self.cursor.col = min(self.cursor.col, self.buffer.row_len(self.cursor.row));
            self.draw();
        }
    }
}

fn main() {
    let mut editor = Editor::default();
    editor.run();
}
