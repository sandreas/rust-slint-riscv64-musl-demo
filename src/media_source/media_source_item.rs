use crate::media_source::media_source_metadata::MediaSourceMetadata;
use crate::media_source::media_type::MediaType;

#[derive(Debug, Clone)]
pub struct MediaSourceItem {
    pub id: String,
    pub location: String,
    pub title: String,
    pub media_type: MediaType,
    pub metadata: MediaSourceMetadata
}