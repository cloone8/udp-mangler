#![doc = include_str!("../README.md")]

use eframe::egui;
use udp_mangler as _;

fn main() {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_simple_native("UDP Mangler", options, run_app).expect("eframe returned an error");
}

fn run_app(ctx: &egui::Context, frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("UDP Mangler");
    });
    // let mangler = Mangler::new(args.input, args.output, ManglerConfig::default());
}
