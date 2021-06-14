use crate::mode::Mode;
use crate::state::State;
use core::cmp::{max, min};
use std::io::{stdout, BufWriter, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use unicode_width::UnicodeWidthStr;

pub(crate) struct Renderer {
    stdout: BufWriter<RawTerminal<Stdout>>,
}

impl Renderer {
    pub(crate) fn new() -> Self {
        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(stdout, "{}", termion::screen::ToAlternateScreen).unwrap();
        stdout.flush().unwrap();
        Self { stdout }
    }
}

impl Renderer {
    pub(crate) fn render(&mut self, state: &State) {
        tracing::error!("{:?}", state);
        write!(
            self.stdout,
            "{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All
        )
        .unwrap();
        let textarea_row = state.size.1 - 2;
        let line_count = state.buffer.count_lines();
        let max_line_digit = format!("{}", line_count).chars().count();
        for (i, line) in state
            .buffer
            .lines_at(state.row_offset)
            .take(textarea_row as usize)
            .enumerate()
        {
            write!(self.stdout, "{}", termion::cursor::Goto(1, (i + 1) as u16),).unwrap();
            write!(
                self.stdout,
                " {:max_line_digit$} {:>1}",
                state.row_offset + i + 1,
                line.as_str(),
                max_line_digit = max_line_digit
            )
            .unwrap();
        }
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto(1, state.size.1 - 1)
        )
        .unwrap();
        match &state.mode {
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
                    termion::cursor::Goto(0, state.size.1),
                    cmd
                )
                .unwrap();
            }
        };
        let col = state.cursor.col;
        let row = state.cursor.row;

        let line = state.buffer.line(row);

        let s = line
            .slice(..min(col + 1, line.count_chars()))
            .as_str()
            .to_string();
        let width = UnicodeWidthStr::width(s.as_str());

        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto((max_line_digit + 2 + max(1, width)) as u16, row as u16 + 1)
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }
}

// impl Drop for Renderer {
//     fn drop(&mut self) {
//         write!(
//             self.stdout,
//             "{}{}",
//             termion::clear::All,
//             termion::screen::ToMainScreen
//         )
//         .unwrap();
//         self.stdout.flush().unwrap();
//     }
// }
