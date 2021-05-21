use nom::{
    branch::alt, bytes::complete::tag, character::complete::digit0, combinator::map,
    sequence::pair, IResult,
};

pub(crate) struct Motion {
    pub(crate) count: usize,
    pub(crate) kind: MotionKind,
}

pub(crate) enum MotionKind {
    CursorLeft,
    CursorDown,
    CursorUp,
    CursorRight,
    ForwardWord,
    BackWord,
    Line,
}

fn motion_kind(input: &str) -> IResult<&str, MotionKind> {
    use MotionKind::*;
    alt((
        map(alt((tag("h"), tag("<Left>"))), |_| CursorLeft),
        map(alt((tag("j"), tag("<Down>"))), |_| CursorDown),
        map(alt((tag("k"), tag("<Up>"))), |_| CursorUp),
        map(alt((tag("l"), tag("<Right>"))), |_| CursorRight),
        map(tag("w"), |_| ForwardWord),
        map(tag("b"), |_| BackWord),
        map(tag("d"), |_| Line),
        map(tag("y"), |_| Line),
    ))(input)
}

fn motion(input: &str) -> IResult<&str, Motion> {
    map(pair(digit0, motion_kind), |(n, kind)| {
        let count = n.parse().unwrap_or(1);
        Motion { count, kind }
    })(input)
}

pub(crate) fn parse(input: &str) -> IResult<&str, Motion> {
    motion(input)
}
