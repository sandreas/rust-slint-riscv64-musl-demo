use crate::item;
use std::any::Any;
use std::ffi::OsStr;
use crate::media_source_trait::{MediaSource, MediaSourceChapter, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaSourceMetadata, MediaType, ReadableSeeker};
use async_trait::async_trait;
use lofty::error::LoftyError;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use lofty::probe::Probe;
use lofty::tag::TagType::Mp4Ilst;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;

use mp4ameta::{DataIdent, FreeformIdent, ImgRef};
use sea_orm::{entity, DatabaseConnection, EntityTrait, IntoActiveValue, Set, QueryFilter, ColumnTrait, DbErr, IntoActiveModel, TryIntoModel};
use sea_orm::sea_query::OnConflict;
use tracing_subscriber::layer::SubscriberExt;
use crate::entity::items_metadata;
use crate::entity::items_metadata::TagField;

#[derive(Clone)]
pub struct FileMediaSource {
    state: Arc<Mutex<FileMediaSourceState>>,
}

struct FileMediaSourceState {
    pub db: DatabaseConnection,
    pub base_path: String,
}

impl FileMediaSource {
    pub fn new(db: DatabaseConnection, base_path: String) -> Self {
        /*
        let audio_extensions = vec!("mp3", "m4b");
        let cache_path = format!("{}/cache/", base_path.trim_end_matches('/').to_string());


        // let music_dir = PathBuf::from(&self.base_path).join("music");
        // let audiobook_dir = PathBuf::from(&self.base_path).join("audiobooks");

        let items = WalkDir::new(&base_path)
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
                let full_path = e.path().to_str().unwrap().to_string();
                let start_index = base_path.len();
                let rel_path = &full_path[start_index..];
                let media_type = if rel_path.starts_with("/music/") {
                    MediaType::Music
                }
                else if rel_path.starts_with("/audiobooks/") {
                    MediaType::Audiobook
                } else {
                    MediaType::Unspecified
                };

                let title = e.file_name().to_string_lossy().to_string().chars().take(15).collect();
                let metadata = Self::load_metadata(cache_path.clone(), full_path.clone());
                let item = MediaSourceItem {
                    id: rel_path.to_string(),
                    media_type,
                    title,
                    metadata,
                };
                // (item.id.clone(), item) // (key, value) for HashMap
                item
            }).collect::<Vec<MediaSourceItem>>();
        */
        Self {
            state: Arc::new(Mutex::new(FileMediaSourceState {
                db,
                base_path
            })),
        }
    }

    fn cache_path(&self) -> String {
        let inner = self.state.lock().unwrap();
        let cache_path = format!("{}/cache/", inner.base_path.trim_end_matches('/').to_string())
        drop(inner);
        cache_path
    }

