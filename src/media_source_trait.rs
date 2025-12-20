use std::io;
use std::io::{BufReader, Read, Seek};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
               chapters: Vec<MediaSourceChapter>) -> Self {
        Self {
            artist,
            title,
            album,
            genre,
            composer,
            series,
            part,
            chapters,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaSourceChapter {
    pub name: String,
    pub start: Duration,
    pub duration: Duration,
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

    async fn open(&self, id: &str) -> io::Result<Arc<Mutex<BufReader<dyn ReadableSeeker + Send + 'static>>>>;

    /// Async run loop - consumes self
    async fn run(
        self,
        cmd_rx: UnboundedReceiver<MediaSourceCommand>,
        evt_tx: UnboundedSender<MediaSourceEvent>,
    );
}
