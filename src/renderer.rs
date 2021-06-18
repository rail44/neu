use crate::compute::{Compute, CurrentLine, LineRange, MaxLineDigit, Reactor, TerminalHeight};
use crate::mode::Mode;
use crate::state::{Cursor, State};
use core::cmp::max;
use std::io::{stdout, BufWriter, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Clone, Debug)]
struct LineNumberProps {
    max_line_digit: usize,
    line_range: std::ops::Range<usize>,
}

impl Compute for LineNumberProps {
    type Source = (MaxLineDigit, LineRange);
    fn compute(source: &Self::Source) -> Self {
        Self {
            max_line_digit: source.0 .0,
            line_range: source.1 .0.clone(),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct CursorProps {
    cursor: Cursor,
    current_line: String,
    max_line_digit: usize,
}

impl Compute for CursorProps {
    type Source = (Cursor, CurrentLine, MaxLineDigit);
    fn compute(source: &Self::Source) -> Self {
        Self {
            cursor: source.0.clone(),
            current_line: source.1 .0.clone(),
            max_line_digit: source.2 .0,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct StatusLineProps {
    mode: Mode,
    terminal_height: usize,
}

impl Compute for StatusLineProps {
    type Source = (Mode, TerminalHeight);
    fn compute(source: &Self::Source) -> Self {
        Self {
            mode: source.0.clone(),
            terminal_height: source.1 .0,
        }
    }
}

pub(crate) struct Renderer {
    stdout: BufWriter<RawTerminal<Stdout>>,
    reactor: Reactor,
}

impl Renderer {
    pub(crate) fn new() -> Self {
        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(stdout, "{}", termion::screen::ToAlternateScreen).unwrap();
        stdout.flush().unwrap();
        Self {
            stdout,
            reactor: Reactor::new(),
        }
    }
}

impl Renderer {
    pub(crate) fn render(&mut self, state: &State) {
        self.reactor.load_state(state.clone());

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
            write!(
                self.stdout,
                "{}",
                termion::cursor::Goto(max_line_digit as u16 + 2, (i + 1) as u16),
            )
            .unwrap();
            write!(self.stdout, "{}", line.as_str(),).unwrap();
        }

        let props = self.reactor.compute();
        self.render_line_number(props);

        let props = self.reactor.compute();
        self.render_status_line(props);

        let props = self.reactor.compute();
        self.render_cursor(props);

        self.stdout.flush().unwrap();
    }

    fn render_line_number(&mut self, props: LineNumberProps) {
        let max_line_digit = props.max_line_digit;
        let line_range = props.line_range;
        for (i, line_index) in line_range.enumerate() {
            write!(self.stdout, "{}", termion::cursor::Goto(1, i as u16 + 1)).unwrap();
            write!(
                self.stdout,
                "{:max_line_digit$}",
                line_index + 1,
                max_line_digit = max_line_digit
            )
            .unwrap();
        }
    }

    fn render_status_line(&mut self, props: StatusLineProps) {
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto(1, props.terminal_height as u16)
        )
        .unwrap();
        match &props.mode {
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
                    termion::cursor::Goto(0, props.terminal_height as u16),
                    cmd
                )
                .unwrap();
            }
        };
    }

    fn render_cursor(&mut self, props: CursorProps) {
        let cursor = props.cursor;
        let col = cursor.col;
        let row = cursor.row;

        let current_line = props.current_line;
        let end = current_line
            .char_indices()
            .take(col + 2)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        let s = &current_line[..end];
        let width = UnicodeWidthStr::width(s);

        let max_line_digit = props.max_line_digit;
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto((max_line_digit + 1 + max(1, width)) as u16, row as u16 + 1)
        )
        .unwrap();
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
