use crate::commands;
use crate::connection::ConnectionManager;
use crate::input::InputEvent;
use crate::sensor::SensorData;
use crate::ui::settings::SettingsState;
use crate::ui::status_bar::StatusBarWidget;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Direction constants for car control
const DIR_FORWARD: u32 = 3;
const DIR_BACKWARD: u32 = 4;
const DIR_LEFT: u32 = 1;
const DIR_RIGHT: u32 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Keyboard,
    Joystick,
    RacingWheel,
}

pub struct MainPanel {
    pub connection: Option<Arc<Mutex<ConnectionManager>>>,
    pub sensor_data: SensorData,
    pub is_connected: bool,
    pub current_mode: u32,
    pub mode_display: String,
    pub status_message: String,
    pub input_mode: InputMode,
    pub speed: u8,
    pub camera_angle: u32,
    pub last_direction: u32,
    pub settings_open: bool,
    pub video_window_open: bool,
    pub settings: SettingsState,
    pub status_bar: StatusBarWidget,

    // Keyboard tracking (pub for app.rs access)
    pub pressed_keys: Vec<super::super::input::Key>,
    // Joystick state
    pub joystick_x: i32,
    pub joystick_y: i32,
    pub joystick_enabled: bool,
    // Racing wheel state
    pub wheel_enabled: bool,

    // Speed control
    pub speed_adjust: i32, // -100 to +100 relative adjustment

    // Timers
    last_sensor_update: Instant,
    last_status_update: Instant,
}

impl MainPanel {
    pub fn new() -> Self {
        Self {
            connection: None,
            sensor_data: SensorData::default(),
            is_connected: false,
            current_mode: 0,
            mode_display: "0 - Manual".to_string(),
            status_message: "Ready".to_string(),
            input_mode: InputMode::Keyboard,
            speed: 100,
            camera_angle: 135,
            last_direction: DIR_FORWARD,
            settings_open: false,
            video_window_open: false,
            settings: SettingsState::default(),
            status_bar: StatusBarWidget::new(),
            pressed_keys: Vec::new(),
            joystick_x: 0,
            joystick_y: 0,
            joystick_enabled: false,
            wheel_enabled: false,
            speed_adjust: 0,
            last_sensor_update: Instant::now(),
            last_status_update: Instant::now(),
        }
    }

    /// Handle an input event from keyboard, gamepad, etc.
    pub async fn handle_input(&mut self, event: &InputEvent) {
        let conn = match &self.connection {
            Some(c) => c,
            None => {
                tracing::warn!("No connection manager");
                return;
            }
        };

        // Check joystick events
        match event {
            InputEvent::JoystickMoved(x, y) => {
                if self.input_mode != InputMode::Joystick {
                    return;
                }
                self.joystick_x = *x;
                self.joystick_y = *y;

                // Dead zone
                if x.abs() < 5 && y.abs() < 5 {
                    // Centered - send stop sequence
                    let cmds = commands::stop_sequence(self.last_direction);
                    for cmd in cmds {
                        conn.lock().await.send_command(&cmd).await;
                    }
                    return;
                }

                // Calculate direction and speed
                let abs_x = x.abs() as f32;
                let abs_y = y.abs() as f32;
                let magnitude = (abs_x * abs_x + abs_y * abs_y).sqrt().min(100.0);
                let mapped_speed = (((magnitude / 100.0) * self.settings.joystick_max_speed() as f32) as u32).min(255);

                let direction = if abs_y > abs_x {
                    if *y > 0 { DIR_FORWARD } else { DIR_BACKWARD }
                } else {
                    if *x > 0 { DIR_RIGHT } else { DIR_LEFT }
                };

                self.last_direction = direction;
                let cmd = commands::car_control(direction, mapped_speed);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = format!("Joystick: {}%", magnitude as u32);
                return;
            }
            InputEvent::GamepadButtonPressed(idx) => {
                if self.input_mode != InputMode::Joystick {
                    return;
                }
                match idx {
                    1 => {
                        // Button 2 - Toggle video
                        self.video_window_open = !self.video_window_open;
                    }
                    2 => {
                        // Button 3 - Mode 1
                        let guard = conn.lock().await;
                        guard.switch_mode(1).await;
                        self.current_mode = 1;
                        self.mode_display = "1 - Line Detection".to_string();
                    }
                    3 => {
                        // Button 4 - Mode 2
                        let guard = conn.lock().await;
                        guard.switch_mode(2).await;
                        self.current_mode = 2;
                        self.mode_display = "2 - Obstacle Avoidance".to_string();
                    }
                    4 => {
                        // Button 5 - Mode 3
                        let guard = conn.lock().await;
                        guard.switch_mode(3).await;
                        self.current_mode = 3;
                        self.mode_display = "3 - Follow Mode".to_string();
                    }
                    _ => {}
                }
                return;
            }
            _ => {}
        }
    }

