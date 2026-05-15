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
            .with_inner_size([850.0, 650.0])
            .with_min_inner_size([700.0, 500.0])
            .with_title("ELEGOO Smart Car V4 Controller"),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "ELEGOO Smart Car V4 Controller",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
