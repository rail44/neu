use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, space1},
    combinator::map,
    multi::many0,
    sequence::separated_pair,
    IResult,
};

pub(crate) enum Cmd {
    Write(String),
    Quit,
}

fn cmd(input: &str) -> IResult<&str, Cmd> {
    use Cmd::*;
    alt((
        map(
            separated_pair(tag("w"), space1, many0(anychar)),
            |(_, arg)| Write(arg.iter().collect()),
        ),
        map(tag("q"), |_| Quit),
    ))(input)
}

pub(crate) fn parse(input: &str) -> IResult<&str, Cmd> {
    cmd(input)
}
