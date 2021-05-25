use crate::buffer::Buffer;
use termion::terminal_size;

#[derive(Clone, Debug)]
pub(crate) enum Mode {
    Normal(String),
    Insert,
    CmdLine(String),
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal(String::new())
    }
}

impl Mode {
    pub(crate) fn get_cmd(&self) -> &String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }

        panic!();
    }

    pub(crate) fn get_cmd_mut(&mut self) -> &mut String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }
        panic!();
    }

    pub(crate) fn get_cmdline(&self) -> &String {
        if let Mode::CmdLine(cmd) = self {
            return cmd;
        }
        panic!();
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct Cursor {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Default, Clone, Debug)]
pub(crate) struct State {
    pub(crate) row_offset: usize,
    pub(crate) cursor: Cursor,
    pub(crate) mode: Mode,
    pub(crate) yanked: Buffer,
    pub(crate) size: (u16, u16),
    pub(crate) buffer: Buffer,
}

impl State {
    pub(crate) fn new() -> Self {
        let size = terminal_size().unwrap();

        Self {
            size,
            ..Default::default()
        }
    }

    pub(crate) fn count_word_back(&self) -> usize {
        self.buffer
            .count_back_word(self.cursor.col, self.cursor.row + self.row_offset)
    }

    pub(crate) fn count_word_forward(&self) -> usize {
        self.buffer
            .count_forward_word(self.cursor.col, self.cursor.row + self.row_offset)
    }
}
