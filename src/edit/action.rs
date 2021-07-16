use crate::movement::MovementKind;
use crate::selection::Selection;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EditKind {
    LineBreak,
    InsertChar(char),
    RemoveChar,
    RemoveSelection(Selection),
    AppendYank,
    InsertYank,
    InsertString(Option<MovementKind>, String),
    Edit(Selection, String),
}
