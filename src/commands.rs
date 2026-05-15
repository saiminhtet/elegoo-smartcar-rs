use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};

static SEQUENCE_NUMBER: AtomicU32 = AtomicU32::new(0);

fn next_seq() -> String {
    let n = SEQUENCE_NUMBER.fetch_add(1, Ordering::SeqCst);
    n.to_string()
}

/// Commands that use H (sequence number) field
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct SequencedCommand {
    h: String,
    n: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    d1: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    d2: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    d3: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    t: Option<u32>,
}

/// Commands without H (sequence number) field
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct SimpleCommand {
    n: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    d1: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    d2: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    d3: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    s: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    i1: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    i2: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    i3: Option<u32>,
}

/// Direction constants
pub mod direction {
    pub const LEFT: u32 = 1;
    pub const RIGHT: u32 = 2;
    pub const FORWARD: u32 = 3;
    pub const BACKWARD: u32 = 4;
    pub const STOP: u32 = 5;
}

/// Camera rotation directions
pub mod camera_dir {
    pub const LEFT: u32 = 3;
    pub const RIGHT: u32 = 4;
    pub const CENTER: u32 = 5;
}

/// Mode constants for SwitchMode (N=101)
pub mod mode {
    pub const MANUAL: u32 = 0; // Uses JoystickClear, not SwitchMode
    pub const LINE_DETECTION: u32 = 1;
    pub const OBSTACLE_AVOIDANCE: u32 = 2;
    pub const FOLLOW: u32 = 3;
}

/// MotorControl (N=1): Control individual motor
/// D1=motor(0-2), D2=speed(0-255), D3=direction(1/2)
pub fn motor_control(motor: u32, speed: u32, dir: u32) -> String {
    assert!(motor <= 2, "Motor must be 0-2, got {}", motor);
    assert!(speed <= 255, "Speed must be 0-255, got {}", speed);
    assert!(dir == 1 || dir == 2, "Direction must be 1 or 2, got {}", dir);

    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 1,
        d1: Some(motor),
        d2: Some(speed),
        d3: Some(dir),
        t: None,
    })
    .expect("JSON serialization failed")
}

/// CarControlTime (N=2): Move in direction for a duration
/// D1=direction(1-4), D2=speed(0-255), T=duration_ms
pub fn car_control_time(direction: u32, speed: u32, duration_ms: u32) -> String {
    assert!(direction >= 1 && direction <= 4, "Direction must be 1-4, got {}", direction);
    assert!(speed <= 255, "Speed must be 0-255, got {}", speed);

    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 2,
        d1: Some(direction),
        d2: Some(speed),
        d3: None,
        t: Some(duration_ms),
    })
    .expect("JSON serialization failed")
}

/// CarControl (N=3): Continuous movement command
/// D1=direction(1-4), D2=speed(0-255)
pub fn car_control(direction: u32, speed: u32) -> String {
    assert!(direction >= 1 && direction <= 4, "Direction must be 1-4, got {}", direction);
    assert!(speed <= 255, "Speed must be 0-255, got {}", speed);

    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 3,
        d1: Some(direction),
        d2: Some(speed),
        d3: None,
        t: None,
    })
    .expect("JSON serialization failed")
}

/// MotorControlSpeed (N=4): Set independent wheel speeds
/// D1=left_speed(0-255), D2=right_speed(0-255)
pub fn motor_control_speed(left_speed: u32, right_speed: u32) -> String {
    assert!(left_speed <= 255, "Left speed must be 0-255, got {}", left_speed);
    assert!(right_speed <= 255, "Right speed must be 0-255, got {}", right_speed);

    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 4,
        d1: Some(left_speed),
        d2: Some(right_speed),
        d3: None,
        t: None,
    })
    .expect("JSON serialization failed")
}

/// ServoControl (N=5): Set servo angle
/// D1=servo(1/2), D2=angle(0-180)
pub fn servo_control(servo: u32, angle: u32) -> String {
    assert!(servo == 1 || servo == 2, "Servo must be 1 or 2, got {}", servo);
    assert!(angle <= 180, "Angle must be 0-180, got {}", angle);

    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 5,
        d1: Some(servo),
        d2: Some(angle),
        d3: None,
        t: None,
    })
    .expect("JSON serialization failed")
}

