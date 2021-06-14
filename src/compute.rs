use crate::buffer::Buffer;
use crate::state::{Cursor, State};

pub(crate) trait Compute {
    type Source;
    fn compute(source: &Self::Source) -> Self;
}

pub(crate) trait ComputableFromState {
    fn compute_from_state(state: &State) -> Self;
}

impl<V, S> ComputableFromState for V
where
    V: Compute<Source = S>,
    S: ComputableFromState,
{
    fn compute_from_state(state: &State) -> Self {
        Self::compute(&S::compute_from_state(state))
    }
}

impl<T1, T2> ComputableFromState for (T1, T2)
where
    T1: ComputableFromState,
    T2: ComputableFromState,
{
    fn compute_from_state(state: &State) -> Self {
        (T1::compute_from_state(state), T2::compute_from_state(state))
    }
}

impl ComputableFromState for State {
    fn compute_from_state(state: &State) -> Self {
        state.clone()
    }
}

impl Compute for Buffer {
    type Source = State;
    fn compute(source: &State) -> Self {
        source.buffer.clone()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct LineCount(pub(crate) usize);

impl Compute for LineCount {
    type Source = Buffer;
    fn compute(source: &Buffer) -> Self {
        Self(source.count_lines())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct MaxLineDigit(pub(crate) usize);

impl Compute for MaxLineDigit {
    type Source = LineCount;
    fn compute(source: &LineCount) -> Self {
        Self(format!("{}", source.0).chars().count())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct CurrentLine(pub(crate) String);

impl Compute for CurrentLine {
    type Source = (Buffer, CursorRow);
    fn compute(source: &Self::Source) -> Self {
        Self(source.0.line(source.1 .0).as_str().to_string())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct CursorRow(pub(crate) usize);

impl Compute for CursorRow {
    type Source = Cursor;
    fn compute(source: &Self::Source) -> Self {
        Self(source.row)
    }
}

impl Compute for Cursor {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        source.cursor.clone()
    }
}
