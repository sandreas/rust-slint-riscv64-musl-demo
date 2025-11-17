// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs



pub struct Player {
}

impl Player {
    pub fn new() -> Self {
        Player {  }
    }

    pub async fn play(&mut self, id: String) {
        // Async play logic (load, decode, output audio)
        println!("Playing file: {}", id);
        if(id != "") {
            /*
            let open_result = awedio::sounds::open_file(id);
            if(open_result.is_ok()) {
                let (sound, notifier) = open_result.ok().unwrap()
                    .pausable()
                    .with_adjustable_volume()
                    .with_adjustable_speed()
                    .with_completion_notifier();
                let sound = Box::new(sound);
                // sound.set_paused(true);
                // let sound_weak = sound.as_weak();

                self.manager.play(sound);
                let _ = notifier.recv();
            }

             */
        }

    }

    pub async fn pause(&mut self) {
        // self.manager.

    }

    // Implement other async controls: next, previous, fast_forward, rewind
}



/*
Ideas:
- PlayerState => Playing, Paused, Buffering, etc.
- Events => TrackStarted, TrackEnded, PositionChanged, etc.
- Metadata retrieval should not be part of the player
- MediaItem => Reference to the Playable Tracks
  - ItemMetadata: Metadata specific to the item (AlbumArtist, etc.)
  - Tracks: List of tracks contained by this media item (e.g. when an audio book has multiple files)
    - TrackMetadata: Metadata specific to the track (title, chapters, x of y, etc.)

 */


/*
    let Some(file_path) = args() else {
        eprintln!("usage: FILE_PATH");
        std::process::exit(2);
    };

    let (mut manager, _backend) = awedio::start()?;
    let (sound, notifier) = awedio::sounds::open_file(file_path)?.with_completion_notifier();

    manager.play(Box::new(sound));
    let _ = notifier.recv();

    Ok(())
 */
