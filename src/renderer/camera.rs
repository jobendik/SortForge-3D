//! An orbit camera and its GPU uniform.
//!
//! The camera orbits a fixed `target` (the center of the bar array) at a given
//! `distance`, `yaw`, and `pitch`. This gives the default angled "observatory"
//! view and supports mouse drag-to-orbit / wheel-to-zoom without any
//! per-frame allocation. Projection uses [`glam::Mat4::perspective_rh`], whose
//! `0..1` depth range matches wgpu's NDC (no GL-style correction matrix needed).

use glam::{Mat4, Vec3};

/// Data uploaded to the GPU each frame for transforming and lighting bars.
///
/// `#[repr(C)]` + `bytemuck` lets us copy it straight into a uniform buffer.
/// Layout is std140-friendly: a `mat4x4<f32>` (64 B) followed by a `vec4<f32>`
/// (16 B), so no padding surprises.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// Combined view-projection matrix (column-major).
    pub view_proj: [[f32; 4]; 4],
    /// World-space camera position; `w` is unused padding for alignment.
    pub camera_pos: [f32; 4],
}

/// An orbiting perspective camera.
pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    /// Horizontal orbit angle in radians.
    pub yaw: f32,
    /// Vertical orbit angle in radians (clamped away from the poles).
    pub pitch: f32,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    /// Create the default angled view for an array whose layout spans roughly
    /// `span` world units across and `height` units tall.
    pub fn new(aspect: f32, span: f32, height: f32) -> Self {
        let mut camera = Self {
            target: Vec3::new(0.0, height * 0.35, 0.0),
            distance: 1.0,
            yaw: -std::f32::consts::FRAC_PI_2 - 0.5, // slightly off head-on
            pitch: 0.45,                             // look down at ~26 degrees
            aspect,
            fovy: 50f32.to_radians(),
            znear: 0.1,
            zfar: 1000.0,
        };
        camera.frame(span, height);
        camera
    }

    /// Re-center and re-distance the camera to frame an array of the given
    /// span/height. Called when the array size changes.
    pub fn frame(&mut self, span: f32, height: f32) {
        self.target = Vec3::new(0.0, height * 0.35, 0.0);
        // Pull back far enough that the whole array fits comfortably in view.
        self.distance = (span * 0.75).max(height * 1.5).max(6.0);
    }

    /// World-space camera position derived from the orbit parameters.
    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let offset = Vec3::new(cp * cy, sp, cp * sy) * self.distance;
        self.target + offset
    }

    pub fn set_aspect(&mut self, width: f32, height: f32) {
        self.aspect = if height > 0.0 { width / height } else { 1.0 };
    }

    /// Orbit by mouse-pixel deltas (called while dragging).
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        const SENSITIVITY: f32 = 0.005;
        self.yaw += delta_yaw * SENSITIVITY;
        self.pitch = (self.pitch + delta_pitch * SENSITIVITY).clamp(0.05, 1.5);
    }

    /// Dolly in/out from a scroll delta (positive = zoom in).
    pub fn zoom(&mut self, delta: f32) {
        let factor = (1.0 - delta * 0.1).clamp(0.5, 1.5);
        self.distance = (self.distance * factor).clamp(2.0, 600.0);
    }

    fn view_proj(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }

    /// Build the GPU uniform for this frame.
    pub fn uniform(&self) -> CameraUniform {
        let eye = self.eye();
        CameraUniform {
            view_proj: self.view_proj().to_cols_array_2d(),
            camera_pos: [eye.x, eye.y, eye.z, 1.0],
        }
    }
}
