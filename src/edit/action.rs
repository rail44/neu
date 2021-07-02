use crate::selection::Selection;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ActionKind {
    LineBreak,
    InsertChar(char),
    RemoveChar,
    RemoveSelection(Selection),
    AppendYank,
    InsertYank,
    InsertString(String),
    Edit(Selection, String),
}
