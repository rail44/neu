use std::fs;

use clap::{crate_authors, crate_version, Clap};
use xtra::prelude::*;
use xtra::spawn::Smol;

mod actor;
mod buffer;
mod cmd;
mod cmdline;
mod editor;
mod renderer;

use crate::actor::StateActor;
use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::renderer::Renderer;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    filename: Option<String>,
}

fn main() {
    smol::block_on(async {
        let opts: Opts = Opts::parse();
        let renderer = Renderer::new().create(None).spawn(&mut Smol::Global);
        let mut state_actor = StateActor::new(renderer).await;

        if let Some(filename) = opts.filename {
            let s = fs::read_to_string(filename).unwrap();
            let buffer = Buffer::from(s.as_str());

            state_actor.set_buffer(buffer).await;
        };

        let state_addr = state_actor.create(None).spawn(&mut Smol::Global);

        let editor = Editor::new(state_addr.clone());
        let addr = editor.create(None).spawn(&mut Smol::Global);
        addr.send(editor::Run).await.unwrap();
    })
}
