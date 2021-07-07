use crate::action::{Action, ActionKind};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, space1},
    combinator::map,
    multi::many1,
    sequence::separated_pair,
    IResult,
};

fn cmdline(input: &str) -> IResult<&str, Action> {
    use ActionKind::*;
    alt((
        map(tag("w"), |_| Save.once()),
        map(
            separated_pair(tag("w"), space1, many1(anychar)),
            |(_, arg)| WriteOut(arg.iter().collect()).once(),
        ),
        map(tag("q"), |_| Quit.once()),
    ))(input)
}

pub(super) fn parse(input: &str) -> IResult<&str, Action> {
    cmdline(input)
}
