use crate::selection::Selection;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ActionKind {
    LineBreak,
    InsertChar(char),
    RemoveChar,
    Remove(Selection),
    AppendYank,
    InsertYank,
    Insert(String),
    Edit(Selection, String),
}
