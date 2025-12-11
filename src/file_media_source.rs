use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;
use crate::media_source_trait::{MediaSource, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaType};

#[derive(Debug)]
pub struct FileMediaSource {
    items: Vec<MediaSourceItem>,
}

impl FileMediaSource {
    pub fn from_path(base_path: String) -> Self {
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
                else if file_name.starts_with("/audiobooks/") {
                    MediaType::Audiobook
                } else {
                    MediaType::Unspecified
                };

                let title = e.file_name().to_string_lossy().to_string();
                let item = MediaSourceItem {
                    id: title.clone(),
                    media_type,
                    title,
                };
                // (item.id.clone(), item) // (key, value) for HashMap
                item
            }).collect::<Vec<MediaSourceItem>>();

        Self::new(audio_files)
    }

    pub fn new(items: Vec<MediaSourceItem>) -> Self {
        Self { items }
    }
}



#[async_trait]
impl MediaSource for FileMediaSource {
    async fn filter(&self, query: &str) -> Vec<MediaSourceItem> {
        let q = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&q)
                    || item.id.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    async fn find(&self, id: &str) -> Option<MediaSourceItem> {
        self.items
            .iter()
            .find(|item| item.id == id)
            .cloned()
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
