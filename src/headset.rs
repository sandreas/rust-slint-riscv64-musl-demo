use crate::player::player::{PlayerCommand, PlayerEvent, TriggerAction};
use evdev::{Device, EventSummary, KeyCode};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};
use debouncer::Debouncer;
use tokio::sync::mpsc;
use crate::debouncer::AsyncDebouncer;

pub struct Headset {
    event_device: String,
    device: Option<Device>,
    clicks: Arc<Mutex<i32>>,
    hold: bool,
}

impl Headset {
    pub fn new(event_device: String) -> Headset {
        Self {
            event_device,
            device: None,
            clicks: Arc::new(Mutex::new(0)),
            hold: false,
        }
    }

    pub fn set_clicks(&mut self, clicks: i32) {
        let mut clicks_guard = self.clicks.lock().unwrap();
        *clicks_guard = clicks;
        drop(clicks_guard);
    }

    pub fn increase_clicks(&mut self) {
        let mut clicks_guard = self.clicks.lock().unwrap();
        *clicks_guard = *clicks_guard + 1;
        drop(clicks_guard);
    }


    pub async fn run(
        &mut self,
        evt_tx: mpsc::UnboundedSender<PlayerEvent>,
    ) {

        let debouncer = AsyncDebouncer::new(Duration::from_millis(400));
        let run_debouncer = Arc::new(Mutex::new(false));
        let hold = Arc::new(Mutex::new(false));
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

                    let mut trigger_action = false;
                    let mut event_str = "";
                    match event.destructure() {
                        EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 1) => {
                            // let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Press, ev.timestamp()));
                            // let _ = evt_tx.send(PlayerEvent::ExternalTrigger(TriggerAction::Toggle));
                            // println!("PLAYPAUSE PRESSED: {:?}", ev);
                            event_str = "PLAYPAUSE (PRESS)  ";
                            let mut hold_guard = hold.lock().unwrap();
                            *hold_guard = true;
                            drop(hold_guard);

                            trigger_action = true
                        },
                        EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 0) => {
                            // let _ = player_button_cmd_tx.send(HandleButton(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                            // let _ = evt_tx.send(PlayerEvent::ExternalTrigger(ButtonKey::PlayPause, ButtonAction::Release, ev.timestamp()));
                            // println!("PLAYPAUSE RELEASED: {:?}", ev);
                            event_str = "PLAYPAUSE (RELEASE)";
                            let mut hold_guard = hold.lock().unwrap();
                            *hold_guard = false;
                            drop(hold_guard);

                            trigger_action = true;

                            let mut clicks_guard = self.clicks.lock().unwrap();
                            *clicks_guard = *clicks_guard + 1;
                            drop(clicks_guard);
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
                        _ => { /*println!("got a different event: {:?}", event.destructure()) */ }
                    }


                    if trigger_action {




                        // println!("==== event: {:?}", event.destructure());


                        let evt_tx_clone = evt_tx.clone();



                        /*
                        debouncer.trigger(self.clicks,
                                          hold, |clicks_arc, hold, reset_clicks| {
                                *clicks_arc.lock().unwrap() = 0;
                                println!("Reset clicks to 0!");
                        });
                        */

                        let mut hold_guard = hold.lock().unwrap();
                        let hold_value = *hold_guard;
                        drop(hold_guard);

                        let dt: DateTime<Utc> = event.timestamp().into();
                        let now: DateTime<Utc> = SystemTime::now().into();

                        // ISO 8601 with milliseconds
                        let iso = dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                        let now_format = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                        println!("event: {} | iso: {} | triggered: {}", event_str, iso,  now_format);

                        debouncer.trigger(
                            self.clicks.clone(),           // ✅ Pass Arc directly as first param
                            hold_value,                    // is_hold
                            |clicks_arc, hold| {      // ✅ Callback receives Arc
                                println!("Main: clicks={}, hold={}", *clicks_arc.lock().unwrap(), hold);

                                // ✅ Reset directly in callback (no final_cleanup needed)
                                *clicks_arc.lock().unwrap() = 0;
                                println!("Reset clicks to 0!");
                            },
                        );

                        /*
                        tokio::spawn(async move {
                            // debouncer.call(async move || {
                            thread::sleep(Duration::from_millis(400));


                            let trigger_action: Option<TriggerAction> = if hold {
                                None
                            } else {
                                match clicks {
                                    1 => Some(TriggerAction::Toggle),
                                    _ => None,
                                }
                            };

                            let _ = evt_tx_clone.send(PlayerEvent::ExternalTrigger(TriggerAction::Toggle));

                        });*/
                        // });
                    }
                }


            }
        }
    }
}