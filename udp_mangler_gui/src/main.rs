#![doc = include_str!("../README.md")]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use initialized::Initialized;
use uninitialized::Uninitialized;

mod initialized;
mod uninitialized;

fn main() {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "UDP Mangler",
        options,
        Box::new(|cc| Ok(Box::new(AppState::new(cc)))),
    )
    .expect("eframe returned an error");
}

/// The main GUI application and its state
#[derive(Debug)]
enum AppState {
    /// Mangler not yet initialized
    Uninitialized(Uninitialized),

    /// Mangler initialized
    Initialized(Initialized),
}

impl AppState {
    /// Creates a new [AppState]
    fn new(_cc: &eframe::CreationContext) -> Self {
        Self::Uninitialized(Uninitialized::default())
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let _ = frame;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("UDP Mangler");
            });

            ui.separator();
            ui.add_space(32.0);

            let new_state = match self {
                Self::Uninitialized(uninitialized) => uninitialized.show(ui),
                Self::Initialized(initialized) => initialized.show(ui),
            };

            if let Some(new_state) = new_state {
                *self = new_state;
            }
        });
    }
}
