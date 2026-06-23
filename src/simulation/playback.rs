//! Playback orchestration: the bridge between algorithms and the renderer.
//!
//! [`PlaybackController`] owns everything needed to play a sort back in real
//! time and is the *only* object the application layer mutates:
//!
//! * the original (unsorted) array, so a reset is exact;
//! * the current [`ArrayState`] the renderer draws;
//! * the [`Timeline`] of recorded events plus its cursor;
//! * the [`Metrics`] counters;
//! * playback state - speed, play/pause, and a time accumulator.
//!
//! Crucially it advances by *consuming events*, never by re-running the
//! algorithm. The algorithm runs once when the timeline is (re)built; after
//! that, play / pause / step / reset are all just cursor movement plus
//! `ArrayState::apply`.

use rand::seq::SliceRandom;

use super::{ArrayState, Metrics, Timeline};
use crate::algorithms::Algorithm;

/// Minimum and maximum playback speed in events per second.
pub const MIN_SPEED: f32 = 1.0;
pub const MAX_SPEED: f32 = 2000.0;

/// Safety cap on how many events a single `update` may apply, so an enormous
/// speed (or a long stall) can never lock up the frame in a runaway loop.
const MAX_STEPS_PER_UPDATE: usize = 20_000;

/// Owns the simulation and drives it forward in time.
pub struct PlaybackController {
    algorithm: Algorithm,
    /// The pristine array; `reset` restores `state` from this.
    original: Vec<u32>,
    state: ArrayState,
    timeline: Timeline,
    metrics: Metrics,

    playing: bool,
    /// Playback speed in events per second.
    speed: f32,
    /// Fractional time-credit carried between frames so slow speeds still
    /// advance smoothly instead of snapping on whole-second boundaries.
    accumulator: f32,
}

impl PlaybackController {
    /// Build a controller with a fresh random array of `size` elements.
    pub fn new(algorithm: Algorithm, size: usize) -> Self {
        let original = random_values(size);
        let mut controller = Self {
            algorithm,
            state: ArrayState::new(original.clone()),
            original,
            timeline: Timeline::default(),
            metrics: Metrics::new(),
            playing: false,
            speed: 20.0,
            accumulator: 0.0,
        };
        controller.rebuild_timeline();
        controller
    }

    // --- Timeline construction -------------------------------------------------

    /// Re-run the current algorithm over the original array and reset playback.
    ///
    /// This is the single point where an algorithm executes; everything else is
    /// pure event playback.
    pub fn rebuild_timeline(&mut self) {
        let events = self.algorithm.generate_events(&self.original);
        self.timeline = Timeline::new(events);
        self.reset();
    }

    /// Generate a brand new random array of `size` and rebuild the timeline.
    pub fn regenerate(&mut self, size: usize) {
        self.original = random_values(size);
        self.rebuild_timeline();
    }

    /// Switch algorithms, keeping the same array, and rebuild the timeline.
    pub fn set_algorithm(&mut self, algorithm: Algorithm) {
        if algorithm != self.algorithm {
            self.algorithm = algorithm;
            self.rebuild_timeline();
        }
    }

    // --- Playback control ------------------------------------------------------

    /// Restore the array to its original order and rewind the cursor.
    pub fn reset(&mut self) {
        self.state = ArrayState::new(self.original.clone());
        self.timeline.rewind();
        self.metrics.reset();
        self.accumulator = 0.0;
        self.playing = false;
    }

    /// Apply exactly one event, updating array state and metrics. Returns
    /// `false` if there was nothing left to apply.
    pub fn step(&mut self) -> bool {
        match self.timeline.advance() {
            Some(event) => {
                self.metrics.record(&event);
                self.state.apply(&event);
                true
            }
            None => {
                self.playing = false;
                false
            }
        }
    }

    /// Advance playback by `dt` seconds when playing.
    ///
    /// Converts elapsed time into a whole number of event steps using the
    /// current speed, carrying the remainder in `accumulator`.
    pub fn update(&mut self, dt: f32) {
        if !self.playing || self.timeline.is_finished() {
            return;
        }
        self.accumulator += dt * self.speed;
        let mut budget = MAX_STEPS_PER_UPDATE;
        while self.accumulator >= 1.0 && budget > 0 {
            self.accumulator -= 1.0;
            budget -= 1;
            if !self.step() {
                self.accumulator = 0.0;
                break;
            }
        }
    }

    pub fn play(&mut self) {
        // Restarting after the end replays from the beginning.
        if self.timeline.is_finished() {
            self.reset();
        }
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn toggle_play(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(MIN_SPEED, MAX_SPEED);
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    // --- Read-only accessors for renderer / UI --------------------------------

    pub fn state(&self) -> &ArrayState {
        &self.state
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// Current step index (events applied so far).
    pub fn current_step(&self) -> usize {
        self.timeline.position()
    }

    /// Total number of events in the run.
    pub fn total_steps(&self) -> usize {
        self.timeline.len()
    }

    pub fn is_finished(&self) -> bool {
        self.timeline.is_finished()
    }

    /// Playback progress in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        let total = self.timeline.len();
        if total == 0 {
            1.0
        } else {
            self.timeline.position() as f32 / total as f32
        }
    }
}

/// Produce a shuffled permutation of `1..=size` (clamped to at least one
/// element). Distinct values give visually unambiguous, evenly-spaced bar
/// heights and a clean staircase when fully sorted.
fn random_values(size: usize) -> Vec<u32> {
    let size = size.max(1);
    let mut values: Vec<u32> = (1..=size as u32).collect();
    values.shuffle(&mut rand::thread_rng());
    values
}
