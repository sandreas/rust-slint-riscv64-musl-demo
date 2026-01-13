use clap::Parser;
use tokio::sync::mpsc;

mod gpio_button_service;
mod headset;
mod player;

mod entity;
mod migrator;

mod button_handler;
mod media_source;
pub mod serde_json_mods;
mod debouncer;
mod audio;
mod display;
mod time;
mod slint_helpers;

const MAGIC_HEADSET_REMOTE_DEBOUNCER_DELAY: u64 = 250;
const MAGIC_REPETITIVE_ACTION_DELAY: u64 = 850;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    base_directory: String,
}

use crate::debouncer::tokio_debouncer::{DebounceMode, Debouncer};
use crate::entity::{item, items_json_metadata, items_metadata, items_progress_history};
use crate::media_source::file_media_source::FileMediaSource;
use crate::media_source::media_source::{
    MediaSource, MediaSourceCommand, MediaSourceEvent
    ,
};
use crate::migrator::Migrator;
use crate::player::player::Player;
use crate::player::player_command::PlayerCommand;
use crate::player::player_event::PlayerEvent;
use crate::player::trigger_action::TriggerAction;
use crate::time::format_duration;
use chrono::{DateTime, Utc};
use evdev::{Device, EventSummary, KeyCode};
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use slint::{
    ComponentHandle, Model, ModelRc, SharedString, ToSharedString,
    VecModel,
};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use std::{iter, thread};
use tokio::select;
use tokio::task::JoinHandle;

slint::include_modules!();

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

    let now: DateTime<Utc> = SystemTime::now().into();


    slint_preferences.set_now(now.format("%H:%M:%S%.3fZ").to_shared_string());
    // load_preferences(&slint_preferences);

    // slint_preferences.set_brightness(100f32);

    let preferences_ui = slint_app_window.clone_strong();
    slint_preferences.on_sync(move || {
        slint_helpers::utils::sync_preferences(preferences_ui.global::<SlintPreferences>());
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
                        inner.set_filter_results(slint_helpers::utils::rust_items_to_slint_model(items, false));
                    }
                    MediaSourceEvent::FindResult(opt_item) => {
                        if let Some(item) = opt_item {
                            inner.set_find_results(slint_helpers::utils::rust_items_to_slint_model(vec![item], true));
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
                let inner = ui.global::<SlintAudioPlayer>();

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

    slint_app_window.run()
}








