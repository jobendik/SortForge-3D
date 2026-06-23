//! The simulation layer: event model, replayable state, metrics, and playback.
//!
//! ```text
//!   algorithms ──emit──▶ Vec<SortEvent> ──build──▶ Timeline
//!                                                     │
//!                                                     ▼
//!                                            PlaybackController
//!                                             ├─ ArrayState  (what to draw)
//!                                             └─ Metrics     (work counters)
//! ```
//!
//! Nothing in this module knows about the GPU, and nothing here runs a sorting
//! algorithm during playback - algorithms execute once to fill the timeline,
//! then playback is pure, deterministic event replay.

mod array_state;
mod metrics;
mod playback;
mod sort_event;
mod timeline;

pub use array_state::ArrayState;
pub use metrics::Metrics;
pub use playback::{PlaybackController, MAX_SPEED, MIN_SPEED};
pub use sort_event::{ElementState, SortEvent};
pub use timeline::Timeline;
