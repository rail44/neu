use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit0},
    combinator::map,
    multi::many_till,
    sequence::pair,
    IResult,
};

use crate::selection;
use crate::selection::Selection;

pub(crate) struct Cmd {
    pub(crate) count: usize,
    pub(crate) kind: CmdKind,
}

pub(crate) enum CmdKind {
    RemoveChar,
    IntoInsertMode,
    IntoAppendMode,
    IntoCmdLineMode,
    AppendYank,
    InsertYank,
    Escape,
    CursorLeft,
    CursorDown,
    CursorUp,
    CursorRight,
    ForwardWord,
    BackWord,
    Remove(Selection),
    Yank(Selection),
}

fn remove(input: &str) -> IResult<&str, CmdKind> {
    map(pair(tag("d"), selection::parse), |(_, s)| {
        CmdKind::Remove(s)
    })(input)
}

fn yank(input: &str) -> IResult<&str, CmdKind> {
    map(pair(tag("y"), selection::parse), |(_, s)| CmdKind::Yank(s))(input)
}

fn cmd_kind(input: &str) -> IResult<&str, CmdKind> {
    use CmdKind::*;
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
            |_| Escape,
        ),
    ))(input)
}

fn cmd(input: &str) -> IResult<&str, Cmd> {
    map(pair(digit0, cmd_kind), |(n, kind)| {
        let count = n.parse().unwrap_or(1);
        Cmd { count, kind }
    })(input)
}

pub(crate) fn parse(input: &str) -> IResult<&str, Cmd> {
    cmd(input)
}
