use crate::selection::Selection;
use crate::state::State;
use flume::Sender;

use crate::edit::EditKind;
use crate::movement::MovementKind;

impl From<EditKind> for ActionKind {
    fn from(e: EditKind) -> Self {
        Self::Edit(e)
    }
}

impl From<MovementKind> for ActionKind {
    fn from(m: MovementKind) -> Self {
        Self::Movement(m)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Action {
    pub(super) count: usize,
    pub(super) kind: ActionKind,
}

#[derive(Clone, Debug)]
pub(super) enum ActionKind {
    Movement(MovementKind),
    Edit(EditKind),
    IntoAppendMode,
    IntoInsertMode,
    IntoNormalMode,
    IntoCmdLineMode,
    IntoSearchMode,
    IntoEditMode(Selection),
    SetYank(String),
    PushCmd(char),
    PushCmdStr(String),
    PopCmd,
    Yank(Selection),
    ClearCmd,
    Repeat,
    Save,
    WriteOut(String),
    Quit,
    GetState(Sender<State>),
    Undo,
    Redo,
    PushSearch(char),
    PopSearch,
    ClearSearch,
}

impl ActionKind {
    pub(super) fn once(self) -> Action {
        Action {
            count: 1,
            kind: self,
        }
    }

    pub(super) fn nth(self, n: usize) -> Action {
        Action {
            count: n,
            kind: self,
        }
    }
}
