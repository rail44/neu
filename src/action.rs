use crate::state::State;
use flume::Sender;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Selection {
    pub(crate) count: usize,
    pub(crate) kind: SelectionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SelectionKind {
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
    pub(crate) fn once(self) -> Selection {
        Selection {
            count: 1,
            kind: self,
        }
    }

    pub(crate) fn nth(self, n: usize) -> Selection {
        Selection {
            count: n,
            kind: self,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Action {
    pub(crate) count: usize,
    pub(crate) kind: ActionKind,
}

#[derive(Clone, Debug)]
pub(crate) enum MovementKind {
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
}

impl MovementKind {
    pub(crate) fn once(self) -> Action {
        Action {
            count: 1,
            kind: self.into(),
        }
    }

    pub(crate) fn nth(self, n: usize) -> Action {
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
pub(crate) enum EditKind {
    LineBreak,
    InsertChar(char),
    RemoveChar,
    Remove(Selection),
    AppendYank,
    InsertYank,
}

impl EditKind {
    pub(crate) fn once(self) -> Action {
        Action {
            count: 1,
            kind: self.into(),
        }
    }

    pub(crate) fn nth(self, n: usize) -> Action {
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
pub(crate) enum ActionKind {
    Movement(MovementKind),
    Edit(EditKind),
    IntoAppendMode,
    IntoInsertMode,
    IntoNormalMode,
    IntoCmdLineMode,
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
}

impl ActionKind {
    pub(crate) fn once(self) -> Action {
        Action {
            count: 1,
            kind: self,
        }
    }

    pub(crate) fn nth(self, n: usize) -> Action {
        Action {
            count: n,
            kind: self,
        }
    }
}
