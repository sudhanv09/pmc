use crate::library::{PlaybackEvent, SharedState};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf, split};
use tokio::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

#[derive(Debug, Serialize)]
struct MpvCommand {
    command: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct MpvResponse {
    data: Option<serde_json::Value>,
    request_id: Option<i32>,
    error: String,
}

#[derive(Debug, Deserialize)]
struct MpvEvent {
    event: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct MpvPropertyChangeEvent {
    name: String,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct MpvEndFileEvent {
    reason: String,
}

pub struct Player {
    writer: BufWriter<WriteHalf<UnixStream>>,
    reader: BufReader<ReadHalf<UnixStream>>,
    next_request_id: i32,
}

impl Player {
    pub async fn init<P: AsRef<Path>>(socket_path: P) -> tokio::io::Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = split(stream);
        Ok(Self {
            writer: BufWriter::new(write_half),
            reader: BufReader::new(read_half),
            next_request_id: 1,
        })
    }

    async fn send_command(&mut self, args: Vec<String>) -> tokio::io::Result<MpvResponse> {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);

        let cmd = MpvCommand {
            command: args.into_iter().map(serde_json::Value::String).collect(),
            request_id: Some(request_id),
        };
        let json_str = serde_json::to_string(&cmd)? + "\n";
        self.writer.write_all(json_str.as_bytes()).await?;
        self.writer.flush().await?;

        // Read responses until we get the one matching our request_id
        loop {
            let mut response = String::new();
            let bytes_read = self.reader.read_line(&mut response).await?;
            if bytes_read == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "MPV socket closed",
                ));
            }
            if response.trim().is_empty() {
                continue;
            }

