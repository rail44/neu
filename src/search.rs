use crate::position::Position;

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Match {
    pub(crate) pos: Position,
    pub(crate) len: usize,
}

impl Match {
    pub(crate) fn new(pos: Position, len: usize) -> Self {
        Self { pos, len }
    }
}

pub(crate) fn get_next<'a>(p: &'a Position, matches: &'a [Match]) -> &'a Position {
    for m in matches {
        if m.pos.row == p.row && m.pos.col >= p.col {
            return &m.pos;
        }

        if m.pos.row > p.row {
            return &m.pos;
        }
    }
    p
}

pub(crate) fn get_prev<'a>(p: &'a Position, matches: &'a [Match]) -> &'a Position {
    for m in matches.iter().rev() {
        if m.pos.row == p.row && m.pos.col < p.col {
            return &m.pos;
        }

        if m.pos.row < p.row {
            return &m.pos;
        }
    }
    p
}
