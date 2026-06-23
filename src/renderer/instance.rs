//! Per-bar instance data and the array->world layout.
//!
//! Each array element becomes one GPU instance of the unit cube. The instance
//! carries a model matrix (translate + non-uniform scale) and a color. Using
//! instancing means the whole array is drawn in a single `draw_indexed` call
//! regardless of element count - the part that gives this project its
//! performance story.

use glam::{Mat4, Vec3};

use crate::simulation::ArrayState;
use crate::utils::color::state_color;

/// One cube instance: model transform plus color.
///
/// Sent to the GPU as a vertex buffer with per-instance step mode. The `mat4`
/// occupies shader locations 5-8 (one `vec4` per column) and the color sits at
/// location 9 - see [`InstanceRaw::layout`].
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

impl InstanceRaw {
    const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}

/// Maps array indices and values to world-space bar transforms.
///
/// Bars are laid out along the x axis, centered on the origin, standing on the
/// ground plane. The layout scales itself to the element count so a 16-element
/// and a 200-element array both frame nicely.
#[derive(Clone, Copy, Debug)]
pub struct BarLayout {
    count: usize,
    bar_width: f32,
    bar_depth: f32,
    /// Distance between adjacent bar centers.
    spacing: f32,
    /// Height of the tallest bar in world units.
    max_height: f32,
}

impl BarLayout {
    /// Choose sensible proportions for an array of `count` elements.
    pub fn for_array(count: usize) -> Self {
        let count = count.max(1);
        let bar_width = 1.0;
        let spacing = bar_width * 1.25;
        // Taller arrays look better a bit shorter so the camera framing holds.
        let max_height = (count as f32 * 0.6).clamp(8.0, 60.0);
        Self {
            count,
            bar_width,
            bar_depth: bar_width,
            spacing,
            max_height,
        }
    }

    /// Total width spanned by the array, used for camera framing.
    pub fn span(&self) -> f32 {
        self.count as f32 * self.spacing
    }

    pub fn max_height(&self) -> f32 {
        self.max_height
    }

    /// Half-extent for the ground plane: a margin beyond the array footprint.
    pub fn ground_half_extent(&self) -> f32 {
        (self.span() * 0.5 + 6.0).max(self.max_height)
    }

    /// World-space x center of bar `index`.
    fn x_of(&self, index: usize) -> f32 {
        (index as f32 - (self.count as f32 - 1.0) * 0.5) * self.spacing
    }

    /// Map a value in `1..=max_value` to a world-space bar height.
    fn height_of(&self, value: u32, max_value: u32) -> f32 {
        let t = value as f32 / max_value.max(1) as f32;
        // A small floor keeps even the smallest bar visible.
        (t * self.max_height).max(0.4)
    }
}

/// Build the instance buffer contents for the current array state.
///
/// Pure function of `(state, layout)`: same inputs always yield the same
/// instances, which keeps the renderer stateless with respect to the sort.
pub fn build_instances(state: &ArrayState, layout: &BarLayout) -> Vec<InstanceRaw> {
    if state.is_empty() {
        return Vec::new();
    }
    let values = state.values();
    let states = state.states();
    let max_value = state.max_value();

    values
        .iter()
        .zip(states.iter())
        .enumerate()
        .map(|(i, (&value, &element_state))| {
            let height = layout.height_of(value, max_value);
            let model = Mat4::from_translation(Vec3::new(layout.x_of(i), 0.0, 0.0))
                * Mat4::from_scale(Vec3::new(layout.bar_width, height, layout.bar_depth));
            InstanceRaw {
                model: model.to_cols_array_2d(),
                color: state_color(element_state),
            }
        })
        .collect()
}
