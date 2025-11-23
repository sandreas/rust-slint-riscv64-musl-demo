use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::sync::mpsc;

mod player;
mod music_player;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "./")]
    base_directory: String,
}

use crate::player::{Player, PlayerCommand, PlayerEvent};
use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStreamBuilder, Sink};
use slint::{
    ComponentHandle,
    Model,
    ModelRc,
    SharedString,
    VecModel
};
use std::iter;
use std::path::Path;

slint::include_modules!();



#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let args = Args::parse();
    println!("{}", args.base_directory);


/*
    let host = cpal::default_host();
    let mut device = None;
    for _ in 1..3 {
        device = Some(host.output_devices().unwrap()
            .find(|d| {
                if let Ok(name) = d.name() {
                    // sysdefault:CARD=A
                    let match_string = "USB-C to 3.5mm Headphone Jack A";
                    let match_string2 = "pipewire";
                    // ALSA device names may include "hw:2,0" or "hw-2-0"
                    // You might need to tune this filter depending on your device naming
                    println!("device: {} contains {}: {}", name, match_string, name.contains(match_string));
                    name.contains(match_string) || name.contains(match_string2)
                } else {
                    false
                }
            })).unwrap();


        if device.is_some() {
            break;
        }
    }

    if !device.is_some() {
        device = Some(host.default_output_device().unwrap());
    }



    let selected_device = device.unwrap();


    let builder_result = OutputStreamBuilder::from_device(selected_device);
    let builder = builder_result.unwrap();

    // let builder = OutputStreamBuilder::from_default_device().unwrap();


    let stream = builder.open_stream_or_fallback().unwrap();

    let sink = Sink::connect_new(stream.mixer());
*/

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<PlayerCommand>();
    let (evt_tx, mut evt_rx) = mpsc::unbounded_channel::<PlayerEvent>();
    // Spawn the background worker
    // tokio::spawn(player.run(cmd_rx, evt_tx));

    tokio::spawn(async move {
        let mut player = Player::new("player".to_string(), "USB-C to 3.5mm Headphone Jack A".to_string(), "pipewire".to_string());
        player.run(cmd_rx, evt_tx).await;
    });

    // Spawn receiver for worker events
    tokio::spawn(async move {
        while let Some(event) = evt_rx.recv().await {
            println!("Received event: {:?}", event);
        }
    });

    // Example: send command to update the string
    cmd_tx.send(PlayerCommand::Update("NewName".to_string())).unwrap();




    let slint_app_window = MainWindow::new()?;
    // let slint_app_window_weak = slint_app_window.as_weak();


    let slint_audio_player = slint_app_window.global::<SlintAudioPlayer>();
    slint_audio_player.on_play({
        let tx = cmd_tx.clone();
        move |file_name: SharedString| {
            tx.send(PlayerCommand::Update(file_name.to_string())).unwrap();
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

