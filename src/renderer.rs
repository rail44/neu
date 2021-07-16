use crate::buffer::Buffer;
use crate::compute::{
    Compute, CurrentLine, CursorView, LineRange, MatchPositionsInView, MaxLineDigit, Reactor,
    RowOffsetView, SearchPattern, TerminalHeight,
};
use crate::mode::Mode;
use crate::position::Position;
use crate::state::SearchDirection;
use core::cmp::min;
use std::io::{stdout, BufWriter, Stdout, Write};
use std::ops::Range;
use termion::raw::{IntoRawMode, RawTerminal};
use tree_sitter::Point;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Clone, Debug)]
struct TextAreaProps {
    line_range: Range<usize>,
    buffer: Buffer,
    max_line_digit: usize,
    match_positions: Vec<(Position, usize)>,
}

impl Compute for TextAreaProps {
    type Source = (LineRange, Buffer, MaxLineDigit, MatchPositionsInView);
    fn compute(source: &Self::Source) -> Self {
        Self {
            line_range: source.0 .0.clone(),
            buffer: source.1.clone(),
            max_line_digit: source.2 .0,
            match_positions: source.3 .0.clone(),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct LineNumberProps {
    max_line_digit: usize,
    line_range: Range<usize>,
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
    cursor: Position,
    current_line: String,
    max_line_digit: usize,
    row_offset: usize,
}

impl Compute for CursorProps {
    type Source = (CursorView, CurrentLine, MaxLineDigit, RowOffsetView);
    fn compute(source: &Self::Source) -> Self {
        Self {
            cursor: source.0 .0,
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
    search_pattern: String,
    search_direction: SearchDirection,
}

impl Compute for StatusLineProps {
    type Source = (Mode, TerminalHeight, SearchPattern, SearchDirection);
    fn compute(source: &Self::Source) -> Self {
        Self {
            mode: source.0.clone(),
            terminal_height: source.1 .0,
            search_pattern: source.2 .0.clone(),
            search_direction: source.3,
        }
    }
}

pub(super) struct Renderer {
    stdout: BufWriter<RawTerminal<Stdout>>,
}

impl Renderer {
    pub(super) fn new() -> Self {
        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(stdout, "{}", termion::screen::ToAlternateScreen).unwrap();
        write!(stdout, "{}", termion::clear::All).unwrap();
        stdout.flush().unwrap();
        Self { stdout }
    }
}

impl Renderer {
    pub(super) fn render(&mut self, reactor: &mut Reactor, highlights: Vec<(Point, String)>) {
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
            .lines_at(props.line_range.start)
            .take(props.line_range.end - props.line_range.start)
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
            let line = props.buffer.line(position.row);
            let s: Vec<u8> = line.bytes().take(position.column).collect();
            let width = UnicodeWidthStr::width(std::str::from_utf8(&s).unwrap());
            write!(
                self.stdout,
                "{}",
                termion::cursor::Goto(
                    max_line_digit as u16 + 2 + width as u16,
                    position.row as u16 - props.line_range.start as u16 + 1
                ),
            )
            .unwrap();
            for (i, l) in highlight.1.lines().enumerate() {
                if position.row + i + 1 > props.line_range.end {
                    write!(self.stdout, "{}", termion::color::Fg(termion::color::Reset)).unwrap();
                    break;
                }
                write!(self.stdout, "{}", l,).unwrap();
                write!(
                    self.stdout,
                    "{}",
                    termion::cursor::Goto(
                        max_line_digit as u16 + 2,
                        position.row as u16 - props.line_range.start as u16 + 2 + i as u16
                    ),
                )
                .unwrap();
            }
        }

        let match_positions = props.match_positions;
        for (position, length) in match_positions {
            let line = props.buffer.line(position.row + props.line_range.start);
            let s: String = line.chars().take(position.col + length).collect();
            let width = UnicodeWidthStr::width(&s[..min(s.len() - 1, position.col)]);
            write!(
                self.stdout,
                "{}{}{}{}",
                termion::cursor::Goto(
                    max_line_digit as u16 + 2 + width as u16,
                    position.row as u16 + 1
                ),
                termion::color::Bg(termion::color::Green),
                &s[min(s.len(), position.col)..],
                termion::color::Bg(termion::color::Reset)
            )
            .unwrap();
        }
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
            termion::cursor::Goto(1, props.terminal_height as u16 - 1)
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
            Mode::Insert(_, _) => {
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
            Mode::Search => {
                let c = match props.search_direction {
                    SearchDirection::Forward => '/',
                    SearchDirection::Reverse => '?',
                };
                write!(
                    self.stdout,
                    "{}SEARCH{}{}{}",
                    termion::cursor::SteadyBlock,
                    termion::cursor::Goto(0, props.terminal_height as u16 + 1),
                    c,
                    props.search_pattern
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
