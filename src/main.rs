use core::cmp::min;
use std::fs::File;
use std::io::{stdin, stdout, BufWriter, Stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::terminal_size;

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

struct Buffer(Vec<Vec<char>>);

impl Buffer {
    fn new() -> Self {
        Buffer(vec![vec![
            'h', 'e', 'l', 'l', 'o', ' ', 'w', 'o', 'r', 'l', 'd',
        ]])
    }

    fn row_len(&self, row: usize) -> usize {
        self.0[row].len()
    }

    fn count_lines(&self) -> usize {
        self.0.len()
    }

    fn lines(&self) -> impl Iterator<Item = &Vec<char>> + '_ {
        self.0.iter()
    }

    fn insert_return(&mut self, col: usize, row: usize) {
        let rest: Vec<char> = self.0[row].drain(col..).collect();
        self.0.insert(row + 1, rest);
    }

    fn insert_char(&mut self, col: usize, row: usize, c: char) {
        self.0[row].insert(col, c);
    }
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
            for c in line {
                write!(self.stdout, "{}", c).unwrap();
            }
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

    fn handle_normal_mode(&mut self, k: Key) -> Signal {
        match k {
            Key::Char('h') => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            Key::Char('j') => {
                if self.cursor.row + 1 < self.buffer.count_lines() {
                    self.cursor.row += 1;
                    self.cursor.col = min(self.buffer.row_len(self.cursor.row), self.cursor.col);
                }
            }
            Key::Char('k') => {
                if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                    self.cursor.col = min(self.buffer.row_len(self.cursor.row), self.cursor.col);
                }
            }
            Key::Char('l') => {
                self.cursor.col = min(self.cursor.col + 1, self.buffer.row_len(self.cursor.row));
            }
            Key::Char('i') => {
                self.mode = Mode::Insert;
            }
            Key::Char('a') => {
                self.cursor.col = min(self.cursor.col + 1, self.buffer.row_len(self.cursor.row));
                self.mode = Mode::Insert;
            }
            Key::Ctrl('q') => return Signal::Quit,
            Key::Ctrl('w') => {
                let f = File::create("/tmp/hoge").unwrap();
                let mut w = BufWriter::new(f);
                for line in self.buffer.lines() {
                    for c in line {
                        write!(w, "{}", c).unwrap();
                    }
                    write!(w, "\r\n").unwrap();
                }
            }
            _ => {}
        };
        Signal::Nope
    }

    fn handle_insert_mode(&mut self, k: Key) {
        match k {
            Key::Char(c) => {
                if c == '\n' {
                    self.buffer.insert_return(self.cursor.col, self.cursor.row);
                    self.cursor.row += 1;
                    self.cursor.col = 0;
                    // scroll();
                    return;
                }
                self.buffer.insert_char(self.cursor.col, self.cursor.row, c);
                self.cursor.col += 1;
            }
            Key::Esc => {
                self.mode = Mode::Normal;
            }
            Key::Ctrl('c') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn run(&mut self) {
        self.draw();
        let stdin = stdin();
        for c in stdin.keys() {
            write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
            self.stdout.flush().unwrap();

            match self.mode {
                Mode::Normal => {
                    if Signal::Quit == self.handle_normal_mode(c.unwrap()) {
                        break;
                    }
                }
                Mode::Insert => self.handle_insert_mode(c.unwrap()),
            }
            self.draw();
        }
    }
}

fn main() {
    let mut editor = Editor::default();
    editor.run();
}
