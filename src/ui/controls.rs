//! The egui side panel: algorithm choice, playback controls, and live metrics.

use egui::Color32;

use crate::algorithms::Algorithm;
use crate::simulation::{PlaybackController, MAX_SPEED, MIN_SPEED};
use crate::utils::color::palette;

/// Minimum and maximum array sizes offered by the size slider.
pub const MIN_ARRAY_SIZE: usize = 4;
pub const MAX_ARRAY_SIZE: usize = 200;

/// Persistent UI widget state that does not belong to the simulation.
pub struct UiState {
    /// Pending array size for the slider / "new array" button.
    pub size: usize,
}

impl UiState {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

/// User intents produced by one UI frame, applied by the application layer.
///
/// Using an explicit action struct (instead of mutating the controller inside
/// the UI) keeps the simulation borrow immutable while the panel is drawn.
#[derive(Default)]
pub struct UiActions {
    pub toggle_play: bool,
    pub step: bool,
    pub reset: bool,
    pub regenerate: bool,
    pub set_algorithm: Option<Algorithm>,
    /// New array size chosen via the slider (implies regenerate).
    pub set_size: Option<usize>,
    pub set_speed: Option<f32>,
}

/// Draw the control panel and return the user's requested actions.
pub fn draw(
    ctx: &egui::Context,
    controller: &PlaybackController,
    ui_state: &mut UiState,
    fps: f32,
) -> UiActions {
    let mut actions = UiActions::default();

    egui::SidePanel::left("controls")
        .resizable(false)
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading("SortForge 3D");
            ui.label(
                egui::RichText::new("real-time 3D sorting visualizer")
                    .small()
                    .weak(),
            );
            ui.separator();

            algorithm_section(ui, controller, &mut actions);
            ui.separator();
            array_section(ui, ui_state, &mut actions);
            ui.separator();
            playback_section(ui, controller, &mut actions);
            ui.separator();
            metrics_section(ui, controller, fps);
            ui.separator();
            legend_section(ui);
            ui.separator();
            shortcuts_section(ui);
        });

    actions
}

fn algorithm_section(ui: &mut egui::Ui, controller: &PlaybackController, actions: &mut UiActions) {
    ui.label(egui::RichText::new("Algorithm").strong());
    let current = controller.algorithm();
    let mut selected = current;
    egui::ComboBox::from_id_salt("algorithm-combo")
        .selected_text(current.name())
        .width(220.0)
        .show_ui(ui, |ui| {
            for algorithm in Algorithm::ALL {
                ui.selectable_value(&mut selected, algorithm, algorithm.name());
            }
        });
    if selected != current {
        actions.set_algorithm = Some(selected);
    }
    ui.label(
        egui::RichText::new(format!("Complexity: {}", current.complexity()))
            .small()
            .weak(),
    );
}

fn array_section(ui: &mut egui::Ui, ui_state: &mut UiState, actions: &mut UiActions) {
    ui.label(egui::RichText::new("Array").strong());
    let mut size = ui_state.size;
    let response = ui.add(
        egui::Slider::new(&mut size, MIN_ARRAY_SIZE..=MAX_ARRAY_SIZE)
            .text("size")
            .clamping(egui::SliderClamping::Always),
    );
    if response.changed() {
        ui_state.size = size;
    }
    // Regenerate only once the interaction settles - on drag release, or on a
    // discrete (keyboard / click) change - so dragging does not thrash the
    // timeline on every pixel.
    if response.drag_stopped() || (response.changed() && !response.dragged()) {
        actions.set_size = Some(ui_state.size);
    }
    if ui
        .add(egui::Button::new("🎲  New Random Array"))
        .on_hover_text("Shuffle a fresh array (R)")
        .clicked()
    {
        actions.regenerate = true;
    }
}

fn playback_section(ui: &mut egui::Ui, controller: &PlaybackController, actions: &mut UiActions) {
    ui.label(egui::RichText::new("Playback").strong());
    ui.horizontal(|ui| {
        let play_label = if controller.is_playing() {
            "⏸  Pause"
        } else if controller.is_finished() {
            "↻  Replay"
        } else {
            "▶  Play"
        };
        if ui.button(play_label).clicked() {
            actions.toggle_play = true;
        }
        if ui
            .add_enabled(!controller.is_finished(), egui::Button::new("⏭  Step"))
            .clicked()
        {
            actions.step = true;
        }
        if ui.button("⟲  Reset").clicked() {
            actions.reset = true;
        }
    });

    let mut speed = controller.speed();
    if ui
        .add(
            egui::Slider::new(&mut speed, MIN_SPEED..=MAX_SPEED)
                .text("events / sec")
                .logarithmic(true),
        )
        .changed()
    {
        actions.set_speed = Some(speed);
    }
}

fn metrics_section(ui: &mut egui::Ui, controller: &PlaybackController, fps: f32) {
    ui.label(egui::RichText::new("Metrics").strong());
    let metrics = controller.metrics();

    egui::Grid::new("metrics-grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            metric_row(ui, "Comparisons", &metrics.comparisons.to_string());
            metric_row(ui, "Swaps", &metrics.swaps.to_string());
            metric_row(ui, "Writes", &metrics.writes.to_string());
            metric_row(
                ui,
                "Step",
                &format!(
                    "{} / {}",
                    controller.current_step(),
                    controller.total_steps()
                ),
            );
            metric_row(ui, "Array size", &controller.state().len().to_string());
            metric_row(ui, "FPS", &format!("{fps:.0}"));
        });

    ui.add_space(4.0);
    ui.add(
        egui::ProgressBar::new(controller.progress())
            .show_percentage()
            .desired_height(10.0),
    );
}

fn metric_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).weak());
    ui.label(egui::RichText::new(value).monospace());
    ui.end_row();
}

fn legend_section(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Legend").strong());
    legend_row(ui, palette::NORMAL.into(), "Normal");
    legend_row(ui, palette::COMPARING.into(), "Comparing");
    legend_row(ui, palette::SWAPPING.into(), "Swapping / writing");
    legend_row(ui, palette::PIVOT.into(), "Pivot / current min");
    legend_row(ui, palette::SORTED.into(), "Sorted (final)");
}

fn legend_row(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 3.0, color);
        ui.label(label);
    });
}

fn shortcuts_section(ui: &mut egui::Ui) {
    ui.collapsing("Keyboard shortcuts", |ui| {
        egui::Grid::new("shortcuts-grid")
            .num_columns(2)
            .spacing([12.0, 2.0])
            .show(ui, |ui| {
                shortcut_row(ui, "Space", "Play / Pause");
                shortcut_row(ui, "→", "Step one event");
                shortcut_row(ui, "R", "New random array");
                shortcut_row(ui, "↑ / ↓", "Speed up / down");
                shortcut_row(ui, "1-4", "Select algorithm");
                shortcut_row(ui, "Drag", "Orbit camera");
                shortcut_row(ui, "Wheel", "Zoom");
            });
    });
}

fn shortcut_row(ui: &mut egui::Ui, key: &str, action: &str) {
    ui.label(egui::RichText::new(key).monospace());
    ui.label(egui::RichText::new(action).weak());
    ui.end_row();
}
