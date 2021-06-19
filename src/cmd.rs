use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit0},
    combinator::map,
    multi::many_till,
    sequence::pair,
    IResult,
};

use crate::action::{Action, ActionKind, EditKind, MovementKind, SelectionKind};
use crate::selection;

fn remove(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(tag("dd"), |_| {
            EditKind::Remove(SelectionKind::Line.once()).into()
        }),
        map(pair(tag("d"), selection::parse), |(_, s)| {
            EditKind::Remove(s).into()
        }),
        map(tag("D"), |_| {
            EditKind::Remove(SelectionKind::LineRemain.once()).into()
        }),
    ))(input)
}

fn yank(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(alt((tag("yy"), tag("Y"))), |_| {
            ActionKind::Yank(SelectionKind::Line.once())
        }),
        map(pair(tag("y"), selection::parse), |(_, s)| {
            ActionKind::Yank(s)
        }),
    ))(input)
}

fn action_kind(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(tag("<C-f>"), |_| MovementKind::ScollScreenDown.into()),
        map(tag("<C-b>"), |_| MovementKind::ScollScreenUp.into()),
        map(tag("x"), |_| EditKind::RemoveChar.into()),
        map(tag("i"), |_| ActionKind::IntoInsertMode),
        map(tag("a"), |_| ActionKind::IntoAppendMode),
        map(tag(":"), |_| ActionKind::IntoCmdLineMode),
        map(tag("p"), |_| EditKind::AppendYank.into()),
        map(tag("P"), |_| EditKind::InsertYank.into()),
        map(tag("."), |_| ActionKind::Repeat),
        map(tag("gg"), |_| MovementKind::MoveLine.into()),
        map(tag("G"), |_| MovementKind::MoveToTail.into()),
        map(alt((tag("h"), tag("<Left>"))), |_| {
            MovementKind::CursorLeft.into()
        }),
        map(alt((tag("j"), tag("<Down>"))), |_| {
            MovementKind::CursorDown.into()
        }),
        map(alt((tag("k"), tag("<Up>"))), |_| {
            MovementKind::CursorUp.into()
        }),
        map(alt((tag("l"), tag("<Right>"))), |_| {
            MovementKind::CursorRight.into()
        }),
        map(tag("w"), |_| MovementKind::ForwardWord.into()),
        map(tag("b"), |_| MovementKind::BackWord.into()),
        remove,
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

pub(crate) fn parse(input: &str) -> IResult<&str, Action> {
    cmd(input)
}
