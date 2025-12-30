// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs
// load multiple sources with rodio: https://stackoverflow.com/questions/75505017/how-can-i-make-rust-with-the-rodio-crate-load-multiple-sources-in-a-vec-so-i

use std::cmp::{max, min};
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
use crate::media_source::media_source_trait::{MediaSource, MediaSourceChapter, MediaSourceItem};

#[derive(Debug)]
pub enum PlayerCommand {

    Update(String),
    PlayTest(),
    PlayMedia(String),
    Pause(),
    Stop(),
    Play(),
    Next(),
    Previous(),
    SeekRelative(i64),
    SeekTo(Duration)
}

#[derive(Debug)]
pub enum PlayerEvent {
    Status(String, String),
    Position(String, Duration),
    Stopped,
}

pub struct Player {
    media_source: Arc<dyn MediaSource>,
    stream: OutputStream, // when removed, the samples do not play
    sink: Sink,
    item: Option<MediaSourceItem>,
}



impl Player {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new(media_source: Arc<dyn MediaSource>, device_name: String, fallback_device_name: String) -> Player {
        let builder = Self::create_device_output_builder(device_name, fallback_device_name);
        let stream = builder.open_stream_or_fallback().unwrap();
        let sink = Sink::connect_new(stream.mixer());
        Self { media_source, sink, stream, item: None }
    }

    fn back_delay(&self) -> Duration {
        Duration::from_secs(2)
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
        self.item = None;
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
        let self_item = self.item.clone();

        if let Some(i) = self_item && id == i.id {
            self.toggle();
            return Ok(());
        }


        self.item = self.media_source.find(&id).await;
        if self.item.is_none() {
            return Ok(());
        }
        let self_item = self.item.clone();
        let item = self_item.unwrap();
        let path = Path::new(item.location.as_str());
        let file = File::open(path)?;
        self.sink.clear();
        self.sink.append(rodio::Decoder::try_from(file).unwrap());
        self.sink.play();
        Ok(())
    }

    fn toggle(&self) {
        if self.sink.is_paused() {
            self.sink.play()
        } else {
            self.sink.pause()
        }
    }

    fn play(&self) {
        self.sink.play();
    }

    fn pause(&self) {
        self.sink.pause()
    }

    fn try_seek(&self, position: Duration) -> Result<(), SeekError> {
        self.sink.try_seek(position)
    }

    fn chapters(&self) -> Vec<MediaSourceChapter> {
        let self_item = self.item.clone();
        if self_item.is_none() {
            return vec![];
        }
        let current_item = self_item.unwrap();
        current_item.metadata.chapters
    }

    fn next_chapter(&self) -> Option<MediaSourceChapter> {
        let current_pos = self.sink.get_pos();
        let chapters = self.chapters();
        for chapter in chapters {
            if chapter.start > current_pos {
                return Some(chapter);
            }
        }
        None
    }

    fn current_chapter(&self) -> Option<MediaSourceChapter> {
        let current_pos = self.sink.get_pos();
        let chapters = self.chapters();
        if chapters.is_empty() {
            return None;
        }
        for chapter in chapters {
            if chapter.start <= current_pos && chapter.end() >= current_pos {
                return Some(chapter);
            }
        }
        None
    }

    fn previous_chapter(&self) -> Option<MediaSourceChapter> {
        let current_pos = self.sink.get_pos();
        let chapters = self.chapters();
        if chapters.is_empty() {
            return None;
        }
        let mut last_chapter: Option<MediaSourceChapter> = None;
        for chapter in chapters {
            if chapter.start <= current_pos && chapter.end() >= current_pos {
                break;
            }
            last_chapter = Some(chapter);
        }

        last_chapter
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
                            self.play();
                            self.update_playing_status(&evt_tx).await;
                        }
                        PlayerCommand::Pause() => {
                            self.pause();
                            self.update_playing_status(&evt_tx).await;
                        }
                        PlayerCommand::Stop() => {
                            let _ = evt_tx.send(PlayerEvent::Stopped);
                            break;
                        },
                        PlayerCommand::Next() => {
                            let next_chapter = self.next_chapter();
                            if(next_chapter.is_some()) {
                                let new_pos = next_chapter.unwrap().start;
                                self.try_seek(new_pos).unwrap();
                                self.update_position(&evt_tx, new_pos).await;
                            } else {
                                self.sink.skip_one()
                            }
                        }
                        PlayerCommand::Previous() => {
                            let current_pos = sink.get_pos();
                            if current_pos <= self.back_delay() {
                                // todo: skip to previous playlist item
                                // return
                            }

                            if let Some(current_chapter) = self.current_chapter()
                                && current_pos - current_chapter.start > self.back_delay() {
                                self.try_seek(current_chapter.start).unwrap();
                                self.update_position(&evt_tx, current_chapter.start).await;

                            } else if let Some(previous_chapter) = self.previous_chapter() {
                                self.try_seek(previous_chapter.start).unwrap();
                                self.update_position(&evt_tx, previous_chapter.start).await;

                            } else {
                                let zero = Duration::from_secs(0);
                                self.try_seek(zero).unwrap();
                                self.update_position(&evt_tx, zero).await;
                            }
                        }
                        PlayerCommand::SeekRelative(millis) => {
                            let new_pos = max(sink.get_pos().as_millis() as i64 + millis, 0) as u64;
                            self.try_seek(Duration::from_millis(new_pos));
                        }
                        PlayerCommand::SeekTo(_) => {}
                    }
                }

                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    self.update_position(&evt_tx, sink.get_pos()).await;
                }
            }
        }
    }

    async fn update_position(&self, evt_tx: &mpsc::UnboundedSender<PlayerEvent>, pos: Duration) {
        if let Some(item) = self.item.clone() {
            let _ = evt_tx.send(PlayerEvent::Position(item.id.to_string(), pos));
        }
    }

    async fn update_playing_status(&self, evt_tx: &UnboundedSender<PlayerEvent>) {
        let self_item_opt = self.item.clone();
        if self_item_opt.is_none() {
            return;
        }
        let self_item_opt = self.item.clone();
        if self_item_opt.is_none() {
            return;
        }
        let self_item = self_item_opt.unwrap();
        if self.sink.is_paused() {
            let _ = evt_tx.send(PlayerEvent::Status(self_item.id.to_string(), "paused".to_string()));
        } else {
            let _ = evt_tx.send(PlayerEvent::Status(self_item.id.to_string(), "playing".to_string()));
        }
    }
}
