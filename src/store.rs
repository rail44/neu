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

pub(crate) struct Store {
    state: State,
    renderer: Address<Renderer>,
}

impl Store {
    pub(crate) async fn new(renderer: Address<Renderer>) -> Self {
        let mut actor = Self {
            renderer,
            state: State::new(),
        };
        actor.notify().await;
        actor
    }

    pub(crate) async fn set_buffer(&mut self, buffer: Buffer) {
        self.state.buffer = buffer;
        self.notify().await;
    }
}

impl Actor for Store {}

impl Store {
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
impl Handler<AddRowOffset> for Store {
    async fn handle(&mut self, msg: AddRowOffset, _ctx: &mut Context<Self>) {
        self.state.row_offset += msg.0;
    }
}

pub(crate) struct SubRowOffset(pub(crate) usize);
impl Message for SubRowOffset {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<SubRowOffset> for Store {
    async fn handle(&mut self, msg: SubRowOffset, _ctx: &mut Context<Self>) {
        self.state.row_offset = self.state.row_offset.saturating_sub(msg.0);
    }
}

pub(crate) struct CursorLeft(pub(crate) usize);
impl Message for CursorLeft {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorLeft> for Store {
    async fn handle(&mut self, msg: CursorLeft, _ctx: &mut Context<Self>) {
        self.state.cursor.col = self.state.cursor.col.saturating_sub(msg.0);
    }
}

pub(crate) struct CursorDown(pub(crate) usize);
impl Message for CursorDown {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorDown> for Store {
    async fn handle(&mut self, msg: CursorDown, _ctx: &mut Context<Self>) {
        self.state.cursor.row += msg.0;
    }
}

pub(crate) struct CursorUp(pub(crate) usize);
impl Message for CursorUp {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorUp> for Store {
    async fn handle(&mut self, msg: CursorUp, _ctx: &mut Context<Self>) {
        if self.state.cursor.row == 0 {
            self.state.row_offset = self.state.row_offset.saturating_sub(msg.0);
            return;
        }
        self.state.cursor.row = self.state.cursor.row.saturating_sub(msg.0);
    }
}

pub(crate) struct CursorRight(pub(crate) usize);
impl Message for CursorRight {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorRight> for Store {
    async fn handle(&mut self, msg: CursorRight, _ctx: &mut Context<Self>) {
        self.state.cursor.col += msg.0;
    }
}

pub(crate) struct CursorLineHead;
impl Message for CursorLineHead {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorLineHead> for Store {
    async fn handle(&mut self, _msg: CursorLineHead, _ctx: &mut Context<Self>) {
        self.state.cursor.col = 0;
    }
}

pub(crate) struct CursorRow(pub(crate) usize);
impl Message for CursorRow {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorRow> for Store {
    async fn handle(&mut self, msg: CursorRow, _ctx: &mut Context<Self>) {
        self.state.cursor.row = msg.0;
    }
}

pub(crate) struct CursorCol(pub(crate) usize);
impl Message for CursorCol {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<CursorCol> for Store {
    async fn handle(&mut self, msg: CursorCol, _ctx: &mut Context<Self>) {
        self.state.cursor.col = msg.0;
    }
}

pub(crate) struct IntoNormalMode;
impl Message for IntoNormalMode {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<IntoNormalMode> for Store {
    async fn handle(&mut self, _msg: IntoNormalMode, _ctx: &mut Context<Self>) {
        self.state.mode = Mode::Normal(String::new());
    }
}

pub(crate) struct IntoInsertMode;
impl Message for IntoInsertMode {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<IntoInsertMode> for Store {
    async fn handle(&mut self, _msg: IntoInsertMode, _ctx: &mut Context<Self>) {
        self.state.mode = Mode::Insert;
    }
}

pub(crate) struct IntoCmdLineMode;
impl Message for IntoCmdLineMode {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<IntoCmdLineMode> for Store {
    async fn handle(&mut self, _msg: IntoCmdLineMode, _ctx: &mut Context<Self>) {
        self.state.mode = Mode::CmdLine(String::new());
    }
}

pub(crate) struct SetYank(pub(crate) Buffer);
impl Message for SetYank {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<SetYank> for Store {
    async fn handle(&mut self, msg: SetYank, _ctx: &mut Context<Self>) {
        self.state.yanked = msg.0;
    }
}

pub(crate) struct HandleState<F>(pub(crate) F);
impl<F: 'static + FnOnce(&mut State) -> V + Send, V: Send> Message for HandleState<F> {
    type Result = V;
}

#[async_trait::async_trait]
impl<F: 'static + FnOnce(&mut State) -> V + Send, V: Send> Handler<HandleState<F>> for Store {
    async fn handle(&mut self, msg: HandleState<F>, _ctx: &mut Context<Self>) -> V {
        msg.0(&mut self.state)
    }
}

pub(crate) struct GetState;
impl Message for GetState {
    type Result = State;
}

#[async_trait::async_trait]
impl Handler<GetState> for Store {
    async fn handle(&mut self, _msg: GetState, _ctx: &mut Context<Self>) -> State {
        self.state.clone()
    }
}

pub(crate) struct PushCmd(pub(crate) char);
impl Message for PushCmd {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<PushCmd> for Store {
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
impl Handler<PushCmdStr> for Store {
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
impl Handler<PopCmd> for Store {
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

pub(crate) struct Notify;
impl Message for Notify {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<Notify> for Store {
    async fn handle(&mut self, _msg: Notify, _ctx: &mut Context<Self>) {
        self.notify().await;
    }
}

pub(crate) struct ForwardWord(pub(crate) usize);
impl Message for ForwardWord {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<ForwardWord> for Store {
    async fn handle(&mut self, msg: ForwardWord, ctx: &mut Context<Self>) {
        let count = self.state.buffer.count_forward_word(
            self.state.cursor.col,
            self.state.cursor.row + self.state.row_offset,
        );
        self.handle(CursorRight(count * msg.0), ctx).await;
    }
}

pub(crate) struct BackWord(pub(crate) usize);
impl Message for BackWord {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<BackWord> for Store {
    async fn handle(&mut self, msg: BackWord, ctx: &mut Context<Self>) {
        let count = self.state.buffer.count_back_word(
            self.state.cursor.col,
            self.state.cursor.row + self.state.row_offset,
        );
        self.handle(CursorLeft(count * msg.0), ctx).await;
    }
}

pub(crate) struct RemoveChars(pub(crate) usize);
impl Message for RemoveChars {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<RemoveChars> for Store {
    async fn handle(&mut self, msg: RemoveChars, ctx: &mut Context<Self>) {
        let yank = self.state.buffer.remove_chars(
            self.state.cursor.col,
            self.state.cursor.row + self.state.row_offset,
            msg.0,
        );
        self.handle(SetYank(yank), ctx).await;
    }
}

pub(crate) struct RemoveLines(pub(crate) usize);
impl Message for RemoveLines {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<RemoveLines> for Store {
    async fn handle(&mut self, msg: RemoveLines, ctx: &mut Context<Self>) {
        let yank = self
            .state
            .buffer
            .remove_lines(self.state.cursor.row + self.state.row_offset, msg.0);
        self.handle(SetYank(yank), ctx).await;
    }
}

pub(crate) struct YankLines(pub(crate) usize);
impl Message for YankLines {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<YankLines> for Store {
    async fn handle(&mut self, msg: YankLines, ctx: &mut Context<Self>) {
        let yank = self
            .state
            .buffer
            .subseq_lines(self.state.cursor.row + self.state.row_offset, msg.0);
        self.handle(SetYank(yank), ctx).await;
    }
}

pub(crate) struct Remove(pub(crate) usize, pub(crate) usize);
impl Message for Remove {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<Remove> for Store {
    async fn handle(&mut self, msg: Remove, ctx: &mut Context<Self>) {
        let yank = self.state.buffer.remove(msg.0..msg.1);
        self.handle(SetYank(yank), ctx).await;
    }
}

pub(crate) struct AppendYank(pub(crate) usize);
impl Message for AppendYank {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<AppendYank> for Store {
    async fn handle(&mut self, msg: AppendYank, ctx: &mut Context<Self>) {
        let col = if self.state.yanked.end_with_line_break() {
            self.handle(CursorDown(1), ctx).await;
            0
        } else {
            self.handle(CursorRight(1), ctx).await;
            self.state.cursor.col
        };
        for _ in 0..msg.0 {
            self.state.buffer.insert(
                col,
                self.state.cursor.row + self.state.row_offset,
                self.state.yanked.clone(),
            );
        }
    }
}

pub(crate) struct InsertYank(pub(crate) usize);
impl Message for InsertYank {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<InsertYank> for Store {
    async fn handle(&mut self, msg: InsertYank, _ctx: &mut Context<Self>) {
        let col = if self.state.yanked.end_with_line_break() {
            0
        } else {
            self.state.cursor.col
        };
        for _ in 0..msg.0 {
            self.state.buffer.insert(
                col,
                self.state.cursor.row + self.state.row_offset,
                self.state.yanked.clone(),
            );
        }
    }
}

pub(crate) struct CountWordForward;
impl Message for CountWordForward {
    type Result = usize;
}

#[async_trait::async_trait]
impl Handler<CountWordForward> for Store {
    async fn handle(&mut self, _msg: CountWordForward, _ctx: &mut Context<Self>) -> usize {
        self.state.buffer.count_forward_word(
            self.state.cursor.col,
            self.state.cursor.row + self.state.row_offset,
        )
    }
}

pub(crate) struct CountWordBack;
impl Message for CountWordBack {
    type Result = usize;
}

#[async_trait::async_trait]
impl Handler<CountWordBack> for Store {
    async fn handle(&mut self, _msg: CountWordBack, _ctx: &mut Context<Self>) -> usize {
        self.state.buffer.count_back_word(
            self.state.cursor.col,
            self.state.cursor.row + self.state.row_offset,
        )
    }
}

pub(crate) struct MoveTo(pub usize);
impl Message for MoveTo {
    type Result = ();
}

#[async_trait::async_trait]
impl Handler<MoveTo> for Store {
    async fn handle(&mut self, msg: MoveTo, _ctx: &mut Context<Self>) {
        let result = self.state.buffer.get_cursor_by_offset(msg.0);
        self.state.cursor.row = result.0;
        self.state.cursor.col = result.1;
    }
}
