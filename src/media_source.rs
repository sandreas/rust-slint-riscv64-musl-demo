use std::fs::File;
use std::io::BufReader;
use tokio::sync::mpsc;
use walkdir::WalkDir;


#[derive(Debug)]
pub enum MediaSourceCommand {
    Filter(MediaSourceFilter),
    Find(String),
}

#[derive(Debug)]
pub enum MediaSourceEvent/*<'a, 'b>*/ {
    FilterResults(Vec</*&'a */MediaSourceItem>),
    FindResult(Option</*&'b */MediaSourceItem>),
}


pub trait MediaSource<T> {
    async fn filter(&self, query: MediaSourceFilter) -> Vec<&MediaSourceItem>;
    async fn find(&self, id: &str) -> Option<&MediaSourceItem>;

    async fn open_buffer(self, item: MediaSourceItem) -> Option<BufReader<T>>;
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MediaType {
    Unspecified = 0,
    Audiobook = 2,
    Music = 4
    // Normal, Audiobook, Bookmark, Music Video, Movie, TV Show, Booklet, Ringtone, Podcast, iTunes U
}

#[derive(Debug)]
pub struct MediaSourceItem {
    pub id: String,
    pub media_type: MediaType,
    pub name: String,
}

impl MediaSourceItem {
    pub fn new(id: String, media_type:MediaType, name: String) -> MediaSourceItem {
        MediaSourceItem {
            id,
            media_type,
            name
        }
    }

    // pub fn from_file()
}

#[derive(Debug)]
pub struct MediaSourceFilter {
    pub media_type: MediaType,
}

impl MediaSourceFilter {
    pub fn new(media_type: MediaType) -> MediaSourceFilter {
        MediaSourceFilter {
            media_type,
        }
    }
}

pub struct FileMediaSource {
    items: Vec<MediaSourceItem>, // todo: Option is useless?!
}

impl /*<'a,'b> */FileMediaSource {
    pub fn new(items: Vec<MediaSourceItem>) -> FileMediaSource {
        FileMediaSource {
            items
        }
    }
    pub async fn run(
        &/*'a */mut self,
        mut cmd_rx: mpsc::UnboundedReceiver<MediaSourceCommand>,
        evt_tx: mpsc::UnboundedSender<MediaSourceEvent/*<'a, 'b>*/>,
    ) {

        loop {

            tokio::select! {

                Some(cmd) = cmd_rx.recv() => {
                    /*
                    let m: MediaSourceEvent = match cmd {
                        MediaSourceCommand::Filter(filter) => {
                            MediaSourceEvent::FilterResults(self.filter(filter).await)
                        }
                        MediaSourceCommand::Find(id) => {
                            MediaSourceEvent::FindResult(self.find(&id).await)
                        }
                    };

                     */
                    // let _ = evt_tx.send(m);

                }
            }

        }
    }
}

impl MediaSource<File> for FileMediaSource {


    /*
    async fn query(&self, _query: Query) -> impl Iterator<Item = &MediaSourceItem> {
        self.items

            .as_ref()                  // Option<&Vec<MediaSourceItem>>
            .map(|v| v.iter())         // Option<impl Iterator<Item=&MediaSourceItem>>
            .unwrap_or_else(|| iter::empty::<&MediaSourceItem>())
    }
*/

    async fn filter/*<'a>*/(&/*'a*/ self, query: MediaSourceFilter<>) -> Vec<&MediaSourceItem> {
        self.items
                .iter()
                .filter(|item| item.media_type == query.media_type)
                .collect()
    }
    /*
    async fn query(&self, query: Query) -> impl Iterator<Item = &MediaSourceItem> {
        match &self.items {
            Some(items) => items
                .iter()
                .filter(|item| item.media_type == query.media_type)
            ,
            None => Vec::new(),
        }
    }

     */
/*
    fn find_all_by_query(&self, query: &Query) -> impl Iterator<Item = &MediaSourceItem> + '_ {
        match &self.items {
            Some(items) => Box::new(
                items.iter()
                    .filter(|item| item.media_type == query.media_type)
            ) as Box<dyn Iterator<Item = &MediaSourceItem> + '_>,
            None => Box::new(iter::empty::<&MediaSourceItem>()) as Box<dyn Iterator<Item = &MediaSourceItem> + '_>,
        }
    }
    */
    async fn find/*<'a>*/(&/*'a*/ self, id: &str) -> Option<&MediaSourceItem> {
        self.items.iter().find(|item| item.id == id)
    }
    async fn open_buffer(self, item: MediaSourceItem) -> Option<BufReader<File>> {
        Some(BufReader::new(File::open(item.id).unwrap()))
    }
}

