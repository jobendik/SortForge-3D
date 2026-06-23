//! Static geometry: the unit bar cube and the ground quad.
//!
//! Both meshes share one [`Vertex`] format (position + normal). The cube is the
//! mesh instanced once per array element; the ground is a single large quad
//! whose grid lines are drawn procedurally in the fragment shader.

/// A single mesh vertex: object-space position and normal.
///
/// Because bars are only ever translated and axis-aligned-scaled (never
/// rotated), object-space normals double as world-space normals, so the shader
/// can light them directly without a normal matrix.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

const fn v(position: [f32; 3], normal: [f32; 3]) -> Vertex {
    Vertex { position, normal }
}

/// Unit cube with its base on the ground plane: x,z in `[-0.5, 0.5]`, y in
/// `[0.0, 1.0]`. Scaling y by a bar's height keeps it standing on the floor.
///
/// Four vertices per face give each face a flat, correct normal (24 verts, 36
/// indices).
pub fn cube() -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        // +Y (top)
        v([-0.5, 1.0, -0.5], [0.0, 1.0, 0.0]),
        v([0.5, 1.0, -0.5], [0.0, 1.0, 0.0]),
        v([0.5, 1.0, 0.5], [0.0, 1.0, 0.0]),
        v([-0.5, 1.0, 0.5], [0.0, 1.0, 0.0]),
        // -Y (bottom)
        v([-0.5, 0.0, 0.5], [0.0, -1.0, 0.0]),
        v([0.5, 0.0, 0.5], [0.0, -1.0, 0.0]),
        v([0.5, 0.0, -0.5], [0.0, -1.0, 0.0]),
        v([-0.5, 0.0, -0.5], [0.0, -1.0, 0.0]),
        // +Z (front)
        v([-0.5, 0.0, 0.5], [0.0, 0.0, 1.0]),
        v([0.5, 0.0, 0.5], [0.0, 0.0, 1.0]),
        v([0.5, 1.0, 0.5], [0.0, 0.0, 1.0]),
        v([-0.5, 1.0, 0.5], [0.0, 0.0, 1.0]),
        // -Z (back)
        v([0.5, 0.0, -0.5], [0.0, 0.0, -1.0]),
        v([-0.5, 0.0, -0.5], [0.0, 0.0, -1.0]),
        v([-0.5, 1.0, -0.5], [0.0, 0.0, -1.0]),
        v([0.5, 1.0, -0.5], [0.0, 0.0, -1.0]),
        // +X (right)
        v([0.5, 0.0, 0.5], [1.0, 0.0, 0.0]),
        v([0.5, 0.0, -0.5], [1.0, 0.0, 0.0]),
        v([0.5, 1.0, -0.5], [1.0, 0.0, 0.0]),
        v([0.5, 1.0, 0.5], [1.0, 0.0, 0.0]),
        // -X (left)
        v([-0.5, 0.0, -0.5], [-1.0, 0.0, 0.0]),
        v([-0.5, 0.0, 0.5], [-1.0, 0.0, 0.0]),
        v([-0.5, 1.0, 0.5], [-1.0, 0.0, 0.0]),
        v([-0.5, 1.0, -0.5], [-1.0, 0.0, 0.0]),
    ];

    // Two triangles per face, wound counter-clockwise (front face = CCW).
    let mut indices = Vec::with_capacity(36);
    for face in 0..6u16 {
        let o = face * 4;
        indices.extend_from_slice(&[o, o + 1, o + 2, o, o + 2, o + 3]);
    }

    (vertices, indices)
}

/// A flat ground quad centered at the origin, spanning `[-half, half]` in x and
/// z at y = 0. Its vertex positions are world positions (identity model), which
/// the grid shader uses to place grid lines.
pub fn ground(half: f32) -> (Vec<Vertex>, Vec<u16>) {
    let n = [0.0, 1.0, 0.0];
    let vertices = vec![
        v([-half, 0.0, -half], n),
        v([half, 0.0, -half], n),
        v([half, 0.0, half], n),
        v([-half, 0.0, half], n),
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    (vertices, indices)
}
