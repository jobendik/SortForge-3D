// SortForge 3D scene shader.
//
// Two pipelines share this module:
//   * vs_bars / fs_bars   - instanced, lit cuboids (one per array element)
//   * vs_ground / fs_ground - a single quad with a procedural, distance-faded grid
//
// All colors are LINEAR; the swap-chain is an sRGB format so the hardware
// performs the linear->sRGB encode on write.

struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

// ---------------------------------------------------------------------------
// Bars
// ---------------------------------------------------------------------------

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

// Per-instance model matrix (locations 5-8) + color (location 9).
struct InstanceInput {
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(9) color: vec4<f32>,
};

struct BarVsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) color: vec3<f32>,
};

@vertex
fn vs_bars(vert: VertexInput, inst: InstanceInput) -> BarVsOut {
    let model = mat4x4<f32>(inst.m0, inst.m1, inst.m2, inst.m3);
    let world = model * vec4<f32>(vert.position, 1.0);

    var out: BarVsOut;
    out.clip_position = camera.view_proj * world;
    out.world_pos = world.xyz;
    // Bars are only translated and axis-aligned-scaled, so the object-space
    // normal is already the world-space direction (just renormalize in fs).
    out.world_normal = vert.normal;
    out.color = inst.color.rgb;
    return out;
}

@fragment
fn fs_bars(in: BarVsOut) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let light_dir = normalize(vec3<f32>(0.4, 1.0, 0.55));
    let view_dir = normalize(camera.camera_pos.xyz - in.world_pos);

    let ambient = 0.30;
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.85;

    // Subtle Blinn-Phong highlight for a clean, polished material read.
    let half_dir = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0) * 0.25;

    let lit = in.color * (ambient + diffuse) + vec3<f32>(specular);
    return vec4<f32>(lit, 1.0);
}

// ---------------------------------------------------------------------------
// Ground grid
// ---------------------------------------------------------------------------

struct GroundVsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
};

@vertex
fn vs_ground(vert: VertexInput) -> GroundVsOut {
    var out: GroundVsOut;
    // The ground quad uses identity model, so position == world position.
    out.clip_position = camera.view_proj * vec4<f32>(vert.position, 1.0);
    out.world_pos = vert.position;
    return out;
}

@fragment
fn fs_ground(in: GroundVsOut) -> @location(0) vec4<f32> {
    let cell = 2.0;
    let coord = in.world_pos.xz / cell;

    // Screen-space-derivative anti-aliased grid lines (the standard technique).
    let deriv = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / max(deriv, vec2<f32>(1e-5));
    let line = min(grid.x, grid.y);
    let intensity = 1.0 - min(line, 1.0);

    // Fade the grid out with distance so the horizon stays calm and dark.
    let dist = length(in.world_pos.xz);
    let fade = clamp(1.0 - dist / 130.0, 0.0, 1.0);

    let base = vec3<f32>(0.015, 0.02, 0.045);
    let line_color = vec3<f32>(0.09, 0.15, 0.28);
    let col = mix(base, line_color, intensity * fade);
    return vec4<f32>(col, 1.0);
}
