#[derive(Clone, Debug)]
pub(crate) enum MovementKind {
    CursorLeft,
    CursorDown,
    CursorUp,
    CursorRight,
    ForwardWord,
    BackWord,
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
