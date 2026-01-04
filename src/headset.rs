use crate::button_handler::{ButtonAction, ButtonKey};
use crate::player::player::{PlayerCommand, PlayerEvent};
use evdev::{Device, EventSummary, KeyCode};
use std::path::Path;
use tokio::sync::mpsc;

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

    pub fn run(
        &mut self,
        evt_tx: mpsc::UnboundedSender<PlayerEvent>,
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
                            // let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Press, ev.timestamp()));
                            let _ = evt_tx.send(PlayerEvent::HandleButton(ButtonKey::PlayPause, ButtonAction::Press, ev.timestamp()));
                            println!("PLAYPAUSE PRESSED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 0) => {
                            // let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                            let _ = evt_tx.send(PlayerEvent::HandleButton(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                            println!("PLAYPAUSE RELEASED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 1) => {
                            println!("VOLUME_UP PRESSED: {:?}", ev);
                            //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeUp, ButtonAction::Press, ev.timestamp()));
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 0) => {
                            //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeUp, ButtonAction::Release, ev.timestamp()));
                            println!("VOLUME_UP RELEASED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 1) => {
                            //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeDown, ButtonAction::Press, ev.timestamp()));
                            println!("VOLUME_DOWN PRESSED: {:?}", ev);
                        },
                        EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 0) => {
                            //let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::VolumeDown, ButtonAction::Release, ev.timestamp()));
                            println!("VOLUME_DOWN RELEASED: {:?}", ev);
                        },
                        _ => println!("got a different event: {:?}", event.destructure())
                    }
                }
            }
        }
    }
}