//! UI State for when the mangler is initialized

use eframe::egui::{self, DragValue, Label, Slider, Vec2, Widget};
use udp_mangler::{Mangler, ManglerConfig};

use crate::AppState;
use crate::uninitialized::Uninitialized;

/// The UI state for when the mangler is active and running
#[derive(Debug)]
pub(crate) struct Initialized {
    config: ManglerConfig,
    mangler: Mangler,
}

impl Initialized {
    /// Creates a new [Initialized] UI state holder
    pub(crate) fn new(mangler: Mangler, config: ManglerConfig) -> Self {
        Self { config, mangler }
    }

    /// Shows the UI for this state
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<AppState> {
        if let Some(new_config) = mangler_ui(ui, &self.config) {
            self.mangler.update_config(new_config.clone());
            self.config = new_config;
        }

        ui.add_space(30.0);

        if ui.button("Reset").clicked() {
            self.mangler.stop();
            Some(AppState::Uninitialized(Uninitialized::default()))
        } else {
            None
        }
    }
}

fn mangler_ui(ui: &mut egui::Ui, config: &ManglerConfig) -> Option<ManglerConfig> {
    const LABEL_SIZE: Vec2 = Vec2::new(125.0, 32.0);

    fn add_input_field(ui: &mut egui::Ui, label: &str, field: impl Widget) -> egui::Response {
        ui.horizontal(|ui| {
            ui.add_sized(LABEL_SIZE, Label::new(label));
            ui.add(field)
        })
        .inner
    }

    let mut new_config = config.clone();
    let mut any_changed = false;

    any_changed |= add_input_field(
        ui,
        "Buffer size:",
        DragValue::new(&mut new_config.buffer_size),
    )
    .changed();

    any_changed |= add_input_field(
        ui,
        "Maximum payload size",
        DragValue::new(&mut new_config.max_payload_size),
    )
    .changed();

    any_changed |= add_input_field(
        ui,
        "Loss factor",
        Slider::new(&mut new_config.loss_factor, 0.0..=1.0),
    )
    .changed();

    let mut ping_ms = (new_config.ping_secs * 1000.0) as usize;
    any_changed |= add_input_field(ui, "Ping (ms)", DragValue::new(&mut ping_ms)).changed();

    new_config.ping_secs = (ping_ms as f64) / 1000.0;

    let mut jitter_ms = (new_config.jitter_secs * 1000.0) as usize;
    any_changed |= add_input_field(ui, "Jitter (ms)", DragValue::new(&mut jitter_ms)).changed();

    new_config.jitter_secs = (jitter_ms as f64) / 1000.0;

    if any_changed { Some(new_config) } else { None }
}
