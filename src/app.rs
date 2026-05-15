use crate::config::AppConfiguration;
use crate::connection::{ConnectionEvent, ConnectionManager};
use crate::input::{InputHandler, Key};
use crate::ui::main_panel::MainPanel;
use crate::ui::video_window::VideoWindowState;
use crate::video::{VideoEvent, VideoStreamViewer};

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;

pub struct SmartCarApp {
    pub main_panel: MainPanel,
    pub video_window: VideoWindowState,
    pub config: AppConfiguration,
    pub connection: Option<Arc<Mutex<ConnectionManager>>>,
    pub video_stream: Option<Arc<Mutex<VideoStreamViewer>>>,
    pub input_handler: Option<Arc<Mutex<InputHandler>>>,
    pub runtime: Arc<Mutex<Runtime>>,
    pub should_exit: bool,
}

impl Default for SmartCarApp {
    fn default() -> Self {
        let runtime = Arc::new(Mutex::new(
            Runtime::new().expect("Failed to create Tokio runtime"),
        ));
        let config = AppConfiguration::default();
        Self {
            main_panel: MainPanel::new(),
            video_window: VideoWindowState::default(),
            config,
            connection: None,
            video_stream: None,
            input_handler: None,
            runtime,
            should_exit: false,
        }
    }
}

impl eframe::App for SmartCarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_egui_keys(ctx);
        self.drain_connection_events();
        self.drain_video_events();

        self.main_panel.show(ctx);
        if self.main_panel.video_window_open {
            self.video_window.show(ctx);
        }

        ctx.request_repaint();
    }
}

impl SmartCarApp {
    pub fn initialize(&mut self) {
        let config = self.config.clone();
        let rt = Arc::new(Mutex::new(
            Runtime::new().expect("Failed to create Tokio runtime"),
        ));
        self.runtime = rt.clone();

        self.input_handler = Some(Arc::new(Mutex::new(InputHandler::new(&config))));

        let conn = Arc::new(Mutex::new(ConnectionManager::new(
            &config.robot.ip_address,
            config.robot.port,
        )));
        self.connection = Some(conn.clone());
        self.main_panel.connection = Some(conn.clone());

        let video = Arc::new(Mutex::new(VideoStreamViewer::new(
            &config.robot.ip_address,
        )));
        self.video_stream = Some(video.clone());

        // Connect to car
        let rt_guard = tokio::task::block_in_place(|| {
            rt.blocking_lock()
        });
        let connected = rt_guard.block_on(async { conn.lock().await.connect().await });
        if !connected {
            tracing::error!("Failed to connect to Smart Car");
            self.main_panel.status_message = format!(
                "Failed to connect to {}:{}",
                config.robot.ip_address, config.robot.port
            );
        } else {
            self.main_panel.is_connected = true;
            self.main_panel.status_message = "Connected - Press WASD to drive!".to_string();
            self.main_panel.status_bar.on_connected();
        }

        if config.video.enabled {
            rt_guard.block_on(async {
                video.lock().await.start().await;
            });
            self.main_panel.video_window_open = true;
        }
    }

    fn process_egui_keys(&mut self, ctx: &egui::Context) {
        let mut current_keys = Vec::new();
        ctx.input(|i| {
            for key in &i.keys_down {
                if let Some(k) = egui_key_to_internal(*key) {
                    if !current_keys.contains(&k) {
                        current_keys.push(k);
                    }
                }
            }
        });

        if current_keys.contains(&Key::Escape) {
            self.should_exit = true;
            return;
        }

        let previously_pressed = self.main_panel.pressed_keys.clone();
        let new_presses: Vec<Key> = current_keys
            .iter()
            .copied()
            .filter(|k| !previously_pressed.contains(k))
            .collect();
        let releases: Vec<Key> = previously_pressed
            .iter()
            .copied()
            .filter(|k| !current_keys.contains(k))
            .collect();

        let rt = self.runtime.clone();
        tokio::task::block_in_place(|| {
            let rt_guard = rt.blocking_lock();
            for key in new_presses {
                rt_guard.block_on(self.main_panel.on_key_down(key));
            }
            for key in releases {
                rt_guard.block_on(self.main_panel.on_key_up(key));
            }
        });

        self.main_panel.pressed_keys = current_keys;
    }

