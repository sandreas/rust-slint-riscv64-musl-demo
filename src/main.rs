use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::sync::mpsc;

mod player;
mod media_source;
mod headset;
mod gpio_button_service;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    base_directory: String,
}

use crate::media_source::MediaType::Audiobook;
use crate::media_source::{FileMediaSource, MediaSource, MediaSourceItem, MediaSourceFilter, MediaType, MediaSourceEvent, MediaSourceCommand};
use crate::player::{Player, PlayerCommand, PlayerEvent};
use slint::{
    ComponentHandle,
    Model,
    ModelRc,
    SharedString,
    VecModel
};
use std::iter;
use std::path::Path;
use std::rc::Rc;
use evdev::Device;
use walkdir::WalkDir;
use MediaType::Music;
use crate::headset::{Headset, HeadsetEvent};

slint::include_modules!();


#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let args = Args::parse();
    println!("base directory is: {}", args.base_directory);
    let base_path = Path::new(&args.base_directory);
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


    let (player_cmd_tx, player_cmd_rx) = mpsc::unbounded_channel::<PlayerCommand>();
    let (player_evt_tx, mut player_evt_rx) = mpsc::unbounded_channel::<PlayerEvent>();

    tokio::spawn(async move {
        let mut player = Player::new("player".to_string(), "USB-C to 3.5mm Headphone Jack A".to_string(), "pipewire".to_string());
        player.run(player_cmd_rx, player_evt_tx).await;
    });


        let (source_cmd_tx, source_cmd_rx) = mpsc::unbounded_channel::<MediaSourceCommand>();
        let (source_evt_tx, mut source_evt_rx) = mpsc::unbounded_channel::<MediaSourceEvent>();

        tokio::spawn(async move {
            let base_path = args.base_directory.as_str();
            let audio_extensions = vec!("mp3", "m4b");

            // let music_dir = PathBuf::from(&self.base_path).join("music");
            // let audiobook_dir = PathBuf::from(&self.base_path).join("audiobooks");

            let audio_files = WalkDir::new(&base_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let e_clone = e.clone();
                    let metadata = e_clone.metadata().unwrap();
                    if !metadata.is_file() {
                        return false;
                    }
                    let path = e_clone.into_path();
                    match path.extension() {
                        Some(ext) => {
                            return audio_extensions.contains(&ext.to_str().unwrap());
                        }
                        None => return false,
                    }

                })
                .map(|e| {
                    let path = e.path();
                    let path_string = path.to_str().unwrap().to_string();
                    let start_index = base_path.len();
                    let rel_path = &path_string[start_index..];
                    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                    let media_type = if rel_path.starts_with("/music/") {
                        MediaType::Music
                    }
                    else if file_name.starts_with("/audiobooks/") {
                        MediaType::Audiobook
                    } else {
                        MediaType::Unspecified
                    };

                    let name = e.file_name().to_string_lossy().to_string();
                    let item = MediaSourceItem {
                        id: name.clone(),
                        media_type,
                        name,
                    };
                    // (item.id.clone(), item) // (key, value) for HashMap
                    item
                }).collect::<Vec<MediaSourceItem>>();

            /*
            let mut source = FileMediaSource::new(audio_files);
            source.run(source_cmd_rx, source_evt_tx).await;
             */
        });



        let (head_event_tx, mut head_event_rx) = mpsc::unbounded_channel::<HeadsetEvent>();
        tokio::spawn(async move {
            let device_path ="/dev/input/event13";
            let device = Device::open(Path::new(&device_path)).unwrap();
            let mut headset = Headset::new(device);
            // player.run(cmd_rx, evt_tx).await;
            headset.run(head_event_tx).await;
        });

            // Spawn receiver for worker events
            tokio::spawn(async move {
                while let Some(event) = player_evt_rx.recv().await {
                    println!("Received event: {:?}", event);
                }
            });
/*

            // Wrap in a VecModel, then ModelRc
            // let files = vec![SharedString::from("a"), SharedString::from("b"), SharedString::from("c")];
            // let model = Rc::new(VecModel::from(files));
            // let model_rc = ModelRc::from(model);


            // todo: this should happen in a background thread
            file_media_source.init().await;

            let query = MediaSourceFilter::new(Music);
            let audiobooks = file_media_source.filter(query).await;
            let len = audiobooks.iter().len();


            let vec_model_slint_items = rust_items_to_slint_model(audiobooks);
            let slint_items = ModelRc::<SlintMediaSourceItem>::from(vec_model_slint_items);
        */
//    let model = Rc::new(VecModel::from(audiobooks));
//    let model_rc = ModelRc::from(model);



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

    slint_app_window.run()
}
fn rust_items_to_slint_model(rust_items: Vec<&MediaSourceItem>) -> ModelRc<SlintMediaSourceItem> {
    // Create VecModel directly
    let model = VecModel::<SlintMediaSourceItem>::from(
        rust_items
            .into_iter()
            .map(|rust_item| SlintMediaSourceItem {
                id: rust_item.id.clone().into(),
                media_type: convert_media_type(&rust_item.media_type),
                name: rust_item.name.clone().into(),
            })
            .collect::<Vec<_>>(),
    );

    // Explicitly wrap in ModelRc if needed (usually not)
    ModelRc::from(Rc::new(model))
}

fn convert_media_type(media_type: &MediaType) -> i32 {
    match media_type {
        MediaType::Unspecified => 0,
        Audiobook => 2,
        Music => 4,
    }
}