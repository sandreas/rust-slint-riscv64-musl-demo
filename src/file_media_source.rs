use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use lofty::error::LoftyError;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::TagType::Mp4Ilst;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;
use crate::media_source_trait::{MediaSource, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaSourceMetadata, MediaType, ReadableSeeker};

use mp4ameta::{FreeformIdent, Tag, Userdata};


#[derive(Clone)]
pub struct FileMediaSource {
    state: Arc<Mutex<FileMediaSourceState>>,
}

struct FileMediaSourceState {
    pub base_path: String,
    pub items: Vec<MediaSourceItem>,
}

impl FileMediaSource {
    pub fn new(base_path: String) -> Self {
        let audio_extensions = vec!("mp3", "m4b");

        // let music_dir = PathBuf::from(&self.base_path).join("music");
        // let audiobook_dir = PathBuf::from(&self.base_path).join("audiobooks");

        let audio_files = WalkDir::new(&base_path)
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
                let start_index = base_path.len();
                let rel_path = &path_string[start_index..];
                let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                let media_type = if rel_path.starts_with("/music/") {
                    MediaType::Music
                }
                else if rel_path.starts_with("/audiobooks/") {
                    MediaType::Audiobook
                } else {
                    MediaType::Unspecified
                };

                let title = e.file_name().to_string_lossy().to_string().chars().take(15).collect();
                let full_path = path_string[start_index..].to_string();
                let item = MediaSourceItem {
                    id: full_path.clone(),
                    media_type,
                    title,
                    metadata: Self::load_metadata(full_path.clone()),
                };
                // (item.id.clone(), item) // (key, value) for HashMap
                item
            }).collect::<Vec<MediaSourceItem>>();

        Self {
            state: Arc::new(Mutex::new(FileMediaSourceState {
                base_path,
                items: audio_files
            })),
        }
    }


    fn load_metadata(p: String) -> MediaSourceMetadata {
        // if p0.ends_with("")
        let path = Path::new(p.as_str());


        if let Some(ext) = path.extension() {
            Self::load_metadata_by_extension(p.clone(), ext.to_str().unwrap().to_string());
        }

        MediaSourceMetadata::new(None, None, None, vec![])
    }

    fn load_metadata_by_extension(path: String, ext: String) -> core::result::Result<MediaSourceMetadata, LoftyError> {

        // Let's guess the format from the content just in case.
        // This is not necessary in this case!
        let tagged_file = Probe::open(path.clone())?.guess_file_type()?.read()?;
        /*
        let tagged_file = Probe::open(path)
            .expect("ERROR: Bad path provided!")
            .read()
            .expect("ERROR: Failed to read file!");
        */

        /*
        let read_cfg = ReadConfig {
    read_meta_items: true,
    read_image_data: false,
    read_chapter_list: false,
    read_chapter_track: false,
    read_audio_info: false,
    chpl_timescale: ChplTimescale::DEFAULT,
};
let mut tag = Tag::read_with_path("music.m4a", &read_cfg).unwrap();
         */
        let tag = match tagged_file.primary_tag() {
            Some(primary_tag) => primary_tag,
            // If the "primary" tag doesn't exist, we just grab the
            // first tag we can find. Realistically, a tag reader would likely
            // iterate through the tags to find a suitable one.
            None => tagged_file.first_tag().expect("ERROR: No tags found!"),
        };

        if tag.tag_type() == Mp4Ilst {
            let mp4tag = mp4ameta::Tag::read_from_path(path.clone()).unwrap();
            let chapters = mp4tag.chapters();
            for chapter in chapters {
                let mins = chapter.start.as_secs() / 60;
                let secs = chapter.start.as_secs() % 60;
                println!("{mins:02}:{secs:02} {}", chapter.title);
            }
            // https://github.com/saecki/mp4ameta/issues/35
            // tag.itunes_string("ASIN");
            let series_indent = FreeformIdent::new_static("com.pilabor.tone", "SERIES");
            let series = mp4tag.strings_of(&series_indent).next().unwrap_or("--NOTFOUND--");

            let part_indent = FreeformIdent::new_static("com.pilabor.tone", "PART");


        }

        Ok(MediaSourceMetadata::new(None, None, None, vec![]))
        /*
        match ext.as_str() {
            "mp4" => Self::load_mp4_metadata(path.clone()),
            _ => MediaSourceMetadata::new(None, None, None, vec![])
        }

         */
    }

}

#[async_trait]
impl MediaSource for FileMediaSource {
    fn id(&self) -> String {
        let inner = self.state.lock().unwrap();
        let id = inner.base_path.clone();
        drop(inner);
        id
    }

    async fn filter(&self, query: &str) -> Vec<MediaSourceItem> {
        let inner = self.state.lock().unwrap();
        // let q = query.to_lowercase();
        let media_type = match query {
            "4" => MediaType::Music,
            "2" => MediaType::Audiobook,
            _ => MediaType::Unspecified
        };

        let results = inner.items
            .iter()
            .filter(|item| {
                item.media_type.eq(&media_type)
            })
            .cloned()
            .collect();
        drop(inner);
        results
    }

    async fn find(&self, id: &str) -> Option<MediaSourceItem> {
        let inner = self.state.lock().unwrap();
        let result = inner.items
            .iter()
            .find(|item| item.id == id)
            .cloned();
        drop(inner);
        result
    }

    async fn open(&self, id: &str) -> io::Result<Arc<Mutex<BufReader<dyn ReadableSeeker + Send + 'static>>>> {
        let inner = self.state.lock().unwrap();
        let path = format!("{}/{}.ogg", inner.base_path, id);
        drop(inner);
        let file = std::fs::File::open(path)?;
        let buf_reader = BufReader::new(file);
        Ok(Arc::new(Mutex::new(buf_reader)))
    }

    async fn run(
        mut self,
        mut cmd_rx: UnboundedReceiver<MediaSourceCommand>,
        evt_tx: UnboundedSender<MediaSourceEvent>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                MediaSourceCommand::Filter(query) => {
                    let results = self.filter(&query).await;
                    let _ = evt_tx.send(MediaSourceEvent::FilterResults(results));
                }
                MediaSourceCommand::Find(id) => {
                    let result = self.find(&id).await;
                    let _ = evt_tx.send(MediaSourceEvent::FindResult(result));
                }
            }
        }
    }
}
