use crate::buffer::Buffer;
use tree_sitter::{InputEdit, Parser, Tree};

pub(crate) fn get_color(syntax_kind: &str) -> String {
    use termion::color;
    match syntax_kind {
        "keyword" => format!("{}", color::Fg(color::Magenta)),
        "attribute" => format!("{}", color::Fg(color::Red)),
        "constant.builtin" => format!("{}", color::Fg(color::Red)),
        "property" => format!("{}", color::Fg(color::Red)),
        "function.macro" => format!("{}", color::Fg(color::Red)),
        "function" => format!("{}", color::Fg(color::Blue)),
        "function.method" => format!("{}", color::Fg(color::Blue)),
        "type.builtin" => format!("{}", color::Fg(color::Yellow)),
        "type" => format!("{}", color::Fg(color::Yellow)),
        "string" => format!("{}", color::Fg(color::Green)),
        "variable" => format!("{}", color::Fg(color::White)),
        "variable.parameter" => format!("{}", color::Fg(color::Red)),
        "variable.builtin" => format!("{}", color::Fg(color::Cyan)),
        "punctuation.bracket" => format!("{}", color::Fg(color::LightCyan)),
        "punctuation.delimiter" => format!("{}", color::Fg(color::LightCyan)),
        "operator" => format!("{}", color::Fg(color::White)),
        "label" => format!("{}", color::Fg(color::Yellow)),
        "comment" => format!("{}", color::Fg(color::LightBlack)),
        "escape" => format!("{}", color::Fg(color::Magenta)),
        "number" => format!("{}", color::Fg(color::Magenta)),
        s => {
            tracing::debug!("{}", s);
            format!("{}", color::Fg(color::Red))
        }
    }
}

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
