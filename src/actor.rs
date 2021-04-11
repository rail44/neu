use crate::editor;
use crate::editor::Editor;
use xtra::prelude::*;

#[derive(Default, Clone, Debug)]
pub(crate) struct Cursor {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Default, Clone, Debug)]
pub(crate) struct State {
    pub(crate) row_offset: usize,
    pub(crate) cursor: Cursor,
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

pub(crate) struct CursorLeft(pub(crate) usize);
impl Message for CursorLeft {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorLeft> for StateActor {
    async fn handle(&mut self, msg: CursorLeft, _ctx: &mut Context<Self>) {
        self.state.cursor.col = self.state.cursor.col.saturating_sub(msg.0);
        self.notify().await;
    }
}

pub(crate) struct CursorDown(pub(crate) usize);
impl Message for CursorDown {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorDown> for StateActor {
    async fn handle(&mut self, msg: CursorDown, _ctx: &mut Context<Self>) {
        self.state.cursor.row += msg.0;
        self.notify().await;
    }
}

pub(crate) struct CursorUp(pub(crate) usize);
impl Message for CursorUp {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorUp> for StateActor {
    async fn handle(&mut self, msg: CursorUp, _ctx: &mut Context<Self>) {
        self.state.cursor.row = self.state.cursor.row.saturating_sub(msg.0);
        self.notify().await;
    }
}

pub(crate) struct CursorRight(pub(crate) usize);
impl Message for CursorRight {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorRight> for StateActor {
    async fn handle(&mut self, msg: CursorRight, _ctx: &mut Context<Self>) {
        self.state.cursor.col += msg.0;
        self.notify().await;
    }
}

pub(crate) struct CursorLineHead;
impl Message for CursorLineHead {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorLineHead> for StateActor {
    async fn handle(&mut self, _msg: CursorLineHead, _ctx: &mut Context<Self>) {
        self.state.cursor.col = 0;
        self.notify().await;
    }
}

pub(crate) struct CursorRow(pub(crate) usize);
impl Message for CursorRow {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorRow> for StateActor {
    async fn handle(&mut self, msg: CursorRow, _ctx: &mut Context<Self>) {
        self.state.cursor.row = msg.0;
        self.notify().await;
    }
}

pub(crate) struct CursorCol(pub(crate) usize);
impl Message for CursorCol {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorCol> for StateActor {
    async fn handle(&mut self, msg: CursorCol, _ctx: &mut Context<Self>) {
        self.state.cursor.col = msg.0;
        self.notify().await;
    }
}
