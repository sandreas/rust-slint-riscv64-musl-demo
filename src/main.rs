use clap::Parser;
use std::cmp::PartialEq;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::sync::mpsc;

mod gpio_button_service;
mod headset;
mod player;

mod entity;
mod migrator;

mod button_handler;
mod media_source;
pub mod serde_json_mods;

const MAGIC_HEADSET_REMOTE_DEBOUNCER_DELAY: u64 = 250;
const MAGIC_REPETITIVE_ACTION_DELAY: u64 = 850;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    base_directory: String,
}

use crate::entity::{item, items_json_metadata, items_metadata, items_progress_history};
use crate::media_source::file_media_source::FileMediaSource;
use crate::media_source::media_source::{
    MediaSource, MediaSourceCommand, MediaSourceEvent, MediaSourceItem,
    MediaType,
};
use crate::migrator::Migrator;
use chrono::{DateTime, Utc};
use cpal::traits::{DeviceTrait, HostTrait};
use evdev::{Device, EventSummary, KeyCode};
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use slint::{
    ComponentHandle, Model, ModelRc, Rgb8Pixel, SharedPixelBuffer, SharedString, ToSharedString,
    VecModel,
};
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use std::{iter, thread};
use tokio::select;

slint::include_modules!();

















use tokio::sync::Notify;
use tokio::time::Instant;


#[cfg(feature = "parking_lot")]
pub use parking_lot::{Mutex, MutexGuard};
#[cfg(not(feature = "parking_lot"))]
pub use std::sync::MutexGuard;
use tokio::task::JoinHandle;
use crate::media_source::media_source_picture::MediaSourcePicture;
use crate::player::player::Player;
use crate::player::player_command::PlayerCommand;
use crate::player::player_event::PlayerEvent;
use crate::player::trigger_action::TriggerAction;

#[cfg(not(feature = "parking_lot"))]
pub trait MutexExt<T> {
    /// Lock the mutex, panicking if poisoned.
    fn risky_lock(&self) -> MutexGuard<T>;
}
#[cfg(not(feature = "parking_lot"))]
impl<T> MutexExt<T> for Mutex<T> {
    fn risky_lock(&self) -> MutexGuard<T> {
        self.lock().expect("Mutex poisoned")
    }
}
#[cfg(feature = "parking_lot")]
pub trait MutexExt<T> {
    /// Lock the parking_lot mutex (never poisoned).
    fn risky_lock(&self) -> MutexGuard<T>;
}
#[cfg(feature = "parking_lot")]
impl<T> MutexExt<T> for Mutex<T> {
    fn risky_lock(&self) -> MutexGuard<T> {
        self.lock()
    }
}

#[derive(Debug)]
pub enum DebounceMode {
    Leading,
    Trailing,
}

struct DebouncerState {
    has_run: bool,
    last_run: Instant,
    triggered: bool,
}

struct DebouncerInner {
    mode: DebounceMode,
    notifier: Notify,
    cooldown: Duration,
    state: Mutex<DebouncerState>,
}

impl DebouncerInner {
    fn finalize(&self, pending: bool) {
        let mut state = self.state.risky_lock();
        if state.triggered {
            state.has_run = true;
            state.triggered = pending;
            state.last_run = tokio::time::Instant::now();
            self.notifier.notify_one();
        }
    }
}

pub struct DebouncerGuard<'a> {
    inner: Arc<DebouncerInner>,
    completed: bool,
    _not_send: PhantomData<*const ()>,
    _not_static: PhantomData<&'a ()>,
}

impl<'a> DebouncerGuard<'a> {
    fn new(inner: Arc<DebouncerInner>) -> Self {
        Self {
            inner,
            completed: false,
            _not_send: PhantomData,
            _not_static: PhantomData,
        }
    }
}

impl<'a> Drop for DebouncerGuard<'a> {
    fn drop(&mut self) {
        if !self.completed {
            let inner = self.inner.clone();
            self.completed = true;
            inner.finalize(false);
        }
    }
}

#[derive(Clone)]
pub struct Debouncer {
    inner: Arc<DebouncerInner>,
}

