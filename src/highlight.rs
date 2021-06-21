#[test]
fn test_highlighter() {
    use tree_sitter_rust;
    use tree_sitter_highlight;
    use tree_sitter_highlight::{Highlighter, HighlightConfiguration};

    let mut rust_config = HighlightConfiguration::new(
        tree_sitter_rust::language(),
        tree_sitter_rust::HIGHLIGHT_QUERY,
        "",
        "",
    ).unwrap();
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
    let mut highlighter = Highlighter::new();
    rust_config.configure(highlight_names);
    use tree_sitter_highlight::HighlightEvent;

    let highlights = highlighter.highlight(
        &rust_config,
        b"'e',",
        None,
        |_| None
    ).unwrap();

    for event in highlights {
        match event.unwrap() {
            HighlightEvent::Source {start, end} => {
                eprintln!("source: {}-{}", start, end);
            },
            HighlightEvent::HighlightStart(s) => {
                eprintln!("highlight style started: {:?}", s);
            },
            HighlightEvent::HighlightEnd => {
                eprintln!("highlight style ended");
            },
        }
    }
}

