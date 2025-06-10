use crate::library::SharedState;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf, split};
use tokio::net::UnixStream;
use tokio::time::{Duration, sleep};

#[derive(Debug, Serialize)]
struct MpvCommand {
    command: Vec<String>,
    request_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct MpvResponse {
    data: Option<serde_json::Value>,
    request_id: Option<i32>,
    error: String,
}

#[derive(Debug, Deserialize)]
struct MpvEventResponse {
    event: Option<String>,
    id: Option<i32>,
    data: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum MpvEvent {
    EndFile,
    Shutdown,
    Pause,
    Unpause,
    Other(()),
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
        self.next_request_id += self.next_request_id.wrapping_add(1);

        let cmd = MpvCommand {
            command: args,
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
            match serde_json::from_str::<MpvResponse>(&response) {
                Ok(resp) if resp.request_id == Some(request_id) => {
                    if resp.error != "success" {
                        println!("MPV error: {}", resp.error);
                    }
                    return Ok(resp);
                }
                Ok(_) => continue, // Response for a different request_id, keep reading
                Err(_) => {
                    // Check if it's an event
                    if let Ok(event) = serde_json::from_str::<MpvEventResponse>(&response) {
                        continue; // Skip events, keep reading for our response
                    } else {
                        println!("Failed to parse MPV response: {}", response);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid JSON response from MPV",
                        ));
                    }
                }
            }
        }
    }

    async fn get_property(&mut self, prop: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["get_property".into(), prop.into()])
            .await
    }

    async fn set_property(&mut self, prop: &str, value: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["set_property".into(), prop.into(), value.into()])
            .await
    }

    async fn observe_property(&mut self, prop: &str, id: i32) -> tokio::io::Result<()> {
        let response = self
            .send_command(vec!["observe_property".into(), id.to_string(), prop.into()])
            .await?;
        if response.error != "success" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to observe property {}: {}", prop, response.error),
            ));
        }
        Ok(())
    }

    pub async fn play_file<P: AsRef<Path>>(&mut self, path: P) -> tokio::io::Result<()> {
        let path_str = path.as_ref().to_string_lossy();
        let response = self
            .send_command(vec![
                "loadfile".into(),
                path_str.into_owned(),
                "replace".into(),
            ])
            .await?;
        if response.error != "success" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to load file: {}", response.error),
            ));
        }
        Ok(())
    }

    pub async fn get_position(&mut self) -> tokio::io::Result<Option<f64>> {
        let response = self.get_property("time-pos").await?;
        if response.error != "success" {
            println!("Error getting time-pos: {}", response.error);
            return Ok(None);
        }
        Ok(response.data.and_then(|data| data.as_f64()))
    }

    pub async fn get_percent_pos(&mut self) -> tokio::io::Result<Option<f64>> {
        let response = self.get_property("percent-pos").await?;
        if response.error != "success" {
            println!("Error getting percent-pos: {}", response.error);
            return Ok(None);
        }
        Ok(response.data.and_then(|data| data.as_f64()))
    }

    pub async fn get_duration(&mut self) -> tokio::io::Result<Option<f64>> {
        let response = self.get_property("duration").await?;
        if response.error != "success" {
            println!("Error getting duration: {}", response.error);
            return Ok(None);
        }
        Ok(response.data.and_then(|data| data.as_f64()))
    }

    pub async fn paused(&mut self) -> tokio::io::Result<Option<bool>> {
        let response = self.get_property("pause").await?;
        if response.error != "success" {
            println!("Error getting pause state: {}", response.error);
            return Ok(None);
        }
        Ok(response.data.and_then(|data| data.as_bool()))
    }

    pub async fn running(&mut self) -> bool {
        // Check if we can send a simple command to verify MPV is responsive
        self.get_property("pause").await.is_ok()
    }

    // New: Check if playback has ended
    pub async fn has_ended(&mut self) -> tokio::io::Result<bool> {
        let response = self.get_property("eof-reached").await?;
        if response.error != "success" {
            println!("Error getting eof-reached: {}", response.error);
            return Ok(false);
        }
        Ok(response
            .data
            .and_then(|data| data.as_bool())
            .unwrap_or(false))
    }

    async fn wait_mpv(&mut self) -> bool {
        println!("Waiting for playback to initialize...");
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

    pub async fn start_monitoring(&mut self, state: SharedState) -> tokio::io::Result<()> {
        // Observe properties for events
        let timed_out = self.wait_mpv().await;
        if timed_out {
            println!("Failed to initialize playback - timeout reached");
            return Ok(());
        }
        println!("Starting playback monitoring...");

        loop {
            // Check if we should stop
            {
                let state_guard = state.lock().unwrap();
                if state_guard.should_stop {
                    println!("Playback stopped");
                    return Ok(());
                }
            }

            // Check if MPV is still running
            if !self.running().await {
                println!("MPV has stopped");
                {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.stop_playback();
                }
                return Ok(());
            }

            // Check if playback has ended
            if self.has_ended().await? {
                println!("Playback has ended");
                {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.stop_playback();
                }
                return Ok(());
            }

            if let Ok(Some(is_paused)) = self.paused().await {
                if is_paused {
                    println!("Playback is paused");
                    // Update state to reflect paused status
                    {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.is_playing = false;
                    }
                    sleep(Duration::from_secs(5)).await;
                    continue;
                } else {
                    // Update state to reflect playing status
                    {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.is_playing = true;
                    }
                }
            } else {
                println!("Failed to get pause state");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // get current pos
            if let Ok(Some(percent_pos)) = self.get_percent_pos().await {
                {
                    let mut state_guard = state.lock().unwrap();
                    state_guard.update_position(percent_pos);
                    println!("Position: {:.1}%", percent_pos);
                }

                // Check if we're near the end (> 95% means essentially finished)
                if percent_pos > 95.0 {
                    println!("Playback nearly complete at {:.1}%", percent_pos);
                    {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.stop_playback();
                    }
                    break;
                }
            } else {
                // If we can't get position, might mean playback ended
                println!("Cannot get position");
            }

            // Wait 5 seconds before poll
            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }
}
