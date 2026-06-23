//! Counters describing how much work an algorithm performed.
//!
//! Metrics are derived from events, never guessed: [`Metrics::record`] is fed
//! the same event stream the [`ArrayState`](super::ArrayState) consumes, so the
//! numbers on screen always match the animation exactly.

use super::SortEvent;

/// Work counters for the current run.
///
/// `writes` counts individual array slot writes: a [`SortEvent::Swap`] touches
/// two slots, an [`SortEvent::Overwrite`] touches one. This mirrors the cost
/// model most sorting references use, so the figures are directly comparable
/// across algorithms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    pub comparisons: u64,
    pub swaps: u64,
    pub writes: u64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a single event into the counters.
    pub fn record(&mut self, event: &SortEvent) {
        match event {
            SortEvent::Compare { .. } => self.comparisons += 1,
            SortEvent::Swap { .. } => {
                self.swaps += 1;
                self.writes += 2;
            }
            SortEvent::Overwrite { .. } => self.writes += 1,
            // Pivot/sorted/clear/finished are bookkeeping, not array work.
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
