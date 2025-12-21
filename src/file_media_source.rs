use crate::item;
use crate::media_source_trait::{MediaSource, MediaSourceChapter, MediaSourceCommand, MediaSourceEvent, MediaSourceItem, MediaSourceMetadata, MediaType, ReadableSeeker};
use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use lofty::error::LoftyError;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use lofty::tag::TagType::Mp4Ilst;
use std::ffi::OsStr;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;

use crate::entity::item::{ActiveModel, ActiveModelEx};
use crate::entity::items_metadata;
use crate::entity::items_metadata::{Entity, TagField};
use mp4ameta::{DataIdent, FreeformIdent, ImgRef};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, HasManyModel, QueryFilter};
use sea_orm::prelude::HasMany;
use crate::entity::items_metadata::TagField::{*};

#[derive(Clone)]
pub struct FileMediaSource {
    pub db: DatabaseConnection,
    state: Arc<Mutex<FileMediaSourceState>>,
}

struct FileMediaSourceState {
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
            db,
            state: Arc::new(Mutex::new(FileMediaSourceState {

                base_path
            })),
        }
    }


    pub fn map_db_model_to_media_item(&self, i: &item::ModelEx, metadata: &HasMany<items_metadata::Entity>) -> MediaSourceItem {
        let mut genre : Option<String> = None;
        let mut title : String = i.location.clone();
        let mut artist : Option<String> = None;
        let mut album : Option<String> = None;
        let mut composer : Option<String> = None;
        let mut series : Option<String> = None;
        let mut part : Option<String> = None;

        for tag in metadata {
            match tag.tag_field {
                Genre => genre = Some(tag.value.clone()),
                Title => title = tag.value.clone(),
                Artist => artist = Some(tag.value.clone()),
                Album => album = Some(tag.value.clone()),
                Composer => composer = Some(tag.value.clone()),
                Series => series = Some(tag.value.clone()),
                Part => part = Some(tag.value.clone()),
            };
        }

        MediaSourceItem {
            id: i.location.to_string(),
            title: title.clone(),
            media_type: MediaType::Unspecified,
            metadata: MediaSourceMetadata {
                title: Some(title.clone()),
                artist,
                album,
                genre,
                composer,
                series,
                part,
                chapters: vec![],
            },
        }
    }



    async fn upsert_item(&self, id: i32, file_id: String, media_type: item::MediaType, location: String, meta: MediaSourceMetadata) -> ActiveModelEx {
        // todo: improve this
        // see https://www.sea-ql.org/blog/2025-11-25-sea-orm-2.0/
        let db = self.db.clone();
        let now = Utc::now();

        // if id == 0 insert, otherwise update
        let builder = if id == 0 {
            ActiveModel::builder()
                .set_file_id(file_id)
                .set_media_type(media_type)
                .set_location(location.trim_start_matches('/'))
                .set_last_scan_random_key("")
                .set_date_modified(now)
                //.add_metadatum(metadata_items)

        } else {
            ActiveModel::builder()
                .set_id(id)
                .set_file_id(file_id)
                .set_media_type(media_type)
                .set_location(location.trim_start_matches('/'))
                .set_last_scan_random_key("")
                .set_date_modified(now)

        };


        let mut result = builder
            // .add_metadatum()
            // .add_picture()
            // .add_progress_history()
            .save(&db)
            .await
            .expect("todo");


        // now sync the metadata
        self.add_metadata(&mut result.metadata, Genre, meta.genre.clone(), now);
        self.add_metadata(&mut result.metadata, Artist, meta.artist.clone(), now);
        self.add_metadata(&mut result.metadata, Title, meta.title.clone(), now);
        self.add_metadata(&mut result.metadata, Album, meta.album.clone(), now);
        self.add_metadata(&mut result.metadata, Composer, meta.composer.clone(), now);
        self.add_metadata(&mut result.metadata, Series, meta.series.clone(), now);
        self.add_metadata(&mut result.metadata, Part, meta.part.clone(), now);

        let res = result.save(&db).await;

        res.unwrap()
    }

    fn add_metadata(&self, metadata: &mut HasManyModel<Entity>, tag_field: TagField, value: Option<String>, date_modified: DateTime<Utc>) {
        if value.is_some() {
            metadata.push(items_metadata::ActiveModel::builder()
                .set_tag_field(Album)
                .set_value(value.unwrap())
                .set_date_modified(date_modified));
        }
    }

    fn cache_path(&self) -> String {
        let inner = self.state.lock().unwrap();
        let cache_path = format!("{}/cache/", inner.base_path.trim_end_matches('/').to_string());
        drop(inner);
        cache_path
    }

    pub async fn scan_media(&self) {
        let audio_extensions = vec!("mp3", "m4b");
        let inner = self.state.lock().unwrap();
        let base_path = inner.base_path.clone();
        let db = self.db.clone();
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

            // update file modification time
            // let file = File::create("Foo.txt").unwrap();
            // file.set_modified(SystemTime::now()).unwrap();


            let file_id = file_id::get_file_id(full_path.clone()).unwrap();
            let file_id_str = format!("{:?}", file_id);

            let file_name_without_ext = audio_file
                .path()                // PathBuf
                .file_stem()           // Option<&OsStr>
                .and_then(OsStr::to_str)
                .map(|s| s.to_owned()).unwrap(); // Option<String>

            let file_date_modified = audio_file.path().metadata().unwrap().modified().unwrap();


            let file_date_mod_compare: DateTime<Local> = DateTime::from(file_date_modified);

            let item_result = item::Entity::find()
                .filter(item::Column::FileId.eq(file_id_str.clone()))
                .one(&db)
                .await;

            let (item_is_modified, id) = if let Ok(item) = item_result && let Some(ix) = item {
                (ix.date_modified < file_date_mod_compare, ix.id)
            } else {
                (true, 0)
            };

            if item_is_modified {
                let empty_meta = MediaSourceMetadata {
                    artist: None,
                    title: None,
                    album: None,
                    genre: None,
                    composer: None,
                    series: None,
                    part: None,
                    chapters: vec![],
                };
                // file_name_without_ext
                let mut item_meta_result = self.extract_metadata(full_path.clone()).await;
                if item_meta_result.is_err() {
                    item_meta_result = Ok(empty_meta.clone());
                }
                println!("item is modified");
                let item_meta = if let Ok(meta) = item_meta_result {
                    meta
                } else {
                    empty_meta.clone()
                };
                let i = self.upsert_item(id, file_id_str.clone(), media_type.clone(), rel_path.clone(), item_meta).await;
            } else {
                println!("item NOT modified");
            }
        }
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
            let mut final_genre: Option<String> = None;

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
                let genre = mp4tag.genre().map(String::from);
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
                final_genre,
                chapters))
        }

        Ok(MediaSourceMetadata::new(None, None, None, None, None, None, None,vec![]))
    }

    fn map_media_source_to_orm_media_type(&self, media_type: MediaType) -> item::MediaType {
        match media_type {
            MediaType::Audiobook => item::MediaType::Audiobook,
            MediaType::Music => item::MediaType::Music,
            _ => item::MediaType::Unspecified,
        }
    }

    async fn extract_metadata(&self, path: String) -> Result<MediaSourceMetadata, LoftyError> {
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

        let tagged_file = Probe::open(path.clone())?.guess_file_type()?.read()?;
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
            let mut final_genre: Option<String> = None;

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

                // composer = ©wrt => Fourcc(*b"\xa9wrt")


                let movement = mp4tag.movement();
                let movement_index = mp4tag.movement_index();
                let final_composer = mp4tag.composer();

                // mp4tag.artist_sort_order()
                let series_indent = FreeformIdent::new_static("com.pilabor.tone", "SERIES");
                let series = mp4tag.strings_of(&series_indent).next();
                let part_indent = FreeformIdent::new_static("com.pilabor.tone", "PART");
                let part = mp4tag.strings_of(&part_indent).next();
                final_genre = mp4tag.genre().map(String::from);
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
                final_genre,
                chapters))
        }

        Ok(MediaSourceMetadata::new(None, None, None, None, None, None, None,vec![]))
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
        let db = self.db.clone();

        // let q = query.to_lowercase();
        let media_type = match query {
            "4" => item::MediaType::Music,
            "2" => item::MediaType::Audiobook,
            _ => item::MediaType::Unspecified
        };

        let items = item::Entity::load()
                .filter(item::Column::MediaType.eq(media_type))
                .with(items_metadata::Entity)
                .all(&db)
                .await;
        if items.is_err() {
            return vec![];
        }

        let items = items.unwrap();
        let result: Vec<MediaSourceItem> = items.iter().map(|i| {
            self.map_db_model_to_media_item(i, &i.metadata)
        }).collect();

        result
    }

    async fn find(&self, id: &str) -> Option<MediaSourceItem> {
        let db = self.db.clone();
        let items = item::Entity::load()
            .filter(item::Column::Id.eq(id))
            .with(items_metadata::Entity)
            .one(&db)
            .await;

        if items.is_err() {
            return None;
        }

        let items = items.unwrap();

        if let Some(i) = items {
            return Some(self.map_db_model_to_media_item(&i, &i.metadata));
        }
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

