use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfiguration {
    pub robot: RobotSettings,
    pub controls: ControlSettings,
    pub video: VideoSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotSettings {
    pub ip_address: String,
    pub port: u16,
    pub connection_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSettings {
    pub keyboard: KeyboardSettings,
    pub joystick: JoystickSettings,
    pub racing_wheel: RacingWheelSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardSettings {
    pub forward_speed: u8,
    pub backward_speed: u8,
    pub turn_speed: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoystickSettings {
    pub deadzone: u32,
    pub max_axis_value: u32,
    pub max_speed: u8,
    pub command_throttle_ms: u64,
    pub position_change_delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacingWheelSettings {
    pub steering_deadzone: u32,
    pub pedal_deadzone: u32,
    pub max_axis_value: u32,
    pub throttle_max_speed: u8,
    pub brake_max_speed: u8,
    pub brake_threshold: u8,
    pub steering_factor: f32,
    pub command_throttle_ms: u64,
    pub input_change_delta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    pub enabled: bool,
    pub status_update_interval_seconds: u64,
}

impl Default for AppConfiguration {
    fn default() -> Self {
        Self {
            robot: RobotSettings {
                ip_address: "192.168.4.1".to_string(),
                port: 100,
                connection_timeout_ms: 5000,
            },
            controls: ControlSettings {
                keyboard: KeyboardSettings {
                    forward_speed: 100,
                    backward_speed: 100,
                    turn_speed: 100,
                },
                joystick: JoystickSettings {
                    deadzone: 5000,
                    max_axis_value: 65535,
                    max_speed: 200,
                    command_throttle_ms: 100,
                    position_change_delta: 3,
                },
                racing_wheel: RacingWheelSettings {
                    steering_deadzone: 2000,
                    pedal_deadzone: 1000,
                    max_axis_value: 65535,
                    throttle_max_speed: 200,
                    brake_max_speed: 150,
                    brake_threshold: 15,
                    steering_factor: 0.65,
                    command_throttle_ms: 80,
                    input_change_delta: 5,
                },
            },
            video: VideoSettings {
                enabled: false,
                status_update_interval_seconds: 2,
            },
        }
    }
}

impl AppConfiguration {
    pub fn load<P: AsRef<Path>>(file_path: P) -> Self {
        let path = file_path.as_ref();
        match fs::read_to_string(path) {
            Ok(json) => {
                match serde_json::from_str::<AppConfiguration>(&json) {
                    Ok(config) => {
                        tracing::info!("Configuration loaded from {}", path.display());
                        config
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse configuration: {}, using defaults", e);
                        let config = AppConfiguration::default();
                        Self::save_default(path, &config);
                        config
                    }
                }
            }
            Err(_) => {
                tracing::info!("Configuration file not found: {}, using defaults", path.display());
                let config = AppConfiguration::default();
                Self::save_default(path, &config);
                config
            }
        }
    }

    fn save_default<P: AsRef<Path>>(file_path: P, config: &AppConfiguration) {
        let path = file_path.as_ref();
        match serde_json::to_string_pretty(config) {
            Ok(json) => {
                if let Err(e) = fs::write(path, &json) {
                    tracing::warn!("Failed to save default configuration: {}", e);
                } else {
                    tracing::info!("Default configuration saved to {}", path.display());
                }
            }
            Err(e) => {
                tracing::warn!("Failed to serialize default configuration: {}", e);
            }
        }
    }
}
