#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! Native Windows GUI for Packet Tracer assessment inspection, command generation, and student comparison.
use eframe::egui;
use std::path::PathBuf;

mod ui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let initial_path = if args.len() > 1 && !args[1].starts_with('-') {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let title = pka_lab_inspector::localization::Localizer::new().text("vi", "app.title");
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 600.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        native_options,
        Box::new(move |cc| Ok(Box::new(ui::InspectorApp::new(Some(cc), initial_path)))),
    )
}
