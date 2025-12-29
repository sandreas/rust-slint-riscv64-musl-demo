use std::{fs, io};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use evdev::{Device, EventSummary, KeyCode};
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

#[derive(Debug,Clone)]
pub struct HeadsetDevice {
    pub path: String,
    pub name: String,
    pub unique_name: String,
}

impl Headset {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new(event_device: Device) -> Headset {
        Self {
            device: event_device
        }
    }


    pub fn list_input_devices() -> Result<Vec<HeadsetDevice>, io::Error> {
        let dir = "/dev/input";

        if !Path::exists(Path::new(dir)) {
            return Ok(vec![]);
        }

        let mut devices = Vec::<HeadsetDevice>::new();

        let mut device_list_debug = "".to_owned();


        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            let file_name_opt = path.file_name()
                .and_then(|n| n.to_str());
            // Only consider event* character devices
            if !file_name_opt
                .map_or(false, |n| n.starts_with("event"))
            {
                continue;
            }

            let file_name = file_name_opt.unwrap();

            let d = Device::open(&path)?;
            let hd = HeadsetDevice {
                path: file_name.to_owned(),
                name: d.name().as_deref().unwrap_or("").to_string(),
                unique_name: d.unique_name().as_deref().unwrap_or("").to_string(),
            };


            devices.push(hd.clone());

            device_list_debug.push_str(&hd.path);
            device_list_debug.push('|');
            device_list_debug.push_str(&hd.name);
            device_list_debug.push('|');
            device_list_debug.push_str(&hd.unique_name);
            device_list_debug.push('\n');

        }



        Ok(devices)
    }

    pub async fn run(
        &mut self,
        _/*evt_tx*/: mpsc::UnboundedSender<HeadsetEvent>,
    ) {

        loop {
            for event in self.device.fetch_events().unwrap() {
                // let _ = evt_tx.send(ev);
                match event.destructure() {
                    EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 1) => {
                        println!("PLAYPAUSE PRESSED: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 0) => {
                        println!("PLAYPAUSE RELEASED: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 1) => {
                        println!("VOLUME_UP PRESSED: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_VOLUMEUP, 0) => {
                        println!("VOLUME_UP RELEASED: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 1) => {
                        println!("VOLUME_DOWN PRESSED: {:?}", ev);
                    },
                    EventSummary::Key(ev, KeyCode::KEY_VOLUMEDOWN, 0) => {
                        println!("VOLUME_DOWN RELEASED: {:?}", ev);
                    },
                    _ => println!("got a different event: {:?}", event.destructure())
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