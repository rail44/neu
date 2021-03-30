use std::fs;

use clap::{crate_authors, crate_version, Clap};

mod buffer;
mod cmd;
mod cmdline;
mod editor;
use crate::buffer::Buffer;
use crate::editor::Editor;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    filename: Option<String>,
}

fn main() {
    let opts: Opts = Opts::parse();
    if let Some(filename) = opts.filename {
        let s = fs::read_to_string(filename).unwrap();
        let buffer = Buffer::from(s.as_str());

        let mut editor = Editor::from(buffer);
        editor.run();
        return;
    }
    let mut editor = Editor::default();
    editor.run();
}
