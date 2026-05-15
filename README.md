# ELEGOO Smart Car V4 Controller (Rust Port)

A cross-platform desktop application for controlling the **ELEGOO Smart Car V4** over WiFi. Built with Rust, egui, and Tokio.

![ELEGOO Smart Car V4](https://img.shields.io/badge/Hardware-ELEGOO%20SmartCar%20V4-brightgreen)
[![Rust](https://img.shields.io/badge/Rust-1.95+-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## Features

### Core Functionality
- **TCP Connection** to car at `192.168.4.1:100` with mandatory heartbeat echo protocol
- **Vehicle Control** via keyboard (WASD), on-screen buttons, or gamepad
- **Motor Speed Control** slider (0-255) with keyboard Up/Down adjustment
- **Camera Rotation** slider (90°-180°) and keyboard Left/Right brackets
- **Live Video Stream** from ESP32 camera (MJPEG on port 81)
- **Sensor Display** - ultrasonic distance, battery voltage, IR line tracking
- **Status Bar** - connection status, signal strength, battery, distance, uptime

### Operating Modes
- **Mode 0 - Manual**: Drive with WASD or connect a gamepad
- **Mode 1 - Line Detection**: Car follows a line using IR sensors
- **Mode 2 - Obstacle Avoidance**: Car navigates around obstacles using ultrasonic sensor
- **Mode 3 - Follow Me**: Car follows objects using ultrasonic sensor

### Input Methods
- **Keyboard**: Full WASD control with shortcuts
- **Gamepad**: Optional gamepad/joystick support (requires `gamepad` feature)
- **Virtual Controls**: On-screen joystick and racing wheel widgets

### UI Features
- Dark theme mission-control layout: **controls left, camera feed center, sensors/modes right, status bar bottom** — no floating windows
- Phosphor icon font for control buttons (arrows, stop, gear, camera) so glyphs render consistently across platforms
- Real-time connection and mode indicators
- Live sensor data display
- Embedded video stream with FPS counter and streaming-status indicator
- Configurable settings (IP, theme, language)

## Screenshots

<!-- TODO: Add screenshots -->
```
Main Panel: [screenshot/main_panel.png]
Video Stream: [screenshot/video_stream.png]
Settings: [screenshot/settings.png]
```

## Hardware Requirements

- **ELEGOO Smart Car V4** with ESP32-WROVER camera module
- The car must be powered on and acting as a WiFi access point
- Your computer must be connected to the car's WiFi network

### Default Connection
- **IP:** `192.168.4.1`
- **Control Port:** `100` (TCP)
- **Video Port:** `81` (HTTP MJPEG stream)

## Software Requirements

- **Rust** 1.85 or later (install via [rustup](https://rustup.rs/))
- System dependencies:
  - **Linux:** `libudev-dev` (optional, for gamepad support)
  - **macOS:** No additional dependencies
  - **Windows:** No additional dependencies

## Building

```bash
# Clone the repository
git clone <your-repo-url>
cd elegoo-smartcar-rs

# Build (release)
cargo build --release

# Build with gamepad support (requires libudev-dev on Linux)
cargo build --release --features gamepad
```

## Running

```bash
# Connect to car's WiFi first, then:
cargo run --release
```

On first run, a default `application.json` configuration file is created.

## Controls Reference

### Keyboard Controls

| Key | Action |
|-----|--------|
| **W** | Move Forward |
| **A** | Turn Left |
| **S** | Move Backward |
| **D** | Turn Right |
| **Space** | Emergency Stop |
| **↑** / **↓** | Increase / Decrease Speed (±10) |
| **[** / **]** | Camera Left / Right |
| **0** | Mode 0: Manual |
| **1** | Mode 1: Line Detection |
| **2** | Mode 2: Obstacle Avoidance |
| **3** | Mode 3: Follow Me |
| **J** | Toggle Joystick/Gamepad |
| **R** | Toggle Racing Wheel |
| **V** | Toggle Video Stream |
| **Esc** | Exit Application |

### Gamepad Controls (when enabled)

| Button | Action |
|--------|--------|
| Left Stick | Drive (direction + speed) |
| Button 2 (1-indexed: 2) | Toggle Video |
| Button 3 | Mode 1: Line Detection |
| Button 4 | Mode 2: Obstacle Avoidance |
| Button 5 | Mode 3: Follow Me |

## Command Protocol

All communication uses JSON over TCP port 100. **Keys are case-sensitive and uppercase** — the car silently drops anything it doesn't recognize. Examples (wire format):

- **Heartbeat**: `{Heartbeat}` — sent by the car; exact echo back is mandatory
- **CarControl** (N=3): `{"H":"1","N":3,"D1":3,"D2":100}` — forward at speed 100
- **CameraRotation** (N=106): `{"N":106,"D1":3}` — camera left
- **SwitchMode** (N=101): `{"N":101,"D1":1}` — line detection mode
- **CarStop**: `{"H":"2","N":2,"D1":5,"D2":0,"T":0}` — synthetic stop
- **JoystickClear** (N=100): `{"N":100}` — clear autonomous state

`H` is a per-message sequence number (string) required on `N=1/2/3/4/5/110`.

The **stop sequence** always sends 3 commands: `CarControl(dir, 0)` → `CarStop()` → `JoystickClear()`.

## Configuration

Edit `application.json`:

```json
{
  "robot": {
    "ip_address": "192.168.4.1",
    "port": 100,
    "connection_timeout_ms": 5000
  },
  "controls": {
    "keyboard": {
      "forward_speed": 100,
      "backward_speed": 100,
      "turn_speed": 100
    },
    "joystick": {
      "deadzone": 5000,
      "max_speed": 200
    },
    "racing_wheel": {
      "throttle_max_speed": 200,
      "steering_factor": 0.65
    }
  },
  "video": {
    "enabled": false,
    "status_update_interval_seconds": 2
  }
}
```

## Project Structure

```
elegoo-smartcar-rs/
├── Cargo.toml
├── application.json      # Auto-generated config
├── README.md
├── src/
│   ├── main.rs           # Entry point
│   ├── app.rs            # Application state, event loop
│   ├── commands.rs       # JSON command builders
│   ├── config.rs         # Configuration loading
│   ├── connection.rs     # TCP client, heartbeat, sensor polling
│   ├── input.rs          # Keyboard + optional gamepad input
│   ├── sensor.rs         # Sensor data parsing
│   ├── video.rs          # MJPEG video stream client
│   └── ui/
│       ├── mod.rs
│       ├── main_panel.rs    # Main control UI
│       ├── video_window.rs  # Video stream viewer
│       ├── joystick.rs      # Virtual joystick widget
│       ├── racing_wheel.rs  # Virtual racing wheel widget
│       ├── settings.rs      # Settings dialog
│       └── status_bar.rs    # Status bar widget
```

## Troubleshooting

### Connection Issues
- **Car not responding**: Ensure the car is powered on and you're connected to its WiFi
- **Heartbeat timeout**: The car disconnects after ~4 missed heartbeat echoes (auto-handled)
- **Connection refused**: Verify IP address and port in `application.json`
- **Connects but WASD does nothing**: Test TCP reachability with `nc -z -v -w 3 <car-ip> 100`. Run with `RUST_LOG=info` and confirm you see `→ Command sent: {"H":"...","N":3,...}` on each keypress — uppercase JSON keys are required by the firmware.

### Video Stream Issues
- **No video**: Ensure car has camera module; video starts on port 81
- **Low FPS**: WiFi bandwidth varies; target is ~20 FPS with auto frame dropping
- **Stream disconnects**: Auto-reconnect with exponential backoff (2s → max 10s)

### Gamepad Not Detected
- **Linux**: Install `libudev-dev` and build with `--features gamepad`
- **Windows/macOS**: Gamepad support is experimental (SDL2-based via gilrs)

## License

MIT

## Acknowledgments

- Original C# WPF controller by [saiminhtet](https://github.com/saiminhtet/ELEGOO-SmartCarV4-Multi-Functions-WPF-Controller)
- ELEGOO for the Smart Car V4 hardware and firmware