    /// Called when keyboard shortcut is triggered (from egui key handling)
    pub async fn on_key_down(&mut self, key: super::super::input::Key) {
        if self.pressed_keys.contains(&key) {
            return; // Key already held
        }
        self.pressed_keys.push(key);

        let conn = match &self.connection {
            Some(c) => c.clone(),
            None => return,
        };

        match key {
            super::super::input::Key::W => {
                self.last_direction = DIR_FORWARD;
                let cmd = commands::car_control(DIR_FORWARD, self.speed as u32);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = "Moving Forward".to_string();
            }
            super::super::input::Key::S => {
                self.last_direction = DIR_BACKWARD;
                let cmd = commands::car_control(DIR_BACKWARD, self.speed as u32);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = "Moving Backward".to_string();
            }
            super::super::input::Key::A => {
                self.last_direction = DIR_LEFT;
                let cmd = commands::car_control(DIR_LEFT, self.speed as u32);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = "Turning Left".to_string();
            }
            super::super::input::Key::D => {
                self.last_direction = DIR_RIGHT;
                let cmd = commands::car_control(DIR_RIGHT, self.speed as u32);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = "Turning Right".to_string();
            }
            super::super::input::Key::Left | super::super::input::Key::BracketLeft => {
                let new_angle = self.camera_angle.saturating_sub(5).max(90);
                self.camera_angle = new_angle;
                // Map angle to camera direction (1-5)
                let dir = if new_angle < 110 { 3 } // Left
                    else if new_angle > 160 { 4 } // Right
                    else { 5 }; // Center
                let cmd = commands::camera_rotation(dir);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = format!("Camera: {}°", new_angle);
            }
            super::super::input::Key::Right | super::super::input::Key::BracketRight => {
                let new_angle = (self.camera_angle + 5).min(180);
                self.camera_angle = new_angle;
                let dir = if new_angle < 110 { 3 }
                    else if new_angle > 160 { 4 }
                    else { 5 };
                let cmd = commands::camera_rotation(dir);
                conn.lock().await.send_command(&cmd).await;
                self.status_message = format!("Camera: {}°", new_angle);
            }
            super::super::input::Key::Up => {
                self.speed = (self.speed as i32 + 10).min(255) as u8;
                self.speed_adjust = self.speed as i32;
                self.status_message = format!("Speed: {}", self.speed);
            }
            super::super::input::Key::Down => {
                self.speed = (self.speed as i32 - 10).max(0) as u8;
                self.speed_adjust = self.speed as i32;
                self.status_message = format!("Speed: {}", self.speed);
            }
            super::super::input::Key::Key0 => {
                conn.lock().await.switch_mode(0).await;
                self.current_mode = 0;
                self.mode_display = "0 - Manual".to_string();
                self.status_message = "Mode 0: Manual".to_string();
            }
            super::super::input::Key::Key1 => {
                conn.lock().await.switch_mode(1).await;
                self.current_mode = 1;
                self.mode_display = "1 - Line Detection".to_string();
                self.status_message = "Mode 1: Line Detection".to_string();
            }
            super::super::input::Key::Key2 => {
                conn.lock().await.switch_mode(2).await;
                self.current_mode = 2;
                self.mode_display = "2 - Obstacle Avoidance".to_string();
                self.status_message = "Mode 2: Obstacle Avoidance".to_string();
            }
            super::super::input::Key::Key3 => {
                conn.lock().await.switch_mode(3).await;
                self.current_mode = 3;
                self.mode_display = "3 - Follow Mode".to_string();
                self.status_message = "Mode 3: Follow".to_string();
            }
            super::super::input::Key::J => {
                // Toggle joystick mode
                self.joystick_enabled = !self.joystick_enabled;
                if self.joystick_enabled {
                    self.input_mode = InputMode::Joystick;
                    self.wheel_enabled = false;
                    self.status_message = "Joystick Enabled".to_string();
                } else {
                    self.input_mode = InputMode::Keyboard;
                    conn.lock().await.send_stop_sequence(self.last_direction).await;
                    self.status_message = "Keyboard Control".to_string();
                }
            }
            super::super::input::Key::R => {
                // Toggle racing wheel mode
                self.wheel_enabled = !self.wheel_enabled;
                if self.wheel_enabled {
                    self.input_mode = InputMode::RacingWheel;
                    self.joystick_enabled = false;
                    self.status_message = "Racing Wheel Enabled".to_string();
                } else {
                    self.input_mode = InputMode::Keyboard;
                    conn.lock().await.send_stop_sequence(self.last_direction).await;
                    self.status_message = "Keyboard Control".to_string();
                }
            }
            super::super::input::Key::V => {
                self.video_window_open = !self.video_window_open;
                self.status_message = if self.video_window_open {
                    "Video: On".to_string()
                } else {
                    "Video: Off".to_string()
                };
            }
            super::super::input::Key::Space => {
                // Emergency stop
                conn.lock().await.send_stop_sequence(self.last_direction).await;
                self.status_message = "EMERGENCY STOP".to_string();
            }
            super::super::input::Key::Escape => {
                // Will be handled by the app
            }
        }
    }

