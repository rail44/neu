use crate::action::{Selection, SelectionKind};
use nom::{
    branch::alt, bytes::complete::tag, character::complete::digit0, combinator::map,
    sequence::pair, IResult,
};

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

pub(crate) fn parse(input: &str) -> IResult<&str, Selection> {
    selection(input)
}
