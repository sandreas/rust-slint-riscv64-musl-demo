use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct MediaSourceChapter {
    pub name: String,
    #[serde(with = "crate::serde_json_mods::duration_millis")]
    pub start: Duration,
    #[serde(with = "crate::serde_json_mods::duration_millis")]
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