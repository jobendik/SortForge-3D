//! Sorting algorithms expressed as *event generators*.
//!
//! Every algorithm implements [`SortingAlgorithm`]: given a slice of values it
//! returns a `Vec<SortEvent>` describing the sort, but it never mutates shared
//! state and never renders. This is what lets the simulation pause, step, and
//! replay, and what lets new algorithms drop in without touching the renderer.
//!
//! To keep the algorithm bodies readable and to guarantee the recorded events
//! actually match the work performed, each algorithm drives a [`Recorder`]: its
//! `swap`/`overwrite` helpers mutate a working copy of the array *and* push the
//! matching event, so the two can never drift apart.

mod bubble_sort;
mod insertion_sort;
mod quick_sort;
mod selection_sort;

pub use bubble_sort::BubbleSort;
pub use insertion_sort::InsertionSort;
pub use quick_sort::QuickSort;
pub use selection_sort::SelectionSort;

use crate::simulation::SortEvent;

/// Anything that can turn an array into a stream of visualization events.
///
/// Implementors are zero-sized strategy structs; the interesting state lives in
/// the returned event list. Adding an algorithm is just a new implementor plus
/// an [`Algorithm`] enum variant.
pub trait SortingAlgorithm {
    /// Human-readable name shown in the UI.
    fn name(&self) -> &'static str;

    /// Run the algorithm against `values` and return the recorded events.
    ///
    /// `values` is borrowed and never mutated; implementors work on a copy.
    fn generate_events(&self, values: &[u32]) -> Vec<SortEvent>;
}

/// Records events while applying value mutations to a working array.
///
/// Algorithms read the working array to make decisions and call these helpers
/// to record what they do. Because `swap`/`overwrite` mutate the array *and*
/// emit the event, replaying the events from the original array is guaranteed
/// to reproduce the same final ordering.
pub struct Recorder {
    events: Vec<SortEvent>,
}

impl Recorder {
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Pre-allocate roughly enough room for an O(n^2) run to avoid reallocs.
    fn with_capacity_for(n: usize) -> Self {
        Self {
            events: Vec::with_capacity(n.saturating_mul(n).min(1 << 20)),
        }
    }

    /// Record a comparison of indices `i` and `j` (no array change).
    fn compare(&mut self, i: usize, j: usize) {
        self.events.push(SortEvent::Compare { i, j });
    }

    /// Swap two elements in the working array and record the swap.
    fn swap(&mut self, array: &mut [u32], i: usize, j: usize) {
        array.swap(i, j);
        self.events.push(SortEvent::Swap { i, j });
    }

    /// Write `value` into `index` of the working array and record the write.
    fn overwrite(&mut self, array: &mut [u32], index: usize, value: u32) {
        array[index] = value;
        self.events.push(SortEvent::Overwrite { index, value });
    }

    /// Mark `index` as the active pivot / running minimum.
    fn set_pivot(&mut self, index: usize) {
        self.events.push(SortEvent::SetPivot { index });
    }

    /// Mark `index` as finalized in its sorted position.
    fn mark_sorted(&mut self, index: usize) {
        self.events.push(SortEvent::MarkSorted { index });
    }

    /// Drop transient highlights (keeps "sorted").
    fn clear_highlights(&mut self) {
        self.events.push(SortEvent::ClearHighlights);
    }

    /// Finish the run: emit `AlgorithmFinished` and hand back the events.
    fn finish(mut self) -> Vec<SortEvent> {
        self.events.push(SortEvent::AlgorithmFinished);
        self.events
    }
}

/// The set of algorithms selectable in the UI.
///
/// This enum is the stable handle the application and UI pass around;
/// [`Algorithm::generate_events`] dispatches to the matching strategy struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
    Bubble,
    Insertion,
    Selection,
    Quick,
}