    /// Called when keyboard key is released
    pub async fn on_key_up(&mut self, key: super::super::input::Key) {
        self.pressed_keys.retain(|k| *k != key);

        // For movement keys, send stop sequence
        match key {
            super::super::input::Key::W
            | super::super::input::Key::A
            | super::super::input::Key::S
            | super::super::input::Key::D => {
                if let Some(conn) = &self.connection {
                    conn.lock().await.send_stop_sequence(self.last_direction).await;
                }
                self.status_message = "Ready".to_string();
            }
            _ => {}
        }
    }

    /// Update sensor data from connection
    pub fn update_sensor_data(&mut self, data: SensorData) {
        self.sensor_data = data;
        self.last_sensor_update = Instant::now();
    }

    /// Draw the main control panel using egui
    pub fn show(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("title_panel")
            .min_height(36.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.heading(
                        egui::RichText::new("ELEGOO Smart Car V4 Controller")
                            .color(egui::Color32::from_rgb(78, 201, 176)),
                    );
                });
            });

        // Quick controls bar
        egui::TopBottomPanel::top("quick_controls")
            .min_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);

                    // Connection indicator
                    let (color, text) = if self.is_connected {
                        (egui::Color32::from_rgb(78, 201, 176), "Connected")
                    } else {
                        (egui::Color32::from_rgb(244, 71, 71), "Disconnected")
                    };
                    ui.colored_label(color, "●");
                    ui.colored_label(color, text);

                    ui.separator();

                    // Mode display
                    let mode_color = if self.current_mode > 0 {
                        egui::Color32::from_rgb(255, 193, 7)
                    } else {
                        egui::Color32::from_rgb(78, 201, 176)
                    };
                    ui.colored_label(mode_color, &self.mode_display);

                    ui.separator();

                    // Speed display
                    ui.label(format!("Speed: {}", self.speed));

                    ui.separator();

                    // Video status
                    if self.video_window_open {
                        ui.colored_label(egui::Color32::from_rgb(78, 201, 176), "Video: On");
                    } else {
                        ui.colored_label(egui::Color32::GRAY, "Video: Off");
                    }

                    ui.separator();

                    // Input mode
                    let input_text = match self.input_mode {
                        InputMode::Keyboard => "Keyboard",
                        InputMode::Joystick => "Joystick",
                        InputMode::RacingWheel => "Wheel",
                    };
                    ui.label(input_text);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚙ Settings").clicked() {
                            self.settings_open = !self.settings_open;
                        }
                        if ui.button("📷 Video").clicked() {
                            self.video_window_open = !self.video_window_open;
                        }
                    });
                });
            });

        egui::SidePanel::left("control_panel")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Vehicle Controls")
                        .size(16.0)
                        .strong(),
                );
                ui.separator();
                ui.add_space(4.0);

                // Directional controls
                let button_size = egui::vec2(60.0, 60.0);

                ui.horizontal(|ui| {
                    ui.add_space(60.0);
                    if self.is_connected {
                        let forward_btn = egui::Button::new(
                            egui::RichText::new("▲\nW").size(14.0),
                        )
                        .min_size(button_size);
                        if ui.add(forward_btn).clicked() {
                            let _cmd = commands::car_control(DIR_FORWARD, self.speed as u32);
                            // Need async handling - send via connection
                        }
                    } else {
                        ui.add_sized(button_size, egui::Label::new("▲\nW"));
                    }
                });

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if self.is_connected {
                        let left_btn = egui::Button::new(
                            egui::RichText::new("◄\nA").size(14.0),
                        )
                        .min_size(button_size);
                        if ui.add(left_btn).clicked() {
                            let _cmd = commands::car_control(DIR_LEFT, self.speed as u32);
                        }
                    } else {
                        ui.add_sized(button_size, egui::Label::new("◄\nA"));
                    }

                    if self.is_connected {
                        let stop_btn = egui::Button::new(
                            egui::RichText::new("■\nSpace")
                                .size(14.0)
                                .color(egui::Color32::RED),
                        )
                        .min_size(button_size);
                        if ui.add(stop_btn).clicked() {
                            // Stop
                        }
                    } else {
                        ui.add_sized(button_size, egui::Label::new("■\nSpace"));
                    }

                    if self.is_connected {
                        let right_btn = egui::Button::new(
                            egui::RichText::new("►\nD").size(14.0),
                        )
                        .min_size(button_size);
                        if ui.add(right_btn).clicked() {
                            let _cmd = commands::car_control(DIR_RIGHT, self.speed as u32);
                        }
                    } else {
                        ui.add_sized(button_size, egui::Label::new("►\nD"));
                    }
                });

                ui.horizontal(|ui| {
                    ui.add_space(60.0);
                    if self.is_connected {
                        let backward_btn = egui::Button::new(
                            egui::RichText::new("▼\nS").size(14.0),
                        )
                        .min_size(button_size);
                        if ui.add(backward_btn).clicked() {
                            let _cmd = commands::car_control(DIR_BACKWARD, self.speed as u32);
                        }
                    } else {
                        ui.add_sized(button_size, egui::Label::new("▼\nS"));
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Speed control slider
                ui.label(
                    egui::RichText::new(format!("Motor Speed: {}", self.speed))
                        .size(14.0)
                        .strong(),
                );
                if ui
                    .add(egui::Slider::new(&mut self.speed, 0..=255).text("Speed"))
                    .changed()
                {
                    let _cmd = commands::motor_control_speed(self.speed as u32, self.speed as u32);
                    // Would send via connection
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Camera rotation
                ui.label(
                    egui::RichText::new(format!("Camera: {}°", self.camera_angle))
                        .size(14.0)
                        .strong(),
                );
                let old_angle = self.camera_angle;
                ui.add(
                    egui::Slider::new(&mut self.camera_angle, 90..=180)
                        .text("Angle"),
                );
                if old_angle != self.camera_angle {
                    let dir = if self.camera_angle < 110 {
                        3
                    } else if self.camera_angle > 160 {
                        4
                    } else {
                        5
                    };
                    let _cmd = commands::camera_rotation(dir);
                    // Would send via connection
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Camera buttons
                ui.horizontal(|ui| {
                    if ui.button("◄ Camera Left").clicked() {
                        let _cmd = commands::camera_rotation(3);
                    }
                    if ui.button("Camera Right ►").clicked() {
                        let _cmd = commands::camera_rotation(4);
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Mode selection
                ui.label(
                    egui::RichText::new("Operating Modes").size(14.0).strong(),
                );

                let modes = [
                    ("0 Manual", egui::Color32::from_rgb(78, 201, 176)),
                    ("1 Line", egui::Color32::from_rgb(255, 193, 7)),
                    ("2 Obstacle", egui::Color32::from_rgb(255, 193, 7)),
                    ("3 Follow", egui::Color32::from_rgb(255, 193, 7)),
                ];

                for (i, (name, color)) in modes.iter().enumerate() {
                    let selected = self.current_mode == i as u32;
                    let btn = egui::Button::new(
                        egui::RichText::new(*name).color(if selected {
                            egui::Color32::WHITE
                        } else {
                            *color
                        }),
                    )
                    .min_size(egui::vec2(240.0, 30.0))
                    .fill(if selected {
                        egui::Color32::from_rgb(60, 60, 60)
                    } else {
                        egui::Color32::from_rgb(37, 37, 38)
                    });

                    if ui.add(btn).clicked() {
                        // Switch mode via connection
                    }
                }
            });

        // Center panel - status and info
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("Status").size(16.0).strong(),
            );
            ui.separator();

            // Status message
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(egui::Color32::from_rgb(78, 201, 176), &self.status_message);
            });

            ui.add_space(8.0);

            // Sensor data display
            ui.label(
                egui::RichText::new("Sensor Data").size(14.0).strong(),
            );

            let sensor = &self.sensor_data;

            ui.horizontal(|ui| {
                ui.label("Distance:");
                ui.colored_label(
                    egui::Color32::from_rgb(220, 220, 202),
                    sensor.distance_string(),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Battery:");
                ui.colored_label(
                    egui::Color32::from_rgb(220, 220, 202),
                    sensor.battery_voltage_string(),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Line Tracking:");
                let l = if sensor.left_line_detected { "■" } else { "□" };
                let m = if sensor.middle_line_detected { "■" } else { "□" };
                let r = if sensor.right_line_detected { "■" } else { "□" };
                ui.colored_label(
                    egui::Color32::from_rgb(220, 220, 202),
                    format!("L:{} M:{} R:{}", l, m, r),
                );
            });

            ui.add_space(16.0);

            // Virtual Joystick area
            ui.label(
                egui::RichText::new("Virtual Controls").size(14.0).strong(),
            );
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Joystick (J)").clicked() {
                    // Toggle joystick
                }
                if ui.button("Wheel (R)").clicked() {
                    // Toggle wheel
                }
            });

            ui.add_space(8.0);

            // Keyboard guide
            ui.label(
                egui::RichText::new("Keyboard Shortcuts").size(14.0).strong(),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(
                    "WASD - Drive\n\
                     Space - Emergency Stop\n\
                     Up/Down - Speed ±10\n\
                     [ / ] - Camera Left/Right\n\
                     0-3 - Switch Mode\n\
                     J - Joystick Toggle\n\
                     R - Wheel Toggle\n\
                     V - Video Toggle\n\
                     Esc - Exit",
                )
                .size(12.0)
                .color(egui::Color32::from_rgb(170, 170, 170)),
            );

            ui.add_space(8.0);

            // Status bar at bottom
            self.status_bar.show(ui, &self.sensor_data, self.is_connected, self.current_mode);
        });

        // Settings window
        if self.settings_open {
            super::settings::show_settings(ctx, &mut self.settings, &mut self.settings_open);
        }

        // Video window
        if self.video_window_open {
            // Shown by app.rs
        }
    }
}
