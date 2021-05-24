use crate::store::{Mode, State};
use std::io::{stdout, BufWriter, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use xtra::prelude::*;

pub(crate) struct Renderer {
    stdout: BufWriter<RawTerminal<Stdout>>,
}

impl Renderer {
    pub(crate) fn new() -> Self {
        let mut stdout = BufWriter::new(stdout().into_raw_mode().unwrap());
        write!(stdout, "{}", termion::clear::All,).unwrap();
        Self { stdout }
    }
}

impl Actor for Renderer {}

pub(crate) struct Render(pub(crate) State);

impl Message for Render {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<Render> for Renderer {
    async fn handle(&mut self, msg: Render, _ctx: &mut Context<Self>) {
        let state = msg.0;

        write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
        write!(self.stdout, "{}", termion::clear::All).unwrap();
        let mut wraps = 0;
        let mut drawed_lines_count = 0;
        let textarea_row = state.size.1 - 2;
        for (i, line) in state.buffer.lines().skip(state.row_offset).enumerate() {
            let wrap = (line.len() as u16) / state.size.0;
            drawed_lines_count += 1;

            let line = if drawed_lines_count >= textarea_row && i != state.cursor.row {
                let s: String = line.chars().take(state.size.0 as usize).collect();
                s.into()
            } else {
                line
            };
            write!(self.stdout, "{}\r\n", line).unwrap();

            if i < state.cursor.row {
                wraps += wrap
            }
            drawed_lines_count += wrap;
            if drawed_lines_count >= textarea_row {
                break;
            }
        }
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto(0, state.size.1 - 1)
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
        let col = state.cursor.col as u16 % state.size.0;
        let row = state.cursor.row as u16 + state.cursor.col as u16 / state.size.0 + wraps;
        write!(self.stdout, "{}", termion::cursor::Goto(col + 1, row + 1)).unwrap();
        self.stdout.flush().unwrap();
    }
}
