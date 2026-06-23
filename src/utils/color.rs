//! Color utilities and the visualization palette.
//!
//! Colors are authored in the familiar sRGB 0-255 space and converted to
//! *linear* RGB before being handed to the GPU. This matters: our swap-chain
//! uses an sRGB texture format, so the hardware automatically encodes the
//! linear values we write back into sRGB. Authoring in sRGB and converting once
//! here keeps the on-screen colors faithful to the palette below.

/// Linear RGBA color, ready to be uploaded to the GPU.
pub type LinearRgba = [f32; 4];

/// Convert a single sRGB channel (0.0-1.0) to linear space.
fn srgb_channel_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert an 8-bit-per-channel sRGB color to linear RGBA.
pub const fn rgb(r: u8, g: u8, b: u8) -> Srgb {
    Srgb { r, g, b }
}

/// An sRGB color authored as 8-bit channels. Convert with [`Srgb::to_linear`].
#[derive(Clone, Copy, Debug)]
pub struct Srgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Srgb {
    /// Convert to linear RGBA (alpha is always 1.0).
    pub fn to_linear(self) -> LinearRgba {
        [
            srgb_channel_to_linear(self.r as f32 / 255.0),
            srgb_channel_to_linear(self.g as f32 / 255.0),
            srgb_channel_to_linear(self.b as f32 / 255.0),
            1.0,
        ]
    }
}

/// The "algorithm observatory" palette used throughout the visualization.
///
/// Each entry maps to an [`crate::simulation::ElementState`] (see
/// [`crate::utils::color::state_color`]) plus a couple of scene colors.
pub mod palette {
    use super::{rgb, Srgb};

    /// Resting elements: calm cyan/blue.
    pub const NORMAL: Srgb = rgb(56, 152, 224);
    /// Elements being compared this step: yellow.
    pub const COMPARING: Srgb = rgb(240, 200, 64);
    /// Elements being swapped or overwritten: warm orange/red.
    pub const SWAPPING: Srgb = rgb(232, 96, 48);
    /// The active pivot (quick sort) or running minimum (selection): magenta.
    pub const PIVOT: Srgb = rgb(186, 96, 232);
    /// Finalized elements that are in their sorted position: green.
    pub const SORTED: Srgb = rgb(72, 200, 120);

    /// Scene clear color: near-black dark navy.
    pub const BACKGROUND: Srgb = rgb(10, 12, 20);
}

/// Map an [`ElementState`](crate::simulation::ElementState) to its linear color.
///
/// This is the single place that ties simulation state to on-screen color, so
/// adding a new state only requires touching the palette and this function -
/// the renderer stays oblivious to color semantics.
pub fn state_color(state: crate::simulation::ElementState) -> LinearRgba {
    use crate::simulation::ElementState::*;
    match state {
        Normal => palette::NORMAL.to_linear(),
        Comparing => palette::COMPARING.to_linear(),
        Swapping => palette::SWAPPING.to_linear(),
        Pivot => palette::PIVOT.to_linear(),
        Sorted => palette::SORTED.to_linear(),
    }
}

/// The wgpu clear color for the scene background, in linear space.
pub fn background_clear() -> wgpu::Color {
    let [r, g, b, a] = palette::BACKGROUND.to_linear();
    wgpu::Color {
        r: r as f64,
        g: g as f64,
        b: b as f64,
        a: a as f64,
    }
}
