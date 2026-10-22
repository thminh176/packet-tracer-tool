//! Native Windows GUI for Packet Tracer assessment inspection, command generation, and student comparison.
use std::path::PathBuf;
use eframe::egui;

mod ui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let initial_path = if args.len() > 1 && !args[1].starts_with('-') {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Packet Tracer Activity Inspector")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 600.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Packet Tracer Activity Inspector",
        native_options,
        Box::new(move |cc| Ok(Box::new(ui::InspectorApp::new(Some(cc), initial_path)))),
    )
}
