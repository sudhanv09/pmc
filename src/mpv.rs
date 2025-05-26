
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

    async fn get(&mut self, prop: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["get_property".into(), prop.into()]).await
    }

    async fn set(&mut self, prop: &str, value: &str) -> tokio::io::Result<MpvResponse> {
        self.send_command(vec!["set_property".into(), prop.into(), value.into()]).await
    }

    pub async fn play_file(&mut self, path: &str) -> tokio::io::Result<()> {
        self.send_command(vec!["loadfile".into(), path.into(), "replace".into()])
            .await?;
        Ok(())
    }
}