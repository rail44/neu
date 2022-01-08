use std::env::temp_dir;
use std::fs;
use std::panic;
use std::sync::Arc;

use backtrace::Backtrace;
use clap::Parser;
use dirs::home_dir;

mod action;
mod buffer;
mod cmd;
mod cmdline;
mod compute;
mod config;
mod edit;
mod editor;
mod highlight;
mod history;
mod language;
mod mode;
mod movement;
mod position;
mod renderer;
mod search;
mod selection;
mod state;
mod store;

use crate::editor::Editor;
use crate::renderer::Renderer;
use crate::store::RootStore;

#[derive(Parser)]
#[clap(version, author)]
struct Opts {
    filename: Option<String>,
}

fn main() {
    let config =
        config::parse(home_dir().unwrap().join(".config/neu/config.toml")).unwrap_or_default();

    if config.debug {
        let temp_path = temp_dir();
        let path = temp_path.join("neu.log");
        let log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        let writer = Arc::new(log_file);
        tracing_subscriber::fmt()
            .pretty()
            .with_writer(writer)
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    panic::set_hook(Box::new(|e| {
        tracing::error!("{:?}", Backtrace::new());
        tracing::error!("{}", e);
    }));

    smol::block_on(async {
        let opts: Opts = Opts::parse();
        let renderer = Renderer::new();

        let (tx, rx) = flume::unbounded();

        let mut store = if let Some(filename) = opts.filename {
            RootStore::open_file(&filename, rx, renderer)
        } else {
            RootStore::new(rx, renderer)
        };

        let editor = Editor::new(tx);
        smol::spawn(async move { editor.run().await }).detach();
        store.run().await;
    })
}
