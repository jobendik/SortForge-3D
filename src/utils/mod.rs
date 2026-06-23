//! Small, dependency-light helpers shared across the engine.
//!
//! These deliberately avoid touching the GPU or the simulation so they can be
//! reused (and unit-tested) in isolation.

pub mod color;
pub mod timing;
