use crate::buffer::Buffer;
use crate::state::{Cursor, State};
use std::any::{TypeId, Any};
use hashbrown::HashMap;

pub(crate) struct Computed<C> where C: Compute {
    prev: C,
    source: C::Source,
}

pub(crate) struct Reactor {
    state: State,
    computed_map: HashMap<TypeId, Box<(dyn Compute, dyn Compute)>>,
}

impl Reactor {
    pub(crate) fn new() -> Self {
        Self {
            state: State::new(),
            computed_map: HashMap::new(),
        }
    }

    pub(crate) fn state(&self) -> &State {
        &self.state
    }

    pub(crate) fn compute<C: ComputeWithReactor>(&self) -> C {
        C::compute_with_reactor(self)
    }

    pub(crate) fn load_state(&mut self, state: State) {
        self.state = state;
    }
}

pub(crate) trait ComputeWithReactor {
    fn compute_with_reactor(reactor: &Reactor) -> Self;
}

pub(crate) trait Compute {
    type Source: ComputeWithReactor;
    fn compute(source: &Self::Source) -> Self;
}

impl<C> ComputeWithReactor for C
where
    C: Compute,
{
    fn compute_with_reactor(reactor: &Reactor) -> Self {
        let source = reactor.compute();
        C::compute(&source)
    }
}

impl<T1, T2> ComputeWithReactor for (T1, T2)
where
    T1: Compute,
    T2: Compute,
{
    fn compute_with_reactor(reactor: &Reactor) -> Self {
        let t1 = reactor.compute();
        let t2 = reactor.compute();
        (t1, t2)
    }
}

impl<T1, T2, T3> ComputeWithReactor for (T1, T2, T3)
where
    T1: Compute,
    T2: Compute,
    T3: Compute,
{
    fn compute_with_reactor(reactor: &Reactor) -> Self {
        let t1 = reactor.compute();
        let t2 = reactor.compute();
        let t3 = reactor.compute();
        (t1, t2, t3)
    }
}

impl ComputeWithReactor for () {
    fn compute_with_reactor(_reactor: &Reactor) -> Self {
        ()
    }
}

impl ComputeWithReactor for State {
    fn compute_with_reactor(reactor: &Reactor) -> Self {
        reactor.state().clone()
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
