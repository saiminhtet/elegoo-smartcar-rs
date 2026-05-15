# CLAUDE.md

Project-specific notes for Claude Code when working on this Rust port of the ELEGOO Smart Car V4 controller. Reference C# implementation lives at `/Users/saiminhtet/Research-Development/elegoo_smartcar/ELEGOO-SmartCarV4-Multi-Functions-WPF-Controller`.

## Wire protocol gotchas

The car's firmware parses commands case-sensitively and silently drops anything it doesn't recognize — there's no error response, so bad commands look exactly like a non-responsive car.

- **All JSON keys are uppercase**: `H`, `N`, `D1`, `D2`, `D3`, `T`, `S`, `I1`–`I3`, `D`. Rust structs in [src/commands.rs](src/commands.rs) and [src/sensor.rs](src/sensor.rs) carry `#[serde(rename_all = "UPPERCASE")]`. Don't add a new command struct without it.
- **Heartbeat echo is mandatory.** The car sends `{Heartbeat}` and expects the exact string echoed back within a few seconds. Receive loop in [src/connection.rs](src/connection.rs) handles this; throttle the echo to ~once per 500 ms.
- **Stop is a 3-command sequence**: `CarControl(last_dir, 0)` → `CarStop()` → `JoystickClear()`. See `commands::stop_sequence`. Sending just one of these is unreliable.
- **Sequence numbers (`H`)** are required on `N=1/2/3/4/5/110` commands; the `SequencedCommand` struct handles this via a global atomic counter.

## Connection architecture

`TcpStream` is split via `into_split()` so each task owns one half — never share a read/write stream behind a mutex.

- `receive_loop` owns `OwnedReadHalf`. It calls `stream.read().await` (which blocks for long stretches waiting on data); holding any shared mutex across that `await` will starve every other task.
- `send_loop` owns `OwnedWriteHalf` plus an `mpsc::UnboundedReceiver<String>`. Wakes only when a command arrives — no polling.
- `inner: Arc<Mutex<ConnectionInner>>` holds only mutable state (heartbeat timestamps, sensor data, the `cmd_tx` sender). Lock briefly to read/write fields; never hold across socket I/O.
- To send a command from anywhere: lock `inner`, call `cmd_tx.send(...)`, drop the lock. `send_command()` in [src/connection.rs](src/connection.rs) is the canonical path.

## eframe ↔ tokio glue

The app runs eframe (synchronous main thread) and drives tokio futures via `runtime.block_on()`:

- `MainPanel.connection` must be wired up alongside `SmartCarApp.connection` in `SmartCarApp::initialize` — both fields point at the same `Arc<Mutex<ConnectionManager>>`. The key handler returns silently if `MainPanel.connection` is `None`.
- In `process_egui_keys`, assign `main_panel.pressed_keys = current_keys` **after** dispatching `on_key_down` calls. `on_key_down` starts with a duplicate-press guard (`if pressed_keys.contains(&key) { return; }`) — assigning beforehand causes every first press to be skipped.

## Useful runtime checks

```bash
# Verify TCP reachability before blaming the app
nc -z -v -w 3 <car-ip> 100

# Watch live command/heartbeat traffic
RUST_LOG=info cargo run --release   # shows "→ Command sent: ..." lines
RUST_LOG=debug cargo run --release  # also shows "← Received: ..." and acks

# Confirm JSON wire format is uppercase
cargo test --release commands::
```

## Config

[application.json](application.json) is auto-generated on first run with defaults pointing at `192.168.4.1:100` (the car's stock AP). When the car is on an existing network, update `robot.ip_address` to the DHCP-assigned address.
