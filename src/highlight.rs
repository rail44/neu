use crate::buffer::Buffer;
use crate::compute::{LineRange, Reactor};
use crate::language::Language;
use tree_sitter::{InputEdit, Parser, Point, Tree, Query, Language as TSLanguage};

fn get_language_info(lang: &Language) -> (TSLanguage, Query) {
    use Language::*;
    match lang {
        Rust => {
            let lang = tree_sitter_rust::language();
            let query = tree_sitter::Query::new(
                lang,
                tree_sitter_rust::HIGHLIGHT_QUERY,
            ).unwrap();
            (lang, query)
        }
        _ => unimplemented!(),
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

pub(crate) struct Highlighter {
    parser: Parser,
    query: Option<Query>,
    tree: Option<Tree>,
}

impl Highlighter {
    pub(crate) fn new() -> Self {
        let parser = Parser::new();
        let mut h = Self { parser, tree: None, query: None };
        h
    }

    pub(crate) fn set_language(&mut self, lang: &Language) {
        let (ts_lang, query) = get_language_info(lang);
        self.parser.set_language(ts_lang).unwrap();
        self.query = Some(query);
    }

    fn load_buffer(&mut self, b: &Buffer) {
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

    pub(crate) fn edit_tree(&mut self, input: &InputEdit) {
        if self.tree.is_none() {
            return;
        }

        let tree = self.tree.as_mut().unwrap();
        tree.edit(&input);
    }

    pub(crate) fn update(&mut self, reactor: &mut Reactor) -> Vec<(Point, String)> {
        if self.query.is_none() {
            return vec![];
        }

        let b = reactor.compute();
        self.load_buffer(&b);

        let mut c = tree_sitter::QueryCursor::new();
        let line_range: LineRange = reactor.compute();
        c.set_point_range(Point::new(line_range.0, 0), Point::new(line_range.1, 0));
        let syntax_tree = self.tree.as_ref().unwrap().root_node();

        let query = self.query.as_mut().unwrap();
        let matches = c.captures(query, syntax_tree, |_| &[]);

        let highlighted = 0;
        let mut result = Vec::new();
        for matched in matches {
            for capture in matched.0.captures {
                let start = capture.node.start_byte();
                if highlighted > start {
                    break;
                }

                let position = capture.node.start_position();
                if position.row < line_range.0 {
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
