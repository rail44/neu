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
    pub(crate) fn push(&mut self, buffer: Buffer, cursor: Cursor, tree: Tree) {
        let record = Record {
            buffer,
            cursor,
            tree,
        };
        self.forward.clear();
        self.back.push(record);
    }

    pub(crate) fn redo(&mut self, count: usize) -> Option<Record> {
        for _ in 0..count {
            let r = self.forward.pop();
            if r.is_none() {
                break;
            }
            self.back.push(r.unwrap());
        }
        self.back.pop()
    }

    pub(crate) fn undo(&mut self, count: usize) -> Option<Record> {
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
