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
    event_device: String,
    device: Option<Device>,
}

#[derive(Debug,Clone)]
pub struct HeadsetDevice {
    pub path: String,
    pub name: String,
    pub unique_name: String,
}

impl Headset {
    // sink:Option<Sink>, stream: Option<OutputStream>
    pub fn new(event_device: String) -> Headset {
        Self {
            event_device,
            device: None
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
            }
                
            
            
            
            
            
        }
    }
}