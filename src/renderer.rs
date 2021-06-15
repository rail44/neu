use crate::compute::{ComputableFromState, Compute, CurrentLine, MaxLineDigit};
use crate::mode::Mode;
use crate::state::{Cursor, State};
use core::cmp::max;
use std::io::{stdout, BufWriter, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct CursorProps {
    cursor: Cursor,
    current_line: CurrentLine,
    max_line_digit: MaxLineDigit,
}

impl Compute for CursorProps {
    type Source = State;
    fn compute(source: &State) -> Self {
        Self {
            cursor: Cursor::compute_from_state(source),
            current_line: CurrentLine::compute_from_state(source),
            max_line_digit: MaxLineDigit::compute_from_state(source),
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
        stdout.flush().unwrap();
        Self { stdout }
    }
}

impl Renderer {
    pub(crate) fn render(&mut self, state: &State) {
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

        let props = CursorProps::compute_from_state(state);
        self.render_cursor(props);

        self.stdout.flush().unwrap();
    }

    fn render_cursor(&mut self, props: CursorProps) {
        let cursor = props.cursor;
        let col = cursor.col;
        let row = cursor.row;

        let current_line = props.current_line.0;
        let end = current_line.char_indices().take(col + 2).last().unwrap().0;
        let s = &current_line[..end];
        let width = UnicodeWidthStr::width(s);

        let max_line_digit = props.max_line_digit.0;
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto((max_line_digit + 2 + max(1, width)) as u16, row as u16 + 1)
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
