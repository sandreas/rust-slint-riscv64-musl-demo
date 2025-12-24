use std::io;
use std::io::{BufReader, Read, Seek};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// Supertrait combining both
pub trait ReadableSeeker: Read + Seek {}
impl<T: Read + Seek> ReadableSeeker for T {}

#[derive(Debug)]
pub enum MediaSourceCommand {
    Filter(String),
    Find(String),
}

#[derive(Debug)]
pub enum MediaSourceEvent {
    FilterResults(Vec<MediaSourceItem>),
    FindResult(Option<MediaSourceItem>),
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    Unspecified = 0,
    Audiobook = 2,
    Music = 4,
}

#[derive(Debug, Clone)]
pub struct MediaSourceItem {
    pub id: String,
    pub location: String,
    pub title: String,
    pub media_type: MediaType,
    pub metadata: MediaSourceMetadata
}

#[derive(Debug, Clone)]
pub struct MediaSourceMetadata {
    // option is important here, because empty can be the real value as well as unset values, which are None
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub composer: Option<String>,
    pub series: Option<String>,
    pub part: Option<String>,
    pub cover: Option<MediaSourcePicture>,
    pub chapters: Vec<MediaSourceChapter>,
}

impl MediaSourceMetadata {
    pub fn new(artist: Option<String>,
               title: Option<String>,
               album: Option<String>,
               composer: Option<String>,
               series: Option<String>,
               part: Option<String>,
               genre: Option<String>,
               cover: Option<MediaSourcePicture>,
               chapters: Vec<MediaSourceChapter>
    ) -> Self {
        Self {
            artist,
            title,
            album,
            genre,
            composer,
            series,
            part,
            cover,
            chapters,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct MediaSourceChapter {
    pub name: String,
    #[serde(with = "crate::serde_json_mods::duration_millis")]
    pub start: Duration,
    #[serde(with = "crate::serde_json_mods::duration_millis")]
    pub duration: Duration,
}




#[derive(Debug, Clone)]
pub enum MediaSourceImageCodec {
    Unknown,
    Jpeg,
    Png,
    Tiff,
    Bmp,
    Gif,
    WebP,
}

#[derive(Debug, Clone)]
pub struct MediaSourcePicture {
    pub cache_dir: String,
    pub hash: String,
    pub codec: MediaSourceImageCodec,
}

impl MediaSourcePicture {
    pub fn path(&self) -> String {
        if self.hash.is_empty() {
            return String::from("");
        }
        let mut chars = self.hash.chars();

        let first_char = chars.next().unwrap();
        let second_char = chars.next().unwrap();
        format!("{}/{}/{}/{}/", self.cache_dir.trim_end_matches('/'), "img", first_char, second_char)
    }

    pub fn pic_full_path(&self, ext: String) -> String {
        self.internal_file(String::from(""), ext)
    }

    pub fn tb_full_path(&self, ext: String) -> String {
        self.internal_file(String::from("tb."), ext)
    }

    fn internal_file(&self, suffix: String, pic_ext: String) -> String {
        if self.hash.is_empty() {
            return String::from("");
        }
        let path = self.path();

        let pic_filename = format!("{}.{}{}", &self.hash.to_string(), suffix, pic_ext);

        format!("{}{}", path, pic_filename)
    }

    fn medias_source_image_codec_to_ext(&self, codec:&MediaSourceImageCodec) -> String {
        let unknown_ext = String::from("dat");
        match codec {
            MediaSourceImageCodec::Png => String::from("png"),
            MediaSourceImageCodec::Jpeg => String::from("jpg"),
            MediaSourceImageCodec::Tiff => String::from("tif"),
            MediaSourceImageCodec::Bmp => String::from("jpg"),
            MediaSourceImageCodec::Gif => String::from("gif"),
            _ => unknown_ext.clone()
        }
    }

}

impl MediaSourceChapter {
    pub fn new(name: String, start: Duration, duration: Duration) -> Self {
        Self { name, start, duration }
    }

    pub fn end(&self) -> Duration {
        self.start + self.duration
    }
}



#[async_trait::async_trait]
pub trait MediaSource: Send + Sync {
    fn id(&self) -> String;
    async fn filter(&self, query: &str) -> Vec<MediaSourceItem>;
    async fn find(&self, id: &str) -> Option<MediaSourceItem>;


    /// Async run loop - consumes self
    async fn run(
        self,
        cmd_rx: UnboundedReceiver<MediaSourceCommand>,
        evt_tx: UnboundedSender<MediaSourceEvent>,
    );
}