impl Debouncer {
    pub fn new(cooldown: Duration, mode: DebounceMode) -> Self {
        let inner = Arc::new(DebouncerInner {
            notifier: Notify::new(),
            cooldown,
            state: Mutex::new(DebouncerState {
                has_run: if matches!(mode, DebounceMode::Leading) {
                    false
                } else {
                    true
                },
                last_run: tokio::time::Instant::now(),
                triggered: false,
            }),
            mode,
        });
        Self { inner }
    }

    pub async fn is_triggered(&self) -> bool {
        let state = self.inner.state.risky_lock();
        state.triggered
    }


    pub fn trigger(&self) {
        {
            let mut guard = self.inner.state.risky_lock();
            if matches!(self.inner.mode, DebounceMode::Trailing) {
                guard.last_run = tokio::time::Instant::now();
            }
            if guard.triggered {
                // Already pending, just update the value
                return;
            }
            guard.triggered = true;
        } // guard dropped here
        self.inner.notifier.notify_one();
    }

    pub async fn ready<'a>(&self) -> DebouncerGuard<'a> {
        loop {
            // Phase 1: inspect state (no awaits)
            let action = {
                let state = self.inner.state.risky_lock();

                if !state.triggered {
                    None
                } else {
                    let now = tokio::time::Instant::now();
                    let next_allowed = state.last_run + self.inner.cooldown;

                    match self.inner.mode {
                        DebounceMode::Leading => {
                            if !state.has_run || now >= next_allowed {
                                Some(None)
                            } else {
                                Some(Some(next_allowed))
                            }
                        }
                        DebounceMode::Trailing => {
                            if now >= next_allowed {
                                Some(None)
                            } else {
                                Some(Some(next_allowed))
                            }
                        }
                    }
                }
            }; // ✅ MutexGuard fully dropped here

            // Phase 2: await
            match action {
                None => {
                    self.inner.notifier.notified().await;
                }
                Some(Some(instant)) => {
                    tokio::time::sleep_until(instant).await;
                }
                Some(None) => {
                    break;
                }
            }
        }

        DebouncerGuard::new(self.inner.clone())
    }

}























enum HeadsetButton {
    PlayPause,
    VolumeUp,
    VolumeDown,
}
enum HeadsetEvent {
    Press(HeadsetButton),
    Release(HeadsetButton),
}

