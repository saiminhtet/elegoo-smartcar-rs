use crate::commands;
use crate::sensor::{self, SensorData, CarJsonResponse};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

pub enum ConnectionEvent {
    MessageReceived(String),
    ConnectionStatusChanged(bool),
    SensorDataUpdated(SensorData),
}

#[derive(Clone)]
pub struct ConnectionManager {
    ip_address: String,
    port: u16,
    inner: Arc<Mutex<ConnectionInner>>,
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<ConnectionEvent>>>,
}

struct ConnectionInner {
    is_connected: bool,
    last_heartbeat_received: chrono::DateTime<Utc>,
    last_keep_alive_sent: chrono::DateTime<Utc>,
    last_command_sent: chrono::DateTime<Utc>,
    current_mode: u32,
    last_line_sensor_requested: i32,
    current_sensor_data: SensorData,
    cmd_tx: Option<mpsc::UnboundedSender<String>>,
}

impl ConnectionManager {
    pub fn new(ip_address: &str, port: u16) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            ip_address: ip_address.to_string(),
            port,
            inner: Arc::new(Mutex::new(ConnectionInner {
                is_connected: false,
                last_heartbeat_received: Utc::now(),
                last_keep_alive_sent: Utc::now(),
                last_command_sent: Utc::now(),
                current_mode: 0,
                last_line_sensor_requested: -1,
                current_sensor_data: SensorData::default(),
                cmd_tx: None,
            })),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        }
    }

    pub fn event_receiver(&self) -> Arc<Mutex<mpsc::UnboundedReceiver<ConnectionEvent>>> {
        self.event_rx.clone()
    }

    pub async fn connect(&self) -> bool {
        let addr = format!("{}:{}", self.ip_address, self.port);
        tracing::info!("Connecting to {}...", addr);

        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                stream.set_nodelay(true).ok();
                use socket2::{SockRef, TcpKeepalive};
                let s = SockRef::from(&stream);
                let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(30));
                s.set_tcp_keepalive(&keepalive).ok();

                tracing::info!("Connected successfully to {}", addr);

                let (reader, writer) = stream.into_split();
                let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<String>();

                {
                    let mut inner = self.inner.lock().await;
                    inner.is_connected = true;
                    inner.last_heartbeat_received = Utc::now();
                    inner.cmd_tx = Some(cmd_tx);
                }

                let event_tx = self.event_tx.clone();
                let inner_arc = self.inner.clone();

                let rx_inner = inner_arc.clone();
                let rx_tx = event_tx.clone();
                tokio::spawn(async move {
                    Self::receive_loop(reader, rx_inner, rx_tx).await;
                });

                let tx_inner = inner_arc.clone();
                let tx_tx = event_tx.clone();
                tokio::spawn(async move {
                    Self::send_loop(writer, cmd_rx, tx_inner, tx_tx).await;
                });

                let hb_inner = inner_arc.clone();
                tokio::spawn(async move {
                    Self::heartbeat_monitor(hb_inner).await;
                });

                let sensor_inner = inner_arc.clone();
                tokio::spawn(async move {
                    Self::sensor_polling(sensor_inner).await;
                });

                let _ = event_tx.send(ConnectionEvent::ConnectionStatusChanged(true));
                true
            }
            Err(e) => {
                tracing::error!("Connection failed: {}", e);
                false
            }
        }
    }

    pub async fn send_command(&self, command: &str) {
        let mut inner = self.inner.lock().await;
        if !inner.is_connected {
            tracing::warn!("Not connected. Command not sent: {}", command);
            return;
        }
        inner.last_command_sent = Utc::now();
        if let Some(tx) = &inner.cmd_tx {
            if tx.send(command.to_string()).is_err() {
                tracing::error!("Command channel closed; marking disconnected");
                inner.is_connected = false;
                inner.cmd_tx = None;
            } else {
                tracing::debug!("Queued command: {}", command);
            }
        }
    }

    /// Switch car operating mode. Mode 0 uses JoystickClear, modes 1-3 use SwitchMode.
    pub async fn switch_mode(&self, mode: u32) {
        let mut inner = self.inner.lock().await;
        inner.current_mode = mode;
        drop(inner);

        if mode == 0 {
            tracing::info!("Switching to Mode 0 (Manual/Normal)");
            self.send_command(&commands::joystick_clear()).await;
        } else {
            tracing::info!("Switching to Mode {}", mode);
            self.send_command(&commands::switch_mode(mode)).await;
        }
    }

    /// Send the 3-command stop sequence
    pub async fn send_stop_sequence(&self, last_direction: u32) {
        for cmd in commands::stop_sequence(last_direction) {
            self.send_command(&cmd).await;
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.inner.lock().await.is_connected
    }

    pub async fn current_sensor_data(&self) -> SensorData {
        self.inner.lock().await.current_sensor_data.clone()
    }

    async fn receive_loop(
        mut reader: OwnedReadHalf,
        inner: Arc<Mutex<ConnectionInner>>,
        event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    ) {
        let mut buffer = vec![0u8; 4096];
        let mut message_buf = String::new();

        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    tracing::error!("Connection closed by remote host (0 bytes read)");
                    let mut guard = inner.lock().await;
                    guard.is_connected = false;
                    guard.cmd_tx = None;
                    let _ = event_tx.send(ConnectionEvent::ConnectionStatusChanged(false));
                    return;
                }
                Ok(n) => {
                    let raw = String::from_utf8_lossy(&buffer[..n]).to_string();
                    if !raw.contains("{Heartbeat}") {
                        tracing::debug!("← Received: {}", raw.trim());
                    }
                    message_buf.push_str(&raw);
                    Self::process_messages(&mut message_buf, &inner, &event_tx).await;
                }
                Err(e) => {
                    tracing::error!("Receive error: {}", e);
                    let mut guard = inner.lock().await;
                    guard.is_connected = false;
                    guard.cmd_tx = None;
                    let _ = event_tx.send(ConnectionEvent::ConnectionStatusChanged(false));
                    return;
                }
            }
        }
    }

    async fn process_messages(
        buffer: &mut String,
        inner: &Arc<Mutex<ConnectionInner>>,
        event_tx: &mpsc::UnboundedSender<ConnectionEvent>,
    ) {
        let data = buffer.clone();
        let mut processed_len = 0;

        // Process {Heartbeat} messages
        if let Some(pos) = data.find("{Heartbeat}") {
            let end = pos + "{Heartbeat}".len();
            processed_len = processed_len.max(end);

            let mut guard = inner.lock().await;
            guard.last_heartbeat_received = Utc::now();
            let now = Utc::now();
            let since_last = now - guard.last_keep_alive_sent;
            if since_last.num_milliseconds() >= 500 {
                guard.last_keep_alive_sent = now;
                if let Some(tx) = &guard.cmd_tx {
                    let _ = tx.send("{Heartbeat}".to_string());
                }
            }
        }

        // Process error messages
        let mut remaining = &data[processed_len..];
        while let Some(err_start) = remaining.find("error:") {
            let err_end = remaining[err_start..]
                .find(['\n', '{'])
                .map(|i| err_start + i)
                .unwrap_or(remaining.len());
            let err_msg = remaining[err_start..err_end].trim();
            tracing::warn!("Car error: {}", err_msg);
            let abs_end = processed_len + err_end;
            processed_len = processed_len.max(abs_end);
            remaining = &data[processed_len..];
        }

        // Process acknowledgment and sensor messages in braces
        let mut idx = processed_len;
        while idx < data.len() {
            let rest = &data[idx..];
            if let Some(open) = rest.find('{') {
                if let Some(close) = rest[open..].find('}') {
                    let absolute_open = idx + open;
                    let absolute_close = absolute_open + close + 1;
                    let content = &data[absolute_open..absolute_close];

                    if content == "{Heartbeat}" {
                        idx = absolute_close;
                        processed_len = idx;
                        continue;
                    }

                    if content == "{ok}" || content.len() > 3
                        && content.starts_with('{')
                        && content.ends_with("_ok}")
                    {
                        tracing::debug!("✓ Acknowledgment: {}", content);
                        idx = absolute_close;
                        processed_len = idx;
                        let _ = event_tx.send(ConnectionEvent::MessageReceived(content.to_string()));
                        continue;
                    }

                    if let Some((_id, value)) = sensor::parse_raw_sensor_value(content) {
                        let mut guard = inner.lock().await;
                        if guard.last_line_sensor_requested >= 0 {
                            let detected = value < 500;
                            match guard.last_line_sensor_requested {
                                0 => {
                                    guard.current_sensor_data.left_line_detected = detected;
                                    guard.current_sensor_data.raw_ir_left = value as i32;
                                }
                                1 => {
                                    guard.current_sensor_data.middle_line_detected = detected;
                                    guard.current_sensor_data.raw_ir_middle = value as i32;
                                }
                                2 => {
                                    guard.current_sensor_data.right_line_detected = detected;
                                    guard.current_sensor_data.raw_ir_right = value as i32;
                                }
                                _ => {}
                            }
                            guard.current_sensor_data.line_tracking_timestamp = Some(Utc::now());
                            guard.last_line_sensor_requested = -1;
                            let _ = event_tx.send(ConnectionEvent::SensorDataUpdated(
                                guard.current_sensor_data.clone(),
                            ));
                        } else {
                            guard.current_sensor_data.ultrasonic_distance = value as i32;
                            guard.current_sensor_data.raw_ultrasonic = value as i32;
                            guard.current_sensor_data.ultrasonic_timestamp = Some(Utc::now());
                            let _ = event_tx.send(ConnectionEvent::SensorDataUpdated(
                                guard.current_sensor_data.clone(),
                            ));
                        }

                        idx = absolute_close;
                        processed_len = idx;
                        continue;
                    }

                    if let Ok(json_resp) =
                        serde_json::from_str::<CarJsonResponse>(content)
                    {
                        let _ = event_tx.send(ConnectionEvent::MessageReceived(
                            content.to_string(),
                        ));
                        if let Some(n) = json_resp.n {
                            let mut guard = inner.lock().await;
                            match n {
                                21 => {
                                    if let Some(d) = json_resp.d {
                                        guard.current_sensor_data.ultrasonic_distance = d as i32;
                                        guard.current_sensor_data.raw_ultrasonic = d as i32;
                                        guard.current_sensor_data.ultrasonic_timestamp =
                                            Some(Utc::now());
                                        let _ = event_tx.send(
                                            ConnectionEvent::SensorDataUpdated(
                                                guard.current_sensor_data.clone(),
                                            ),
                                        );
                                    }
                                }
                                22 => {
                                    guard.current_sensor_data.left_line_detected =
                                        json_resp.d1.unwrap_or(1) == 0;
                                    guard.current_sensor_data.middle_line_detected =
                                        json_resp.d2.unwrap_or(1) == 0;
                                    guard.current_sensor_data.right_line_detected =
                                        json_resp.d3.unwrap_or(1) == 0;
                                    guard.current_sensor_data.line_tracking_timestamp =
                                        Some(Utc::now());
                                    let _ = event_tx.send(ConnectionEvent::SensorDataUpdated(
                                        guard.current_sensor_data.clone(),
                                    ));
                                }
                                _ => {}
                            }
                        }
                        idx = absolute_close;
                        processed_len = idx;
                        continue;
                    }

                    idx = absolute_close + 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if processed_len > 0 {
            buffer.drain(..processed_len);
        }
    }

    async fn send_loop(
        mut writer: OwnedWriteHalf,
        mut cmd_rx: mpsc::UnboundedReceiver<String>,
        inner: Arc<Mutex<ConnectionInner>>,
        event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            if let Err(e) = writer.write_all(cmd.as_bytes()).await {
                tracing::error!("Send failed: {}", e);
                let mut guard = inner.lock().await;
                guard.is_connected = false;
                guard.cmd_tx = None;
                let _ = event_tx.send(ConnectionEvent::ConnectionStatusChanged(false));
                return;
            }
            if cmd != "{Heartbeat}" {
                tracing::info!("→ Command sent: {}", cmd);
            }
        }
        tracing::debug!("Send loop exiting (channel closed)");
    }

    async fn heartbeat_monitor(inner: Arc<Mutex<ConnectionInner>>) {
        loop {
            sleep(Duration::from_secs(10)).await;

            let mut guard = inner.lock().await;
            if !guard.is_connected {
                continue;
            }

            let elapsed = (Utc::now() - guard.last_heartbeat_received).num_seconds();
            if elapsed > 30 {
                tracing::warn!("Heartbeat timeout - no heartbeat for {}s", elapsed);
                guard.is_connected = false;
                guard.cmd_tx = None;
            } else if elapsed > 10 {
                tracing::warn!("Slow heartbeat - last received {}s ago", elapsed);
            }
        }
    }

    async fn sensor_polling(inner: Arc<Mutex<ConnectionInner>>) {
        sleep(Duration::from_secs(5)).await;
        tracing::info!("Sensor polling service started");

        loop {
            sleep(Duration::from_secs(5)).await;

            let (mode, cmd_tx) = {
                let guard = inner.lock().await;
                (guard.current_mode, guard.cmd_tx.clone())
            };

            let Some(tx) = cmd_tx else { continue };

            if mode == 0 {
                let _ = tx.send(commands::ultrasonic_status(1));
            } else {
                tracing::debug!("Re-asserting Mode {} for autonomous operation", mode);
                let _ = tx.send(commands::switch_mode(mode));
            }
        }
    }
}
