use crate::buffer::Buffer;
use crate::compute::{
    Compute, CurrentLine, LineRange, MaxLineDigit, Reactor, RowOffset, TerminalHeight,
};
use crate::mode::Mode;
use crate::state::{Cursor, State};
use core::cmp::max;
use once_cell::sync::Lazy;
use std::io::{stdout, BufWriter, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use tree_sitter::{Node, Point};
use unicode_width::UnicodeWidthStr;

static QUERY: Lazy<tree_sitter::Query> = Lazy::new(|| {
    tree_sitter::Query::new(
        tree_sitter_rust::language(),
        tree_sitter_rust::HIGHLIGHT_QUERY,
    )
    .unwrap()
});

fn get_color(syntax_kind: &str) -> String {
    use termion::color;
    match syntax_kind {
        "keyword" => format!("{}", color::Fg(color::Magenta)),
        "property" => format!("{}", color::Fg(color::Red)),
        "function.method" => format!("{}", color::Fg(color::Blue)),
        "type" => format!("{}", color::Fg(color::Yellow)),
        s => {
            tracing::debug!("{}", s);
            format!("{}", color::Fg(color::Red))
        }
    }
}

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
    reactor: Reactor,
}

impl Renderer {
    pub(crate) fn new() -> Self {
        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(stdout, "{}", termion::screen::ToAlternateScreen).unwrap();
        write!(stdout, "{}", termion::clear::All).unwrap();
        stdout.flush().unwrap();
        Self {
            stdout,
            reactor: Reactor::new(),
        }
    }
}

impl Renderer {
    pub(crate) fn render(&mut self, state: &State, syntax_tree: &Node) {
        write!(self.stdout, "{}", termion::clear::All).unwrap();
        self.reactor.load_state(state.clone());

        let props = self.reactor.compute();
        self.render_text_area(props, syntax_tree);

        let props = self.reactor.compute();
        self.render_line_number(props);

        let props = self.reactor.compute();
        self.render_status_line(props);

        let props = self.reactor.compute();
        self.render_cursor(props);

        self.stdout.flush().unwrap();
    }

    fn render_text_area(&mut self, props: TextAreaProps, syntax_tree: &Node) {
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

        let mut c = tree_sitter::QueryCursor::new();
        c.set_point_range(
            Point::new(props.line_range.0, 0),
            Point::new(props.line_range.1, 0),
        );
        let source = "";
        let matches = c.captures(&QUERY, syntax_tree.clone(), |_| source.as_bytes());
        for matched in matches {
            tracing::debug!("{:?}", matched.0.captures);
            for capture in matched.0.captures {
                let start = capture.node.start_byte();
                let end = capture.node.end_byte();
                let syntax_kind = &(*QUERY).capture_names()[capture.index as usize];
                let position = capture.node.start_position();
                write!(
                    self.stdout,
                    "{}",
                    termion::cursor::Goto(
                        max_line_digit as u16 + 2 + position.column as u16,
                        position.row as u16 - props.line_range.0 as u16 + 1
                    ),
                )
                .unwrap();
                write!(
                    self.stdout,
                    "{}{}{}",
                    get_color(syntax_kind),
                    props.buffer.slice(start..end).as_str(),
                    termion::color::Fg(termion::color::Reset),
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
