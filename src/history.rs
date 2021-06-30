use crate::buffer::Buffer;
use crate::state::Cursor;
use tree_sitter::Tree;

#[derive(Debug)]
pub(crate) struct Record {
    pub(crate) buffer: Buffer,
    pub(crate) cursor: Cursor,
    pub(crate) tree: Tree,
}

#[derive(Debug, Default)]
pub(crate) struct History {
    forward: Vec<Record>,
    back: Vec<Record>,
}

impl History {
    pub(crate) fn push(&mut self, r: Record) {
        self.forward.clear();
        self.back.push(r);
    }

    pub(crate) fn redo(&mut self, current: Record, count: usize) -> Option<Record> {
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

    pub(crate) fn undo(&mut self, current: Record, count: usize) -> Option<Record> {
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
