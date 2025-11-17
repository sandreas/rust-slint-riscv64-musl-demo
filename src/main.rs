mod player;

use std::fs::File;
use std::io::BufReader;
use std::iter;
use rodio::{cpal, Decoder, DeviceTrait, OutputStream, OutputStreamBuilder, Sink, Source};
use rodio::cpal::traits::HostTrait;
use rodio::source::Buffered;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
slint::include_modules!();


#[derive(Debug)]
enum PlayerCommand { Play(String), Pause, Next, Previous, FastForward, Rewind }

/*
type SoundSource = Buffered<Decoder<BufReader<File>>>;

pub struct Audio {
    stream_handle: OutputStreamBuilder,
    garand_m1_single_shot: SoundSource,
}

fn source(sound_file_string_path: &str) -> SoundSource {
    let file = BufReader::new(File::open(sound_file_string_path).unwrap());
    Decoder::new(file).unwrap().buffered()
}


impl Audio {
    pub fn new() -> Self {
        let (_stream, stream_handle) = OutputStream::try_into().unwrap();
        // let (_, stream_handle) = OutputStream::try_from().unwrap();
        let garand_m1_single_shot = source("resources/audio/lmg_fire01.mp3");
        Self {
            stream_handle,
            garand_m1_single_shot,
        }
    }

    pub fn shot(&self) {
        self.stream_handle.play_raw(self.garand_m1_single_shot.clone().convert_samples());
    }
}
*/
/*
fn display_sample_format(sformat: &cpal::SampleFormat) -> &'static str {
    match sformat {
        cpal::SampleFormat::F32 => "FLOAT32LE",
        cpal::SampleFormat::I16 => "S16LE",
        cpal::SampleFormat::U16 => "U16LE",
        _ => "unknown"
    }
}


fn print_conf_range(conf: &cpal::SupportedStreamConfigRange) {
    let channels = conf.channels();
    let sample_rate_min = conf.min_sample_rate();
    let sample_rate_max = conf.max_sample_rate();
    let sformat = display_sample_format(&conf.sample_format());
    println!("      channels: {}, samplerate min: {} max: {}, format: {}", channels, sample_rate_min.0, sample_rate_max.0, sformat);
}


fn print_supported_conf(conf: &cpal::SupportedStreamConfig) {
    let channels = conf.channels();
    let sample_rate = conf.sample_rate();
    let sformat = display_sample_format(&conf.sample_format());
    println!("      channels: {}, samplerate: {}, format: {}", channels, sample_rate.0, sformat);
}
*/

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
/*
    let available_hosts = cpal::available_hosts();
    println!("Available hosts:\n  {:?}", available_hosts);

    for host_id in available_hosts {
        println!("{}", host_id.name());
        let host = cpal::host_from_id(host_id).unwrap();
        if let Some(default_in) = host.default_input_device() {
            println!("  Default Input Device:\n    {:?}", default_in.name());
        } else {
            println!("  Failed getting Default Input Device");
        }
        if let Some(default_out) = host.default_output_device() {
            println!("  Default Output Device:\n    {:?}", default_out.name());
        } else {
            println!("  Failed getting Default Output Device");
        }

        let devices = host.devices().unwrap();
        println!("  Devices: ");
        for (_device_index, device) in devices.enumerate() {
            println!("\n\n  Device: \"{}\"", device.name().unwrap());
            println!("  ============================================================");
            println!("\n    Capture\n    ------------------------------------------------------------");
            if let Ok(conf) = device.default_input_config() {
                println!("    Default input stream config:");
                print_supported_conf(&conf);
            }
            let mut input_configs = match device.supported_input_configs() {
                Ok(f) => f.peekable(),
                Err(e) => {
                    println!("    Error: {:?}", e);
                    continue;
                }
            };
            if input_configs.peek().is_some() {
                println!("    All supported input stream configs:");
                for (_config_index, config) in input_configs.enumerate() {
                    print_conf_range(&config);
                }
            }
            println!("\n    Playback\n    ------------------------------------------------------------");
            if let Ok(conf) = device.default_output_config() {
                println!("    Default output stream config:");
                print_supported_conf(&conf);
            }
            let mut output_configs = match device.supported_output_configs() {
                Ok(f) => f.peekable(),
                Err(e) => {
                    println!("    Error: {:?}", e);
                    continue;
                }
            };
            if output_configs.peek().is_some() {
                println!("    All supported output stream configs:");
                for (_config_index, config) in output_configs.enumerate() {
                    print_conf_range(&config);
                }
            }
        }
    }
    return Ok(());
    for dev in cpal::available_hosts() {
        println!("{:?}", dev);


    }
*/

    /*
    let builder = OutputStreamBuilder::from_default_device().unwrap();
    let stream = builder.open_stream_or_fallback().unwrap();
    let sink = Sink::connect_new(stream.mixer());


    let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    let file = std::fs::File::open("/home/andreas/projects/sandreas/rust-slint-riscv64-musl-demo/assets/audio/sample-3s.mp3").unwrap();
    sink.append(rodio::Decoder::try_from(file).unwrap());

    sink.sleep_until_end();
*/


    let ui = MainWindow::new()?;
    let ui_handle = ui.as_weak();


    // Create your player instance
    let audioPlayer = ui.global::<AudioPlayer>();
    audioPlayer.on_play({

        // let tx = tx.clone();
        move |file_name: SharedString| {



            // tx.send(PlayerCommand::Play(file_name.to_string())).unwrap();
        }
    });


    let navigation = ui.global::<Navigation>();
    let goto_ui = ui.clone_strong();
    navigation.on_goto(move |value| {
        let nav = goto_ui.global::<Navigation>();
        nav.set_route(value);
        let history_item = nav.get_route();
        // inner_ui.global::<Navigation>().
        // inner_ui.global::<Navigation>().set_history()

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

    let back_ui = ui.clone_strong();
    navigation.on_back(move || {
        let nav = back_ui.global::<Navigation>();
        let current_index = nav.get_history_index();
        let vec_index = current_index as usize;
        let vec_of_history: Vec<ModelRc<SharedString>> = nav.get_history().iter().collect();
        if current_index == 0 || vec_of_history.is_empty() {
            return;
        }
        nav.set_route(vec_of_history[vec_index - 1].clone());
        nav.set_history_index(current_index - 1);
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
        nav.set_route(vec_of_history[vec_index + 1].clone());
        nav.set_history_index(current_index + 1);
    });


    ui.run()
}

