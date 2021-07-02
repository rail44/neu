use crate::selection::Selection;
use crate::state::State;
use flume::Sender;

pub(super) use crate::edit::EditActionKind as EditKind;

impl From<EditKind> for ActionKind {
    fn from(e: EditKind) -> Self {
        Self::Edit(e)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Action {
    pub(super) count: usize,
    pub(super) kind: ActionKind,
}

#[derive(Clone, Debug)]
pub(super) enum MovementKind {
    CursorLeft,
    CursorDown,
    CursorUp,
    CursorRight,
    CursorLineHead,
    ForwardWord,
    BackWord,
    MoveTo(usize),
    MoveLine,
    MoveToTail,
    ScollScreenUp,
    ScollScreenDown,
    MoveToLineTail,
    MoveToLineIndentHead,
    MoveAsSeenOnView,
    GoToNextMatch,
    GoToPrevMatch,
}

impl MovementKind {
    pub(super) fn once(self) -> Action {
        Action {
            count: 1,
            kind: self.into(),
        }
    }

    pub(super) fn nth(self, n: usize) -> Action {
        Action {
            count: n,
            kind: self.into(),
        }
    }
}

impl From<MovementKind> for ActionKind {
    fn from(m: MovementKind) -> Self {
        Self::Movement(m)
    }
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
    pub(super) fn edit(k: EditKind) -> Action {
        Action {
            count: 1,
            kind: k.into(),
        }
    }

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
