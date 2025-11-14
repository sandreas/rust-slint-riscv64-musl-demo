use std::time::Duration;
use awedio::manager::Manager;
use awedio::Sound;
// https://github.com/PaulWoitaschek/Voice/blob/main/core/playback/src/main/kotlin/voice/core/playback/player/VoicePlayer.kt
// https://github.com/tsirysndr/music-player/blob/master/playback/src/audio_backend/rodio.rs


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

struct AudioPlayer {
    manager: Manager,
    sound: dyn Sound,

}

impl AudioPlayer {

    fn is_playing() -> bool {
        true
    }
    fn play(&mut self) {

        // self.manager.play()
        // self.sound.with_adjustable_speed()
        // self.sound.with_adjustable_volume()
    }

    fn pause() {

    }

    fn next() {

    }

    fn previous() {

    }

    fn fast_forward() {

    }

    fn rewind() {

    }

    fn seek( position: Duration) {

    }
}