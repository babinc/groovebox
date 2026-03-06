use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use super::types::PlaybackState;

const SOCKET_PATH: &str = "/tmp/groovebox-mpv.sock";

pub struct MpvPlayer {
    _process: Child,
    writer: tokio::io::WriteHalf<UnixStream>,
    request_id: u64,
}

impl MpvPlayer {
    pub async fn spawn() -> Result<(Self, mpsc::Receiver<PlaybackState>)> {
        // Clean up old socket
        let _ = std::fs::remove_file(SOCKET_PATH);

        let process = Command::new("mpv")
            .args([
                "--idle=yes",
                &format!("--input-ipc-server={SOCKET_PATH}"),
                "--no-video",
                "--no-terminal",
                "--volume=100",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        // Wait for socket
        for _ in 0..50 {
            if Path::new(SOCKET_PATH).exists() {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }

        let stream = UnixStream::connect(SOCKET_PATH).await?;
        let (reader, writer) = tokio::io::split(stream);

        let (state_tx, state_rx) = mpsc::channel(32);

        // Spawn reader task
        tokio::spawn(async move {
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            let mut current_state = PlaybackState::default();

            loop {
                line.clear();
                match buf_reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if let Ok(msg) = serde_json::from_str::<Value>(line.trim()) {
                            if let Some(event) = msg.get("event").and_then(|e| e.as_str()) {
                                match event {
                                    "playback-restart" => {
                                        current_state.status = super::types::PlayStatus::Playing;
                                    }
                                    "pause" => {
                                        current_state.status = super::types::PlayStatus::Paused;
                                    }
                                    "unpause" => {
                                        current_state.status = super::types::PlayStatus::Playing;
                                    }
                                    "end-file" => {
                                        current_state.status = super::types::PlayStatus::Stopped;
                                        current_state.position = 0.0;
                                    }
                                    _ => {}
                                }
                            }
                            // Property change events
                            if let Some(name) = msg.get("name").and_then(|n| n.as_str()) {
                                let data = msg.get("data");
                                match name {
                                    "playback-time" => {
                                        if let Some(v) = data.and_then(|d| d.as_f64()) {
                                            current_state.position = v;
                                        }
                                    }
                                    "duration" => {
                                        if let Some(v) = data.and_then(|d| d.as_f64()) {
                                            current_state.duration = v;
                                        }
                                    }
                                    "volume" => {
                                        if let Some(v) = data.and_then(|d| d.as_f64()) {
                                            current_state.volume = v;
                                        }
                                    }
                                    "audio-params/samplerate" => {
                                        if let Some(v) = data.and_then(|d| d.as_u64()) {
                                            current_state.sample_rate = Some(v as u32);
                                        }
                                    }
                                    "audio-params/channel-count" => {
                                        if let Some(v) = data.and_then(|d| d.as_u64()) {
                                            current_state.channels = Some(v as u32);
                                        }
                                    }
                                    "audio-codec-name" => {
                                        if let Some(v) = data.and_then(|d| d.as_str()) {
                                            current_state.codec = Some(v.to_string());
                                        }
                                    }
                                    "audio-bitrate" => {
                                        if let Some(v) = data.and_then(|d| d.as_f64()) {
                                            current_state.bitrate = Some((v / 1000.0) as u32);
                                        }
                                    }
                                    "pause" => {
                                        if let Some(v) = data.and_then(|d| d.as_bool()) {
                                            if v {
                                                current_state.status = super::types::PlayStatus::Paused;
                                            } else if current_state.status == super::types::PlayStatus::Paused {
                                                current_state.status = super::types::PlayStatus::Playing;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            let _ = state_tx.try_send(current_state.clone());
                        }
                    }
                }
            }
        });

        let mut player = Self {
            _process: process,
            writer,
            request_id: 0,
        };

        // Observe properties
        player.observe_property("playback-time", 1).await?;
        player.observe_property("duration", 2).await?;
        player.observe_property("volume", 3).await?;
        player.observe_property("audio-params/samplerate", 4).await?;
        player.observe_property("audio-params/channel-count", 5).await?;
        player.observe_property("audio-codec-name", 6).await?;
        player.observe_property("audio-bitrate", 7).await?;
        player.observe_property("pause", 8).await?;

        Ok((player, state_rx))
    }

    async fn send_command(&mut self, command: Value) -> Result<()> {
        self.request_id += 1;
        let msg = json!({
            "command": command,
            "request_id": self.request_id,
        });
        let mut line = serde_json::to_string(&msg)?;
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await
            .map_err(|e| anyhow::anyhow!("mpv socket write failed (process may have crashed): {e}"))?;
        Ok(())
    }

    async fn observe_property(&mut self, name: &str, id: u64) -> Result<()> {
        self.send_command(json!(["observe_property", id, name])).await
    }

    pub async fn load_file(&mut self, url: &str) -> Result<()> {
        self.send_command(json!(["loadfile", url])).await
    }

    pub async fn set_pause(&mut self, paused: bool) -> Result<()> {
        self.send_command(json!(["set_property", "pause", paused])).await
    }

    pub async fn seek(&mut self, seconds: f64) -> Result<()> {
        self.send_command(json!(["seek", seconds, "relative"])).await
    }

    pub async fn seek_absolute(&mut self, seconds: f64) -> Result<()> {
        self.send_command(json!(["seek", seconds, "absolute"])).await
    }

    pub async fn set_volume(&mut self, volume: f64) -> Result<()> {
        self.send_command(json!(["set_property", "volume", volume])).await
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.send_command(json!(["stop"])).await
    }

    pub async fn quit(&mut self) -> Result<()> {
        let _ = self.send_command(json!(["quit"])).await;
        let _ = self._process.kill().await;
        Ok(())
    }
}
