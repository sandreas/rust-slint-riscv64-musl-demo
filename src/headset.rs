use evdev::{Device, EventSummary, InputEvent, KeyCode};
use std::path::Path;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum HeadsetEvent {
    PlayPause(),
    VolumeUp(),
    VolumeDown(),
}

pub struct Headset {
    device: Device
}

impl Headset {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new(event_device: Device) -> Headset {
        Self {
            device: event_device
        }
    }

    pub async fn run(
        &mut self,
        evt_tx: mpsc::UnboundedSender<HeadsetEvent>,
    ) {

        loop {
            for event in self.device.fetch_events().unwrap() {
                // let _ = evt_tx.send(ev);

                match event.destructure(){
                    EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 1) => {
                        println!("Key 'playpause' was pressed, got event: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 1) => {
                        println!("Key 'vol up' was pressed, got event: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 1) => {
                        println!("Key 'vol down' was pressed, got event: {:?}", ev);
                    },
                    _ => println!("got a different event!")
                }
            }
            /*
            tokio::select! {

                _ = tokio::time::sleep(Duration::from_secs(0)) => {
                    // let _ = evt_tx.send(PlayerEvent::Status(format!("Current name: {}", name)));
                }
            }

             */
        }
    }
}