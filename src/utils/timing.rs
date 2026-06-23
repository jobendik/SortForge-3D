//! Frame timing helpers.
//!
//! Kept tiny and native-focused (uses [`std::time::Instant`]). A WASM build
//! would swap this for `web-time`/`instant`; the rest of the engine only ever
//! sees the `f32` delta seconds produced here.

use std::time::Instant;

/// Tracks per-frame delta time and a smoothed frames-per-second estimate.
pub struct FrameTimer {
    last: Instant,
    /// Exponentially smoothed frame duration in seconds.
    smoothed_dt: f32,
    fps: f32,
}

impl FrameTimer {
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            smoothed_dt: 1.0 / 60.0,
            fps: 60.0,
        }
    }

    /// Advance the timer, returning the delta time since the previous call.
    ///
    /// The delta is clamped to avoid huge jumps after the window is dragged,
    /// minimized, or the process is paused by the OS - a single enormous `dt`
    /// would otherwise fast-forward the whole sort in one frame.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().clamp(0.0, 0.1);
        self.last = now;

        // Smooth the FPS readout so the UI number does not flicker.
        self.smoothed_dt = self.smoothed_dt * 0.9 + dt * 0.1;
        if self.smoothed_dt > 0.0 {
            self.fps = 1.0 / self.smoothed_dt;
        }
        dt
    }

    /// Smoothed frames-per-second, suitable for display.
    pub fn fps(&self) -> f32 {
        self.fps
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}
