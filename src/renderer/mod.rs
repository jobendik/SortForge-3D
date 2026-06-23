//! The rendering layer: turns simulation state into pixels via wgpu.
//!
//! The renderer is a *consumer* of state. It never advances the sort; the
//! application hands it a [`Camera`], an [`ArrayState`](crate::simulation::ArrayState),
//! and a [`BarLayout`] each frame and asks it to draw. This keeps the GPU code
//! independent of how (or whether) the array is changing.

mod camera;
mod instance;
mod mesh;
// The `renderer` submodule deliberately mirrors the requested project layout
// (`renderer/renderer.rs`); the inception lint is expected here.
#[allow(clippy::module_inception)]
mod renderer;

pub use camera::Camera;
pub use instance::BarLayout;
pub use renderer::{EguiFrame, Renderer};

#[cfg(test)]
mod tests {
    /// Parse and validate the WGSL shader with naga. This catches syntax and
    /// type errors at `cargo test` time - no GPU required - which would
    /// otherwise only surface at runtime during pipeline creation.
    #[test]
    fn wgsl_shader_parses_and_validates() {
        let source = include_str!("shader.wgsl");
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("WGSL parse error:\n{}", e.emit_to_string(source)));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .expect("WGSL failed naga validation");
    }
}
