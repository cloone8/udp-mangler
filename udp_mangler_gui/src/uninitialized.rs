//! UI State for when the mangler is uninitialized

use std::net::ToSocketAddrs;

use eframe::egui::{self, Color32, Label, TextEdit, Vec2};
use udp_mangler::{Mangler, ManglerConfig};

use crate::AppState;
use crate::initialized::Initialized;

/// Holds the UI state for when the mangler is not yet initialized
#[derive(Debug, Default)]
pub(crate) struct Uninitialized {
    listen_addr_string: String,
    forward_addr_string: String,
    error_string: Option<String>,
}

impl Uninitialized {
    /// Shows the UI for this state
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<AppState> {
        const LABEL_SIZE: Vec2 = Vec2::new(125.0, 32.0);

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.add_sized(LABEL_SIZE, Label::new("Listen address:"));
                ui.add(TextEdit::singleline(&mut self.listen_addr_string));
            });
        });

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.add_sized(LABEL_SIZE, Label::new("Forward address:"));
                ui.add(TextEdit::singleline(&mut self.forward_addr_string));
            });
        });

        ui.add_space(16.0);
        if ui.button("Start").clicked() {
            match try_start_mangler(&self.listen_addr_string, &self.forward_addr_string) {
                Ok((mangler, config)) => {
                    return Some(AppState::Initialized(Initialized::new(mangler, config)));
                }
                Err(e) => {
                    self.error_string = Some(e.to_string());
                }
            };
        }

        if let Some(err) = &self.error_string {
            ui.add_space(16.0);
            ui.scope(|ui| {
                ui.style_mut().visuals.override_text_color = Some(Color32::RED);
                ui.add(Label::new(err.as_str()))
            });
        }

        None
    }
}

#[allow(clippy::result_large_err, reason = "Not that large")]
fn try_start_mangler(
    listen_addr: &str,
    forward_addr: &str,
) -> Result<(Mangler, ManglerConfig), String> {
    let listen_addr = listen_addr
        .to_socket_addrs()
        .map_err(|e| format!("Error while resolving listen address: {e}"))?
        .next()
        .unwrap();

    let forward_addr = forward_addr
        .to_socket_addrs()
        .map_err(|e| format!("Error while resolving forward address: {e}"))?
        .next()
        .unwrap();

    let config = ManglerConfig::default();
    let mangler = Mangler::new(listen_addr, forward_addr, config.clone())
        .map_err(|e| format!("Error starting mangler with provided addresses: {e}"))?;

    Ok((mangler, config))
}
