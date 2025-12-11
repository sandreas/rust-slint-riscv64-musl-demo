use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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
    Unspecified,
    Audiobook,
    Music,
}

#[derive(Debug, Clone)]
pub struct MediaSourceItem {
    pub id: String,
    pub title: String,
    pub media_type: MediaType,
}


#[async_trait::async_trait]
pub trait MediaSource: Send + Sync {
    async fn filter(&self, query: &str) -> Vec<MediaSourceItem>;
    async fn find(&self, id: &str) -> Option<MediaSourceItem>;

    /// Async run loop - consumes self
    async fn run(
        self,
        cmd_rx: UnboundedReceiver<MediaSourceCommand>,
        evt_tx: UnboundedSender<MediaSourceEvent>,
    );
}
