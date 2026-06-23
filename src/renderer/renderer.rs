//! The wgpu renderer: owns all GPU state and draws one frame on demand.
//!
//! Responsibilities are deliberately narrow. The renderer knows how to draw a
//! ground grid and an instanced array of colored bars given a camera and an
//! [`ArrayState`], and how to composite an egui overlay on top. It knows
//! nothing about sorting, playback, or input - it is handed state and asked to
//! draw it.

use std::sync::Arc;

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::camera::Camera;
use super::instance::{build_instances, BarLayout, InstanceRaw};
use super::mesh::{self, Vertex};
use crate::simulation::ArrayState;
use crate::utils::color::background_clear;

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Tessellated egui output for one frame, produced by the application layer.
pub struct EguiFrame<'a> {
    pub textures_delta: &'a egui::TexturesDelta,
    pub paint_jobs: &'a [egui::ClippedPrimitive],
    pub pixels_per_point: f32,
}

/// Owns every GPU resource and renders a frame.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    depth_view: wgpu::TextureView,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    bar_pipeline: wgpu::RenderPipeline,
    ground_pipeline: wgpu::RenderPipeline,

    cube_vbuf: wgpu::Buffer,
    cube_ibuf: wgpu::Buffer,
    cube_index_count: u32,

    ground_vbuf: wgpu::Buffer,
    ground_ibuf: wgpu::Buffer,
    ground_index_count: u32,
    /// Half-extent the ground buffer was built for, so we only rebuild it when
    /// the array (and therefore the desired footprint) actually changes.
    ground_half_extent: f32,

    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    instance_count: u32,

    egui_renderer: egui_wgpu::Renderer,
}

impl Renderer {
    /// Initialize wgpu against `window` and build all pipelines and buffers.
    pub async fn new(window: Arc<Window>, initial_layout: &BarLayout) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .context("failed to create rendering surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("no suitable GPU adapter found")?;
        log::info!("Using adapter: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sortforge-device"),
                ..Default::default()
            })
            .await
            .context("failed to create GPU device")?;

        let caps = surface.get_capabilities(&adapter);
        // Prefer an sRGB surface so both the scene and egui display correctly.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_view = create_depth_view(&device, config.width, config.height);

        // --- Camera uniform + bind group --------------------------------------
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-uniform"),
            size: std::mem::size_of::<super::camera::CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // --- Pipelines ---------------------------------------------------------
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene-pipeline-layout"),
            bind_group_layouts: &[&camera_bgl],
            push_constant_ranges: &[],
        });

        let bar_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            "vs_bars",
            "fs_bars",
            &[Vertex::layout(), InstanceRaw::layout()],
        );
        let ground_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            "vs_ground",
            "fs_ground",
            &[Vertex::layout()],
        );

        // --- Static geometry ---------------------------------------------------
        let (cube_v, cube_i) = mesh::cube();
        let cube_vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-vertices"),
            contents: bytemuck::cast_slice(&cube_v),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-indices"),
            contents: bytemuck::cast_slice(&cube_i),
            usage: wgpu::BufferUsages::INDEX,
        });

        let ground_half_extent = initial_layout.ground_half_extent();
        let (ground_vbuf, ground_ibuf, ground_index_count) =
            build_ground(&device, ground_half_extent);

        // --- Dynamic instance buffer ------------------------------------------
        let instance_capacity = initial_layout_capacity(initial_layout);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bar-instances"),
            size: (instance_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            depth_view,
            camera_buffer,
            camera_bind_group,
            bar_pipeline,
            ground_pipeline,
            cube_vbuf,
            cube_ibuf,
            cube_index_count: cube_i.len() as u32,
            ground_vbuf,
            ground_ibuf,
            ground_index_count,
            ground_half_extent,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
            egui_renderer,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Reconfigure the surface and depth buffer after a window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return; // minimized; skip until we have a real size again
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_view = create_depth_view(&self.device, width, height);
    }

    /// Rebuild the ground footprint if the array layout changed enough to need
    /// a different size. Cheap no-op when the extent is unchanged.
    pub fn sync_layout(&mut self, layout: &BarLayout) {
        let extent = layout.ground_half_extent();
        if (extent - self.ground_half_extent).abs() > f32::EPSILON {
            let (v, i, count) = build_ground(&self.device, extent);
            self.ground_vbuf = v;
            self.ground_ibuf = i;
            self.ground_index_count = count;
            self.ground_half_extent = extent;
        }
    }

    /// Grow the instance buffer if `needed` exceeds current capacity.
    fn ensure_instance_capacity(&mut self, needed: usize) {
        if needed <= self.instance_capacity {
            return;
        }
        let new_capacity = needed.next_power_of_two();
        self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bar-instances"),
            size: (new_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = new_capacity;
    }

    /// Draw one frame: the scene first, then the egui overlay on top.
    pub fn render(
        &mut self,
        camera: &Camera,
        state: &ArrayState,
        layout: &BarLayout,
        egui: EguiFrame<'_>,
    ) -> Result<(), wgpu::SurfaceError> {
        // Upload camera uniform.
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.uniform()]),
        );

        // Build and upload per-bar instances.
        let instances = build_instances(state, layout);
        self.ensure_instance_capacity(instances.len());
        self.instance_count = instances.len() as u32;
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        // Hand egui's textures and geometry to the GPU before the render passes.
        for (id, image_delta) in &egui.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: egui.pixels_per_point,
        };
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            egui.paint_jobs,
            &screen,
        );

        // Scene pass: clear, draw ground, then the instanced bars (depth-tested).
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_clear()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            pass.set_pipeline(&self.ground_pipeline);
            pass.set_vertex_buffer(0, self.ground_vbuf.slice(..));
            pass.set_index_buffer(self.ground_ibuf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..self.ground_index_count, 0, 0..1);

            if self.instance_count > 0 {
                pass.set_pipeline(&self.bar_pipeline);
                pass.set_vertex_buffer(0, self.cube_vbuf.slice(..));
                pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                pass.set_index_buffer(self.cube_ibuf.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..self.cube_index_count, 0, 0..self.instance_count);
            }
        }

        // egui pass: composite the UI over the scene (no depth, load existing).
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                // egui-wgpu requires a 'static render pass (wgpu 22+ lifetimes).
                .forget_lifetime();
            self.egui_renderer
                .render(&mut pass, egui.paint_jobs, &screen);
        }

        for id in &egui.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

/// Pick an initial instance-buffer capacity that comfortably holds the array
/// (rounded up to a power of two) without immediate reallocation.
fn initial_layout_capacity(layout: &BarLayout) -> usize {
    // `span / spacing` recovers the element count; round up generously.
    (layout.span() as usize).max(64).next_power_of_two()
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth-texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn build_ground(device: &wgpu::Device, half_extent: f32) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let (v, i) = mesh::ground(half_extent);
    let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ground-vertices"),
        contents: bytemuck::cast_slice(&v),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ground-indices"),
        contents: bytemuck::cast_slice(&i),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vbuf, ibuf, i.len() as u32)
}

/// Build a render pipeline from shared bind layout and shader, varying only the
/// entry points and vertex buffer layouts. Both pipelines use the same depth
/// state and disable face culling (the boxes are closed and depth handles
/// occlusion, which sidesteps any winding subtleties).
fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    vs_entry: &str,
    fs_entry: &str,
    buffers: &[wgpu::VertexBufferLayout<'_>],
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene-pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs_entry),
            buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}
