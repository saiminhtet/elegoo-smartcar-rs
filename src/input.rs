use crate::config::AppConfiguration;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// Keyboard key pressed
    KeyPressed(Key),
    /// Keyboard key released
    KeyReleased(Key),
    /// Gamepad/joystick axis movement
    JoystickMoved(i32, i32),
    /// Gamepad button pressed
    GamepadButtonPressed(u32),
    /// Gamepad button released
    GamepadButtonReleased(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    W,
    A,
    S,
    D,
    Space,
    Up,
    Down,
    Left,
    Right,
    BracketLeft,
    BracketRight,
    Key0,
    Key1,
    Key2,
    Key3,
    J,
    R,
    V,
    Escape,
}

pub struct InputHandler {
    event_tx: mpsc::UnboundedSender<InputEvent>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<InputEvent>>>,
    _config: AppConfiguration,
}

impl InputHandler {
    pub fn new(config: &AppConfiguration) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        #[cfg(feature = "gamepad")]
        {
            let tx = event_tx.clone();
            let config_clone = config.clone();
            std::thread::spawn(move || {
                Self::gamepad_loop(tx, config_clone);
            });
        }

        #[cfg(not(feature = "gamepad"))]
        {
            tracing::info!("Gamepad support disabled. Enable with --features gamepad (requires libudev-dev on Linux)");
        }

        Self {
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            _config: config.clone(),
        }
    }

    pub fn event_receiver(&self) -> Arc<Mutex<mpsc::UnboundedReceiver<InputEvent>>> {
        self.event_rx.clone()
    }

    /// Send a key event from the UI layer
    pub fn send_key_event(&self, event: InputEvent) {
        let _ = self.event_tx.send(event);
    }

    #[cfg(feature = "gamepad")]
    fn gamepad_loop(event_tx: mpsc::UnboundedSender<InputEvent>, config: AppConfiguration) {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!("No gamepad detected: {}", e);
                return;
            }
        };

        let deadzone = config.controls.joystick.deadzone;
        let max_axis = config.controls.joystick.max_axis_value;
        let center = (max_axis / 2) as f32;

        tracing::info!("Gamepad polling started");

        let mut last_x = 0i32;
        let mut last_y = 0i32;
        let mut last_buttons = [false; 32];

        loop {
            while let Some(gilrs::Event { event, .. }) = gilrs.next_event_blocking(None) {
                match event {
                    gilrs::EventType::AxisChanged(axis, value, _) => {
                        let raw = value as f32;
                        let normalized = if raw > center + deadzone as f32 {
                            ((raw - center) / center) * 100.0
                        } else if raw < center - deadzone as f32 {
                            ((raw - center) / center) * 100.0
                        } else {
                            0.0
                        };

                        let (mut nx, mut ny) = (last_x, last_y);

                        match axis {
                            gilrs::Axis::LeftStickX | gilrs::Axis::DPadX => {
                                nx = normalized as i32;
                            }
                            gilrs::Axis::LeftStickY | gilrs::Axis::DPadY => {
                                ny = -(normalized as i32);
                            }
                            _ => {}
                        }

                        let delta_x = (nx - last_x).abs();
                        let delta_y = (ny - last_y).abs();
                        if delta_x >= config.controls.joystick.position_change_delta
                            || delta_y >= config.controls.joystick.position_change_delta
                        {
                            last_x = nx;
                            last_y = ny;
                            let _ = event_tx.try_send(InputEvent::JoystickMoved(nx, ny));
                        }
                    }
                    gilrs::EventType::ButtonChanged(button, val, _) => {
                        let idx = button as u32;
                        let pressed = val > 0.5;
                        if let Some(was_pressed) = last_buttons.get_mut(idx as usize) {
                            if pressed && !*was_pressed {
                                let _ = event_tx.try_send(InputEvent::GamepadButtonPressed(idx));
                            } else if !pressed && *was_pressed {
                                let _ = event_tx.try_send(InputEvent::GamepadButtonReleased(idx));
                            }
                            *was_pressed = pressed;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
