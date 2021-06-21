use once_cell::sync::Lazy;
use std::sync::Mutex;
use tree_sitter_highlight;
pub use tree_sitter_highlight::HighlightEvent;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter};
use tree_sitter_rust;

static HIGHLIGHTER: Lazy<Mutex<Highlighter>> = Lazy::new(|| Mutex::new(Highlighter::new()));

pub(crate) fn highlight(source: &str) -> Vec<HighlightEvent> {
    let mut rust_config = HighlightConfiguration::new(
        tree_sitter_rust::language(),
        tree_sitter_rust::HIGHLIGHT_QUERY,
        "",
        "",
    )
    .unwrap();
    let highlight_names = &[
        "annotation",
        "attribute",
        "boolean",
        "character",
        "comment",
        "conditional",
        "constant",
        "constant.builtin",
        "constant.macro",
        "constructor",
        "error",
        "exception",
        "field",
        "float",
        "function",
        "function.builtin",
        "function.macro",
        "include",
        "keyword",
        "keyword.function",
        "keyword.operator",
        "label",
        "method",
        "namespace",
        "none",
        "number",
        "operator",
        "parameter",
        "parameter.reference",
        "property",
        "punctuation.delimiter",
        "punctuation.bracket",
        "punctuation.special",
        "repeat",
        "string",
        "string.regex",
        "string.escape",
        "symbol",
        "tag",
        "tag.delimiter",
        "text",
        "text.strong",
        "text.emphasis",
        "text.underline",
        "text.strike",
        "text.title",
        "text.literal",
        "text.uri",
        "text.math",
        "text.reference",
        "text.environment",
        "text.environment.name",
        "text.note",
        "text.warning",
        "text.danger",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
    ];
    rust_config.configure(highlight_names);

    let mut highlighter = HIGHLIGHTER.lock().unwrap();
    highlighter
        .highlight(&rust_config, source.as_bytes(), None, |_| None)
        .unwrap()
        .map(|r| r.unwrap().clone())
        .collect()
}
