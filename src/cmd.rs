use nom::{
    IResult,
    combinator::map,
    branch::alt,
    bytes::complete::tag,
    character::complete::anychar,
    multi::many_till,
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
        map(tag("<C-q>"), |_| Quit),
        map(tag("dd"), |_| RemoveLine),
        map(tag("yy"), |_| YankLine),
        map(tag("p"), |_| AppendYank),
        map(tag("P"), |_| InsertYank),
        map(many_till(anychar, alt((tag("<C-c>"), tag("<Esc>")))), |_| Escape),
    ))(input)
}

fn cmd(input: &str) -> IResult<&str, Cmd> {
    map(cmd_kind, |kind| Cmd { count: 1, kind })(input)
}


pub(crate) fn parse(input: &str) -> IResult<&str, Cmd> {
    cmd(input)
}