    fn drain_connection_events(&mut self) {
        let conn = match &self.connection {
            Some(c) => c,
            None => return,
        };

        // For tokio::sync::Mutex wrapped in Arc, we need to block on lock
        let rx = tokio::task::block_in_place(|| {
            let rt_guard = self.runtime.blocking_lock();
            rt_guard.block_on(async {
                let guard = conn.lock().await;
                guard.event_receiver()
            })
        });

        let mut rx_guard = rx.try_lock();
        if let Ok(ref mut rx_guard) = rx_guard {
            while let Ok(event) = rx_guard.try_recv() {
                match event {
                    ConnectionEvent::SensorDataUpdated(data) => {
                        self.main_panel.update_sensor_data(data);
                        self.main_panel.status_bar
                            .set_signal_strength(if self.main_panel.is_connected { 85 } else { 0 });
                    }
                    ConnectionEvent::ConnectionStatusChanged(connected) => {
                        self.main_panel.is_connected = connected;
                        if connected {
                            self.main_panel.status_bar.on_connected();
                            self.main_panel.status_message =
                                "Connected - Press WASD to drive!".to_string();
                        } else {
                            self.main_panel.status_bar.on_disconnected();
                            self.main_panel.status_message =
                                "Disconnected from Smart Car".to_string();
                        }
                    }
                    ConnectionEvent::MessageReceived(msg) => {
                        tracing::debug!("Message: {}", msg);
                    }
                }
            }
        }
    }

    fn drain_video_events(&mut self) {
        let video = match &self.video_stream {
            Some(v) => v,
            None => return,
        };

        let rx = tokio::task::block_in_place(|| {
            let rt_guard = self.runtime.blocking_lock();
            rt_guard.block_on(async {
                let guard = video.lock().await;
                guard.event_receiver()
            })
        });

        let mut rx_guard = rx.try_lock();
        if let Ok(ref mut rx_guard) = rx_guard {
            while let Ok(event) = rx_guard.try_recv() {
                match event {
                    VideoEvent::FrameReceived(data, width, height) => {
                        self.video_window.update_frame(data, width, height);
                    }
                    VideoEvent::FrameDropped => {
                        self.video_window.dropped_frames += 1;
                    }
                    VideoEvent::StreamStatus(connected, _msg) => {
                        if connected {
                            self.video_window.streaming_status =
                                crate::ui::video_window::StreamingStatus::Streaming;
                        } else {
                            self.video_window.streaming_status =
                                crate::ui::video_window::StreamingStatus::Disconnected;
                        }
                    }
                }
            }
        }
    }
}

fn egui_key_to_internal(key: egui::Key) -> Option<Key> {
    match key {
        egui::Key::W => Some(Key::W),
        egui::Key::A => Some(Key::A),
        egui::Key::S => Some(Key::S),
        egui::Key::D => Some(Key::D),
        egui::Key::Space => Some(Key::Space),
        egui::Key::ArrowUp => Some(Key::Up),
        egui::Key::ArrowDown => Some(Key::Down),
        egui::Key::ArrowLeft => Some(Key::Left),
        egui::Key::ArrowRight => Some(Key::Right),
        egui::Key::Escape => Some(Key::Escape),
        egui::Key::V => Some(Key::V),
        egui::Key::J => Some(Key::J),
        egui::Key::R => Some(Key::R),
        egui::Key::Num0 => Some(Key::Key0),
        egui::Key::Num1 => Some(Key::Key1),
        egui::Key::Num2 => Some(Key::Key2),
        egui::Key::Num3 => Some(Key::Key3),
        egui::Key::OpenBracket => Some(Key::BracketLeft),
        egui::Key::CloseBracket => Some(Key::BracketRight),
        _ => None,
    }
}
