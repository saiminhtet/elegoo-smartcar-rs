use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Sensor data from the ELEGOO Smart Car V4
#[derive(Debug, Clone)]
pub struct SensorData {
    /// Ultrasonic distance in cm (-1 if unknown)
    pub ultrasonic_distance: i32,
    /// Timestamp of last ultrasonic reading
    pub ultrasonic_timestamp: Option<DateTime<Utc>>,

    /// IR line tracking - left sensor
    pub left_line_detected: bool,
    /// IR line tracking - middle sensor
    pub middle_line_detected: bool,
    /// IR line tracking - right sensor
    pub right_line_detected: bool,
    /// Timestamp of last line tracking reading
    pub line_tracking_timestamp: Option<DateTime<Utc>>,

    /// Battery voltage in mV (0 if unknown)
    pub battery_mv: u32,

    /// Raw signal strength (0-100)
    pub signal: u8,

    /// Raw sensor values for display
    pub raw_ultrasonic: i32,
    pub raw_ir_left: i32,
    pub raw_ir_middle: i32,
    pub raw_ir_right: i32,
}

impl Default for SensorData {
    fn default() -> Self {
        Self {
            ultrasonic_distance: -1,
            ultrasonic_timestamp: None,
            left_line_detected: false,
            middle_line_detected: false,
            right_line_detected: false,
            line_tracking_timestamp: None,
            battery_mv: 0,
            signal: 0,
            raw_ultrasonic: -1,
            raw_ir_left: -1,
            raw_ir_middle: -1,
            raw_ir_right: -1,
        }
    }
}

impl SensorData {
    /// Check if any sensor data is available (received within 5 seconds)
    pub fn is_sensor_data_available(&self) -> bool {
        let now = Utc::now();
        let five_secs = chrono::Duration::seconds(5);

        if let Some(ts) = self.ultrasonic_timestamp {
            if now.signed_duration_since(ts) < five_secs {
                return true;
            }
        }
        if let Some(ts) = self.line_tracking_timestamp {
            if now.signed_duration_since(ts) < five_secs {
                return true;
            }
        }
        false
    }

    /// Get battery voltage as a formatted string
    pub fn battery_voltage_string(&self) -> String {
        if self.battery_mv == 0 {
            "---".to_string()
        } else {
            format!("{:.2}V", self.battery_mv as f32 / 1000.0)
        }
    }

    /// Get ultrasonic distance as a formatted string
    pub fn distance_string(&self) -> String {
        if self.ultrasonic_distance < 0 {
            "---".to_string()
        } else {
            format!("{} cm", self.ultrasonic_distance)
        }
    }
}

/// JSON response format from car: {"N":21,"D":44} for ultrasonic
/// or {"N":22,"D1":0,"D2":1,"D3":1} for line tracking
#[derive(Debug, Deserialize)]
pub struct CarJsonResponse {
    pub n: Option<u32>,
    pub d: Option<u32>,
    pub d1: Option<u32>,
    pub d2: Option<u32>,
    pub d3: Option<u32>,
    pub s: Option<u32>,
}

/// Parse sensor data from the car's raw response format.
/// Format: "SD:" then JSON array: [ultrasonic_cm, battery_mv, sensor1, sensor2, speed]
pub fn parse_sensor_response(data: &str) -> Option<SensorData> {
    // Check for SD: prefix (newer firmware format)
    if let Some(sd_data) = data.strip_prefix("SD:") {
        if let Ok(values) = serde_json::from_str::<Vec<i32>>(sd_data) {
            let mut sensor = SensorData::default();
            if values.len() > 0 {
                sensor.ultrasonic_distance = values[0];
                sensor.raw_ultrasonic = values[0];
                sensor.ultrasonic_timestamp = Some(Utc::now());
            }
            if values.len() > 1 {
                sensor.battery_mv = values[1] as u32;
            }
            if values.len() > 4 {
                sensor.signal = values[4] as u8;
            }
            // sensor1, sensor2 are at indices 2, 3 but their meaning varies
            return Some(sensor);
        }
    }

    None
}

/// Parse a raw sensor response like {457_44} → ultrasonic distance 44 cm
/// or {458_930} → line tracking value 930
pub fn parse_raw_sensor_value(response: &str) -> Option<(u32, u32)> {
    let trimmed = response.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = trimmed.split('_').collect();
    if parts.len() == 2 {
        let id = parts[0].parse::<u32>().ok()?;
        let value = parts[1].parse::<u32>().ok()?;
        Some((id, value))
    } else {
        None
    }
}
