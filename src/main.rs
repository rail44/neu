use std::fs;

use clap::{crate_authors, crate_version, Clap};
use xtra::prelude::*;
use xtra::spawn::Smol;

mod actor;
mod buffer;
mod cmd;
mod cmdline;
mod editor;

use crate::actor::StateActor;
use crate::buffer::Buffer;
use crate::editor::Editor;

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    filename: Option<String>,
}

fn main() {
    smol::block_on(async {
        let opts: Opts = Opts::parse();
        let state_actor = StateActor::default().create(None).spawn(&mut Smol::Global);
        let mut editor = Editor::new(state_actor.clone());
        if let Some(filename) = opts.filename {
            let s = fs::read_to_string(filename).unwrap();
            let buffer = Buffer::from(s.as_str());

            editor.set_buffer(buffer);
        };

        let addr = editor.create(None).spawn(&mut Smol::Global);
        state_actor
            .send(actor::Subscribe(addr.clone()))
            .await
            .unwrap();
        addr.send(editor::Run).await.unwrap();
    })
}
