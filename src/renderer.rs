use crate::buffer::Buffer;
use crate::compute::{
    Compute, CurrentLine, LineRange, MaxLineDigit, Reactor, RowOffset, TerminalHeight,
};
use crate::mode::Mode;
use crate::state::Cursor;
use core::cmp::min;
use std::io::{stdout, BufWriter, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use tree_sitter::Point;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Clone, Debug)]
struct TextAreaProps {
    line_range: (usize, usize),
    buffer: Buffer,
    max_line_digit: usize,
}

impl Compute for TextAreaProps {
    type Source = (LineRange, Buffer, MaxLineDigit);
    fn compute(source: &Self::Source) -> Self {
        Self {
            line_range: (source.0 .0, source.0 .1),
            buffer: source.1.clone(),
            max_line_digit: source.2 .0,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct LineNumberProps {
    max_line_digit: usize,
    line_range: (usize, usize),
}

impl Compute for LineNumberProps {
    type Source = (MaxLineDigit, LineRange);
    fn compute(source: &Self::Source) -> Self {
        Self {
            max_line_digit: source.0 .0,
            line_range: (source.1 .0, source.1 .1),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct CursorProps {
    cursor: Cursor,
    current_line: String,
    max_line_digit: usize,
    row_offset: usize,
}

impl Compute for CursorProps {
    type Source = (Cursor, CurrentLine, MaxLineDigit, RowOffset);
    fn compute(source: &Self::Source) -> Self {
        Self {
            cursor: source.0.clone(),
            current_line: source.1 .0.clone(),
            max_line_digit: source.2 .0,
            row_offset: source.3 .0,
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
}

impl Renderer {
    pub(crate) fn new() -> Self {
        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(stdout, "{}", termion::screen::ToAlternateScreen).unwrap();
        write!(stdout, "{}", termion::clear::All).unwrap();
        stdout.flush().unwrap();
        Self { stdout }
    }
}

impl Renderer {
    pub(crate) fn render(&mut self, reactor: &mut Reactor, highlights: Vec<(Point, String)>) {
        write!(self.stdout, "{}", termion::clear::All).unwrap();

        let props = reactor.compute();
        self.render_text_area(props, highlights);

        let props = reactor.compute();
        self.render_line_number(props);

        let props = reactor.compute();
        self.render_status_line(props);

        let props = reactor.compute();
        self.render_cursor(props);

        self.stdout.flush().unwrap();
    }

    fn render_text_area(&mut self, props: TextAreaProps, highlights: Vec<(Point, String)>) {
        let max_line_digit = props.max_line_digit;
        for (i, line) in props
            .buffer
            .lines_at(props.line_range.0)
            .take(props.line_range.1 - props.line_range.0)
            .enumerate()
        {
            write!(
                self.stdout,
                "{}",
                termion::cursor::Goto(max_line_digit as u16 + 2, (i + 1) as u16),
            )
            .unwrap();
            write!(self.stdout, "{}", line.as_str()).unwrap();
        }

        for highlight in highlights {
            let position = highlight.0;
            write!(
                self.stdout,
                "{}",
                termion::cursor::Goto(
                    max_line_digit as u16 + 2 + position.column as u16,
                    position.row as u16 - props.line_range.0 as u16 + 1
                ),
            )
            .unwrap();
            for (i, l) in highlight.1.lines().enumerate() {
                write!(self.stdout, "{}", l,).unwrap();
                write!(
                    self.stdout,
                    "{}",
                    termion::cursor::Goto(
                        max_line_digit as u16 + 2,
                        position.row as u16 - props.line_range.0 as u16 + 2 + i as u16
                    ),
                )
                .unwrap();
            }
        }
    }

    fn render_line_number(&mut self, props: LineNumberProps) {
        let max_line_digit = props.max_line_digit;
        let line_range = props.line_range;
        for (i, line_index) in (line_range.0..line_range.1).enumerate() {
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
                    termion::cursor::Goto(0, props.terminal_height as u16 + 1),
                    cmd
                )
                .unwrap();
            }
        };
    }

    fn render_cursor(&mut self, props: CursorProps) {
        let cursor = props.cursor;
        let row_offset = props.row_offset;
        let col = cursor.col;
        let row = cursor.row - row_offset;

        let current_line = props.current_line;
        let s: String = current_line
            .chars()
            .take(min(col + 1, current_line.chars().count() - 1))
            .collect();
        let width = UnicodeWidthStr::width(s.as_str());
        let col_pos = width + (col + 1 - s.chars().count());

        let max_line_digit = props.max_line_digit;
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto((max_line_digit + 1 + col_pos) as u16, row as u16 + 1)
        )
        .unwrap();
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        write!(
            self.stdout,
            "{}{}",
            termion::clear::All,
            termion::screen::ToMainScreen
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }
}
