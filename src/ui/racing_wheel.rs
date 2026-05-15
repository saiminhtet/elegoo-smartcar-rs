/// Virtual Racing Wheel widget - mouse-drag steering control
pub struct RacingWheelWidget {
    /// Steering angle: -1.0 (full left) to 1.0 (full right)
    pub steering: f32,
    /// Throttle: 0.0 to 1.0
    pub throttle: f32,
    /// Brake: 0.0 to 1.0
    pub brake: f32,
    /// Whether the wheel is currently being dragged
    pub active: bool,
    /// Wheel radius in pixels
    pub radius: f32,
    /// Minimum motor speed to prevent buzzing
    pub min_motor_speed: u8,
    /// Steering reduction factor for inside wheel
    pub steering_factor: f32,
}

impl Default for RacingWheelWidget {
    fn default() -> Self {
        Self {
            steering: 0.0,
            throttle: 0.0,
            brake: 0.0,
            active: false,
            radius: 80.0,
            min_motor_speed: 40,
            steering_factor: 0.65,
        }
    }
}

impl RacingWheelWidget {
    /// Draw the virtual racing wheel
    pub fn show(&mut self, ui: &mut egui::Ui, label: &str) {
        let (response, painter) = ui.allocate_painter(
            egui::vec2(self.radius * 2.0 + 40.0, self.radius * 2.0 + 80.0),
            egui::Sense::click_and_drag(),
        );

        let center = response.rect.center();
        let wheel_center = egui::pos2(center.x, center.y - 30.0);

        // Draw steering wheel (circle)
        let wheel_color = if self.active {
            egui::Color32::from_rgb(78, 201, 176)
        } else {
            egui::Color32::from_rgb(55, 55, 60)
        };

        painter.circle_stroke(
            wheel_center,
            self.radius,
            egui::Stroke::new(3.0, wheel_color),
        );

        // Draw steering line
        let angle = self.steering * std::f32::consts::FRAC_PI_4; // Max 45° rotation
        let line_end = egui::pos2(
            wheel_center.x + angle.sin() * self.radius * 0.7,
            wheel_center.y - angle.cos() * self.radius * 0.7,
        );
        painter.line_segment(
            [wheel_center, line_end],
            egui::Stroke::new(3.0, egui::Color32::from_rgb(78, 201, 176)),
        );

        // Draw center hub
        painter.circle_filled(wheel_center, 12.0, egui::Color32::from_rgb(40, 40, 45));

        // Draw throttle and brake bars below wheel
        let bar_y = wheel_center.y + self.radius + 20.0;
        let bar_width = 120.0;
        let bar_height = 20.0;
        let bar_x = center.x - bar_width / 2.0;

        // Throttle bar
        let throttle_rect = egui::Rect::from_min_size(
            egui::pos2(bar_x, bar_y),
            egui::vec2(bar_width * self.throttle, bar_height),
        );
        let throttle_bg = egui::Rect::from_min_size(
            egui::pos2(bar_x, bar_y),
            egui::vec2(bar_width, bar_height),
        );
        painter.rect(
            throttle_bg,
            4.0,
            egui::Color32::from_rgb(30, 30, 30),
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::StrokeKind::Inside,
        );
        if self.throttle > 0.0 {
            painter.rect_filled(
                throttle_rect,
                4.0,
                egui::Color32::from_rgb(78, 201, 120),
            );
        }
        painter.text(
            egui::pos2(bar_x + bar_width + 8.0, bar_y + bar_height / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("Gas {}%", (self.throttle * 100.0) as u32),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(170, 170, 170),
        );

        // Brake bar
        let brake_y = bar_y + bar_height + 8.0;
        let brake_rect = egui::Rect::from_min_size(
            egui::pos2(bar_x, brake_y),
            egui::vec2(bar_width * self.brake, bar_height),
        );
        let brake_bg = egui::Rect::from_min_size(
            egui::pos2(bar_x, brake_y),
            egui::vec2(bar_width, bar_height),
        );
        painter.rect(
            brake_bg,
            4.0,
            egui::Color32::from_rgb(30, 30, 30),
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
            egui::StrokeKind::Inside,
        );
        if self.brake > 0.0 {
            painter.rect_filled(
                brake_rect,
                4.0,
                egui::Color32::from_rgb(244, 71, 71),
            );
        }
        painter.text(
            egui::pos2(bar_x + bar_width + 8.0, brake_y + bar_height / 2.0),
            egui::Align2::LEFT_CENTER,
            format!("Brake {}%", (self.brake * 100.0) as u32),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(170, 170, 170),
        );

        // Label
        ui.put(
            egui::Rect::from_min_size(
                egui::pos2(response.rect.min.x, response.rect.max.y - 20.0),
                egui::vec2(self.radius * 2.0 + 40.0, 20.0),
            ),
            egui::Label::new(
                egui::RichText::new(label)
                    .size(12.0)
                    .color(egui::Color32::from_rgb(170, 170, 170)),
            ),
        );

        // Handle drag on wheel
        if response.dragged() {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let dx = pointer_pos.x - wheel_center.x;
                let dy = pointer_pos.y - wheel_center.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > self.radius * 0.3 && distance < self.radius * 1.5 {
                    // Angle-based steering
                    let angle = dy.atan2(dx);
                    let normalized_angle = angle / std::f32::consts::FRAC_PI_4;
                    self.steering = normalized_angle.clamp(-1.0, 1.0);
                    self.active = true;
                }
            }
        } else if response.drag_stopped() {
            self.steering = 0.0;
            self.active = false;
        }

        // Handle vertical drag for throttle/brake
        // (We use the wheel center y as reference - drag up = throttle, drag down = brake)
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let dy = wheel_center.y - pointer_pos.y;
                if dy > 10.0 {
                    self.throttle = (dy / 100.0).clamp(0.0, 1.0);
                    self.brake = 0.0;
                } else if dy < -10.0 {
                    self.brake = ((-dy) / 100.0).clamp(0.0, 1.0);
                    self.throttle = 0.0;
                } else {
                    self.throttle = 0.0;
                    self.brake = 0.0;
                }
            }
        }
    }

    /// Calculate motor speeds using differential steering
    /// Returns (left_speed, right_speed, is_reverse)
    pub fn get_motor_speeds(&self, max_forward_speed: u8, max_reverse_speed: u8) -> (u32, u32, bool) {
        let steering_factor = self.steering_factor;
        let min_speed = self.min_motor_speed as u32;

        if self.throttle > 0.02 {
            // Forward - with differential steering
            let base_speed = (self.throttle * max_forward_speed as f32) as u32;
            let mut left_speed = base_speed;
            let mut right_speed = base_speed;

            if self.steering.abs() > 0.1 {
                if self.steering > 0.0 {
                    // Turning right - reduce right motor
                    let reduced = (base_speed as f32 * (1.0 - self.steering * steering_factor)) as u32;
                    right_speed = if reduced < min_speed { 0 } else { reduced };
                } else {
                    // Turning left - reduce left motor
                    let reduced = (base_speed as f32 * (1.0 + self.steering * steering_factor)) as u32;
                    left_speed = if reduced < min_speed { 0 } else { reduced };
                }
            }

            (left_speed.min(255), right_speed.min(255), false)
        } else if self.brake > 0.02 {
            // Reverse
            let speed = (self.brake * max_reverse_speed as f32) as u32;
            (speed.min(255), speed.min(255), true)
        } else {
            (0, 0, false)
        }
    }
}
