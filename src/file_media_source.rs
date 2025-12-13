use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;
use crate::media_source_trait::{MediaSource, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaType, ReadableSeeker};

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
                let item = MediaSourceItem {
                    id: path_string[start_index..].to_string(), // title.clone(),
                    media_type,
                    title,
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