/// UltrasonicStatus (N=21): Request ultrasonic sensor reading
pub fn ultrasonic_status(mode: u32) -> String {
    assert!(mode == 1 || mode == 2, "Mode must be 1 or 2, got {}", mode);

    serde_json::to_string(&SimpleCommand {
        n: 21,
        d1: Some(mode),
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// InfraredStatus (N=22): Request IR line tracking status
/// D1=sensor(0=left, 1=middle, 2=right)
pub fn infrared_status(sensor: u32) -> String {
    assert!(sensor <= 2, "Sensor must be 0-2, got {}", sensor);

    serde_json::to_string(&SimpleCommand {
        n: 22,
        d1: Some(sensor),
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// LeftGround (N=23): Query left ground sensor
pub fn left_ground() -> String {
    serde_json::to_string(&SimpleCommand {
        n: 23,
        d1: None,
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// SwitchMode (N=101): Switch autonomous behavior mode
/// D1=mode(1=line detection, 2=obstacle avoidance, 3=follow)
pub fn switch_mode(mode: u32) -> String {
    assert!(mode >= 1 && mode <= 3, "Mode must be 1-3, got {}", mode);

    serde_json::to_string(&SimpleCommand {
        n: 101,
        d1: Some(mode),
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// JoystickMovement (N=102): Set joystick direction for autonomous modes
pub fn joystick_movement(direction: u32) -> String {
    assert!(direction <= 9, "Direction must be 0-9, got {}", direction);

    serde_json::to_string(&SimpleCommand {
        n: 102,
        d1: Some(direction),
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// CameraRotation (N=106): Rotate camera
/// D1=direction(1-5)
pub fn camera_rotation(direction: u32) -> String {
    assert!(direction >= 1 && direction <= 5, "Direction must be 1-5, got {}", direction);

    serde_json::to_string(&SimpleCommand {
        n: 106,
        d1: Some(direction),
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// CarStop (N=2 synthetic): Stop car using direction=5 special value
/// Uses CarControlTime protocol with D1=5
pub fn car_stop() -> String {
    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 2,
        d1: Some(direction::STOP),
        d2: Some(0),
        d3: None,
        t: Some(0),
    })
    .expect("JSON serialization failed")
}

/// JoystickClear (N=100): Clear joystick/autonomous state
pub fn joystick_clear() -> String {
    serde_json::to_string(&SimpleCommand {
        n: 100,
        d1: None,
        d2: None,
        d3: None,
        s: None,
        i1: None,
        i2: None,
        i3: None,
    })
    .expect("JSON serialization failed")
}

/// ProgramingClear (N=110 + H): Clear programming state
pub fn programing_clear() -> String {
    serde_json::to_string(&SequencedCommand {
        h: next_seq(),
        n: 110,
        d1: None,
        d2: None,
        d3: None,
        t: None,
    })
    .expect("JSON serialization failed")
}

/// Build the 3-command stop sequence:
/// 1. CarControl(last_direction, 0) - set speed to 0
/// 2. CarStop() - send stop command
/// 3. JoystickClear() - clear autonomous state
pub fn stop_sequence(last_direction: u32) -> Vec<String> {
    vec![
        car_control(last_direction, 0),
        car_stop(),
        joystick_clear(),
    ]
}

/// Heartbeat command format
pub fn heartbeat_command() -> &'static str {
    "{Heartbeat}"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn car_control_uses_uppercase_keys() {
        let cmd = car_control(direction::FORWARD, 100);
        assert!(cmd.contains("\"N\":3"), "got: {}", cmd);
        assert!(cmd.contains("\"D1\":3"), "got: {}", cmd);
        assert!(cmd.contains("\"D2\":100"), "got: {}", cmd);
        assert!(cmd.contains("\"H\":"), "got: {}", cmd);
        assert!(!cmd.contains("\"n\":"), "lowercase key found: {}", cmd);
    }

    #[test]
    fn joystick_clear_uses_uppercase_keys() {
        let cmd = joystick_clear();
        assert_eq!(cmd, "{\"N\":100}");
    }
}
