use crate::buffer::Buffer;

pub(crate) struct Selection {
    pub(crate) count: usize,
    pub(crate) kind: SelectionKind,
}

pub(crate) enum SelectionKind {
    Left,
    Down,
    Up,
    Right,
    ForwardWord,
    BackWord,
    Word,
    Line,
}

pub(crate) struct Action {
    pub(crate) count: usize,
    pub(crate) kind: ActionKind,
}

pub(crate) enum ActionKind {
    CursorLeft,
    CursorDown,
    CursorUp,
    CursorRight,
    CursorLineHead,
    IntoAppendMode,
    IntoInsertMode,
    IntoNormalMode,
    IntoCmdLineMode,
    SetYank(Buffer),
    LineBreak,
    InsertChar(char),
    PushCmd(char),
    PushCmdStr(String),
    PopCmd,
    Notify,
    ForwardWord,
    BackWord,
    RemoveChar,
    RemoveLine(usize),
    YankLine(usize),
    Remove(Selection),
    Yank(Selection),
    AppendYank,
    InsertYank,
    MoveTo(usize),
    ClearCmd,
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
