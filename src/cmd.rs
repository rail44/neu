use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit0},
    combinator::map,
    multi::many_till,
    sequence::pair,
    IResult,
};

use crate::action::{Action, ActionKind, EditKind, MovementKind};
use crate::selection::{Selection, SelectionKind};

fn edit(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(tag("cc"), |_| {
            ActionKind::IntoEditMode(SelectionKind::Line.once())
        }),
        map(pair(tag("c"), selection), |(_, s)| {
            ActionKind::IntoEditMode(s)
        }),
        map(tag("C"), |_| {
            ActionKind::IntoEditMode(SelectionKind::LineRemain.once())
        }),
    ))(input)
}

fn remove(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(tag("dd"), |_| {
            EditKind::RemoveSelection(SelectionKind::Line.once()).into()
        }),
        map(pair(tag("d"), selection), |(_, s)| {
            EditKind::RemoveSelection(s).into()
        }),
        map(tag("D"), |_| {
            EditKind::RemoveSelection(SelectionKind::LineRemain.once()).into()
        }),
    ))(input)
}

fn yank(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(alt((tag("yy"), tag("Y"))), |_| {
            ActionKind::Yank(SelectionKind::Line.once())
        }),
        map(pair(tag("y"), selection), |(_, s)| ActionKind::Yank(s)),
    ))(input)
}

fn movement_kind(input: &str) -> IResult<&str, MovementKind> {
    alt((
        map(tag("<C-f>"), |_| MovementKind::ScollScreenDown),
        map(tag("<C-b>"), |_| MovementKind::ScollScreenUp),
        map(tag("^"), |_| MovementKind::MoveToLineIndentHead),
        map(tag("$"), |_| MovementKind::MoveToLineTail),
        map(tag("gg"), |_| MovementKind::MoveLine),
        map(tag("G"), |_| MovementKind::MoveToTail),
        map(alt((tag("h"), tag("<Left>"))), |_| MovementKind::CursorLeft),
        map(alt((tag("j"), tag("<Down>"))), |_| MovementKind::CursorDown),
        map(alt((tag("k"), tag("<Up>"))), |_| MovementKind::CursorUp),
        map(alt((tag("l"), tag("<Right>"))), |_| {
            MovementKind::CursorRight
        }),
        map(tag("w"), |_| MovementKind::ForwardWord),
        map(tag("b"), |_| MovementKind::BackWord),
        map(tag("n"), |_| MovementKind::GoToNextMatch),
        map(tag("N"), |_| MovementKind::GoToPrevMatch),
    ))(input)
}

fn action_kind(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(movement_kind, |k| k.into()),
        map(tag("x"), |_| EditKind::RemoveChar.into()),
        map(tag("u"), |_| ActionKind::Undo),
        map(tag("<C-r>"), |_| ActionKind::Redo),
        map(tag("i"), |_| ActionKind::IntoInsertMode),
        map(tag("a"), |_| ActionKind::IntoAppendMode),
        map(tag(":"), |_| ActionKind::IntoCmdLineMode),
        map(tag("/"), |_| ActionKind::IntoSearchMode),
        map(tag("p"), |_| EditKind::AppendYank.into()),
        map(tag("P"), |_| EditKind::InsertYank.into()),
        map(tag("."), |_| ActionKind::Repeat),
        remove,
        edit,
        yank,
        map(
            many_till(anychar, alt((tag("<C-c>"), tag("<Esc>")))),
            |_| ActionKind::ClearCmd,
        ),
    ))(input)
}

fn cmd(input: &str) -> IResult<&str, Action> {
    map(pair(digit0, action_kind), |(n, kind)| {
        let count = n.parse().unwrap_or(1);
        Action { count, kind }
    })(input)
}

pub(super) fn parse(input: &str) -> IResult<&str, Action> {
    cmd(input)
}

fn selection_kind(input: &str) -> IResult<&str, SelectionKind> {
    use SelectionKind::*;
    alt((
        map(alt((tag("h"), tag("<Left>"))), |_| Left),
        map(alt((tag("j"), tag("<Down>"))), |_| Down),
        map(alt((tag("k"), tag("<Up>"))), |_| Up),
        map(alt((tag("l"), tag("<Right>"))), |_| Right),
        map(tag("w"), |_| ForwardWord),
        map(tag("b"), |_| BackWord),
        map(tag("iw"), |_| Word),
    ))(input)
}

fn selection(input: &str) -> IResult<&str, Selection> {
    map(pair(digit0, selection_kind), |(n, kind)| {
        let count = n.parse().unwrap_or(1);
        Selection { count, kind }
    })(input)
}
