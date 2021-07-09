#[derive(Clone, Debug)]
pub(crate) enum MovementKind {
    Left,
    Down,
    Up,
    Right,
    ForwardWord,
    BackWord,
    Line,
    Tail,
    ScreenUp,
    ScreenDown,
    LineTail,
    LineHead,
    IndentHead,
    AsSeenOnView,
    NextMatch,
    PrevMatch,
}