async fn connect_db(db_url: &str, first_run: bool) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(db_url).await?;
    // todo: dirty hack to prevent startup failure if db exists
    // this has to be solved with migrations or at least better than this
    if first_run {
        db.get_schema_builder()
            .register(item::Entity)
            .register(items_metadata::Entity)
            .register(items_json_metadata::Entity)
            .register(items_progress_history::Entity)
            .apply(&db)
            .await?;
    }
    Migrator::up(&db, None).await?;
    Ok(db)
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    /*
        for i in 1..5 {
            let hosts_ids = cpal::available_hosts();

            for host_id in hosts_ids {
                println!("==== HOST {:?}", host_id);

                let host_result = cpal::host_from_id(host_id);

                if let Ok(host) = host_result {
                    let devices_result = host.devices();
                    if let Ok(devices) = devices_result {

                        for device in devices {
                            let devicename_result = device.name();
                            if let Ok(devicename) = devicename_result {
                                println!("   ----{:?}", devicename);
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        return Ok(());
    */










    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();


    let args = Args::parse();
    let base_dir = args.base_directory.clone();
    println!("base directory is: {}", base_dir.clone());
    let base_path = Path::new(&base_dir);
    if !Path::exists(base_path) {
        match std::env::current_dir() {
            Ok(cwd) => {
                println!(
                    "base directory does not exist: {:?}, current dir is: {:?}",
                    base_path, cwd
                );
            }
            Err(_) => {
                println!("base directory does not exist: {}", args.base_directory);
            }
        }

        return Err(slint::PlatformError::Other(format!(
            "Base directory does not exist: {}",
            args.base_directory
        )));
    }

    let db_path = format!(
        "{}/{}",
        base_dir.clone().trim_end_matches("/"),
        String::from("player.db")
    );
    let first_run = !Path::new(&db_path).exists();
    let db_url = format!("sqlite://{}?mode=rwc", db_path);
    let connect_result = connect_db(&db_url, first_run).await;
    if connect_result.is_err() {
        return Err(slint::PlatformError::Other(format!(
            "Could not find, create or migrate database: {}",
            connect_result.err().unwrap()
        )));
    }

    let db = connect_result.unwrap();


    // let settings_manager = SettingsManager::new(db.clone());

    let display_brightness = 1000; // settings_manager.get("display.brightness", 1000).await;
    let dark_mode = true; // settings_manager.get("appearance.dark_mode", true);

    let file_source = FileMediaSource::new(db.clone(), args.base_directory);
    let (source_cmd_tx, source_cmd_rx) = mpsc::unbounded_channel::<MediaSourceCommand>();
    let (source_evt_tx, source_evt_rx) = mpsc::unbounded_channel::<MediaSourceEvent>();
    file_source.scan_media().await;

    tokio::spawn(file_source.clone().run(source_cmd_rx, source_evt_tx));

    let (player_cmd_tx, player_cmd_rx) = mpsc::unbounded_channel::<PlayerCommand>();
    let (player_evt_tx, mut player_evt_rx) = mpsc::unbounded_channel::<PlayerEvent>();

    let player_evt_tx_clone = player_evt_tx.clone();
    let player_evt_tx_clone2 = player_evt_tx.clone();


    let mut player = Player::new(
        Arc::new(file_source.clone()),
        "USB-C to 3.5mm Headphone Jack A".to_string(),
        "pipewire".to_string(),
    );

    tokio::spawn(async move {
        player.run(player_cmd_rx, player_evt_tx.clone()).await;
    });



    let btn_click_count = Arc::new(Mutex::new(0));
    let btn_click_count_clone = btn_click_count.clone();

    let btn_is_down = Arc::new(Mutex::new(false));
    let btn_is_down_clone = btn_is_down.clone();

    let btn_stop_ongoing = Arc::new(Mutex::new(false));
    let btn_stop_ongoing_clone = btn_stop_ongoing.clone();
    let btn_stop_ongoing_clone2 = btn_stop_ongoing.clone();

    // let btn_ongoing = Arc::new(Mutex::new(false));
    // let btn_ongoing_clone = btn_ongoing.clone();


    let debouncer = Debouncer::new(Duration::from_millis(MAGIC_HEADSET_REMOTE_DEBOUNCER_DELAY), DebounceMode::Trailing);
    let debouncer_clone = debouncer.clone();

    let handle = thread::spawn(move || {
        loop {
            let device_paths = vec!["/dev/input/event1", "/dev/input/event13"];

            let mut device_opt: Option<Device> = None;
            for path_str in device_paths {
                let path = Path::new(path_str);
                if !Path::exists(path) {
                    continue;
                }
                let device_result = Device::open(path_str);
                if device_result.is_err() {
                    continue;
                }

                let d = device_result.unwrap();
                if d.name().is_some() && d.name().unwrap().contains("Apple") {
                    device_opt = Some(d);
                }

            }


            if device_opt.is_none() {
                thread::sleep(Duration::from_millis(5000));
                continue;
            }

            let mut device = device_opt.unwrap();

            for event in device.fetch_events().unwrap() {
                match event.destructure() {
                    EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 1) => {
                        let dt: DateTime<Utc> = event.timestamp().into();
                        let now: DateTime<Utc> = SystemTime::now().into();
                        let iso = dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                        let now_format = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                        println!(
                            "event: PRESS | trigger_time: {} | process_time: {}",
                            iso, now_format
                        );

                        let mut hold_guard = btn_is_down.lock().unwrap();
                        *hold_guard = true;
                        drop(hold_guard);

                        // let mut ongoing_guard = btn_ongoing.lock().unwrap();
                        // *ongoing_guard = true;
                        // drop(ongoing_guard);

                        debouncer_clone.trigger();
                        // println!("debouncer.trigger()");
                    }
                    EventSummary::Key(ev, KeyCode::KEY_PLAYPAUSE, 0) => {
                        let dt: DateTime<Utc> = event.timestamp().into();
                        let now: DateTime<Utc> = SystemTime::now().into();
                        let iso = dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                        let now_format = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                        println!(
                            "event: RELEASE | trigger_time: {} | process_time: {}",
                            iso, now_format
                        );

                        // we ignore releasing after a hold by setting hold = false when the debouncer triggers
                        let mut hold_guard = btn_is_down.lock().unwrap();
                        if *hold_guard {
                            *hold_guard = false;

                            let mut clicks_guard = btn_click_count.lock().unwrap();
                            *clicks_guard = *clicks_guard + 1;
                            drop(clicks_guard);
                        }

                        drop(hold_guard);
                        debouncer_clone.trigger();
                        // println!("debouncer.trigger()");
                    }
                    _ => { /*println!("got a different event: {:?}", event.destructure())*/  }
                }
            }
        }
    });

