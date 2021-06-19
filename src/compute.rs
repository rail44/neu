use crate::buffer::Buffer;
use crate::mode::Mode;
use crate::state::{Cursor, State};
use core::cmp::min;
use hashbrown::HashMap;
use std::any::{Any, TypeId};

struct Computed<C>
where
    C: Compute,
{
    generation: usize,
    value: C,
    source: C::Source,
}

pub(crate) struct Reactor {
    generation: usize,
    state: State,
    computed_map: HashMap<TypeId, Box<dyn Any>>,
}

impl Reactor {
    pub(crate) fn new() -> Self {
        Self {
            generation: 0,
            state: State::new(),
            computed_map: HashMap::new(),
        }
    }

    pub(crate) fn generation(&self) -> usize {
        self.generation
    }

    pub(crate) fn compute<C: ComputeWithReactor>(&mut self) -> C {
        C::compute_with_reactor(self)
    }

    pub(crate) fn get_update<C: Compute>(&mut self) -> Option<C> {
        let source = C::Source::compute_with_reactor(self);
        if let Some(computed) = self.get_computed::<C>() {
            if source == computed.source {
                return None;
            }
        }
        let v = C::compute(&source);
        self.insert_computed(v.clone(), source);
        Some(v)
    }

    pub(crate) fn load_state(&mut self, state: State) {
        self.generation = self.generation.wrapping_add(1);
        self.state = state;
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn insert_computed<C>(&mut self, value: C, source: C::Source)
    where
        C: Compute,
    {
        let type_id = TypeId::of::<C>();
        self.computed_map.insert(
            type_id,
            Box::new(Computed {
                generation: self.generation,
                value,
                source,
            }),
        );
    }

    fn get_computed<C>(&self) -> Option<&Computed<C>>
    where
        C: Compute,
    {
        let type_id = TypeId::of::<C>();
        self.computed_map
            .get(&type_id)
            .and_then(|any| any.downcast_ref())
    }
}

pub(crate) trait ComputeWithReactor: PartialEq {
    fn compute_with_reactor(reactor: &mut Reactor) -> Self;
}

pub(crate) trait Compute: 'static + PartialEq + Clone {
    type Source: ComputeWithReactor;
    fn compute(source: &Self::Source) -> Self;
}

impl<C> ComputeWithReactor for C
where
    C: Compute,
{
    fn compute_with_reactor(reactor: &mut Reactor) -> Self {
        let source = reactor.compute();
        let computed = reactor.get_computed::<Self>();
        if let Some(computed) = computed {
            if reactor.generation() == computed.generation {
                return computed.value.clone();
            }

            if source == computed.source {
                return computed.value.clone();
            }
        }

        let v = C::compute(&source);
        reactor.insert_computed(v.clone(), source);
        v
    }
}

impl<T1, T2> ComputeWithReactor for (T1, T2)
where
    T1: Compute,
    T2: Compute,
{
    fn compute_with_reactor(reactor: &mut Reactor) -> Self {
        (reactor.compute(), reactor.compute())
    }
}

impl<T1, T2, T3> ComputeWithReactor for (T1, T2, T3)
where
    T1: Compute,
    T2: Compute,
    T3: Compute,
{
    fn compute_with_reactor(reactor: &mut Reactor) -> Self {
        (reactor.compute(), reactor.compute(), reactor.compute())
    }
}

impl<T1, T2, T3, T4> ComputeWithReactor for (T1, T2, T3, T4)
where
    T1: Compute,
    T2: Compute,
    T3: Compute,
    T4: Compute,
{
    fn compute_with_reactor(reactor: &mut Reactor) -> Self {
        (
            reactor.compute(),
            reactor.compute(),
            reactor.compute(),
            reactor.compute(),
        )
    }
}

impl ComputeWithReactor for () {
    fn compute_with_reactor(_reactor: &mut Reactor) -> Self {}
}

impl ComputeWithReactor for State {
    fn compute_with_reactor(reactor: &mut Reactor) -> Self {
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
    type Source = (Buffer, CursorRow, RowOffset);
    fn compute(source: &Self::Source) -> Self {
        Self(
            source
                .0
                .line(source.1 .0 + source.2 .0)
                .as_str()
                .to_string(),
        )
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

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct TerminalHeight(pub(crate) usize);

impl Compute for TerminalHeight {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        Self(source.size.1 as usize - 1)
    }
}

impl Compute for Mode {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        source.mode.clone()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct RowOffset(pub(crate) usize);

impl Compute for RowOffset {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        Self(source.row_offset)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct LineRange(pub(crate) usize, pub(crate) usize);

impl Compute for LineRange {
    type Source = (RowOffset, LineCount, TerminalHeight);
    fn compute(source: &Self::Source) -> Self {
        let row_offset = source.0 .0;
        let line_count = source.1 .0;
        let textarea_row = source.2 .0 - 2;

        Self(row_offset, min(line_count, textarea_row + row_offset + 1))
    }
}
