use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit0},
    combinator::map,
    multi::many_till,
    sequence::pair,
    IResult,
};

use crate::action::{Action, ActionKind, SelectionKind};
use crate::selection;

fn remove(input: &str) -> IResult<&str, ActionKind> {
    alt((
        map(tag("dd"), |_| {
            ActionKind::Remove(SelectionKind::Line.once())
        }),
        map(pair(tag("d"), selection::parse), |(_, s)| {
            ActionKind::Remove(s)
        }),
    ))(input)
}

fn yank(input: &str) -> IResult<&str, ActionKind> {
    map(pair(tag("y"), selection::parse), |(_, s)| {
        ActionKind::Yank(s)
    })(input)
}

fn action_kind(input: &str) -> IResult<&str, ActionKind> {
    use ActionKind::*;
    alt((
        map(tag("x"), |_| RemoveChar),
        map(tag("i"), |_| IntoInsertMode),
        map(tag("a"), |_| IntoAppendMode),
        map(tag(":"), |_| IntoCmdLineMode),
        map(tag("p"), |_| AppendYank),
        map(tag("P"), |_| InsertYank),
        map(alt((tag("h"), tag("<Left>"))), |_| CursorLeft),
        map(alt((tag("j"), tag("<Down>"))), |_| CursorDown),
        map(alt((tag("k"), tag("<Up>"))), |_| CursorUp),
        map(alt((tag("l"), tag("<Right>"))), |_| CursorRight),
        map(tag("w"), |_| ForwardWord),
        map(tag("b"), |_| BackWord),
        remove,
        yank,
        map(
            many_till(anychar, alt((tag("<C-c>"), tag("<Esc>")))),
            |_| ClearCmd,
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
