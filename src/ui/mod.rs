//! The egui control panel.
//!
//! The UI is a *pure view + intent* layer: [`controls::draw`] reads an immutable
//! snapshot of the [`PlaybackController`](crate::simulation::PlaybackController)
//! and returns a [`UiActions`] describing what the user asked for. The
//! application applies those actions afterwards. This keeps egui from ever
//! mutating the simulation directly and avoids borrow tangles.

pub mod controls;

pub use controls::{UiActions, UiState};
