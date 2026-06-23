//! The reconstructable array state that the renderer draws.
//!
//! [`ArrayState`] is the single source of truth for "what does the array look
//! like right now": the values *and* the per-element highlight states. It is
//! mutated exclusively by applying [`SortEvent`]s, which is what makes the whole
//! timeline replayable - applying every event from `0..k` against a fresh copy
//! of the original array always reproduces the exact state at step `k`.

use super::{ElementState, SortEvent};

/// Current values plus per-element [`ElementState`], driven entirely by events.
#[derive(Clone, Debug)]
pub struct ArrayState {
    values: Vec<u32>,
    states: Vec<ElementState>,
    /// The maximum value present, cached so the renderer can normalize heights
    /// without rescanning the array every frame.
    max_value: u32,
}

impl ArrayState {
    /// Create a fresh state from the given values, all elements `Normal`.
    pub fn new(values: Vec<u32>) -> Self {
        let max_value = values.iter().copied().max().unwrap_or(1).max(1);
        let states = vec![ElementState::Normal; values.len()];
        Self {
            values,
            states,
            max_value,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn values(&self) -> &[u32] {
        &self.values
    }

    pub fn states(&self) -> &[ElementState] {
        &self.states
    }

    pub fn max_value(&self) -> u32 {
        self.max_value
    }

    /// `true` when the values are in non-decreasing order. Used by tests and by
    /// a `debug_assert` sanity check after a timeline finishes playing.
    pub fn is_sorted(&self) -> bool {
        self.values.windows(2).all(|w| w[0] <= w[1])
    }

    /// Apply one event, updating both values and highlight states.
    ///
    /// The highlight rules are designed so that *forward stepping* and *replay
    /// from scratch* produce identical results:
    ///
    /// * Transient highlights (`Comparing`/`Swapping`) are cleared on the next
    ///   comparison/swap so only the current action glows.
    /// * `Pivot` and `Sorted` are preserved through comparisons - a pivot keeps
    ///   its color even while it is being compared against.
    /// * `Sorted` is sticky and only cleared by a full reset (rebuilding the
    ///   state), never by `ClearHighlights`-style events at finer granularity...
    ///   except `ClearHighlights` itself, which the algorithms emit at partition
    ///   boundaries to wipe the pivot.
    pub fn apply(&mut self, event: &SortEvent) {
        match *event {
            SortEvent::Compare { i, j } => {
                self.clear_transient();
                self.set_highlight(i, ElementState::Comparing);
                self.set_highlight(j, ElementState::Comparing);
            }
            SortEvent::Swap { i, j } => {
                self.clear_transient();
                self.values.swap(i, j);
                self.set_highlight(i, ElementState::Swapping);
                self.set_highlight(j, ElementState::Swapping);
            }
            SortEvent::Overwrite { index, value } => {
                self.clear_transient();
                self.values[index] = value;
                self.set_highlight(index, ElementState::Swapping);
            }
            SortEvent::SetPivot { index } => {
                // Only one pivot at a time: demote any previous pivot first.
                for s in &mut self.states {
                    if *s == ElementState::Pivot {
                        *s = ElementState::Normal;
                    }
                }
                if self.states[index] != ElementState::Sorted {
                    self.states[index] = ElementState::Pivot;
                }
            }
            SortEvent::MarkSorted { index } => {
                self.states[index] = ElementState::Sorted;
            }
            SortEvent::ClearHighlights => {
                for s in &mut self.states {
                    if *s != ElementState::Sorted {
                        *s = ElementState::Normal;
                    }
                }
            }
            SortEvent::AlgorithmFinished => {
                for s in &mut self.states {
                    *s = ElementState::Sorted;
                }
            }
        }
    }

    /// Reset `Comparing`/`Swapping` elements back to `Normal`, leaving `Pivot`
    /// and `Sorted` untouched.
    fn clear_transient(&mut self) {
        for s in &mut self.states {
            if matches!(s, ElementState::Comparing | ElementState::Swapping) {
                *s = ElementState::Normal;
            }
        }
    }

    /// Set a transient highlight, but never override a `Pivot` or `Sorted`
    /// element (those carry more important meaning).
    fn set_highlight(&mut self, index: usize, state: ElementState) {
        if matches!(
            self.states[index],
            ElementState::Pivot | ElementState::Sorted
        ) {
            return;
        }
        self.states[index] = state;
    }
}
