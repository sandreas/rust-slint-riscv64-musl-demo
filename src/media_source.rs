use std::fs::File;
use std::io::BufReader;
use walkdir::WalkDir;

pub trait MediaSource<T> {
    async fn init(&mut self) -> ();
    async fn query(&self, query: MediaSourceQuery) -> Vec<&MediaSourceItem>;
    async fn find_by_id(&self, id: &str) -> Option<&MediaSourceItem>;

    async fn open_buffer(self, item: MediaSourceItem) -> Option<BufReader<T>>;
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum MediaType {
    Unspecified = 0,
    Audiobook = 2,
    Music = 4
    // Normal, Audiobook, Bookmark, Music Video, Movie, TV Show, Booklet, Ringtone, Podcast, iTunes U
}

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

pub struct MediaSourceQuery {
    pub media_type: MediaType,
}

impl MediaSourceQuery {
    pub fn new(media_type: MediaType) -> MediaSourceQuery {
        MediaSourceQuery {
            media_type,
        }
    }
}

pub struct FileMediaSource {
    base_path: String,
    items: Option<Vec<MediaSourceItem>>, // todo: Option is useless?!
}

impl FileMediaSource {
    pub fn new(base_path: String) -> FileMediaSource {
        FileMediaSource {
            base_path,
            items: None
        }
    }
}

impl MediaSource<File> for FileMediaSource {

    async fn init(&mut self) -> () {
        let audio_extensions = vec!("mp3", "m4b");

        // let music_dir = PathBuf::from(&self.base_path).join("music");
        // let audiobook_dir = PathBuf::from(&self.base_path).join("audiobooks");

        let audio_files = WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let e_clone = e.clone();
                let metadata = e_clone.metadata().unwrap();
                if !metadata.is_file() {
                    return false;
                }
                let path = e_clone.into_path();
                match path.extension() {
                    Some(ext) => {
                        return audio_extensions.contains(&ext.to_str().unwrap());
                    }
                    None => return false,
                }

            })
            .map(|e| {
                let path = e.path();
                let path_string = path.to_str().unwrap().to_string();
                let start_index = self.base_path.len();
                let rel_path = &path_string[start_index..];
                let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                let media_type = if rel_path.starts_with("/music/") {
                    MediaType::Music
                }
                else if file_name.starts_with("/audiobooks/") {
                    MediaType::Audiobook
                } else {
                    MediaType::Unspecified
                };

                let name = e.file_name().to_string_lossy().to_string();
                let item = MediaSourceItem {
                    id: name.clone(),
                    media_type,
                    name,
                };
                // (item.id.clone(), item) // (key, value) for HashMap
                item
            }).collect::<Vec<MediaSourceItem>>();
            // .collect::<HashMap<String, MediaSourceItem>>();

        self.items = Some(audio_files);
    }

    /*
    async fn query(&self, _query: Query) -> impl Iterator<Item = &MediaSourceItem> {
        self.items

            .as_ref()                  // Option<&Vec<MediaSourceItem>>
            .map(|v| v.iter())         // Option<impl Iterator<Item=&MediaSourceItem>>
            .unwrap_or_else(|| iter::empty::<&MediaSourceItem>())
    }
*/

    async fn query(&self, query: MediaSourceQuery) -> Vec<&MediaSourceItem> {
        match &self.items {
            Some(items) => items
                .iter()
                .filter(|item| item.media_type == query.media_type)
                .collect(),
            None => Vec::new(),
        }
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
    async fn find_by_id(&self, id: &str) -> Option<&MediaSourceItem> {
        self.items
            .as_ref()                           // Option<&Vec<_>>
            .and_then(|v| v.iter().find(|item| item.id == id))
    }
    async fn open_buffer(self, item: MediaSourceItem) -> Option<BufReader<File>> {
        Some(BufReader::new(File::open(item.id).unwrap()))
    }
}

