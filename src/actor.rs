use crate::buffer::Buffer;
use crate::renderer;
use crate::renderer::Renderer;
use core::cmp::min;
use termion::terminal_size;
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
    pub(crate) size: (u16, u16),
    pub(crate) buffer: Buffer,
}

impl State {
    pub(crate) fn new() -> Self {
        let size = terminal_size().unwrap();

        Self {
            size,
            ..Default::default()
        }
    }
}

pub(crate) struct StateActor {
    state: State,
    renderer: Address<Renderer>,
}

impl StateActor {
    pub(crate) async fn new(renderer: Address<Renderer>) -> Self {
        let state = State::new();
        renderer
            .send(renderer::Render(state.clone()))
            .await
            .unwrap();
        Self {
            renderer,
            state: State::new(),
        }
    }

    pub(crate) async fn set_buffer(&mut self, buffer: Buffer) {
        self.state.buffer = buffer;
        self.notify().await;
    }
}

impl Actor for StateActor {}

impl StateActor {
    fn coerce_cursor(&mut self) {
        let row = min(
            self.state.cursor.row,
            self.state.buffer.count_lines().saturating_sub(1),
        );
        self.state.cursor.row = row;

        let textarea_row = (self.state.size.1 - 3) as usize;
        let actual_row = textarea_row - self.wrap_offset();
        if self.state.cursor.row > actual_row {
            let row_offset = min(
                self.state.row_offset + self.state.cursor.row - actual_row,
                self.state.buffer.count_lines().saturating_sub(actual_row),
            );
            self.state.row_offset = row_offset;
            self.state.cursor.row = actual_row;
        }
        let col = min(
            self.state.cursor.col,
            self.state
                .buffer
                .row_len(self.state.cursor.row + self.state.row_offset),
        );
        self.state.cursor.col = col;
    }

    fn wrap_offset(&self) -> usize {
        let mut wraps = 0;
        let mut lines_count = 0;
        for line in self.state.buffer.lines().skip(self.state.row_offset) {
            let wrap = (line.len() as u16) / self.state.size.0;
            wraps += wrap;
            lines_count += 1 + wrap;
            if lines_count >= self.state.size.1 - 2 {
                break;
            }
        }
        wraps as usize
    }

    async fn notify(&mut self) {
        self.coerce_cursor();
        self.renderer
            .send(renderer::Render(self.state.clone()))
            .await
            .unwrap();
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

pub(crate) struct HandleState<F>(pub(crate) F);
impl<F: 'static + FnOnce(&mut State) -> V + Send, V: Send> Message for HandleState<F> {
    type Result = V;
}

#[async_trait::async_trait]
impl<F: 'static + FnOnce(&mut State) -> V + Send, V: Send> Handler<HandleState<F>> for StateActor {
    async fn handle(&mut self, msg: HandleState<F>, _ctx: &mut Context<Self>) -> V {
        let v = msg.0(&mut self.state);
        self.notify().await;
        v
    }
}

pub(crate) struct GetState;
impl Message for GetState {
    type Result = State;
}

#[async_trait::async_trait]
impl Handler<GetState> for StateActor {
    async fn handle(&mut self, _msg: GetState, _ctx: &mut Context<Self>) -> State {
        self.state.clone()
    }
}

pub(crate) struct PushCmd(pub(crate) char);
impl Message for PushCmd {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<PushCmd> for StateActor {
    async fn handle(&mut self, msg: PushCmd, _ctx: &mut Context<Self>) {
        match &mut self.state.mode {
            Mode::Normal(cmd) => {
                cmd.push(msg.0);
            }
            Mode::Insert => (),
            Mode::CmdLine(cmd) => {
                cmd.push(msg.0);
            }
        }
    }
}

pub(crate) struct PushCmdStr(pub(crate) String);
impl Message for PushCmdStr {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<PushCmdStr> for StateActor {
    async fn handle(&mut self, msg: PushCmdStr, _ctx: &mut Context<Self>) {
        match &mut self.state.mode {
            Mode::Normal(cmd) => {
                cmd.push_str(&msg.0);
            }
            Mode::Insert => (),
            Mode::CmdLine(cmd) => {
                cmd.push_str(&msg.0);
            }
        }
    }
}

pub(crate) struct PopCmd;
impl Message for PopCmd {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<PopCmd> for StateActor {
    async fn handle(&mut self, _msg: PopCmd, _ctx: &mut Context<Self>) {
        match &mut self.state.mode {
            Mode::Normal(cmd) => {
                cmd.pop();
            }
            Mode::Insert => (),
            Mode::CmdLine(cmd) => {
                cmd.pop();
            }
        }
    }
}
