// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Device;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use rodio::source::SeekError;
use tokio::sync::mpsc;
use tokio::time::sleep;

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
    Stopped,
}

pub struct Player {
    pub name: String,
    stream: OutputStream, // when removed, the samples do not play
    sink: Sink,
}

impl Player {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new( name: String, device_name: String, fallback_device_name: String) -> Player {
        let builder = Player::create_device_output_builder(device_name, fallback_device_name);
        let stream = builder.open_stream_or_fallback().unwrap();
        let sink = Sink::connect_new(stream.mixer());
        Self { name, sink, stream }
    }
    //                     let match_string = "USB-C to 3.5mm Headphone Jack A";
    //                     let match_string2 = "pipewire";
    pub fn create_device_output_builder(preferred_name: String, fallback_name: String) -> OutputStreamBuilder {
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

    async fn play_test(sink: &Sink) {
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

    async fn play_media(sink:&Sink, id: String) {
        let path = Path::new(&id);
        if !path.exists() {
            return
        }

        let file = File::open(path).unwrap();
        sink.clear();
        sink.append(rodio::Decoder::try_from(file).unwrap());
        sink.play();
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
            let name = &self.name.clone();
            let sink = &self.sink;
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        PlayerCommand::Update(s) => {
                            let x = s.clone();
                            Player::play_media(sink, s.clone()).await;
                            let _ = evt_tx.send(PlayerEvent::Status(format!("Playing {}", x)));
                        }
                        PlayerCommand::PlayTest() => {
                            Player::play_test(sink).await;
                        }
                        PlayerCommand::PlayMedia(s) => {
                            Player::play_media(sink, s).await;
                        }
                        PlayerCommand::Play() => {
                            Player::play(sink);
                        }
                        PlayerCommand::Pause() => {
                            Player::pause(sink);
                        }
                        PlayerCommand::Stop() => {
                            let _ = evt_tx.send(PlayerEvent::Stopped);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(10)) => {

                    let _ = evt_tx.send(PlayerEvent::Status(format!("Current name: {}", name)));
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

        }

        let file = File::open(path).unwrap();
        self.sink.append(rodio::Decoder::try_from(file).unwrap());
        self.sink.sleep_until_end();
    }

     */

}
