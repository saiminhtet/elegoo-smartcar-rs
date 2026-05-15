use image::load_from_memory;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub enum VideoEvent {
    FrameReceived(Vec<u8>, u32, u32), // raw RGBA bytes, width, height
    FrameDropped,
    StreamStatus(bool, String), // connected, status message
}

pub struct VideoStreamViewer {
    stream_url: String,
    event_tx: mpsc::UnboundedSender<VideoEvent>,
    event_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<VideoEvent>>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl VideoStreamViewer {
    pub fn new(ip_address: &str) -> Self {
        let stream_url = format!("http://{}:81/stream", ip_address);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            stream_url,
            event_tx: event_tx.clone(),
            event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn event_receiver(&self) -> Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<VideoEvent>>> {
        self.event_rx.clone()
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn start(&self) {
        if self.is_running() {
            return;
        }

        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);

        let stream_url = self.stream_url.clone();
        let event_tx = self.event_tx.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            Self::stream_loop(stream_url, event_tx, is_running).await;
        });

        tracing::info!("Video stream task started");
    }

    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Video stream stopped");
    }

    async fn stream_loop(
        stream_url: String,
        event_tx: mpsc::UnboundedSender<VideoEvent>,
        is_running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 999;

        while is_running.load(std::sync::atomic::Ordering::Relaxed) && retry_count < MAX_RETRIES {
            tracing::info!("[Video] Connecting to stream... (attempt {})", retry_count + 1);

            match client
                .get(&stream_url)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        tracing::warn!("[Video] Stream returned status: {}", response.status());
                        retry_count += 1;
                        let delay = std::cmp::min(retry_count * 2, 10);
                        sleep(Duration::from_secs(delay as u64)).await;
                        continue;
                    }

                    tracing::info!("[Video] Stream connected successfully");
                    retry_count = 0;
                    let _ = event_tx.send(VideoEvent::StreamStatus(true, "Streaming".to_string()));

                    let mut frame_buffer = Vec::new();
                    let mut last_frame_time = Instant::now();
                    let target_frame_time = Duration::from_millis(50); // ~20 FPS
                    let mut consecutive_drops = 0;

                    // Stream body chunks
                    let mut byte_stream = response.bytes_stream();
                    use futures_util::StreamExt;

                    while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                        let chunk = match tokio::time::timeout(
                            Duration::from_secs(10),
                            byte_stream.next(),
                        )
                        .await
                        {
                            Ok(Some(Ok(bytes))) => bytes,
                            Ok(Some(Err(e))) => {
                                tracing::warn!("[Video] Stream read error: {}", e);
                                break;
                            }
                            Ok(None) => {
                                tracing::info!("[Video] Stream ended");
                                break;
                            }
                            Err(_) => {
                                tracing::warn!("[Video] Stream read timeout");
                                break;
                            }
                        };

                        for &byte in chunk.iter() {
                            frame_buffer.push(byte);

                            // Check for JPEG end marker FF D9
                            if frame_buffer.len() >= 2
                                && frame_buffer[frame_buffer.len() - 2] == 0xFF
                                && frame_buffer[frame_buffer.len() - 1] == 0xD9
                            {
                                // Find JPEG start marker FF D8
                                let mut start_idx = frame_buffer.len() - 1;
                                for j in (0..frame_buffer.len() - 1).rev() {
                                    if frame_buffer[j] == 0xFF && frame_buffer[j + 1] == 0xD8 {
                                        start_idx = j;
                                        break;
                                    }
                                }

                                if start_idx < frame_buffer.len() - 2 {
                                    let jpeg_data: Vec<u8> =
                                        frame_buffer[start_idx..].to_vec();
                                    frame_buffer.clear();

                                    // Frame rate limiting
                                    let now = Instant::now();
                                    if last_frame_time.elapsed() < target_frame_time
                                        && last_frame_time.elapsed().as_millis() > 0
                                    {
                                        let _ = event_tx.send(VideoEvent::FrameDropped);
                                        consecutive_drops += 1;
                                        if consecutive_drops > 100 {
                                            tracing::warn!(
                                                "[Video] {} consecutive frames dropped",
                                                consecutive_drops
                                            );
                                            consecutive_drops = 0;
                                        }
                                        continue;
                                    }
                                    last_frame_time = now;
                                    consecutive_drops = 0;

                                    // Decode JPEG
                                    match load_from_memory(&jpeg_data) {
                                        Ok(img) => {
                                            let (width, height) = (img.width(), img.height());
                                            let rgba = img.to_rgba8();
                                            let raw_data = rgba.into_raw();
                                            let _ = event_tx.send(VideoEvent::FrameReceived(
                                                raw_data,
                                                width,
                                                height,
                                            ));
                                        }
                                        Err(e) => {
                                            tracing::debug!("[Video] JPEG decode error: {}", e);
                                        }
                                    }
                                }

                                // Prevent buffer overflow
                                if frame_buffer.len() > 512 * 1024 {
                                    frame_buffer.clear();
                                }
                            }
                        }
                    }

                    // Stream ended
                    let _ = event_tx.send(VideoEvent::StreamStatus(false, "Disconnected".to_string()));

                    if is_running.load(std::sync::atomic::Ordering::Relaxed) {
                        tracing::info!("[Video] Stream ended, retrying in 2s...");
                        sleep(Duration::from_secs(2)).await;
                    }
                }
                Err(e) => {
                    retry_count += 1;
                    tracing::warn!("[Video] Stream error: {}", e);

                    if is_running.load(std::sync::atomic::Ordering::Relaxed) {
                        let delay = std::cmp::min(retry_count * 2, 10);
                        sleep(Duration::from_secs(delay as u64)).await;
                    }
                }
            }
        }

        tracing::info!("[Video] Stream task ended");
    }
}
