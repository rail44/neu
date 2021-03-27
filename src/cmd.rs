use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, digit0},
    combinator::map,
    multi::many_till,
    sequence::pair,
    IResult,
};

pub(crate) struct Cmd {
    pub(crate) count: usize,
    pub(crate) kind: CmdKind,
}

pub(crate) enum CmdKind {
    CursorLeft,
    CursorDown,
    CursorUp,
    CursorRight,
    IntoInsertMode,
    IntoAppendMode,
    IntoCmdLineMode,
    RemoveChar,
    Quit,
    RemoveLine,
    YankLine,
    AppendYank,
    InsertYank,
    Escape,
}

fn cmd_kind(input: &str) -> IResult<&str, CmdKind> {
    use CmdKind::*;
    alt((
        map(tag("h"), |_| CursorLeft),
        map(tag("j"), |_| CursorDown),
        map(tag("k"), |_| CursorUp),
        map(tag("l"), |_| CursorRight),
        map(tag("i"), |_| IntoInsertMode),
        map(tag("a"), |_| IntoAppendMode),
        map(tag(":"), |_| IntoCmdLineMode),
        map(tag("x"), |_| RemoveChar),
        map(tag("<C-q>"), |_| Quit),
        map(tag("dd"), |_| RemoveLine),
        map(tag("yy"), |_| YankLine),
        map(tag("p"), |_| AppendYank),
        map(tag("P"), |_| InsertYank),
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
