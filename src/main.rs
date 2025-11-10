use std::iter;
use std::rc::Rc;
use awedio::{backends, Sound};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
slint::include_modules!();
use awedio::backends::CpalBufferSize;
use awedio::sounds::wrappers::AddSound;
use cpal::traits::{DeviceTrait, HostTrait};


fn main() -> Result<(), slint::PlatformError> {

    let ui = MainWindow::new()?;
    // ui.window().set_fullscreen(true);
    /*
    let globals = MainWindow::global::<Preferences>(&ui);
    for argument in std::env::args() {
        if argument == "--fullscreen" {
            globals.set_fullscreen(true);
        }
    }
    */
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

    let mut manager = backend.start(|error| eprintln!("error with cpal output stream: {}", error)).unwrap();

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




        // let (mut manager, _backend) = awedio::start().unwrap();
    /*
    for d in host.output_devices().unwrap() {
        println!("{:?}", d.name())

    }

     */
    // let mut backend2 = backends::CpalBackend::new(2, )
    /*

        let mut backend =
            backends::CpalBackend::with_defaults().ok_or(backends::CpalBackendError::NoDevice).unwrap();
        let mut manager = backend.start(|error| eprintln!("error with cpal output stream: {}", error))?;

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
/*
    */
/*
    ui.on_run_code_callback(move |extension| {
        _ = Command::new("aplay")
            .arg("--device=hw:2,0")
            .arg("/root/sample-3s.wav")
            .output()
            .expect("failed to execute process");
        sleep(Duration::from_secs(3));
    });
*/
    let navigation = ui.global::<Navigation>();
    let goto_ui = ui.clone_strong();
    navigation.on_goto(move |value| {

        goto_ui.global::<Navigation>().set_route(value);
        let history_item = goto_ui.global::<Navigation>().get_route();
        // inner_ui.global::<Navigation>().
        // inner_ui.global::<Navigation>().set_history()

        let mut next_index = goto_ui.global::<Navigation>().get_history_index() + 1;
        let mut skip = 0;
        if(next_index > 1000) {
            skip = 1;
            next_index = 1000;
        }

        let vec_of_history: Vec<ModelRc<SharedString>> = goto_ui.global::<Navigation>()
            .get_history()
            .iter()
            .skip(skip)
            .chain(iter::once(history_item)).collect();
        let history = VecModel::from(vec_of_history);
        goto_ui.global::<Navigation>().set_history(ModelRc::new(history));

        goto_ui.global::<Navigation>().set_history_index(next_index);
        /*
        let model: Rc<VecModel<SharedString>> = Rc::new(VecModel::from(vec![
            "Item 1".into(),
            "Item 2".into(),
            "Item 3".into(),
        ]));
*/
        // Convert Rc<VecModel<_>> to ModelRc<_>
        // let model_rc: ModelRc<SharedString> = ModelRc::from(model.clone());
        // history.iter().skip(1)

        // let history = inner_ui.global::<Navigation>().get_history();

    });

    let back_ui = ui.clone_strong();
    navigation.on_back(move || {
        let current_index = back_ui.global::<Navigation>().get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = back_ui.global::<Navigation>().get_history().iter().collect();
        if current_index == 0 || vec_of_history.is_empty() {
            return;
        }
        back_ui.global::<Navigation>().set_route(vec_of_history[vec_index-1].clone());
        back_ui.global::<Navigation>().set_history_index(current_index-1);
    });

    let forward_ui = ui.clone_strong();
    navigation.on_forward(move || {
        let current_index = forward_ui.global::<Navigation>().get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = forward_ui.global::<Navigation>().get_history().iter().collect();
        if vec_of_history.len() < vec_index + 2 {
            return;
        }
        forward_ui.global::<Navigation>().set_route(vec_of_history[vec_index+1].clone());
        forward_ui.global::<Navigation>().set_history_index(current_index+1);
    });

    /*
    navigation.on_back(move |value| {

    });

     */
/*
    ui.on_run_code_callback(move |extension| {
        _ = Command::new("aplay")
            .arg("--device=hw:2,0")
            .arg("/root/sample-3s.wav")
            .output()
            .expect("failed to execute process");
        sleep(Duration::from_secs(3));
    });

 */
    ui.run()
}


// use slint::{ModelRc, VecModel, SharedString};

fn convert_iterator_to_modelrc(
    iter: impl Iterator<Item = ModelRc<SharedString>>,
) -> ModelRc<ModelRc<SharedString>> {
    // Collect items into a Vec
    let vec_of_models: Vec<ModelRc<SharedString>> = iter.collect();

    // Create a VecModel from Vec<ModelRc<SharedString>>
    let vec_model = VecModel::from(vec_of_models);

    // Wrap into ModelRc
    ModelRc::new(vec_model)
}