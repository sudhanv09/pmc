use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf, split};
use tokio::net::UnixStream;

#[derive(Debug, Serialize)]
struct MpvCommand {
    command: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MpvResponse {
    data: Option<serde_json::Value>,
    request_id: Option<i32>,
    error: Option<String>,
}

#[derive(Debug)]
pub enum MpvEvent {
    EndFile,
    Shutdown,
    Other(String),
}

pub struct Player {
    writer: BufWriter<WriteHalf<UnixStream>>,
    reader: BufReader<ReadHalf<UnixStream>>,
}

impl Player {
    pub async fn init<P: AsRef<Path>>(socket_path: P) -> tokio::io::Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = split(stream);
        Ok(Self {
            writer: BufWriter::new(write_half),
            reader: BufReader::new(read_half),
        })
    }

    async fn send_command(&mut self, args: Vec<String>) -> tokio::io::Result<MpvResponse> {
        let cmd = MpvCommand { command: args };
        let json_str = serde_json::to_string(&cmd)? + "\n";
        self.writer.write_all(json_str.as_bytes()).await?;

        let mut reader = BufReader::new(&mut self.reader);
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        Ok(serde_json::from_str(&response)?)
    }

    async fn get_property(&mut self, prop: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["get_property".into(), prop.into()])
            .await
    }

    async fn set_property(&mut self, prop: &str, value: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["set_property".into(), prop.into(), value.into()])
            .await
    }

    pub async fn next_event(&mut self) -> Option<MpvEvent> {
        let mut line = String::new();
        match self.reader.read_line(&mut line).await {
            Ok(0) => return None, // EOF
            Ok(_) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(event) = value.get("event").and_then(|e| e.as_str()) {
                        return match event {
                            "end-file" => Some(MpvEvent::EndFile),
                            "shutdown" => Some(MpvEvent::Shutdown),
                            _ => Some(MpvEvent::Other(event.to_string())),
                        };
                    }
                }
            }
            Err(_) => return None,
        }
        None
    }

    pub async fn play_file<P: AsRef<Path>>(&mut self, path: P) -> tokio::io::Result<()> {
        let path_str = path.as_ref().to_string_lossy();
        self.send_command(vec![
            "loadfile".into(),
            path_str.into_owned(),
            "replace".into(),
        ])
        .await?;
        Ok(())
    }

    pub async fn get_position(&mut self) -> tokio::io::Result<Option<f64>> {
        let response = self.get_property("time-pos").await?;
        if let Some(data) = response.data {
            if let Some(pos) = data.as_f64() {
                return Ok(Some(pos));
            }
        }
        Ok(None)
    }

    pub async fn monitor_playback(&mut self) -> Result<(i16, bool), Box<dyn Error>> {
        let resp = self.get_property("duration").await?;
        let duration = resp.data.and_then(|d| d.as_f64()).unwrap_or(0.0);
        loop {
            if let Some(event) = self.next_event().await {
                match event {
                    MpvEvent::EndFile | MpvEvent::Shutdown => {
                        let position = match self.get_position().await {
                            Ok(Some(p)) => p,
                            _ => 0.0,
                        };

                        let progress = if duration > 0.0 {
                            ((position / duration) * 100.0).round() as i16
                        } else {
                            0
                        };

                        let complete = progress > 95;
                        return Ok((progress, complete));
                    }
                    _ => {}
                }
            }
        }
    }
}
