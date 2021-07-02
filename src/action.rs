use crate::state::State;
use flume::Sender;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Selection {
    pub(super) count: usize,
    pub(super) kind: SelectionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SelectionKind {
    Left,
    Down,
    Up,
    Right,
    ForwardWord,
    BackWord,
    Word,
    Line,
    LineRemain,
}

impl SelectionKind {
    pub(super) fn once(self) -> Selection {
        Selection {
            count: 1,
            kind: self,
        }
    }

    pub(super) fn nth(self, n: usize) -> Selection {
        Selection {
            count: n,
            kind: self,
        }
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

#[derive(Clone, Debug, PartialEq)]
pub(super) enum EditKind {
    LineBreak,
    InsertChar(char),
    RemoveChar,
    Remove(Selection),
    AppendYank,
    InsertYank,
    Insert(String),
    Edit(Selection, String),
}

impl EditKind {
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

impl From<EditKind> for ActionKind {
    fn from(e: EditKind) -> Self {
        Self::Edit(e)
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
