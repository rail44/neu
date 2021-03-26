use core::cmp::min;
use std::fs::File;
use std::io::{stdin, stdout, BufWriter, Stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::terminal_size;

mod buffer;
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
    Normal,
    Insert,
}

struct Editor {
    size: (u16, u16),
    mode: Mode,
    cursor: Cursor,
    buffer: Buffer,
    stdout: RawTerminal<Stdout>,
}

impl Default for Editor {
    fn default() -> Self {
        let cursor = Cursor::default();
        let mode = Mode::Normal;

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
            cursor,
            buffer: Buffer::new(),
            stdout,
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
        match self.mode {
            Mode::Normal => {
                write!(self.stdout, "{}NORMAL", termion::cursor::SteadyBlock).unwrap();
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

    fn handle_normal_mode(&mut self, cmd: &mut Vec<Key>) -> Signal {
        match cmd.as_slice() {
            [Key::Char('h')] => {
                self.cursor.col = self.cursor.col.saturating_sub(1);
                cmd.clear();
            }
            [Key::Char('j')] => {
                self.cursor.row += 1;
                cmd.clear();
            }
            [Key::Char('k')] => {
                self.cursor.row = self.cursor.row.saturating_sub(1);
                cmd.clear();
            }
            [Key::Char('l')] => {
                self.cursor.col += 1;
                cmd.clear();
            }
            [Key::Char('i')] => {
                self.mode = Mode::Insert;
                cmd.clear();
            }
            [Key::Char('a')] => {
                self.cursor.col += 1;
                self.mode = Mode::Insert;
                cmd.clear();
            }
            [Key::Ctrl('q')] => return Signal::Quit,
            [Key::Ctrl('w')] => {
                let f = File::create("/tmp/hoge").unwrap();
                let mut w = BufWriter::new(f);
                write!(w, "{}", self.buffer.as_str()).unwrap();
                cmd.clear();
            }
            [Key::Char('d'), Key::Char('d')] => {
                self.buffer.remove_line(self.cursor.row);
                cmd.clear();
            }
            [.., Key::Esc] | [.., Key::Ctrl('c')] => {
                cmd.clear();
            }
            _ => {}
        };
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
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn run(&mut self) {
        self.draw();
        let stdin = stdin();
        let mut cmd = Vec::new();
        for c in stdin.keys() {
            write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();

            match self.mode {
                Mode::Normal => {
                    cmd.push(c.unwrap());
                    let signal = self.handle_normal_mode(&mut cmd);
                    if Signal::Quit == signal {
                        break;
                    }
                }
                Mode::Insert => self.handle_insert_mode(c.unwrap()),
            }
            self.cursor.row = min(self.cursor.row, self.buffer.lines().count() - 1);
            self.cursor.col = min(self.cursor.col, self.buffer.row_len(self.cursor.row));
            self.draw();
        }
    }
}

fn main() {
    let mut editor = Editor::default();
    editor.run();
}
