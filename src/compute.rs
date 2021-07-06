use crate::buffer::Buffer;
use crate::mode::Mode;
use crate::position::Position;
use crate::state::State;
use core::cmp::min;
use hashbrown::HashMap;
use regex::Regex;
use std::any::{Any, TypeId};
use std::ops::Range;

#[derive(Clone, Debug)]
struct Computed<C>
where
    C: Compute,
{
    generation: usize,
    value: C,
    source: C::Source,
}

pub(super) struct Reactor {
    generation: usize,
    state: State,
    computed_map: HashMap<TypeId, Box<dyn Any>>,
}

impl Reactor {
    pub(super) fn new() -> Self {
        Self {
            generation: 0,
            state: State::new(),
            computed_map: HashMap::new(),
        }
    }

    pub(super) fn generation(&self) -> usize {
        self.generation
    }

    pub(super) fn compute<C: ComputeWithReactor>(&mut self) -> C {
        C::compute_with_reactor(self)
    }

    pub(super) fn get_update<C: Compute>(&mut self) -> Option<C> {
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

    pub(super) fn load_state(&mut self, state: State) {
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

pub(super) trait ComputeWithReactor: PartialEq + Clone {
    fn compute_with_reactor(reactor: &mut Reactor) -> Self;
}

pub(super) trait Compute: 'static + PartialEq + Clone {
    type Source: ComputeWithReactor;
    fn compute(source: &Self::Source) -> Self;
}

impl<C> ComputeWithReactor for C
where
    C: Compute,
{
    fn compute_with_reactor(reactor: &mut Reactor) -> Self {
        let computed = reactor.get_computed::<Self>();
        if computed.is_none() {
            let source = reactor.compute();
            let v = C::compute(&source);
            reactor.insert_computed(v.clone(), source);
            return v;
        }

        let computed = computed.unwrap().clone();
        if reactor.generation() == computed.generation {
            return computed.value;
        }

        let source = reactor.compute::<C::Source>();
        if source == computed.source {
            return computed.value;
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
pub(super) struct LineCount(pub(super) usize);

impl Compute for LineCount {
    type Source = Buffer;
    fn compute(source: &Buffer) -> Self {
        Self(source.count_lines())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct MaxLineDigit(pub(super) usize);

impl Compute for MaxLineDigit {
    type Source = LineCount;
    fn compute(source: &LineCount) -> Self {
        Self(format!("{}", source.0).chars().count())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct CurrentLine(pub(super) String);

impl Compute for CurrentLine {
    type Source = (Buffer, CursorRow);
    fn compute(source: &Self::Source) -> Self {
        Self(source.0.line(source.1 .0).as_str().to_string())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct CursorRow(pub(super) usize);

impl Compute for CursorRow {
    type Source = Cursor;
    fn compute(source: &Self::Source) -> Self {
        Self(source.0.row)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct Cursor(pub(super) Position);
impl Compute for Cursor {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        Self(source.cursor)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct TerminalHeight(pub(super) usize);

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
pub(super) struct RowOffset(pub(super) usize);

impl Compute for RowOffset {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        Self(source.row_offset)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct LineRange(pub(super) Range<usize>);

impl Compute for LineRange {
    type Source = (RowOffset, LineCount, TerminalHeight);
    fn compute(source: &Self::Source) -> Self {
        let row_offset = source.0 .0;
        let line_count = source.1 .0;
        let textarea_row = source.2 .0 - 2;

        Self(row_offset..min(line_count, textarea_row + row_offset + 1))
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct SearchPattern(pub(super) String);

impl Compute for SearchPattern {
    type Source = State;
    fn compute(source: &Self::Source) -> Self {
        Self(source.search_pattern.clone())
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct MatchPositions(pub(super) Vec<(Position, usize)>);

impl Compute for MatchPositions {
    type Source = (SearchPattern, Buffer);
    fn compute(source: &Self::Source) -> Self {
        let pattern = &source.0 .0;
        if pattern.is_empty() {
            return Self(Vec::new());
        }
        let re = Regex::new(pattern);
        if re.is_err() {
            return Self(Vec::new());
        }
        let re = re.unwrap();
        let result = re
            .find_iter(&source.1.as_str())
            .map(|m| {
                let range = m.range();
                let start_position = source.1.get_cursor_by_byte(range.start);
                let end_position = source.1.get_cursor_by_byte(range.end);
                (
                    Position {
                        row: start_position.row,
                        col: start_position.col,
                    },
                    end_position.col - start_position.col,
                )
            })
            .collect();
        Self(result)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct MatchPositionsInView(pub(super) Vec<(Position, usize)>);

impl Compute for MatchPositionsInView {
    type Source = (MatchPositions, LineRange);
    fn compute(source: &Self::Source) -> Self {
        let mut result = Vec::new();
        let line_range = &source.1 .0;
        for (pos, l) in &source.0 .0 {
            if line_range.start > pos.row {
                continue;
            }
            if line_range.end <= pos.row {
                break;
            }
            result.push((
                Position {
                    row: pos.row - line_range.start,
                    col: pos.col,
                },
                *l,
            ));
        }
        Self(result)
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(super) struct CursorView(pub(super) Position);

impl Compute for CursorView {
    type Source = (Cursor, Mode, MatchPositions);

    fn compute(source: &Self::Source) -> Self {
        let cursor = &source.0 .0;
        if source.1 != Mode::Search {
            return CursorView(Position {
                row: cursor.row,
                col: cursor.col,
            });
        }

        let matches = &source.2 .0;

        if matches.is_empty() {
            return CursorView(Position {
                row: cursor.row,
                col: cursor.col,
            });
        }

        for (pos, _) in matches {
            if pos.row == cursor.row && pos.col >= cursor.col {
                return CursorView(*pos);
            }

            if pos.row > cursor.row {
                return CursorView(*pos);
            }
        }

        let pos = matches.first().unwrap().0;
        CursorView(pos)
    }
}
