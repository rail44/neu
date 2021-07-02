use crate::selection::Selection;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Mode {
    Normal(String),
    Insert(InsertKind, String),
    CmdLine(String),
    Search,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum InsertKind {
    Insert,
    Edit(Selection),
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal(String::new())
    }
}

impl Mode {
    pub(super) fn is_insert(&self) -> bool {
        if let Mode::Insert(_, _) = self {
            return true;
        }

        false
    }

    pub(super) fn get_cmd(&self) -> &String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }

        unreachable!();
    }

    pub(super) fn get_cmdline(&self) -> &String {
        if let Mode::CmdLine(cmd) = self {
            return cmd;
        }
        unreachable!();
    }
}