    async fn scan_media(&self) {
        let audio_extensions = vec!("mp3", "m4b");
        let inner = self.state.lock().unwrap();
        let base_path = inner.base_path.clone();
        let db = inner.db.clone();
        drop(inner);



        let audio_files = WalkDir::new(base_path.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let e_clone = e.clone();
                let metadata = e_clone.metadata().unwrap();
                if !metadata.is_file() {
                    false;
                }
                let path = e_clone.into_path();
                return match path.extension() {
                    Some(ext) => {
                        audio_extensions.contains(&ext.to_str().unwrap())
                    }
                    None => false,
                }

            });
        let cache_path = self.cache_path();
        for audio_file in audio_files {
            let full_path = audio_file.path().to_str().unwrap().to_string();
            let start_index = base_path.len();
            let rel_path = full_path[start_index..].to_string();
            let media_type = if rel_path.starts_with("/music/") {
                item::MediaType::Music
            } else if rel_path.starts_with("/audiobooks/") {
                item::MediaType::Audiobook
            } else {
                item::MediaType::Unspecified
            };

            // todo: check inode

            let file_id = file_id::get_file_id(full_path.clone()).unwrap();
            let file_id_str = format!("{:?}", file_id);

            let file_name_without_ext =     audio_file
                .path()                // PathBuf
                .file_stem()           // Option<&OsStr>
                .and_then(OsStr::to_str)
                .map(|s| s.to_owned()).unwrap(); // Option<String>

            let up_item = upsert_item(&db,
                                      file_name_without_ext,
                                      file_id_str,
                                      media_type,
                                      rel_path.clone()).await;

            let metadata_entries = upsert_metadata(&db, full_path);

            let metadata = items_metadata::ActiveModel {
                item_id: Set(up_item.unwrap().unwrap().id),
                tag_field: Set(TagField::Album),
                value: Set("Album".to_owned()),
                date_modified: Default::default(),
                ..Default::default()  // Unset fields like id
            };





            /*
            let item = item::ActiveModel {
                name: Set("Example Item".to_string()),
                file_id: file_id_str.clone().into_active_value(),
                media_type: media_type.into_active_value(),
                location: rel_path.into_active_value(),
                date_modified: Default::default(),
                ..Default::default()  // Unset fields like id
            };

             */
            // item.insert(db).await
            // Find by primary key
            // .filter(cake::Column::Name.contains("chocolate"))
            // let db_item: Option<item::Model> = item::Model::filter(1).one(db).await?;
            /*
            let db_item = item::Entity::find()
                .filter(item::Column::FileId.eq(file_id_str.clone()))
                .one(&db)
                .await;

             */
            /*item::Entity::find()
                .filter(item::Column::FileId.eq(file_id_str.to_string()))
                .one(db)
                .await*/
        }
            /*
            .map(|e| {


                let title = e.file_name().to_string_lossy().to_string().chars().take(15).collect();
                let metadata = Self::load_metadata(cache_path.clone(), full_path.clone());
                let item = MediaSourceItem {
                    id: rel_path.to_string(),
                    media_type,
                    title,
                    metadata,
                };
                // (item.id.clone(), item) // (key, value) for HashMap
                item
            }).collect::<Vec<MediaSourceItem>>();
            */

    }


    fn load_metadata(cache_path: String, p: String) -> MediaSourceMetadata {
        // if p0.ends_with("")
        let path = Path::new(p.as_str());


        if let Some(ext) = path.extension() {
            Self::load_metadata_by_extension(cache_path.clone(), p.clone(), ext.to_str().unwrap().to_string());
        }

        MediaSourceMetadata::new(None, None, None, None, None, None, vec![])
    }

    fn load_metadata_by_extension(cache_path: String, path: String, ext: String) -> core::result::Result<MediaSourceMetadata, LoftyError> {
        /*
        Idea for covers:
        - Softlink files to a hashed central file (see https://doc.rust-lang.org/std/fs/fn.soft_link.html)
        - Alternatively: Create a filename with the hash in the filename
        - create softlink in cache for relative path linking to the real cache file
            - Audio file: ./media/audiobooks/Harry Potter.m4b
            - Softlink Big (500x500px): ./cache/audiobooks/Harry Potter.m4b.cover.jpg
            - Softlink Listing (25%-33% of 368px => 92-128px): ./cache/audiobooks/Harry Potter.m4b.listing.jpg
            - audio file hash marker: ./cache/audiobooks/Harry Potter.m4b.<a-fast-hash-over-the-content>
            - Real files:
                - ./cache/images/<a-fast-hash-over-the-content>.cover.jpg
                - ./cache/images/<a-fast-hash-over-the-content>.tb.jpg

         */


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
            Some(primary_tag) => Some(primary_tag),
            // If the "primary" tag doesn't exist, we just grab the
            // first tag we can find. Realistically, a tag reader would likely
            // iterate through the tags to find a suitable one.
            None => tagged_file.first_tag(),
        };

        let properties = tagged_file.properties();
        let duration = properties.duration();

        let mut chapters: Vec<MediaSourceChapter> = Vec::new();

        if let Some(tag) = tag {
            let mut tag_cover = if tag.picture_count() > 0 {
                Some(tag.pictures().first().unwrap().data())
            } else {
                None
            };

            let mut final_series : Option<String> = None;
            let mut final_part : Option<String> = None;
            let mut final_composer: Option<String> = None;
            if tag.tag_type() == Mp4Ilst {
                let mp4tag = mp4ameta::Tag::read_from_path(path.clone()).unwrap();
                let mp4images: Vec<(&DataIdent, ImgRef<'_>)> = mp4tag.images().collect();
                tag_cover = if mp4images.len() > 0 {
                    Some(mp4images.first().unwrap().1.data)
                } else {
                    None
                };

                let tmp_chaps = mp4tag.chapters().iter().rev();
                let mut end = duration;
                for tmp_chap in tmp_chaps {
                let duration = end - tmp_chap.start;
                chapters.push(MediaSourceChapter::new(tmp_chap.title.clone(), tmp_chap.start, duration));
                end -= duration;
                }
                chapters.reverse();

                // https://github.com/saecki/mp4ameta/issues/35
                // tag.itunes_string("ASIN");
                // let artist_ident = Fourcc(*b"\xa9mvmt");
                // mp4tag.movement()

                let movement = mp4tag.movement();
                let movement_index = mp4tag.movement_index();
                let final_composer = mp4tag.composer();

                // mp4tag.artist_sort_order()
                let series_indent = FreeformIdent::new_static("com.pilabor.tone", "SERIES");
                let series = mp4tag.strings_of(&series_indent).next();
                let part_indent = FreeformIdent::new_static("com.pilabor.tone", "PART");
                let part = mp4tag.strings_of(&part_indent).next();

                // let series_part = format!("{} {}", series, part);

                if series.is_some() {
                    final_series = series.map(|s| s.to_string());
                } else if movement.is_some() {
                    final_series = movement.map(|s| s.to_string());
                }

                if part.is_some() {
                    final_part = part.map(|s| s.to_string());
                } else if movement_index.is_some() {
                    final_part = movement_index.map(|s| s.to_string());
                }

            }




            return Ok(MediaSourceMetadata::new(
                tag.artist().map(|s| s.to_string()),
                tag.title().map(|s| s.to_string()),
                tag.album().map(|s| s.to_string()),

                final_composer,
                final_series,
                final_part,
                chapters))
        }

        Ok(MediaSourceMetadata::new(None, None, None, None, None, None,vec![]))
    }

    fn map_media_source_to_orm_media_type(&self, media_type: MediaType) -> item::MediaType {
        match media_type {
            MediaType::Audiobook => item::MediaType::Audiobook,
            MediaType::Music => item::MediaType::Music,
            _ => item::MediaType::Unspecified,
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
        let db = inner.db.clone();
        let db_item = item::Entity::find()
            .filter(item::Column::Id.eq(id))
            .one(&db)
            // .with(items_metadata::Entity)
            .await;
        // let metadata: Vec<items_metadata::Entity> =

        drop(inner);

        if let Ok(model_option) = db_item && let Some(model) = model_option {
            Some(MediaSourceItem {
                id: model.id.to_string(),

                title: model.name,
                media_type: match model.media_type {
                    item::MediaType::Music => MediaType::Music,
                    item::MediaType::Audiobook => MediaType::Audiobook,
                    _ => MediaType::Unspecified,
                },
                metadata: MediaSourceMetadata {
                    artist: None,
                    title: None,
                    album: None,
                    composer: None,
                    series: None,
                    part: None,
                    chapters: vec![],
                },
            });
        };
        None
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



async fn upsert_item(db: &DatabaseConnection, name: String, file_id: String, media_type: item::MediaType, location: String) -> Result<Option<item::Model>, DbErr> {
    let item = item::ActiveModel {
        name: Set(name.to_string()),
        file_id: Set(file_id),
        media_type: Set(media_type),
        location: Set(location),
        date_modified: Default::default(),
        ..Default::default()  // Unset fields like id
    };

    // Insert if no conflict on name, do nothing if exists
    let result = item::Entity::insert(item.clone())
        .on_conflict(
            OnConflict::column(item::Column::FileId)
                .do_nothing()  // or .update_column(item::Column::Price) for update
                .to_owned()
        )
        .exec(db)
        .await?;

    // item.id = result.last_insert_id();
    // result.last_insert_id()

    let model_only = item.clone().try_into_model();
    if let Ok(model) = model_only {
        return Ok(Some(model));
    }
    // Returns inserted model or None if conflicted
    Ok(None)
    // result
}

/*
async fn bulk_upsert(db: &DatabaseConnection) {
    let items = vec![
        item::ActiveModel { name: Set("Item1".into()),  ..Default::default() },
        item::ActiveModel { name: Set("Item2".into()), ..Default::default() },
    ];

    let result = item::Entity::insert_many(items)
        .on_conflict(OnConflict::column(item::Column::Name).do_nothing().to_owned())
        .exec(db)
        .await;
    // result.rows_affected shows how many were actually inserted [web:1][web:3]

}

 */