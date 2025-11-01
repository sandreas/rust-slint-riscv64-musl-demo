use std::fs;
use awedio::Sound;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    // ui.window().set_fullscreen(true);

    let (mut manager, _backend) = awedio::start().unwrap();

    let sound: Box<dyn Sound> = Box::new(awedio::sounds::SineWave::new(100.0));
    manager.play(sound);
    let mut is_playing = true;

    // let ui_handle = ui.as_weak();
    ui.on_run_code_callback(move |extension| {
        if is_playing {
            manager.clear();
            is_playing = false;
        } else {
            let mut sound: Box<dyn Sound> = Box::new(awedio::sounds::SineWave::new(100.0));

            if extension != "sine" {
                let audio_file = format!("/tmp/test.{extension}");
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

    });
    ui.run()
}