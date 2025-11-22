// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs

use rodio::Sink;
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum PlayerCommand {
    Update(String),
    Stop,
}

#[derive(Debug)]
pub enum PlayerEvent {
    Status(String),
    Stopped,
}

pub struct Player {
    pub name: String,
    pub sink: Sink,
}

impl Player {
    pub fn new( name: String, sink:Sink) -> Player {
        Self { name, sink }
    }
    pub async fn run(
        self,
        mut cmd_rx: mpsc::UnboundedReceiver<PlayerCommand>,
        evt_tx: mpsc::UnboundedSender<PlayerEvent>,
    ) {
        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        PlayerCommand::Update(s) => {

                            // self.name = s.clone();

                            self.play(s.clone()).await;
                            let _ = evt_tx.send(PlayerEvent::Status(format!("Playing {}", s.clone())));
                        }
                        PlayerCommand::Stop => {
                            let _ = evt_tx.send(PlayerEvent::Stopped);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    let _ = evt_tx.send(PlayerEvent::Status(format!("Current name: {}", self.name)));
                }
            }
        }
    }

    async fn play(&self, p0: String) {
        if p0 != "preftest" {
            return;
        }

        let path_string = "/tmp/alert-work.ogg";
        let path_string_alternative = "/root/alert-work.ogg";
        let mut path = Path::new(path_string);
        if !path.exists() {
            path = Path::new(path_string_alternative);
            if !path.exists() {
                return;
            }

        }

        let file = File::open(path).unwrap();
        self.sink.append(rodio::Decoder::try_from(file).unwrap());
        self.sink.sleep_until_end();
    }
}
