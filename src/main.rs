#![allow(dead_code)]

mod app;
mod commands;
mod config;
mod connection;
mod input;
mod sensor;
mod ui;
mod video;

use app::SmartCarApp;
use config::AppConfiguration;
use tracing_subscriber::EnvFilter;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("=== ELEGOO Smart Car V4 Controller (Rust) ===");

    // Load configuration
    let config = AppConfiguration::load("application.json");
    tracing::info!(
        "Configuration: IP={}:{}, Video={}",
        config.robot.ip_address,
        config.robot.port,
        if config.video.enabled { "on" } else { "off" }
    );

    // Initialize the app
    let mut app = SmartCarApp::default();
    app.config = config.clone();
    app.initialize();

    // Configure native options for eframe
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_title("ELEGOO Smart Car V4 Mission Control"),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "ELEGOO Smart Car V4 Controller",
        native_options,
        Box::new(|cc| {
            // Register Phosphor icon font so directional/arrow glyphs render
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(app))
        }),
    )
}
