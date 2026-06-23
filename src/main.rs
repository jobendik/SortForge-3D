//! SortForge 3D - a real-time 3D sorting algorithm visualization engine.
//!
//! The crate is organized as a small visualization engine with a strict,
//! one-directional data flow:
//!
//! ```text
//!   algorithms ──emit──▶ SortEvents ──▶ simulation (timeline + playback)
//!                                              │ state
//!                                              ▼
//!                          ui (egui) ◀── app ──▶ renderer (wgpu)
//! ```
//!
//! * [`algorithms`] turn an array into a stream of events; they never render.
//! * [`simulation`] replays events to drive the array state, metrics, and
//!   playback (play / pause / step / reset / speed).
//! * [`renderer`] draws the current state with wgpu; it never sorts.
//! * [`ui`] reads a snapshot and emits user intent.
//! * [`app`] wires them together around a winit event loop.

mod algorithms;
mod app;
mod renderer;
mod simulation;
mod ui;
mod utils;

use anyhow::Result;

fn main() -> Result<()> {
    // `info` by default; override with e.g. `RUST_LOG=sortforge_3d=debug`.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting SortForge 3D");
    app::run()
}
