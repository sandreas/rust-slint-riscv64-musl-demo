use crate::media_source::media_source_chapter::MediaSourceChapter;
use crate::media_source::media_source_picture::MediaSourcePicture;

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
    pub cover: Option<MediaSourcePicture>,
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
               cover: Option<MediaSourcePicture>,
               chapters: Vec<MediaSourceChapter>
    ) -> Self {
        Self {
            artist,
            title,
            album,
            genre,
            composer,
            series,
            part,
            cover,
            chapters,
        }
    }
}