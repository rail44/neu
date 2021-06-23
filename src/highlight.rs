use crate::buffer::Buffer;
use tree_sitter::{InputEdit, Parser, Tree};

pub(crate) struct Highlighter {
    parser: Parser,
    tree: Option<Tree>,
}

impl Highlighter {
    pub(crate) fn new() -> Self {
        let parser = Parser::new();
        let mut h = Self { parser, tree: None };
        h.set_rust_language();
        h
    }

    pub(crate) fn set_rust_language(&mut self) {
        let language = tree_sitter_rust::language();
        self.parser.set_language(language).unwrap();
    }

    pub(crate) fn load_buffer(&mut self, b: &Buffer) {
        let tree = self
            .parser
            .parse_with(
                &mut |byte, _| {
                    if let Some((s, i, _, _)) = b.get_chunk_at_byte(byte) {
                        return &s.as_bytes()[byte - i..];
                    }
                    &[]
                },
                self.tree.as_ref(),
            )
            .unwrap();
        self.tree = Some(tree);
    }

    pub(crate) fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }

    pub(crate) fn edit_tree(&mut self, input: &InputEdit) {
        if self.tree.is_none() {
            return;
        }

        let tree = self.tree.as_mut().unwrap();
        tree.edit(&input);
    }
}
