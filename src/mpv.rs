use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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

pub struct Player {
    socket: UnixStream,
}

impl Player {
    pub async fn init<P: AsRef<Path>>(socket_path: P) -> tokio::io::Result<Self> {
        let socket = UnixStream::connect(socket_path).await?;
        Ok(Self { socket })
    }

    async fn send_command(&mut self, args: Vec<String>) -> tokio::io::Result<MpvResponse> {
        let cmd = MpvCommand { command: args };
        let json_str = serde_json::to_string(&cmd)? + "\n";
        self.socket.write_all(json_str.as_bytes()).await?;

        let mut reader = BufReader::new(&mut self.socket);
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

    pub async fn play_file<P: AsRef<Path>>(&mut self, path: P) -> tokio::io::Result<()> {
        let path_str = path.as_ref().to_string_lossy();
        self.send_command(vec!["loadfile".into(), path_str.into_owned(), "replace".into()])
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
}
