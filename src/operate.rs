use nom::{
    branch::alt, bytes::complete::tag, character::complete::digit0, combinator::map,
    sequence::pair, IResult,
};

#[derive(Clone, Debug)]
pub(crate) struct Operate {
    pub(crate) count: usize,
    pub(crate) kind: OperateKind,
}

#[derive(Clone, Debug)]
pub(crate) enum OperateKind {
    Remove,
    Yank,
}

fn operate_kind(input: &str) -> IResult<&str, OperateKind> {
    use OperateKind::*;
    alt((map(tag("d"), |_| Remove), map(tag("y"), |_| Yank)))(input)
}

fn operate(input: &str) -> IResult<&str, Operate> {
    map(pair(digit0, operate_kind), |(n, kind)| {
        let count = n.parse().unwrap_or(1);
        Operate { count, kind }
    })(input)
}

pub(crate) fn parse(input: &str) -> IResult<&str, Operate> {
    operate(input)
}
