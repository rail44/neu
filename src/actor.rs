use crate::editor;
use crate::editor::Editor;
use xtra::prelude::*;

#[derive(Default, Clone, Debug)]
pub(crate) struct State {
    pub(crate) row_offset: usize,
}

#[derive(Default)]
pub(crate) struct StateActor {
    state: State,
    editor_addr: Option<Address<Editor>>,
}

impl Actor for StateActor {}

impl StateActor {
    async fn notify(&self) {
        if let Some(editor) = &self.editor_addr {
            editor
                .send(editor::ChangeState(self.state.clone()))
                .await
                .unwrap();
        }
    }
}

pub(crate) struct Subscribe(pub(crate) Address<Editor>);
impl Message for Subscribe {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<Subscribe> for StateActor {
    async fn handle(&mut self, msg: Subscribe, _ctx: &mut Context<Self>) {
        self.editor_addr = Some(msg.0);
    }
}

pub(crate) struct AddRowOffset(pub(crate) usize);
impl Message for AddRowOffset {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<AddRowOffset> for StateActor {
    async fn handle(&mut self, msg: AddRowOffset, _ctx: &mut Context<Self>) {
        self.state.row_offset += msg.0;
        self.notify().await;
    }
}

pub(crate) struct SubRowOffset(pub(crate) usize);
impl Message for SubRowOffset {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<SubRowOffset> for StateActor {
    async fn handle(&mut self, msg: SubRowOffset, _ctx: &mut Context<Self>) {
        self.state.row_offset = self.state.row_offset.saturating_sub(msg.0);
        self.notify().await;
    }
}
