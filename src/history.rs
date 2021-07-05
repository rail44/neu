use crate::buffer::Buffer;
use crate::position::Position;
use tree_sitter::Tree;

#[derive(Debug)]
pub(super) struct Record {
    pub(super) buffer: Buffer,
    pub(super) cursor: Position,
    pub(super) tree: Option<Tree>,
}

#[derive(Debug, Default)]
pub(super) struct History {
    forward: Vec<Record>,
    back: Vec<Record>,
}

impl History {
    pub(super) fn push(&mut self, r: Record) {
        self.forward.clear();
        self.back.push(r);
    }

    pub(super) fn pop(&mut self) -> Option<Record> {
        self.back.pop()
    }

    pub(super) fn redo(&mut self, current: Record, count: usize) -> Option<Record> {
        if self.forward.is_empty() {
            return None;
        }
        self.back.push(current);
        for _ in 0..count {
            let r = self.forward.pop();
            if r.is_none() {
                break;
            }
            self.back.push(r.unwrap());
        }
        self.back.pop()
    }

    pub(super) fn undo(&mut self, current: Record, count: usize) -> Option<Record> {
        if self.back.is_empty() {
            return None;
        }
        self.forward.push(current);
        for _ in 0..count {
            let r = self.back.pop();
            if r.is_none() {
                break;
            }
            self.forward.push(r.unwrap());
        }
        self.forward.pop()
    }
}
