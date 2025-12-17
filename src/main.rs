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

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    base_directory: String,
}

use crate::player::{Player, PlayerCommand, PlayerEvent};
use slint::{ComponentHandle, Model, ModelRc, SharedString, SharedVector, VecModel};
use std::iter;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use evdev::Device;
use sea_orm::{Database, DatabaseConnection, DbConn, DbErr};
use crate::file_media_source::FileMediaSource;
use crate::headset::{Headset, HeadsetEvent};
use crate::media_source_trait::{MediaSource, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaType};

slint::include_modules!();


const DB_URL: &str = "";



async fn connect_db(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(db_url).await?;

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



    let db_url = format!("sqlite://{}/player.db?mode=rwc", base_dir.clone().trim_end_matches("/"));

    let db_result = connect_db(&db_url).await;
        if(db_result.is_err()) {
            return Err(slint::PlatformError::Other("Could not open database".to_string()));
        }

        let db = db_result.unwrap();
    /*
        // Connecting SQLite
        // Setup database schema
        setup_schema(&db).await?;
    */



    let file_source = FileMediaSource::new(args.base_directory);
    let (source_cmd_tx, source_cmd_rx) = mpsc::unbounded_channel::<MediaSourceCommand>();
    let (source_evt_tx, source_evt_rx) = mpsc::unbounded_channel::<MediaSourceEvent>();

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
    let preferences_ui = slint_app_window.clone_strong();
    slint_preferences.on_sync(move || {
        let pref = preferences_ui.global::<SlintPreferences>();
        
        let brightness = pref.get_brightness();
        let brightness_target_value = (brightness * 2500f32).round() as i32;
        let path = Path::new("/sys/class/pwm/pwmchip8/pwm2/duty_cycle");
        if path.exists() {

            let mut file = OpenOptions::new()
                .write(true) // <--------- this
                .open(path).ok().unwrap();
            let _ = write!(file, "{}", brightness_target_value);
        }
        println!("brightness: {}", brightness_target_value);
        
        println!("color-scheme: {}", pref.get_color_scheme());
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


async fn setup_schema(db: &DbConn) -> Result<(), String> {
/*
    // it doesn't matter which order you register entities.
    // SeaORM figures out the foreign key dependencies and
    // creates the tables in the right order along with foreign keys
    db.get_schema_builder()
        .register(cake::Entity)
        .register(cake_filling::Entity)
        .register(filling::Entity)
        .apply(db)
        .await?;

    // or, write DDL manually
    db.execute(
        Table::create()
            .table(cake::Entity)
            .col(pk_auto(cake::Column::Id))
            .col(string(cake::Column::Name))
    ).await?;
*/
    Ok(())
}