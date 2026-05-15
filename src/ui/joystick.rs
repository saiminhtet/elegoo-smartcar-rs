/// Virtual Joystick widget - on-screen joystick control
pub struct JoystickWidget {
    pub x: f32,
    pub y: f32,
    pub active: bool,
    pub radius: f32,
}

impl Default for JoystickWidget {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            active: false,
            radius: 60.0,
        }
    }
}

impl JoystickWidget {
    /// Draw the virtual joystick at the given position
    pub fn show(&mut self, ui: &mut egui::Ui, label: &str) {
        let (response, painter) = ui.allocate_painter(
            egui::vec2(self.radius * 2.0 + 20.0, self.radius * 2.0 + 40.0),
            egui::Sense::click_and_drag(),
        );

        let center = response.rect.center();
        let joystick_center = egui::pos2(center.x, center.y - 10.0);

        // Draw outer circle (base)
        painter.circle(
            joystick_center,
            self.radius,
            egui::Color32::from_rgb(30, 30, 30),
            egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 80)),
        );

        // Draw inner circle (stick position)
        let stick_x = joystick_center.x + (self.x * self.radius * 0.5);
        let stick_y = joystick_center.y + (self.y * self.radius * 0.5);
        let stick_pos = egui::pos2(stick_x, stick_y);

        painter.circle(
            stick_pos,
            15.0,
            if self.active {
                egui::Color32::from_rgb(78, 201, 176)
            } else {
                egui::Color32::from_rgb(60, 60, 60)
            },
            egui::Stroke::new(1.0, egui::Color32::from_rgb(78, 201, 176)),
        );

        // Label
        ui.put(
            egui::Rect::from_min_size(
                egui::pos2(response.rect.min.x, response.rect.max.y - 25.0),
                egui::vec2(self.radius * 2.0 + 20.0, 20.0),
            ),
            egui::Label::new(
                egui::RichText::new(label)
                    .size(12.0)
                    .color(egui::Color32::from_rgb(170, 170, 170)),
            ),
        );

        // Handle drag interaction
        if response.dragged() {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let dx = pointer_pos.x - joystick_center.x;
                let dy = pointer_pos.y - joystick_center.y;
                let distance = (dx * dx + dy * dy).sqrt();

                let max_distance = self.radius * 0.5;
                if distance > max_distance {
                    let scale = max_distance / distance;
                    self.x = (dx * scale) / max_distance;
                    self.y = (dy * scale) / max_distance;
                } else {
                    self.x = dx / max_distance;
                    self.y = dy / max_distance;
                }

                self.x = self.x.clamp(-1.0, 1.0);
                self.y = self.y.clamp(-1.0, 1.0);
                self.active = true;
            }
        } else if response.drag_stopped() {
            self.x = 0.0;
            self.y = 0.0;
            self.active = false;
        }
    }

    /// Get the current direction (1-4) and speed (0-255)
    pub fn get_car_control(&self, max_speed: u8) -> (u32, u32) {
        if !self.active && self.x.abs() < 0.1 && self.y.abs() < 0.1 {
            return (3, 0); // Stop (forward direction, speed 0)
        }

        let magnitude = (self.x * self.x + self.y * self.y).sqrt().min(1.0);
        let speed = (magnitude * max_speed as f32).round() as u32;

        let abs_x = self.x.abs();
        let abs_y = self.y.abs();

        let direction = if abs_y > abs_x {
            if self.y < 0.0 { 3 } else { 4 } // Forward/Backward
        } else {
            if self.x > 0.0 { 2 } else { 1 } // Right/Left
        };

        (direction, speed.min(255))
    }
}
