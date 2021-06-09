use std::env::temp_dir;
use std::fs;
use std::sync::Mutex;

use clap::{crate_authors, crate_version, Clap};
use dirs::home_dir;

mod action;
mod buffer;
mod cmd;
mod cmdline;
mod config;
mod editor;
mod mode;
mod renderer;
mod selection;
mod state;
mod store;

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::renderer::Renderer;
use crate::store::Store;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    filename: Option<String>,
}

fn main() {
    let config =
        config::parse(home_dir().unwrap().join(".config/neu/config.toml")).unwrap_or_default();

    if config.debug {
        let mut i = 1;
        let temp_path = temp_dir();
        let mut path = temp_path.join(format!("neu.{}.log", i));
        while path.exists() {
            i += 1;
            path = temp_path.join(format!("neu.{}.log", i));
        }
        let log_file = fs::File::create(path).unwrap();
        let writer = Mutex::new(log_file);
        tracing_subscriber::fmt().with_writer(writer).init();
    }

    smol::block_on(async {
        let opts: Opts = Opts::parse();
        let renderer = Renderer::new();

        let (tx, rx) = flume::unbounded();
        let mut store = Store::new(rx, renderer);

        if let Some(filename) = opts.filename {
            let s = fs::read_to_string(filename).unwrap();
            let buffer = Buffer::from(s.as_str());

            store.set_buffer(buffer);
        };

        let editor = Editor::new(tx);
        smol::spawn(async move { editor.run().await }).detach();
        store.run().await;
    })
}
