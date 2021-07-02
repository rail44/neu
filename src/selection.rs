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