impl Algorithm {
    /// Every algorithm, in display order. Iterating this is how the UI builds
    /// its selector, so a new variant shows up automatically.
    pub const ALL: [Algorithm; 4] = [
        Algorithm::Bubble,
        Algorithm::Insertion,
        Algorithm::Selection,
        Algorithm::Quick,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Algorithm::Bubble => BubbleSort.name(),
            Algorithm::Insertion => InsertionSort.name(),
            Algorithm::Selection => SelectionSort.name(),
            Algorithm::Quick => QuickSort.name(),
        }
    }

    /// Big-O of the average-case time complexity, shown in the metrics panel.
    pub fn complexity(&self) -> &'static str {
        match self {
            Algorithm::Bubble => "O(n^2)",
            Algorithm::Insertion => "O(n^2)",
            Algorithm::Selection => "O(n^2)",
            Algorithm::Quick => "O(n log n) avg",
        }
    }

    /// Generate the event stream for this algorithm over `values`.
    pub fn generate_events(&self, values: &[u32]) -> Vec<SortEvent> {
        match self {
            Algorithm::Bubble => BubbleSort.generate_events(values),
            Algorithm::Insertion => InsertionSort.generate_events(values),
            Algorithm::Selection => SelectionSort.generate_events(values),
            Algorithm::Quick => QuickSort.generate_events(values),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::ArrayState;

    /// Replay an algorithm's events against a fresh `ArrayState` built from
    /// `input`, then assert the array is genuinely sorted, is a permutation of
    /// the input, and that every element ended up flagged `Sorted`.
    ///
    /// This is the credibility check the whole project hinges on: it proves the
    /// *events* (not some private array the algorithm kept) produce a correct
    /// sort, which is exactly what the renderer animates.
    fn assert_events_sort(algorithm: Algorithm, input: &[u32]) {
        let events = algorithm.generate_events(input);

        let mut state = ArrayState::new(input.to_vec());
        for event in &events {
            state.apply(event);
        }

        assert!(
            state.is_sorted(),
            "{} did not sort {:?} -> {:?}",
            algorithm.name(),
            input,
            state.values()
        );

        // The replayed values must be a permutation of the input (no values
        // invented or lost by Swap/Overwrite events).
        let mut got = state.values().to_vec();
        let mut expected = input.to_vec();
        got.sort_unstable();
        expected.sort_unstable();
        assert_eq!(got, expected, "{} changed the multiset", algorithm.name());

        // A finished run must leave every element marked sorted.
        assert!(
            state
                .states()
                .iter()
                .all(|s| *s == crate::simulation::ElementState::Sorted),
            "{} left elements unmarked",
            algorithm.name()
        );
    }

    /// A spread of inputs: already-sorted, reversed, duplicates, single, empty,
    /// and a fixed "tricky" permutation. Run against every algorithm.
    fn test_inputs() -> Vec<Vec<u32>> {
        vec![
            vec![],
            vec![1],
            vec![2, 1],
            vec![1, 2, 3, 4, 5],
            vec![5, 4, 3, 2, 1],
            vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5],
            vec![5, 1, 4, 2, 8, 3, 7, 6, 9, 0],
            vec![7, 7, 7, 7, 7],
        ]
    }

    #[test]
    fn all_algorithms_sort_all_inputs() {
        for algorithm in Algorithm::ALL {
            for input in test_inputs() {
                assert_events_sort(algorithm, &input);
            }
        }
    }

    #[test]
    fn handles_a_larger_random_array() {
        // Deterministic pseudo-random fill so the test is reproducible.
        let mut x: u32 = 0x1234_5678;
        let input: Vec<u32> = (0..128)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x % 1000
            })
            .collect();
        for algorithm in Algorithm::ALL {
            assert_events_sort(algorithm, &input);
        }
    }

    #[test]
    fn metrics_match_event_counts() {
        use crate::simulation::Metrics;
        let input = vec![5, 4, 3, 2, 1];
        let events = Algorithm::Bubble.generate_events(&input);
        let mut metrics = Metrics::new();
        for e in &events {
            metrics.record(e);
        }
        // Worst-case bubble sort on n=5 reversed: 10 comparisons, 10 swaps.
        assert_eq!(metrics.comparisons, 10);
        assert_eq!(metrics.swaps, 10);
        assert_eq!(metrics.writes, 20); // two slot-writes per swap
    }
}
