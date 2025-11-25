# Event handling
https://docs.rs/evdev/latest/evdev/index.html

# Audiotags

https://docs.rs/audiotags/latest/audiotags/
https://github.com/tianyishi2001/audiotags

# Implementing an audio player in rust

https://dev.to/paradoxy/i-created-a-cli-music-player-in-rust-5a3f
https://github.com/Parado-xy/rust-cli-player

```bash
cargo add clap colored rodio ctrlc
```

## 1. Command-Line Interface (CLI) Configuration

```rust
fn cli_config() -> Command {
    Command::new("musicplayer")
        .version("0.1.0")
        .author("ojalla")
        .about("Command-line music player")
        .arg(
            Arg::new("music-dir")
                .short('d')
                .long("dir")
                .value_name("DIRECTORY")
                .help("Sets the music directory")
                .required_unless_present("how-to"),
        )
        .arg(
            Arg::new("how-to")
                .long("how-to")
                .help("Shows operation commands and how to use the application.")
                .action(clap::ArgAction::SetTrue),
        )
}

```

This ensures that users provide a valid music directory or request help using --how-to.

##  2. Handling User Input

```rust
fn input() -> String {
let mut user_input = String::new();

    print!("{}", "musicplayer> ".cyan().bold());
    io::stdout().flush().expect("Failed To Flush Output");

    io::stdin()
        .read_line(&mut user_input)
        .expect("Error Getting User Input");

    user_input.trim().to_string()
}
```

## 3. Implementing the Music Player

```rust
struct CliPlayer {
    sink: rodio::Sink,
    stream: rodio::OutputStream,
    stream_handle: OutputStreamHandle,
    is_playing: bool,
    is_paused: bool,
    main_dir: Option<String>,
    current_file: Option<String>,
    last_input: Option<String>,
    available_songs: Option<HashMap<i32, DirEntry>>,
    start_time: Option<Instant>,
}

```


## 4. Loading Songs from a Directory 

```rust
fn load_songs(&mut self) -> io::Result<()> {
    let mut index = 1;
    if let Some(dir) = &self.main_dir {
        if let Some(sound_map) = &mut self.available_songs {
            for entry in read_dir(dir)? {
                let entry = entry?;
                if entry.path().is_file() {
                    sound_map.insert(index, entry);
                    index += 1;
                }
            }
        }
    }
    Ok(())
}

```


##  5. Playing a Song

```rust
pub fn play(&mut self, sound_index: i32) -> Result<(), Box<dyn std::error::Error>> {
    if self.is_playing {
        self.sink.stop();
        self.sink = Sink::try_new(&self.stream_handle)?;
    }

    if let Some(sound_map) = &self.available_songs {
        if let Some(song) = sound_map.get(&sound_index) {
            let file = BufReader::new(File::open(song.path())?);
            let source = Decoder::new(file)?;
            self.sink.set_volume(1.0);
            self.sink.append(source.convert_samples::<f32>());
            self.is_playing = true;
            self.is_paused = false;
            self.current_file = Some(song.file_name().to_string_lossy().to_string());
            self.start_time = Some(Instant::now());
            println!("{}: Playing {}", "Now playing".green().bold(), self.current_file.as_ref().unwrap().blue());
            Ok(())
        } else {
            Err(format!("{}: Invalid song index", "Error".red()).into())
        }
    } else {
        Err("No songs available".into())
    }
}


```

##  6. Implementing Playback Controls 


```rust
match command {
InputCommands::Play => { /* Calls play() */ }
InputCommands::Pause => {
if self.is_playing {
self.sink.pause();
self.is_paused = true;
println!("{}: Playback paused", "Info".yellow());
}
}
InputCommands::Resume => {
if self.is_paused {
self.sink.play();
self.is_paused = false;
self.is_playing = true;
println!("{}: Playback resumed", "Info".green());
}
}
InputCommands::Stop => {
if self.is_playing {
self.sink.stop();
self.is_playing = false;
println!("{}: Playback stopped", "Info".red());
}
}
InputCommands::Volume(vol) => {
if (0.0..=1.0).contains(&vol) {
self.sink.set_volume(vol);
println!("{}: Volume set to {:.1}", "Success".green(), vol);
} else {
println!("{}: Volume must be 0.0 to 1.0", "Error".red());
}
}
_ => println!("{}: Invalid command", "Error".red()),
}


```


##  7. Displaying Available Songs 

```rust
pub fn list(&self) {
    if let Some(sound_map) = &self.available_songs {
        println!("\n{}", "Available Songs:".green().bold());
        println!("{}", "-------------------------------".green());
        println!("{:<6} {:<}", "Index".to_string().bold(), "Filename".to_string().bold());
        for (index, entry) in sound_map {
            let filename = entry.file_name().to_string_lossy();
            if let Some(current) = &self.current_file {
                if filename == *current {
                    println!("{:<6} {:<} {}", index.to_string().green(), filename.green(), "▶".green());
                } else {
                    println!("{:<6} {:<}", index, filename);
                }
            } else {
                println!("{:<6} {:<}", index, filename);
            }
        }
        println!();
    }
}


```