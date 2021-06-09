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

    pub(crate) fn get_cmdline(&self) -> &String {
        if let Mode::CmdLine(cmd) = self {
            return cmd;
        }
        panic!();
    }
}
