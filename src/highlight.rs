use crate::buffer::Buffer;
use crate::compute::{LineRange, Reactor};
use crate::language::Language;
use tree_sitter::{InputEdit, Language as TSLanguage, Node, Parser, Point, Query, Tree};

fn get_language_info(lang: &Language) -> Option<(TSLanguage, Query)> {
    use Language::*;
    match lang {
        Rust => {
            let lang = tree_sitter_rust::language();
            let query = tree_sitter::Query::new(lang, tree_sitter_rust::HIGHLIGHT_QUERY).unwrap();
            Some((lang, query))
        }
        JavaScript => {
            let lang = tree_sitter_javascript::language();
            let query =
                tree_sitter::Query::new(lang, tree_sitter_javascript::HIGHLIGHT_QUERY).unwrap();
            Some((lang, query))
        }
        _ => None,
    }
}

fn get_color(syntax_kind: &str) -> String {
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

pub(super) struct Highlighter {
    parser: Parser,
    query: Option<Query>,
    tree: Option<Tree>,
}

impl Highlighter {
    pub(super) fn new(buffer: &Buffer, lang: &Language) -> Self {
        let mut parser = Parser::new();

        let mut query = None;
        if let Some((ts_lang, q)) = get_language_info(lang) {
            parser.set_language(ts_lang).unwrap();
            query = Some(q);
        }

        let mut highlighter = Self {
            parser,
            query,
            tree: None,
        };
        highlighter.load_buffer(buffer);
        highlighter
    }

    pub(super) fn set_tree(&mut self, tree: Tree) {
        self.tree = Some(tree);
    }

    pub(super) fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }

    fn load_buffer(&mut self, b: &Buffer) {
        if self.parser.language().is_none() {
            return;
        }
        self.tree = self.parser.parse_with(
            &mut |byte, _| {
                if let Some((s, i, _, _)) = b.get_chunk_at_byte(byte) {
                    return &s.as_bytes()[byte - i..];
                }
                &[]
            },
            self.tree.as_ref(),
        )
    }

    pub(super) fn edit_tree(&mut self, input: &InputEdit) {
        if let Some(tree) = &mut self.tree {
            tree.edit(&input);
        }
    }

    pub(super) fn update(&mut self, reactor: &mut Reactor) -> Vec<(Point, String)> {
        if self.query.is_none() {
            return vec![];
        }

        let b = reactor.compute();
        self.load_buffer(&b);

        let mut c = tree_sitter::QueryCursor::new();
        let line_range: LineRange = reactor.compute();
        c.set_point_range(Point::new(line_range.0.start, 0)..Point::new(line_range.0.end, 0));
        let syntax_tree = self.tree.as_mut().unwrap().root_node();

        let query = self.query.as_mut().unwrap();
        let matches = c.captures(query, syntax_tree, |node: Node| {
            let start = node.start_byte();
            if let Some((chunks, _, _, _)) = b.get_chunks_at_byte(start) {
                return chunks.map(|s| s.as_bytes());
            }
            unreachable!()
        });

        let mut result = Vec::new();
        for matched in matches {
            for capture in matched.0.captures {
                let start = capture.node.start_byte();
                let position = capture.node.start_position();
                if line_range.0.start > position.row {
                    break;
                }


                let end = capture.node.end_byte();
                let syntax_kind = &query.capture_names()[capture.index as usize];

                let bytes: Vec<_> = b.bytes_at(start).take(end - start).collect();
                let s = std::str::from_utf8(&bytes).unwrap();
                result.push((
                    position,
                    format!(
                        "{}{}{}",
                        get_color(syntax_kind),
                        s,
                        termion::color::Fg(termion::color::Reset)
                    ),
                ));
            }
        }
        result
    }
}
