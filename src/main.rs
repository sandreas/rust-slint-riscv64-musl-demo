use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::sync::mpsc;

mod player;
mod headset;
mod gpio_button_service;
mod file_media_source;
mod media_source_trait;
mod migrator;
mod entity;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    base_directory: String,
}

use crate::entity::{item, items_json_metadata, items_metadata, items_progress_history};
use crate::player::{Player, PlayerCommand, PlayerEvent};
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::iter;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use crate::file_media_source::FileMediaSource;
use crate::media_source_trait::{MediaSource, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaType};
use crate::migrator::Migrator;

slint::include_modules!();


const DB_URL: &str = "";



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
                println!("base directory does not exist: {:?}, current dir is: {:?}", base_path, cwd);
            }
            Err(_) => {
                println!("base directory does not exist: {}", args.base_directory);
            }
        }

        return Err(slint::PlatformError::Other(format!("Base directory does not exist: {}", args.base_directory)));
    }



    let db_path = format!("{}/{}", base_dir.clone().trim_end_matches("/"), String::from("player.db"));
    let first_run = !Path::new(&db_path).exists();
    let db_url = format!("sqlite://{}?mode=rwc", db_path);
    let connect_result = connect_db(&db_url, first_run).await;
    if connect_result.is_err() {
        return Err(slint::PlatformError::Other(format!("Could not find, create or migrate database: {}", connect_result.err().unwrap())));
    }

    let db = connect_result.unwrap();

    /*
    // this works
    let now = Utc::now();
    let picture_model = picture::ActiveModel::builder()
        .set_hash(16601657817183584017u64)
        .set_codec(ImageCodec::Jpeg)
        .set_date_modified(now);
    picture_model.save(&db).await.unwrap();
    */

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
    tokio::spawn(async move {
        let mut player = Player::new(Arc::new(file_source.clone()), "USB-C to 3.5mm Headphone Jack A".to_string(), "pipewire".to_string());
        player.run(player_cmd_rx, player_evt_tx).await;
    });

    tokio::spawn(async move {
        while let Some(event) = player_evt_rx.recv().await {
            println!("Received event: {:?}", event);
        }
    });

    // this part only works when USB-C is plugged in
    //     let (head_event_tx, mut head_event_rx) = mpsc::unbounded_channel::<HeadsetEvent>();
    //     tokio::spawn(async move {
    //         let device_path ="/dev/input/event13";
    //         let device = Device::open(Path::new(&device_path)).unwrap();
    //         let mut headset = Headset::new(device);
    //         headset.run(head_event_tx).await;
    //     });

    let slint_app_window = MainWindow::new()?;
    // slint_app_window.set_items(slint_items);

    // let slint_app_window_weak = slint_app_window.as_weak();


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
            tx.send(PlayerCommand::PlayMedia(file_name.to_string())).unwrap();
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
        let next_index = if tmp_next_index > 1000 { 1000 } else { tmp_next_index };
        let skip = if tmp_next_index > 1000 { 1 } else { 0 };
        let take = next_index - skip;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav
            .get_history()
            .iter()
            .skip(skip as usize)
            .take(take as usize)
            .chain(iter::once(history_item)).collect();
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
    slint_media_source.on_query({
        let inner = slint_media_source_ui.global::<SlintMediaSource>();
        inner.set_is_loading(true);
        inner.set_query_results(ModelRc::default());
        move |query| {
            source_cmd_tx.send(MediaSourceCommand::Filter(query.to_string())).unwrap();
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
                        inner.set_query_results(rust_items_to_slint_model(items));
                    }
                    MediaSourceEvent::FindResult(opt_item) => {
                        if let Some(item) = opt_item {
                            inner.set_query_results(
                                rust_items_to_slint_model(vec![item])
                            );
                        } else {
                            // clear results if nothing found
                            inner.set_query_results(slint::ModelRc::default());
                        }
                    }
                }
            } else {
                // UI was dropped; stop listening
                break;
            }
        }
    }).unwrap();


    slint_app_window.run()
}

fn load_preferences(_: SlintPreferences) {
    todo!()
}

fn brightness_percent_to_target_value(brightness_percent: f32) -> i32{
    (brightness_percent * 2500f32).round() as i32
}

fn update_brightness(brightness_target_value: i32) {
    let path = Path::new("/sys/class/pwm/pwmchip8/pwm2/duty_cycle");
    if path.exists() {
        let mut file = OpenOptions::new()
            .write(true) // <--------- this
            .open(path).ok().unwrap();
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

fn rust_items_to_slint_model(rust_items: Vec<MediaSourceItem>) -> ModelRc<SlintMediaSourceItem> {
    // Create VecModel directly
    let model = VecModel::<SlintMediaSourceItem>::from(
        rust_items
            .into_iter()
            .map(|rust_item| SlintMediaSourceItem {
                id: rust_item.id.clone().into(),
                media_type: convert_media_type_to_int(&rust_item.media_type),
                name: rust_item.title.clone().into(),
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


fn convert_int_to_media_type(media_type: i32) -> MediaType {
    match media_type {
        2 => MediaType::Audiobook,
        4 => MediaType::Music,
        _ => MediaType::Unspecified,
    }
}
