mod player;
mod awedio_extensions;

use std::iter;
use awedio::{backends};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
slint::include_modules!();
use awedio::backends::CpalBufferSize;
use cpal::traits::{DeviceTrait, HostTrait};
use player::Player;
use tokio::sync::mpsc;
use std::sync::Arc;

#[derive(Debug)]
enum PlayerCommand { Play(String), Pause, Next, Previous, FastForward, Rewind }


async fn player_task(mut player: Player, mut rx: mpsc::UnboundedReceiver<PlayerCommand>) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            PlayerCommand::Play(file) => player.play(file).await,
            PlayerCommand::Pause => player.pause().await,
            PlayerCommand::Next => { /* handle */ }
            PlayerCommand::Previous => { /* handle */ }
            PlayerCommand::FastForward => { /* handle */ }
            PlayerCommand::Rewind => { /* handle */ }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {

    let ui = MainWindow::new()?;
    let ui_handle = ui.as_weak();
    let (tx, mut rx) = mpsc::unbounded_channel::<PlayerCommand>();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Create your player instance


    let audio_player = ui.global::<AudioPlayer>();
    audio_player.on_play({
        let tx = tx.clone();
        move |file_name: SharedString| {
            tx.send(PlayerCommand::Play(file_name.to_string())).unwrap();
        }
    });
    /*
    audio_player.on_play({
        let tx = tx.clone(); // your channel sender for commands
        move |file_name: SharedString| {
            // Convert to standard Rust String if needed
            let file: String = file_name.to_string();
            tx.send(PlayerCommand::Play(file)).unwrap();
        }
    });
    */
    /*
    // Hook up UI callbacks to send commands
    ui.on_play({
        let tx = tx.clone();
        move || { tx.send(AudioCommand::Play).unwrap(); }
    });
    ui.on_pause({
        let tx = tx.clone();
        move || { tx.send(AudioCommand::Pause).unwrap(); }
    });
*/

    let navigation = ui.global::<Navigation>();
    let goto_ui = ui.clone_strong();
    navigation.on_goto(move |value| {
        let nav = goto_ui.global::<Navigation>();
        nav.set_route(value);
        let history_item = nav.get_route();
        // inner_ui.global::<Navigation>().
        // inner_ui.global::<Navigation>().set_history()

        let tmp_next_index = nav.get_history_index() + 1;
        let next_index = if tmp_next_index > 1000 { 1000 } else {tmp_next_index};
        let skip = if tmp_next_index > 1000 { 1 } else {0};
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

    let back_ui = ui.clone_strong();
    navigation.on_back(move || {
        let nav = back_ui.global::<Navigation>();
        let current_index = nav.get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav.get_history().iter().collect();
        if current_index == 0 || vec_of_history.is_empty() {
            return;
        }
        nav.set_route(vec_of_history[vec_index-1].clone());
        nav.set_history_index(current_index-1);
    });

    let forward_ui = ui.clone_strong();
    navigation.on_forward(move || {
        let nav = forward_ui.global::<Navigation>();
        let current_index = nav.get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav.get_history().iter().collect();
        if vec_of_history.len() < vec_index + 2 {
            return;
        }
        nav.set_route(vec_of_history[vec_index+1].clone());
        nav.set_history_index(current_index+1);
    });


    let host = cpal::default_host();
    let mut device = None;
    for _ in 1..3 {
        device = Some(host.output_devices().unwrap()
            .find(|d| {
                if let Ok(name) = d.name() {
                    // ALSA device names may include "hw:2,0" or "hw-2-0"
                    // You might need to tune this filter depending on your device naming
                    println!("device: {} contains {}: {}", name, "sysdefault:CARD=A", name.contains("sysdefault:CARD=A"));
                    name.contains("sysdefault:CARD=A")
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
    let default_config = selected_device.default_output_config().ok().unwrap();
    let sample_rate = default_config.sample_rate().0;
    let channel_count = default_config.channels();
    let sample_format = default_config.sample_format();


    let mut backend = backends::CpalBackend::new(channel_count,
                                                 sample_rate,
                                                 CpalBufferSize::Default,
                                                 selected_device,
                                                 sample_format);

    let manager = backend.start(|error| eprintln!("error with cpal output stream: {}", error)).unwrap();

    let player = Player::new(manager);

    // Spawn background player task
    tokio::spawn(player_task(player, rx));
    
    /*

    // Spawn the audio background service
    tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            // Here, handle command (e.g. play, pause, ...) using your player backend
            // Pretend playback happens here
            slint::invoke_from_event_loop({
                let ui_handle = ui_handle.clone();
                move || {
                    let ui = ui_handle.unwrap();
                    // ui.set_status(SharedString::from(format!("Handled: {:?}", cmd)));
                }
            }).unwrap();

            match cmd {
                PlayerCommand::Play(file_name) => {

                }
                // ...
                _ => {}
            }
        }
    });
    */
    ui.run()
}



fn init_audio() {
    
    
    /*
            // List output devices and find one that roughly matches "hw:2,0" or its ALSA name
            let device = host.output_devices().unwrap()
                .find(|d| {
                    if let Ok(name) = d.name() {
                        // ALSA device names may include "hw:2,0" or "hw-2-0"
                        // You might need to tune this filter depending on your device naming
                        name.contains("hw:2,0") || name.contains("hw-2-0")
                    } else {
                        false
                    }
                })
                .ok_or("Desired device hw:2,0 not found")?;

            println!("Using output device: {}", device.name()?);
    */




    /*
            let sound: Box<dyn Sound> = Box::new(awedio::sounds::SineWave::new(100.0));
            manager.play(sound);
            let mut is_playing = true;
    */
    // let ui_handle = ui.as_weak();
    /*
            ui.on_run_code_callback(move |extension| {
                if is_playing {
                    manager.clear();
                    is_playing = false;
                } else {
                    let mut sound: Box<dyn Sound> = Box::new(awedio::sounds::SineWave::new(100.0));

                    if extension != "sine" {
                        let audio_file = format!("/root/test.{extension}");
                        if fs::metadata(audio_file.clone()).is_ok() {
                            sound = awedio::sounds::open_file(audio_file.clone()).unwrap();
                        } else {
                            println!("Audiofile does not exist: {}", audio_file.clone());
                        }
                    }

                    manager.play(sound);
                    is_playing = true;
                }
                // std::thread::sleep(std::time::Duration::from_millis(3000));

            });*/
}