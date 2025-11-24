use std::error::Error;
use walkdir::WalkDir;
use crate::player::Player;

pub trait MediaSource {
    async fn query(self, query: Query) -> impl Iterator;
    async fn find_by_id(self, id: String) -> Option<MediaSourceItem>;
}

pub struct MediaSourceItem {
    pub id: String,
    pub name: String,
}

pub struct Query {

}

struct FileMediaSource {
    base_path: String,
}

impl MediaSource for FileMediaSource {
    async fn query(self, query: Query) -> impl Iterator {
         WalkDir::new(self.base_path.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.metadata().unwrap().is_file()).map(|e| {
            MediaSourceItem {
                id: String::from(e.file_name().to_str().unwrap()),
                name: String::from(e.file_name().to_str().unwrap()),
            }
        })
    }

    async fn find_by_id(self, id: String) -> Option<MediaSourceItem> {
        WalkDir::new(id.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.metadata().unwrap().is_file()).map(|e| {
            MediaSourceItem {
                id: String::from(e.file_name().to_str().unwrap()),
                name: String::from(e.file_name().to_str().unwrap()),
            }
        }).next()
    }
}

