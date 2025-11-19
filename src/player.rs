// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs

use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStreamBuilder, Sink};
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
}

impl Player {
    pub async fn run(
        mut self,
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
        if(p0 != "preftest") {
            return;
        }

        let host = cpal::default_host();
        let mut device = None;
        for _ in 1..3 {
            device = Some(host.output_devices().unwrap()
                .find(|d| {
                    if let Ok(name) = d.name() {
                        // sysdefault:CARD=A
                        let match_string = "USB-C to 3.5mm Headphone Jack A";
                        // ALSA device names may include "hw:2,0" or "hw-2-0"
                        // You might need to tune this filter depending on your device naming
                        println!("device: {} contains {}: {}", name, match_string, name.contains(match_string));
                        name.contains(match_string)
                    } else {
                        false
                    }
                })).unwrap();


            if device.is_some() {
                break;
            }
        }

        if !device.is_some() {
            device = Some(host.default_output_device().unwrap());
        }



        let selected_device = device.unwrap();
        /*
        let default_config = selected_device.default_output_config().ok().unwrap();
        let sample_rate = default_config.sample_rate().0;
        let channel_count = default_config.channels();
        let sample_format = default_config.sample_format();
        */

        let builder_result = OutputStreamBuilder::from_device(selected_device);
        let builder = builder_result.unwrap();

        // let builder = OutputStreamBuilder::from_default_device().unwrap();


        let stream = builder.open_stream_or_fallback().unwrap();


        let sink = Sink::connect_new(stream.mixer());
        /*
        let stream_handle = OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(stream_handle.mixer());

        */



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
        sink.append(rodio::Decoder::try_from(file).unwrap());
        sink.sleep_until_end();
    }
}

/*

impl Player {
    pub fn new() -> Self {
        Player { name: "init".to_string() }
    }

    pub async fn play(&mut self, id: String) {
        // Async play logic (load, decode, output audio)
        println!("Playing file: {}", id);
        if(id != "") {
            /*
            let open_result = awedio::sounds::open_file(id);
            if(open_result.is_ok()) {
                let (sound, notifier) = open_result.ok().unwrap()
                    .pausable()
                    .with_adjustable_volume()
                    .with_adjustable_speed()
                    .with_completion_notifier();
                let sound = Box::new(sound);
                // sound.set_paused(true);
                // let sound_weak = sound.as_weak();

                self.manager.play(sound);
                let _ = notifier.recv();
            }

             */
        }

    }

    pub async fn pause(&mut self) {
        // self.manager.

    }

    // Implement other async controls: next, previous, fast_forward, rewind
}

*/

/*
Ideas:
- PlayerState => Playing, Paused, Buffering, etc.
- Events => TrackStarted, TrackEnded, PositionChanged, etc.
- Metadata retrieval should not be part of the player
- MediaItem => Reference to the Playable Tracks
  - ItemMetadata: Metadata specific to the item (AlbumArtist, etc.)
  - Tracks: List of tracks contained by this media item (e.g. when an audio book has multiple files)
    - TrackMetadata: Metadata specific to the track (title, chapters, x of y, etc.)

 */


/*
    let Some(file_path) = args() else {
        eprintln!("usage: FILE_PATH");
        std::process::exit(2);
    };

    let (mut manager, _backend) = awedio::start()?;
    let (sound, notifier) = awedio::sounds::open_file(file_path)?.with_completion_notifier();

    manager.play(Box::new(sound));
    let _ = notifier.recv();

    Ok(())
 */
