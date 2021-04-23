use crate::editor;
use crate::editor::Editor;
use crate::buffer::Buffer;
use xtra::prelude::*;

#[derive(Clone, Debug)]
pub(crate) enum Mode {
    Normal(String),
    Insert,
    CmdLine(String),
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal(String::new())
    }
}

impl Mode {
    pub(crate) fn get_cmd(&self) -> &String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }
        panic!();
    }

    pub(crate) fn get_cmd_mut(&mut self) -> &mut String {
        if let Mode::Normal(cmd) = self {
            return cmd;
        }
        panic!();
    }

pub(crate) fn get_cmdline(&self) -> &String {
        if let Mode::CmdLine(cmd) = self {
            return cmd;
        }
        panic!();
    }
}


#[derive(Default, Clone, Debug)]
pub(crate) struct Cursor {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Default, Clone, Debug)]
pub(crate) struct State {
    pub(crate) row_offset: usize,
    pub(crate) cursor: Cursor,
    pub(crate) mode: Mode,
    pub(crate) yanked: Buffer,
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

pub(crate) struct IntoNormalMode;
impl Message for IntoNormalMode {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<IntoNormalMode> for StateActor {
    async fn handle(&mut self, _msg: IntoNormalMode, _ctx: &mut Context<Self>) {
        self.state.mode = Mode::Normal(String::new());
        self.notify().await;
    }
}

pub(crate) struct IntoInsertMode;
impl Message for IntoInsertMode {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<IntoInsertMode> for StateActor {
    async fn handle(&mut self, _msg: IntoInsertMode, _ctx: &mut Context<Self>) {
        self.state.mode = Mode::Insert;
        self.notify().await;
    }
}

pub(crate) struct IntoCmdLineMode;
impl Message for IntoCmdLineMode {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<IntoCmdLineMode> for StateActor {
    async fn handle(&mut self, _msg: IntoCmdLineMode, _ctx: &mut Context<Self>) {
        self.state.mode = Mode::CmdLine(String::new());
        self.notify().await;
    }
}

pub(crate) struct SetYank(pub(crate) Buffer);
impl Message for SetYank {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<SetYank> for StateActor {
    async fn handle(&mut self, msg: SetYank, _ctx: &mut Context<Self>) {
        self.state.yanked = msg.0;
        self.notify().await;
    }
}
