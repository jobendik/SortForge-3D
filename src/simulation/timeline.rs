//! An ordered, replayable list of events with a playback cursor.
//!
//! A [`Timeline`] is "dumb on purpose": it owns the recorded
//! [`SortEvent`]s and a cursor into them. It knows how to advance and rewind the
//! cursor but nothing about *time*, speed, or array state - that orchestration
//! lives in [`PlaybackController`](super::PlaybackController). Keeping the
//! cursor here makes stepping, scrubbing, and "is it finished?" trivial.

use super::SortEvent;

/// A recorded run of an algorithm plus a cursor marking how far it has played.
#[derive(Clone, Debug, Default)]
pub struct Timeline {
    events: Vec<SortEvent>,
    /// Number of events already applied (`0..=events.len()`).
    cursor: usize,
}

impl Timeline {
    pub fn new(events: Vec<SortEvent>) -> Self {
        Self { events, cursor: 0 }
    }

    /// Total number of events in the run.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// How many events have been applied so far (the current step index).
    pub fn position(&self) -> usize {
        self.cursor
    }

    /// `true` once every event has been applied.
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.events.len()
    }

    /// Move the cursor back to the start without dropping the events.
    pub fn rewind(&mut self) {
        self.cursor = 0;
    }

    /// Return the next event and advance the cursor, or `None` at the end.
    pub fn advance(&mut self) -> Option<SortEvent> {
        let event = self.events.get(self.cursor).copied();
        if event.is_some() {
            self.cursor += 1;
        }
        event
    }

    /// Borrow the full event list (used when replaying from scratch).
    pub fn events(&self) -> &[SortEvent] {
        &self.events
    }
}