            // Try parsing as a response or event
            if let Ok(resp) = serde_json::from_str::<MpvResponse>(&response) {
                if resp.request_id == Some(request_id) {
                    if resp.error != "success" {
                        println!("MPV error: {}", resp.error);
                    }
                    return Ok(resp);
                }
            }
        }
    }

    async fn observe_property(&mut self, name: &str, id: i32) -> tokio::io::Result<()> {
        let cmd = MpvCommand {
            command: vec![
                serde_json::Value::String("observe_property".to_string()),
                serde_json::Value::Number(id.into()),
                serde_json::Value::String(name.to_string()),
            ],
            request_id: None,
        };

        let json_str = serde_json::to_string(&cmd)? + "\n";
        self.writer.write_all(json_str.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn get_property(&mut self, prop: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["get_property".into(), prop.into()])
            .await
    }

    pub async fn play_file<P: AsRef<Path>>(&mut self, path: P) -> tokio::io::Result<()> {
        let path_str = path.as_ref().to_string_lossy();
        self
            .send_command(vec![
                "loadfile".into(),
                path_str.into_owned(),
                "replace".into(),
            ])
            .await?;

        Ok(())
    }

    async fn wait_mpv(&mut self) -> bool {
        let max_init_attempts = 30; // 30 seconds max wait

        for attempt in 0..max_init_attempts {
            if let Ok(response) = self.get_property("playback-time").await {
                if response.data.is_some() {
                    println!("Playback initialized!");
                    return false; // Success
                }
            }

            if attempt == max_init_attempts - 1 {
                println!("Timeout waiting for playback to start");
                return true; // Timeout
            }

            sleep(Duration::from_secs(1)).await;
        }

        true // Should never reach here
    }

    pub async fn start_monitoring(
        player: &Arc<Mutex<Player>>,
        state: SharedState,
        tx: UnboundedSender<PlaybackEvent>,
    ) {
        println!("Starting playback monitoring...");

        {
            let mut player_guard = player.lock().await;
            if player_guard.wait_mpv().await {
                println!("Timed out");
                return;
            }

            // Set up property observation
            if let Err(e) = player_guard.observe_property("pause", 1).await {
                println!("Failed to observe pause property: {}", e);
                let _ = tx.send(PlaybackEvent::Error(format!(
                    "Failed to observe pause: {}",
                    e
                )));
                return;
            }

            if let Err(e) = player_guard.observe_property("percent-pos", 2).await {
                println!("Failed to observe percent-pos property: {}", e);
                let _ = tx.send(PlaybackEvent::Error(format!(
                    "Failed to observe percent-pos: {}",
                    e
                )));
                return;
            }

            if let Err(e) = player_guard.observe_property("eof-reached", 3).await {
                println!("Failed to observe eof-reached property: {}", e);
                let _ = tx.send(PlaybackEvent::Error(format!(
                    "Failed to observe eof-reached: {}",
                    e
                )));
                return;
            }
        }

        // Send initial started event
        let _ = tx.send(PlaybackEvent::Started);

        loop {
            let mut player_guard = player.lock().await;
            let mut line = String::new();
            match player_guard.reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("MPV has exited.");
                    let _ = tx.send(PlaybackEvent::Exited);
                    let mut state_guard = state.lock().await;
                    state_guard.stop_playback();
                    break;
                }

                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<MpvEvent>(&line) {
                        match event.event.as_str() {
                            "property-change" => {
                                if let Ok(prop_change) =
                                    serde_json::from_value::<MpvPropertyChangeEvent>(event.data)
                                {
                                    match prop_change.name.as_str() {
                                        "pause" => {
                                            if let Some(is_paused) =
                                                prop_change.data.and_then(|d| d.as_bool())
                                            {
                                                let event = if is_paused {
                                                    PlaybackEvent::Paused
                                                } else {
                                                    PlaybackEvent::Resumed
                                                };
                                                let _ = tx.send(event);
                                                let mut state_guard = state.lock().await;
                                                state_guard.is_playing = !is_paused;
                                            }
                                        }
                                        "percent-pos" => {
                                            if let Some(pos) =
                                                prop_change.data.and_then(|d| d.as_f64())
                                            {
                                                let _ = tx.send(PlaybackEvent::Position(pos));
                                                let mut state_guard = state.lock().await;
                                                state_guard.update_position(pos);
                                            }
                                        }
                                        "eof-reached" => {
                                            if let Some(eof) =
                                                prop_change.data.and_then(|d| d.as_bool())
                                            {
                                                if eof {
                                                    println!("EOF reached via property change");
                                                    let _ = tx.send(PlaybackEvent::Completed);
                                                    let mut state_guard = state.lock().await;
                                                    state_guard.stop_playback();
                                                }
                                            }
                                        }
                                        _ => {} // Ignore other property changes
                                    }
                                }
                            }
                            "end-file" => {
                                if let Ok(end_event) =
                                    serde_json::from_value::<MpvEndFileEvent>(event.data)
                                {
                                    match end_event.reason.as_str() {
                                        "eof" => {
                                            let _ = tx.send(PlaybackEvent::Completed);
                                            let mut state_guard = state.lock().await;
                                            state_guard.stop_playback();
                                        }
                                        "stop" | "quit" => {
                                            let _ = tx.send(PlaybackEvent::Stopped);
                                            let mut state_guard = state.lock().await;
                                            state_guard.stop_playback();
                                        }
                                        _ => {
                                            println!(
                                                "Unhandled end-file reason: {}",
                                                end_event.reason
                                            );
                                        }
                                    }
                                }
                            }
                            "shutdown" => {
                                println!("MPV is shutting down.");
                                let _ = tx.send(PlaybackEvent::Stopped);
                                let mut state_guard = state.lock().await;
                                state_guard.stop_playback();
                                break; // Exit the loop
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading from MPV socket: {}", e);
                    let _ = tx.send(PlaybackEvent::Error(format!("Socket read error: {}", e)));
                    break;
                }
            }
        }
        println!("Monitoring stopped.");
    }
}

pub fn spawn_mpv(socket_path: &str) -> std::io::Result<Child> {
    let child = Command::new("mpv")
        .arg("--idle")
        .arg("--force-window")
        .arg("--no-terminal")
        .arg(format!("--input-ipc-server={}", socket_path))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(child)
}
