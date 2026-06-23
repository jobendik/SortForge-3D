//! The event vocabulary that connects algorithms to the renderer.
//!
//! This is the architectural keystone of SortForge 3D. Sorting algorithms never
//! touch the array the user sees, and they never touch the GPU. Instead they
//! emit a flat, ordered list of [`SortEvent`]s describing *what they did*. The
//! simulation replays those events to drive an [`ArrayState`](super::ArrayState),
//! and the renderer only ever reads that state.
//!
//! Because the event stream is data (not behavior), it can be paused, stepped,
//! reset, and inspected freely - and brand new algorithms can be added without
//! the renderer or playback code knowing anything about them.

/// A single observable action taken by a sorting algorithm.
///
/// The variants are intentionally small and orthogonal. Adding a new algorithm
/// should almost always be expressible in terms of the existing variants; if a
/// new *visual* concept is genuinely needed, adding a variant here and a branch
/// in [`ArrayState::apply`](super::ArrayState::apply) is all that's required -
/// the renderer is unaffected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortEvent {
    /// The algorithm compared the elements at indices `i` and `j`.
    Compare { i: usize, j: usize },
    /// The algorithm swapped the elements at indices `i` and `j`.
    Swap { i: usize, j: usize },
    /// The algorithm wrote `value` into `index` (e.g. insertion-sort shifts).
    Overwrite { index: usize, value: u32 },
    /// `index` was chosen as the current pivot / running minimum.
    SetPivot { index: usize },
    /// `index` is now in its final sorted position and should stay highlighted.
    MarkSorted { index: usize },
    /// Drop all transient highlights (comparisons, pivots) but keep "sorted".
    ClearHighlights,
    /// The algorithm finished; every element is in its final position.
    AlgorithmFinished,
}

/// The visual state of a single array element, derived purely from events.
///
/// The renderer maps these to colors via
/// [`state_color`](crate::utils::color::state_color); it has no idea *why* an
/// element is in a given state, which keeps rendering decoupled from algorithms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElementState {
    /// Resting element.
    #[default]
    Normal,
    /// Currently being compared.
    Comparing,
    /// Currently being swapped or overwritten.
    Swapping,
    /// The active pivot or running minimum.
    Pivot,
    /// Finalized in its sorted position (sticky until reset).
    Sorted,
}