/*
    let mut ongoing_handle_store: Option<JoinHandle<()>> = None;
    let file_exists = Path::new("/tmp/test").exists();
    loop {
        select! {
                _ = debouncer.ready() => {
                    if !file_exists {
                        let handle = tokio::spawn(async move {

                        });
                        ongoing_handle_store = Some(handle);
                    } else {
                        if let Some(handle) = ongoing_handle_store.take() {
                            handle.abort();
                        }
                    }

                }
        }
    }
*/

    tokio::spawn(async move {
        let mut ongoing_player_operation: Option<JoinHandle<()>> = None;
        loop {

            select! {
                _ = debouncer.ready() => {




                    let mut clicks_guard = btn_click_count_clone.lock().unwrap();
                    let mut hold_guard = btn_is_down_clone.lock().unwrap();

                    // println!("debouncer ready | btn_repeat_count: {}, btn_state: {:?}", *clicks_guard, *hold_guard);


                    if *clicks_guard > 0 || *hold_guard {
                        let trigger_action_opt: Option<TriggerAction> = if *hold_guard {
                            match *clicks_guard {
                                0 => Some(TriggerAction::StepBack),
                                1 => Some(TriggerAction::StepForward),
                                _ => None
                            }
                        } else {
                            match *clicks_guard {
                                1 => Some(TriggerAction::Toggle),
                                2 => Some(TriggerAction::Next),
                                3 => Some(TriggerAction::Previous),
                                _ => None
                            }
                        };


                        // idea: If hold, periodically send events in an extra thread
                        // until the next event comes in
                        if trigger_action_opt.is_some() {
                            let loop_event = if *hold_guard {true} else {false};
                            let tx = player_evt_tx_clone2.clone();

                            if loop_event {
                               let handle = tokio::spawn(async move {
                                    loop {
                                        let _ = tx.send(PlayerEvent::ExternalTrigger(trigger_action_opt.unwrap()));
                                        tokio::time::sleep(Duration::from_millis(MAGIC_REPETITIVE_ACTION_DELAY)).await;
                                    }
                                });
                                ongoing_player_operation = Some(handle);
                            } else {
                                let _ = tx.send(PlayerEvent::ExternalTrigger(trigger_action_opt.unwrap()));
                            }

                        }

                        println!("debouncer exec  | btn_repeat_count: {}, btn_state: {:?}", *clicks_guard, *hold_guard);
                    } else {
                        if let Some(handle) = ongoing_player_operation.take() {
                            handle.abort();
                        }

                        let _ = player_evt_tx_clone.send(PlayerEvent::ExternalTrigger(TriggerAction::StopOngoing));
                    }

                    *clicks_guard = 0;
                    drop(clicks_guard);


                    // reset hold and clicks
                    *hold_guard = false;
                    drop(hold_guard);
                }
        }
        }
    });


    let slint_app_window = MainWindow::new()?;

    // slint_app_window.set_items(slint_items);


    // let slint_app_window_weak = slint_app_window.as_weak();
    // let slint_callbacks = slint_app_window.global::<SlintCallbacks>();
    // slint_callbacks.on_format_duration(|slint_duration: i64| {
    //     let duration = Duration::from_millis(slint_duration as u64);
    //     format_duration(duration)
    // });


    let slint_audio_player = slint_app_window.global::<SlintAudioPlayer>();
    slint_audio_player.on_play_test({
        let tx = player_cmd_tx.clone();
        move || {
            tx.send(PlayerCommand::PlayTest()).unwrap();
        }
    });

    slint_audio_player.on_play_media({
        let tx = player_cmd_tx.clone();
        move |file_name: SharedString| {
            tx.send(PlayerCommand::PlayMedia(file_name.to_string()))
                .unwrap();
        }
    });

    slint_audio_player.on_play({
        let tx = player_cmd_tx.clone();
        move || {
            tx.send(PlayerCommand::Play()).unwrap();
        }
    });

    slint_audio_player.on_pause({
        let tx = player_cmd_tx.clone();
        move || {
            tx.send(PlayerCommand::Pause()).unwrap();
        }
    });

    slint_audio_player.on_next({
        let tx = player_cmd_tx.clone();
        move || {
            tx.send(PlayerCommand::Next()).unwrap();
        }
    });

    slint_audio_player.on_previous({
        let tx = player_cmd_tx.clone();
        move || {
            tx.send(PlayerCommand::Previous()).unwrap();
        }
    });

    slint_audio_player.on_seek_relative({
        let tx = player_cmd_tx.clone();
        move |millis_i64| {
            tx.send(PlayerCommand::SeekRelative(millis_i64)).unwrap();
        }
    });

    slint_audio_player.on_seek_to({
        let tx = player_cmd_tx.clone();
        move |millis_i64: i64| {
            tx.send(PlayerCommand::SeekTo(Duration::from_millis(
                millis_i64 as u64,
            )))
            .unwrap();
        }
    });

    let slint_preferences = slint_app_window.global::<SlintPreferences>();
    // load_preferences(&slint_preferences);

    // slint_preferences.set_brightness(100f32);

    let preferences_ui = slint_app_window.clone_strong();
    slint_preferences.on_sync(move || {
        sync_preferences(preferences_ui.global::<SlintPreferences>());
    });

    let navigation = slint_app_window.global::<SlintNavigation>();
    let goto_ui = slint_app_window.clone_strong();
    navigation.on_goto(move |value| {
        let nav = goto_ui.global::<SlintNavigation>();
        nav.set_route(value);
        let history_item = nav.get_route();
        // inner_ui.global::<SlintNavigation>().
        // inner_ui.global::<SlintNavigation>().set_history()

        let tmp_next_index = nav.get_history_index() + 1;
        let next_index = if tmp_next_index > 1000 {
            1000
        } else {
            tmp_next_index
        };
        let skip = if tmp_next_index > 1000 { 1 } else { 0 };
        let take = next_index - skip;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav
            .get_history()
            .iter()
            .skip(skip as usize)
            .take(take as usize)
            .chain(iter::once(history_item))
            .collect();
        let history = VecModel::from(vec_of_history);
        nav.set_history(ModelRc::new(history));
        nav.set_history_index(next_index);
    });

    let back_ui = slint_app_window.clone_strong();
    navigation.on_back(move || {
        let nav = back_ui.global::<SlintNavigation>();
        let current_index = nav.get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav.get_history().iter().collect();
        if current_index == 0 || vec_of_history.is_empty() {
            return;
        }
        nav.set_route(vec_of_history[vec_index - 1].clone());
        nav.set_history_index(current_index - 1);
    });

    let forward_ui = slint_app_window.clone_strong();
    navigation.on_forward(move || {
        let nav = forward_ui.global::<SlintNavigation>();
        let current_index = nav.get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav.get_history().iter().collect();
        if vec_of_history.len() < vec_index + 2 {
            return;
        }
        nav.set_route(vec_of_history[vec_index + 1].clone());
        nav.set_history_index(current_index + 1);
    });

    let slint_media_source = slint_app_window.global::<SlintMediaSource>();
    let slint_media_source_ui = slint_app_window.clone_strong();
    let filter_tx = source_cmd_tx.clone();
    slint_media_source.on_filter({
        let inner = slint_media_source_ui.global::<SlintMediaSource>();
        inner.set_is_loading(true);
        inner.set_filter_results(ModelRc::default());
        move |query| {
            filter_tx
                .send(MediaSourceCommand::Filter(query.to_string()))
                .unwrap();
        }
    });

    let slint_media_source_find_ui = slint_app_window.clone_strong();
    let find_tx = source_cmd_tx.clone();
    slint_media_source.on_find({
        let inner = slint_media_source_find_ui.global::<SlintMediaSource>();
        inner.set_is_loading(true);
        inner.set_find_results(ModelRc::default());
        move |id| {
            find_tx
                .send(MediaSourceCommand::Find(id.to_string()))
                .unwrap();
        }
    });

    let ui_handle = slint_media_source_ui.as_weak();
    slint::spawn_local(async move {
        // now owned in this async block
        let mut source_evt_rx = source_evt_rx;
        while let Some(event) = source_evt_rx.recv().await {
            if let Some(ui) = ui_handle.upgrade() {
                let inner = ui.global::<SlintMediaSource>();

                match event {
                    MediaSourceEvent::FilterResults(items) => {
                        inner.set_filter_results(rust_items_to_slint_model(items, false));
                    }
                    MediaSourceEvent::FindResult(opt_item) => {
                        if let Some(item) = opt_item {
                            inner.set_find_results(rust_items_to_slint_model(vec![item], true));
                        } else {
                            // clear results if nothing found
                            inner.set_find_results(slint::ModelRc::default());
                        }
                    }
                }
            } else {
                // UI was dropped; stop listening
                break;
            }
        }
    })
    .unwrap();

    let ui_handle_player = slint_media_source_ui.as_weak();

    slint::spawn_local(async move {

        // maybe the debouncer can be integrated here, if it does not work elsewhere


        while let Some(event) = player_evt_rx.recv().await {
            if let Some(ui) = ui_handle_player.upgrade() {
                let mut inner = ui.global::<SlintAudioPlayer>();

                match event {
                    PlayerEvent::Status(item_id, status) => {
                        inner.set_current_item_id(item_id.to_shared_string());
                        inner.set_status(status.to_shared_string());
                    }

                    PlayerEvent::Stopped => {}

                    PlayerEvent::Position(item_id, position) => {

                        inner.set_current_item_id(item_id.to_shared_string());
                        inner.set_position_formatted(format_duration(position).to_shared_string());
                    }
                    PlayerEvent::ExternalTrigger(trigger_action) => {
                        // println!("trigger action: {:?}", trigger_action);

                        match trigger_action {
                            TriggerAction::Toggle => if inner.get_status().to_string() == "playing" {
                                inner.invoke_pause();
                            } else {
                                inner.invoke_play();
                            }
                            TriggerAction::Next => {inner.invoke_next();}
                            TriggerAction::Previous => {inner.invoke_previous();}
                            TriggerAction::StepBack => {inner.invoke_seek_relative(-15000);}
                            TriggerAction::StepForward => {inner.invoke_seek_relative(15000);}
                            TriggerAction::StopOngoing => if inner.get_status().to_string() == "playing" {
                                inner.invoke_play();
                            } else {
                                inner.invoke_pause();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            } else {
                // UI was dropped; stop listening
                break;
            }
        }
    })
    .unwrap();

    slint_app_window.run()
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    let secs = millis / 1000;
    let h = secs / (60 * 60);
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:0>2}:{:0>2}:{:0>2}", h, m, s)
}

fn load_preferences(_: SlintPreferences) {
    todo!()
}

fn brightness_percent_to_target_value(brightness_percent: f32) -> i32 {
    (brightness_percent * 2500f32).round() as i32
}

fn update_brightness(brightness_target_value: i32) {
    let path = Path::new("/sys/class/pwm/pwmchip8/pwm2/duty_cycle");
    if path.exists() {
        let mut file = OpenOptions::new()
            .write(true) // <--------- this
            .open(path)
            .ok()
            .unwrap();
        let _ = write!(file, "{}", brightness_target_value);
    }
}

fn sync_preferences(pref: SlintPreferences) {
    let new_brightness = pref.get_brightness();
    let brightness_target_value = brightness_percent_to_target_value(new_brightness);
    update_brightness(brightness_target_value);

    println!("brightness: {}", brightness_target_value);

    // dark / light
    println!("color-scheme: {}", pref.get_color_scheme());
}

fn option_to_slint_string(option: &Option<String>) -> SharedString {
    if option.is_some() {
        option.as_ref().unwrap().to_shared_string()
    } else {
        SharedString::from("")
    }
}

fn option_to_slint_cover(option: &Option<MediaSourcePicture>) -> (SharedString, SharedString) {
    if option.is_some() {
        let media_source_picture = option.as_ref().unwrap();
        (
            media_source_picture
                .pic_full_path(String::from("jpg"))
                .to_shared_string(),
            media_source_picture
                .tb_full_path(String::from("jpg"))
                .to_shared_string(),
        )
    } else {
        (SharedString::from(""), SharedString::from(""))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadCoverResult {
    Image,
    Placeholder,
    None,
}

fn load_cover_with_fallback(
    cover_path: &str,
    media_type: &MediaType,
) -> (slint::Image, LoadCoverResult) {
    let cover_result = slint::Image::load_from_path(Path::new(cover_path));

    if let Ok(cover) = cover_result {
        return (cover, LoadCoverResult::Image);
    }

    // todo: implement fallback image
    let fallback_image_result = match media_type {
        MediaType::Audiobook => slint::Image::load_from_svg_data(include_bytes!(
            "../ui/images/icons/home/audiobooks.png"
        )),
        _ => slint::Image::load_from_svg_data(include_bytes!("../ui/images/icons/home/music.png")),
    };
    if let Ok(fallback_image) = fallback_image_result {
        return (fallback_image, LoadCoverResult::Placeholder);
    }
    empty_cover_result()
}

fn empty_cover_result() -> (slint::Image, LoadCoverResult) {
    (
        slint::Image::from_rgb8(SharedPixelBuffer::<Rgb8Pixel>::new(1, 1)),
        LoadCoverResult::None,
    )
}

fn rust_items_to_slint_model(
    rust_items: Vec<MediaSourceItem>,
    details: bool,
) -> ModelRc<SlintMediaSourceItem> {
    // Create VecModel directly
    let model = VecModel::<SlintMediaSourceItem>::from(
        rust_items
            .into_iter()
            .map(|rust_item| {
                let (cover_path, thumbnail_path) = option_to_slint_cover(&rust_item.metadata.cover);

                let (thumbnail, thumbnail_type) =
                    load_cover_with_fallback(&thumbnail_path, &rust_item.media_type);

                let (cover, cover_type) = if details {
                    load_cover_with_fallback(&cover_path, &rust_item.media_type)
                } else {
                    empty_cover_result()
                };

                let mut slint_chapters_vec = VecModel::default();
                for chapter in &rust_item.metadata.chapters {
                    let start: i64 = chapter
                        .start
                        .as_millis()
                        .try_into()
                        .expect("Duration too long for u64");
                    let duration: i64 = chapter
                        .duration
                        .as_millis()
                        .try_into()
                        .expect("Duration too long for u64");

                    let slint_chapter = SlintMediaSourceChapter {
                        name: chapter.name.to_shared_string(),
                        start,
                        duration,
                    };

                    slint_chapters_vec.push(slint_chapter);
                }

                let chapters_model = ModelRc::new(slint_chapters_vec);

                SlintMediaSourceItem {
                    id: rust_item.id.clone().into(),
                    media_type: convert_media_type_to_int(&rust_item.media_type),
                    name: rust_item.title.clone().into(),
                    genre: option_to_slint_string(&rust_item.metadata.genre),
                    artist: option_to_slint_string(&rust_item.metadata.artist),
                    album: option_to_slint_string(&rust_item.metadata.album),
                    composer: option_to_slint_string(&rust_item.metadata.composer),
                    series: option_to_slint_string(&rust_item.metadata.series),
                    part: option_to_slint_string(&rust_item.metadata.part),
                    has_cover: cover_type != LoadCoverResult::None,
                    cover,
                    has_thumbnail: thumbnail_type != LoadCoverResult::None,
                    thumbnail,
                    chapters: chapters_model,
                }
            })
            .collect::<Vec<_>>(),
    );

    // Explicitly wrap in ModelRc if needed (usually not)
    ModelRc::from(Rc::new(model))
}

fn convert_media_type_to_int(media_type: &MediaType) -> i32 {
    match media_type {
        MediaType::Unspecified => 0,
        MediaType::Audiobook => 2,
        MediaType::Music => 4,
    }
}
