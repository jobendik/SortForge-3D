//! Application shell: window, event loop, input, and the per-frame update.
//!
//! This is the only place the three layers meet. Each frame it:
//!   1. advances the [`PlaybackController`] by the elapsed time,
//!   2. builds the egui panel and applies the user's [`UiActions`],
//!   3. asks the [`Renderer`] to draw the current state plus the UI.
//!
//! Input is split cleanly: egui gets first refusal on every event, and the
//! orbit camera only reacts to the mouse when egui does not want it.

use std::sync::Arc;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::algorithms::Algorithm;
use crate::renderer::{BarLayout, Camera, EguiFrame, Renderer};
use crate::simulation::PlaybackController;
use crate::ui::{self, UiState};
use crate::utils::timing::FrameTimer;

/// Default array size when the app launches.
const DEFAULT_ARRAY_SIZE: usize = 48;
/// Default starting algorithm.
const DEFAULT_ALGORITHM: Algorithm = Algorithm::Quick;

/// Build the event loop and run the application to completion.
pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    // Poll continuously so the animation keeps advancing even without input.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Top-level handler. GPU state is created lazily on `resumed`, so this starts
/// as `None` and becomes `Some` once a window and surface exist.
#[derive(Default)]
struct App {
    state: Option<RunningState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return; // already initialized (e.g. a spurious second resume)
        }
        match RunningState::new(event_loop) {
            Ok(state) => self.state = Some(state),
            Err(err) => {
                log::error!("Failed to initialize SortForge 3D: {err:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        state.window_event(event_loop, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            // Drive a steady stream of redraws for smooth playback.
            state.window.request_redraw();
        }
    }
}

/// All live state once the window and GPU are up.
struct RunningState {
    window: Arc<Window>,
    renderer: Renderer,
    controller: PlaybackController,
    camera: Camera,
    layout: BarLayout,
    timer: FrameTimer,

    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    ui_state: UiState,

    // --- mouse orbit tracking ---
    orbiting: bool,
    last_cursor: Option<PhysicalPosition<f64>>,
}

impl RunningState {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("SortForge 3D")
                    .with_inner_size(LogicalSize::new(1280.0, 800.0)),
            )?,
        );

        let layout = BarLayout::for_array(DEFAULT_ARRAY_SIZE);
        let renderer = pollster::block_on(Renderer::new(window.clone(), &layout))?;

        let (w, h) = renderer.size();
        let mut camera = Camera::new(
            w as f32 / h.max(1) as f32,
            layout.span(),
            layout.max_height(),
        );
        camera.frame(layout.span(), layout.max_height());

        let mut controller = PlaybackController::new(DEFAULT_ALGORITHM, DEFAULT_ARRAY_SIZE);
        // Start playing so the visualization is alive the moment the window opens.
        controller.play();

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        Ok(Self {
            window,
            renderer,
            controller,
            camera,
            layout,
            timer: FrameTimer::new(),
            egui_ctx,
            egui_state,
            ui_state: UiState::new(DEFAULT_ARRAY_SIZE),
            orbiting: false,
            last_cursor: None,
        })
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        // egui gets first look at every event; `consumed` tells us to back off.
        let response = self.egui_state.on_window_event(&self.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
                self.camera
                    .set_aspect(size.width as f32, size.height as f32);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !self.egui_ctx.wants_keyboard_input() {
                    self.handle_key(event.physical_key);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.orbiting = state == ElementState::Pressed && !response.consumed;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(prev) = self.last_cursor {
                    if self.orbiting && !self.egui_ctx.wants_pointer_input() {
                        let dx = (position.x - prev.x) as f32;
                        let dy = (position.y - prev.y) as f32;
                        // Invert dy so dragging up tilts the view from higher up.
                        self.camera.orbit(dx, -dy);
                    }
                }
                self.last_cursor = Some(position);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if !self.egui_ctx.wants_pointer_input() {
                    let amount = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32 / 50.0,
                    };
                    self.camera.zoom(amount);
                }
            }

            WindowEvent::RedrawRequested => self.render_frame(event_loop),

            _ => {}
        }
    }

    /// Translate a keyboard shortcut into a controller action.
    fn handle_key(&mut self, key: PhysicalKey) {
        let PhysicalKey::Code(code) = key else {
            return;
        };
        match code {
            KeyCode::Space => self.controller.toggle_play(),
            KeyCode::KeyR => self.regenerate(self.ui_state.size),
            KeyCode::ArrowRight => {
                self.controller.pause();
                self.controller.step();
            }
            KeyCode::Digit1 => self.controller.set_algorithm(Algorithm::Bubble),
            KeyCode::Digit2 => self.controller.set_algorithm(Algorithm::Insertion),
            KeyCode::Digit3 => self.controller.set_algorithm(Algorithm::Selection),
            KeyCode::Digit4 => self.controller.set_algorithm(Algorithm::Quick),
            KeyCode::ArrowUp => {
                let speed = self.controller.speed();
                self.controller.set_speed(speed * 1.5);
            }
            KeyCode::ArrowDown => {
                let speed = self.controller.speed();
                self.controller.set_speed(speed / 1.5);
            }
            _ => {}
        }
    }

    /// Regenerate the array at `size` and re-frame the camera / ground.
    fn regenerate(&mut self, size: usize) {
        self.controller.regenerate(size);
        self.layout = BarLayout::for_array(size);
        self.camera
            .frame(self.layout.span(), self.layout.max_height());
        self.renderer.sync_layout(&self.layout);
        self.ui_state.size = size;
    }

    fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        let dt = self.timer.tick();
        self.controller.update(dt);

        // --- Build the UI and collect the user's intent ---
        let fps = self.timer.fps();
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let ctx = self.egui_ctx.clone();
        let mut actions = ui::UiActions::default();
        let full_output = ctx.run(raw_input, |ctx| {
            actions = ui::controls::draw(ctx, &self.controller, &mut self.ui_state, fps);
        });
        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);
        self.apply_actions(actions);

        // --- Draw the scene + UI overlay ---
        let paint_jobs = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let egui_frame = EguiFrame {
            textures_delta: &full_output.textures_delta,
            paint_jobs: &paint_jobs,
            pixels_per_point: full_output.pixels_per_point,
        };

        match self.renderer.render(
            &self.camera,
            self.controller.state(),
            &self.layout,
            egui_frame,
        ) {
            Ok(()) => {}
            // The surface was lost or is stale: reconfigure with the last size.
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let (w, h) = self.renderer.size();
                self.renderer.resize(w, h);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("GPU out of memory; exiting");
                event_loop.exit();
            }
            Err(err) => log::warn!("Dropped frame: {err:?}"),
        }
    }

    /// Apply UI-requested actions to the simulation after the panel is drawn.
    fn apply_actions(&mut self, actions: ui::UiActions) {
        if let Some(algorithm) = actions.set_algorithm {
            self.controller.set_algorithm(algorithm);
        }
        if let Some(size) = actions.set_size {
            self.regenerate(size);
        }
        if actions.regenerate {
            self.regenerate(self.ui_state.size);
        }
        if let Some(speed) = actions.set_speed {
            self.controller.set_speed(speed);
        }
        if actions.toggle_play {
            self.controller.toggle_play();
        }
        if actions.step {
            self.controller.pause();
            self.controller.step();
        }
        if actions.reset {
            self.controller.reset();
        }
    }
}
