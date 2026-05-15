use crate::sensor::SensorData;
use std::time::{Duration, Instant};

/// Status bar widget showing connection status, timers, battery, distance
pub struct StatusBarWidget {
    connected_since: Option<Instant>,
    pub signal_strength: u8,
    pub app_uptime: Duration,
    session_start: Instant,
}

impl Default for StatusBarWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBarWidget {
    pub fn new() -> Self {
        Self {
            connected_since: None,
            signal_strength: 0,
            app_uptime: Duration::ZERO,
            session_start: Instant::now(),
        }
    }

    pub fn on_connected(&mut self) {
        self.connected_since = Some(Instant::now());
    }

    pub fn on_disconnected(&mut self) {
        self.connected_since = None;
    }

    pub fn set_signal_strength(&mut self, strength: u8) {
        self.signal_strength = strength;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, sensor: &SensorData, is_connected: bool, mode: u32) {
        self.app_uptime = self.session_start.elapsed();

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Connection status
            let (color, text) = if is_connected {
                (egui::Color32::from_rgb(78, 201, 176), "Connected")
            } else {
                (egui::Color32::from_rgb(244, 71, 71), "Disconnected")
            };
            ui.colored_label(color, "●");
            ui.colored_label(color, text);

            // Connection duration
            if let Some(start) = self.connected_since {
                let dur = start.elapsed();
                ui.colored_label(
                    egui::Color32::from_rgb(170, 170, 170),
                    format!("{:02}:{:02}", dur.as_secs() / 60, dur.as_secs() % 60),
                );
            }

            ui.separator();

            // Signal strength
            let signal_color = if self.signal_strength > 60 {
                egui::Color32::from_rgb(78, 201, 176)
            } else if self.signal_strength > 30 {
                egui::Color32::from_rgb(255, 193, 7)
            } else {
                egui::Color32::from_rgb(244, 71, 71)
            };
            ui.colored_label(signal_color, format!("📶 {}%", self.signal_strength));

            ui.separator();

            // Battery
            let battery_str = sensor.battery_voltage_string();
            let battery_color = if sensor.battery_mv > 7000 {
                egui::Color32::from_rgb(78, 201, 176)
            } else if sensor.battery_mv > 6000 {
                egui::Color32::from_rgb(255, 193, 7)
            } else {
                egui::Color32::from_rgb(244, 71, 71)
            };
            ui.colored_label(battery_color, format!("🔋 {}", battery_str));

            ui.separator();

            // Ultrasonic distance
            let dist_str = sensor.distance_string();
            ui.label(format!("📏 Distance: {}", dist_str));

            ui.separator();

            // Mode
            let mode_str = match mode {
                0 => "Manual",
                1 => "Line",
                2 => "Obstacle",
                3 => "Follow",
                _ => "Unknown",
            };
            ui.label(format!("Mode: {}", mode_str));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // App uptime
                let uptime = self.app_uptime;
                ui.colored_label(
                    egui::Color32::from_rgb(170, 170, 170),
                    format!(
                        "Uptime: {:02}:{:02}:{:02}",
                        uptime.as_secs() / 3600,
                        (uptime.as_secs() % 3600) / 60,
                        uptime.as_secs() % 60
                    ),
                );
            });
        });
    }
}
