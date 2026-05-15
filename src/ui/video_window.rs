/// Video viewer window - displays MJPEG stream
use std::time::Instant;

pub struct VideoWindowState {
    pub visible: bool,
    pub frame_data: Option<VideoFrame>,
    pub fps: f64,
    pub dropped_frames: u32,
    pub frame_count: u32,
    pub last_fps_update: Instant,
    pub resolution: String,
    pub streaming_status: StreamingStatus,
    pub mode: u32,
    pub car_status: String,
}

pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub enum StreamingStatus {
    Streaming,
    Slow,
    Stalled,
    Disconnected,
}

impl Default for VideoWindowState {
    fn default() -> Self {
        Self {
            visible: false,
            frame_data: None,
            fps: 0.0,
            dropped_frames: 0,
            frame_count: 0,
            last_fps_update: Instant::now(),
            resolution: "---".to_string(),
            streaming_status: StreamingStatus::Disconnected,
            mode: 0,
            car_status: "Ready".to_string(),
        }
    }
}

impl VideoWindowState {
    /// Render the camera feed as the central panel (no floating window).
    /// `stream_active` indicates whether the user has toggled the stream on (V key).
    pub fn render_embedded(&mut self, ctx: &egui::Context, stream_active: bool) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();

            if let Some(frame) = &self.frame_data {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width as usize, frame.height as usize],
                    &frame.data,
                );
                let texture = ui.ctx().load_texture(
                    "video_frame",
                    color_image,
                    egui::TextureOptions::default(),
                );

                // Status overlay across the top of the video area
                ui.horizontal(|ui| {
                    let (color, text) = match &self.streaming_status {
                        StreamingStatus::Streaming => {
                            (egui::Color32::from_rgb(78, 201, 176), "Streaming")
                        }
                        StreamingStatus::Slow => {
                            (egui::Color32::from_rgb(206, 145, 120), "Slow")
                        }
                        StreamingStatus::Stalled => {
                            (egui::Color32::from_rgb(244, 71, 71), "Stalled")
                        }
                        StreamingStatus::Disconnected => {
                            (egui::Color32::GRAY, "Disconnected")
                        }
                    };
                    ui.colored_label(color, "●");
                    ui.colored_label(color, text);
                    ui.separator();
                    ui.label(format!("FPS: {:.1}", self.fps));
                    if self.dropped_frames > 0 {
                        ui.label(format!("(-{})", self.dropped_frames));
                    }
                    ui.separator();
                    ui.label(&self.resolution);
                });
                ui.separator();

                let frame_size = ui.available_size();
                ui.add(
                    egui::Image::new(&texture).fit_to_exact_size(frame_size),
                );
            } else {
                ui.allocate_ui_with_layout(
                    available,
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        if stream_active {
                            ui.label(
                                egui::RichText::new("Connecting to camera stream…")
                                    .size(18.0)
                                    .color(egui::Color32::from_rgb(170, 170, 170)),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Camera feed off — press V to start")
                                    .size(18.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                        }
                    },
                );
            }
        });
    }

    pub fn update_frame(&mut self, data: Vec<u8>, width: u32, height: u32) {
        self.frame_data = Some(VideoFrame {
            data,
            width,
            height,
        });
        self.frame_count += 1;

        // Update FPS every second
        let elapsed = self.last_fps_update.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f64 / elapsed;
            self.frame_count = 0;
            self.dropped_frames = 0;
            self.last_fps_update = Instant::now();

            // Update streaming status based on FPS
            if self.fps > 5.0 {
                self.streaming_status = StreamingStatus::Streaming;
            } else if self.fps > 1.0 {
                self.streaming_status = StreamingStatus::Slow;
            } else {
                self.streaming_status = StreamingStatus::Stalled;
            }

            self.resolution = format!("{}x{}", width, height);
        }
    }
}
