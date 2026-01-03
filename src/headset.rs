use crate::button_handler::{ButtonAction, ButtonKey};
use crate::player::player::PlayerCommand::HandleButton;
use crate::player::player::PlayerCommand;
use evdev::{Device, EventSummary, KeyCode};
use std::path::Path;
use std::time::SystemTime;
use evdev_rs_tokio::enums::EV_KEY::KEY_PLAYPAUSE;
use evdev_rs_tokio::enums::EventCode::EV_KEY;
use evdev_rs_tokio::ReadFlag;
use tokio::fs::File;
use tokio::sync::mpsc;
use crate::SlintAudioPlayer;

pub struct Headset {
    event_device: String,
    device: Option<Device>,
}

impl Headset {
    pub fn new(event_device: String) -> Headset {
        Self {
            event_device,
            device: None,
        }
    }

    pub async fn run(
        &mut self,
        player: SlintAudioPlayer<'_>,
    ) {
        let file = File::open(self.event_device.clone()).await.unwrap();
        let mut d = evdev_rs_tokio::Device::new_from_file(file).unwrap();

        loop {

            let ev = d.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING).map(|val| val.1);
            match ev {
                Ok(ev) => {

                    match ev.event_code {
                        EV_KEY(key) => {
                            // value = 1 => keydown
                            if key == KEY_PLAYPAUSE && ev.value == 0 {
                                player.invoke_play();
                            }

                            println!("Event: {:?}, Value: {}", key, ev.value);
                        }
                        _ => {}
                    }

                    /*
                    println!("Event: time {}.{}, ++++++++++{}++++++++++ {} +++++++++++++++",
                                   ev.time.tv_sec,
                                   ev.time.tv_usec,
                                    ev.value,
                                   ev.event_type().map(|ev_type| format!("{}", ev_type)).unwrap_or("".to_owned()))
                    */
                },

                Err(e) => (),
            }
        }
    }

    pub async fn run_new(
        &mut self,
        player_button_cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
    ) {
        let file = File::open(self.event_device.clone()).await.unwrap();
        let mut d = evdev_rs_tokio::Device::new_from_file(file).unwrap();

        loop {

            let ev = d.next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING).map(|val| val.1);
            match ev {
                Ok(ev) => {

                    match ev.event_code {
                        EV_KEY(key) => {
                            // value = 1 => keydown
                            if key == KEY_PLAYPAUSE && ev.value == 0 {
                                let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Press, SystemTime::now()));
                            }

                            println!("Event: {:?}, Value: {}", key, ev.value);
                        }
                        _ => {}
                    }

                    /*
                    println!("Event: time {}.{}, ++++++++++{}++++++++++ {} +++++++++++++++",
                                   ev.time.tv_sec,
                                   ev.time.tv_usec,
                                    ev.value,
                                   ev.event_type().map(|ev_type| format!("{}", ev_type)).unwrap_or("".to_owned()))
                    */
                },

                Err(e) => (),
            }
        }
    }

        pub async fn run_old(
        &mut self,
        player_button_cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
    ) {


        loop {
            if self.device.is_none() {
                let device_path = Path::new(&self.event_device);
                if !Path::exists(device_path) {
                    continue;
                }

                let device_result = Device::open(device_path);
                if let Ok(device) = device_result {
                    self.device = Some(device);
                }
            }

            if let Some(device) = &mut self.device {
                for event in device.fetch_events().unwrap() {
                    // let _ = evt_tx.send(ev);
                    match event.destructure() {
                        EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 1) => {
                            let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Press, ev.timestamp()));
                            println!("PLAYPAUSE PRESSED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 0) => {
                            let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                            println!("PLAYPAUSE RELEASED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 1) => {
                            println!("VOLUME_UP PRESSED: {:?}", ev);
                            let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeUp, ButtonAction::Press, ev.timestamp()));
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 0) => {
                            let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeUp, ButtonAction::Release, ev.timestamp()));
                            println!("VOLUME_UP RELEASED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 1) => {
                            let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeDown, ButtonAction::Press, ev.timestamp()));
                            println!("VOLUME_DOWN PRESSED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 0) => {
                            let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeDown, ButtonAction::Release, ev.timestamp()));
                            println!("VOLUME_DOWN RELEASED: {:?}", ev);
                        },
                        _ => println!("got a different event: {:?}", event.destructure())
                    }
                }
            }
        }
    }
}