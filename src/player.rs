// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs
// load multiple sources with rodio: https://stackoverflow.com/questions/75505017/how-can-i-make-rust-with-the-rodio-crate-load-multiple-sources-in-a-vec-so-i


use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::io;
use std::path::{Path};
use std::sync::Arc;
use std::time::Duration;
use rodio::source::SeekError;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use crate::media_source_trait::MediaSource;

#[derive(Debug)]
pub enum PlayerCommand {

    Update(String),
    PlayTest(),
    PlayMedia(String),
    Pause(),
    Stop(),
    Play(),
}

#[derive(Debug)]
pub enum PlayerEvent {
    Status(String),
    Position(Duration),
    Stopped,
}

pub struct Player {
    media_source: Arc<dyn MediaSource>,
    stream: OutputStream, // when removed, the samples do not play
    sink: Sink,
    loaded_id: String,
}

impl Player {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new(media_source: Arc<dyn MediaSource>, device_name: String, fallback_device_name: String) -> Player {
        let builder = Self::create_device_output_builder(device_name, fallback_device_name);
        let stream = builder.open_stream_or_fallback().unwrap();
        let sink = Sink::connect_new(stream.mixer());
        Self { media_source, sink, stream, loaded_id: String::from("") }
    }
    //                     let match_string = "USB-C to 3.5mm Headphone Jack A";
    //                     let match_string2 = "pipewire";
    fn create_device_output_builder(preferred_name: String, fallback_name: String) -> OutputStreamBuilder {
        let host = cpal::default_host();
        let devices = host.output_devices().unwrap();

        let device : Option<Device> = {
            let mut preferred_dev: Option<cpal::Device> = None;
            let mut fallback_dev: Option<cpal::Device> = None;
            let mut first_dev: Option<cpal::Device> = None;
            for d in devices {
                println!("====={}", d.name().unwrap().to_string());
                if d.name().unwrap() == preferred_name {
                    preferred_dev = Some(d);
                    break;
                } else if d.name().unwrap() == fallback_name {
                    fallback_dev = Some(d);
                } else if first_dev.is_none() {
                    first_dev = Some(d)
                }
            }

            if preferred_dev.is_some() {
                preferred_dev
            } else if fallback_dev.is_some() {
                fallback_dev
            } else {
                first_dev
            }
        };


        let builder: OutputStreamBuilder = if device.is_some() {
            let selected_device = device.unwrap();
            let builder_result = OutputStreamBuilder::from_device(selected_device);
            builder_result.unwrap()
        } else {
            OutputStreamBuilder::from_default_device().unwrap()
        };

        builder
    }

    async fn play_test(&mut self) {

        let sink = &self.sink;
        self.loaded_id = "".to_string();
        sink.clear();
        let waves = vec!(230f32, 270f32, 330f32,270f32, 230f32);
        for w in waves {
            let source = rodio::source::SineWave::new(w).amplify(0.1);
            sink.append(source);
            sink.play();
            sleep(Duration::from_millis(200)).await;
            sink.stop();
            sink.clear();
        }
    }
    async fn play_media(&mut self, id: String) -> io::Result<()> {

        if id == self.loaded_id {
            self.toggle();
            return Ok(());
        }


        let item_option = self.media_source.find(&id).await;
        
        
        if item_option.is_none() {
            return Ok(());
        }
        
        let item = item_option.unwrap();
        let path = Path::new(item.location.as_str());

        /*
        // todo: this is a dirty hack, because somehow self.media_source.open is more complex to implement to work with rodio
        let base_dir = self.media_source.id();
        let relative_dir = item.location.trim_start_matches('/');
        let path = Path::new(base_dir.as_str()).join(relative_dir);
        if !path.exists() {
            return Ok(()); // todo handle error
        }
        */
        
        let file = File::open(path)?;
        self.sink.clear();
        self.sink.append(rodio::Decoder::try_from(file).unwrap());
        self.sink.play();
        self.loaded_id = id;
        Ok(())
    }

    fn toggle(&self) {
        if self.sink.is_paused() {
            self.sink.play()
        } else {
            self.sink.pause()
        }
    }

    fn play(sink: &Sink) {
        sink.play();
    }

    fn pause(sink: &Sink) {
        sink.pause()
    }

    fn try_seek(sink: &Sink, position: Duration) -> Result<(), SeekError> {
        sink.try_seek(position)

    }

    // todo:
    // next, previous, set_volume, set_speed



    pub async fn run(
        &mut self,
        mut cmd_rx: mpsc::UnboundedReceiver<PlayerCommand>,
        evt_tx: mpsc::UnboundedSender<PlayerEvent>,
    ) {

        loop {
            let sink = &self.sink;
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        PlayerCommand::Update(s) => {
                            let x = s.clone();
                            self.play_media(s.clone()).await;
                            // format!("Playing {}", x)
                            // todo: implement player.is_playing / player.status

                            self.update_playing_status(&evt_tx).await;
                            /*
                            if self.sink.is_paused() {
                                let _ = evt_tx.send(PlayerEvent::Status("paused".to_string()));
                            } else {
                                let _ = evt_tx.send(PlayerEvent::Status("playing".to_string()));
                            }

                             */
                        }
                        PlayerCommand::PlayTest() => {
                            self.play_test().await;
                        }
                        PlayerCommand::PlayMedia(s) => {
                            self.play_media(s).await;
                            self.update_playing_status(&evt_tx).await;
                        }
                        PlayerCommand::Play() => {
                            Player::play(sink);
                            self.update_playing_status(&evt_tx).await;
                        }
                        PlayerCommand::Pause() => {
                            Player::pause(sink);
                            self.update_playing_status(&evt_tx).await;
                        }
                        PlayerCommand::Stop() => {
                            let _ = evt_tx.send(PlayerEvent::Stopped);
                            break;
                        }
                    }
                }

                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // sink.get_pos()
                    // let _ = evt_tx.send(PlayerEvent::Status(format!("Current name: {}", "<player name>")));
                    let _ = evt_tx.send(PlayerEvent::Position(sink.get_pos()));
                }
            }
        }
    }

    /*
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

        }<

        let file = File::open(path).unwrap();
        self.sink.append(rodio::Decoder::try_from(file).unwrap());
        self.sink.sleep_until_end();
    }

     */
    async fn update_playing_status(&self, evt_tx: &UnboundedSender<PlayerEvent>) {
        if self.sink.is_paused() {
            let _ = evt_tx.send(PlayerEvent::Status("paused".to_string()));
        } else {
            let _ = evt_tx.send(PlayerEvent::Status("playing".to_string()));
        }
    }
}
