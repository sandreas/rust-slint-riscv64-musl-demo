use crate::item;
use crate::media_source_trait::{MediaSource, MediaSourceChapter, MediaSourceCommand, MediaSourceEvent, MediaSourceImageCodec, MediaSourceItem, MediaSourceMetadata, MediaSourcePicture, MediaType, ReadableSeeker};
use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use lofty::error::LoftyError;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, Tag};
use lofty::tag::TagType::Mp4Ilst;
use std::ffi::OsStr;
use std::{fs, io};
use std::fs::File;
use std::io::{BufReader, BufWriter, Error, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use image::{load_from_memory, DynamicImage, GenericImageView};
use image::imageops::FilterType;
use lofty::picture::MimeType;
use lofty::picture::PictureType::Media;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use walkdir::WalkDir;

use crate::entity::item::{ActiveModel, ActiveModelEx};
use crate::entity::{items_metadata};
use crate::entity::items_metadata::{Entity, TagField};
use mp4ameta::{DataIdent, FreeformIdent, ImgRef};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, HasManyModel, QueryFilter};
use sea_orm::prelude::HasMany;
use xxhash_rust::xxh3::xxh3_64;
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

    pub fn empty_metadata(&self) -> MediaSourceMetadata {
        MediaSourceMetadata {
            artist: None,
            title: None,
            album: None,
            genre: None,
            composer: None,
            series: None,
            part: None,
            cover: None,
            chapters: vec![],
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
        let mut cover = Some(MediaSourcePicture {
            hash: i.cover_hash.clone(),
            codec: MediaSourceImageCodec::Jpeg
        });
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
                cover,
                chapters: vec![],
            },
        }
    }



    async fn upsert_item(&self, id: i32, file_id: String, media_type: item::MediaType, location: String, meta: &MediaSourceMetadata) -> ActiveModelEx {
        // todo: improve this
        // see https://www.sea-ql.org/blog/2025-11-25-sea-orm-2.0/
        let db = self.db.clone();
        let now = Utc::now();
        let cover = meta.cover.clone();

        let cover_hash = if cover.is_some() {
            cover.unwrap().hash
        } else {
            String::from("")
        };



        // if id == 0 insert, otherwise update
        let builder = if id == 0 {
            ActiveModel::builder()
                .set_file_id(file_id)
                .set_media_type(media_type)
                .set_location(location.trim_start_matches('/'))
                .set_cover_hash(cover_hash)
                .set_last_scan_random_key("")
                .set_date_modified(now)
                //.add_metadatum(metadata_items)

        } else {
            ActiveModel::builder()
                .set_id(id)
                .set_file_id(file_id)
                .set_media_type(media_type)
                .set_location(location.trim_start_matches('/'))
                .set_cover_hash(cover_hash)
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
        let cache_path = format!("{}/{}", inner.base_path.trim_end_matches('/').to_string(), self.rel_cache_path());
        drop(inner);
        cache_path
    }

    fn rel_cache_path(&self) -> String {
        String::from("cache/")
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
                // file_name_without_ext
                let mut item_meta_result = self.extract_metadata(full_path.clone()).await;
                if item_meta_result.is_err() {
                    item_meta_result = Ok(self.empty_metadata());
                }
                // println!("item is modified");
                let item_meta = if let Ok(meta) = item_meta_result {
                    meta
                } else {
                    self.empty_metadata()
                };


                // INSERT INTO "pictures" ("hash", "codec", "date_modified") VALUES (16601657817183584017, 1, '2025-12-22 08:15:58.110775 +00:00') RETURNING "id", "hash", "codec", "date_modified"
                let now = Utc::now();

                /*
                let mut pic_save_results: Vec<picture::ActiveModelEx> = Vec::new();
                for pic in &item_meta.pictures {
                    let codec = match pic.codec {
                        MediaSourceImageCodec::Png => ImageCodec::Png,
                        MediaSourceImageCodec::Jpeg =>ImageCodec ::Jpeg,
                        MediaSourceImageCodec::Tiff =>ImageCodec ::Tiff,
                        MediaSourceImageCodec::Bmp => ImageCodec::Bmp,
                        MediaSourceImageCodec::Gif => ImageCodec::Gif,
                        _ => ImageCodec::Unknown,
                    };
                    let picture_model = picture::ActiveModel::builder()
                        .set_hash(&pic.hash)
                        .set_codec(codec)
                        .set_date_modified(now);

                    let pic_save_result = picture_model.save(&db).await;

                    println!("xx");

                    if let Ok(pic_save) = pic_save_result {
                        pic_save_results.push(pic_save)
                    }

                }
                */
                let result_model = self.upsert_item(id, file_id_str.clone(), media_type.clone(), rel_path.clone(), &item_meta).await;








            } else {
                println!("item NOT modified");
            }
        }
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
        let tag_result = match tagged_file.primary_tag() {
            Some(primary_tag) => Some(primary_tag),
            // If the "primary" tag doesn't exist, we just grab the
            // first tag we can find. Realistically, a tag reader would likely
            // iterate through the tags to find a suitable one.
            None => tagged_file.first_tag(),
        };

        let properties = tagged_file.properties();
        let duration = properties.duration();

        if tag_result.is_none() {
            return Ok(self.empty_metadata());
        }
        let tag = tag_result.unwrap();
        let mut media_source_metadata = MediaSourceMetadata::new(
            tag.artist().map(|s| s.to_string()),
            tag.title().map(|s| s.to_string()),
            tag.album().map(|s| s.to_string()),
            None, // composer
            None, // series
            None, // part
            None, // genre
            None, // cover
            vec![], // chapters
        );
        let pictures = self.extract_pictures(tag).await?;
        if pictures.len() > 0 {
            media_source_metadata.cover = Some(pictures[0].clone());
        }

        if tag.tag_type() == Mp4Ilst {
            self.extract_mp4_metadata(&mut media_source_metadata, path.clone(), duration);
        }

        Ok(media_source_metadata)
    }

    fn extract_mp4_metadata(&self, meta: &mut MediaSourceMetadata, path: String, duration: Duration) {
        let mut chapters: Vec<MediaSourceChapter> = Vec::new();
        let mp4tag = mp4ameta::Tag::read_from_path(path.clone()).unwrap();
        let mp4images: Vec<(&DataIdent, ImgRef<'_>)> = mp4tag.images().collect();
        let tag_cover = if mp4images.len() > 0 {
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
        meta.chapters = chapters;
        // https://github.com/saecki/mp4ameta/issues/35
        // tag.itunes_string("ASIN");
        // let artist_ident = Fourcc(*b"\xa9mvmt");
        // mp4tag.movement()

        // composer = ©wrt => Fourcc(*b"\xa9wrt")


        let movement = mp4tag.movement();
        let movement_index = mp4tag.movement_index();
        meta.composer = mp4tag.composer().map(|s| s.to_string());

        // mp4tag.artist_sort_order()
        let series_indent = FreeformIdent::new_static("com.pilabor.tone", "SERIES");
        let series = mp4tag.strings_of(&series_indent).next();
        let part_indent = FreeformIdent::new_static("com.pilabor.tone", "PART");
        let part = mp4tag.strings_of(&part_indent).next();
        meta.genre = mp4tag.genre().map(String::from);
        // let series_part = format!("{} {}", series, part);

        if series.is_some() {
            meta.series = series.map(|s| s.to_string());
        } else if movement.is_some() {
            meta.series = movement.map(|s| s.to_string());
        }

        if part.is_some() {
            meta.part = part.map(|s| s.to_string());
        } else if movement_index.is_some() {
            meta.part = movement_index.map(|s| s.to_string());
        }
    }


    fn medias_source_image_codec_to_ext(&self, codec:MediaSourceImageCodec) -> String {
            let unknown_ext = String::from("dat");
        match codec {
            MediaSourceImageCodec::Png => String::from("png"),
            MediaSourceImageCodec::Jpeg => String::from("jpg"),
            MediaSourceImageCodec::Tiff => String::from("tif"),
            MediaSourceImageCodec::Bmp => String::from("jpg"),
            MediaSourceImageCodec::Gif => String::from("gif"),
                _ => unknown_ext.clone()
            }

    }

    fn pic_full_path(&self, hash:u64, codec: MediaSourceImageCodec) -> String {
        let hash_hex = format!("{:016x}", hash); // 16 chars, lowercase, zero-padded
        let first_char = hash_hex.chars().next().unwrap();
        let pic_ext = self.medias_source_image_codec_to_ext(codec);
        let pic_filename = format!("{}.{}", &hash_hex.to_string(),pic_ext);
        let pic_filename_small = format!("{}.tb.{}", &hash_hex.to_string(),pic_ext);

        // let location = format!("{}{}/{}/{}", self.rel_cache_path(), "img", first_char, pic_filename);
        let pic_path_str = format!("{}{}/{}/", self.cache_path(), "img", first_char);
        let pic_path = Path::new(&pic_path_str);
        let pic_full_path = pic_path.join(&pic_filename);
        let tb_full_path = pic_path.join(&pic_filename_small);

        pic_path_str
    }

    async fn extract_pictures(&self, tag: &Tag) -> Result<Vec<MediaSourcePicture>, LoftyError> {
        let mut pics: Vec<MediaSourcePicture> = Vec::new();

        for pic in tag.pictures() {
            // todo: use self.pic_full_path (refactoring required)
            // probably implement location() on MediaSourcePicture to return the full path
            // and tb_location for the thumbnail?

            let hash_u64 = xxh3_64(&pic.data());
            let hash = format!("{:016x}", hash_u64); // 16 chars, lowercase, zero-padded
            let codec = mime_to_codec(pic.mime_type());

            let media_source_picture = MediaSourcePicture {
                hash,
                codec: self.map_encoding(pic.mime_type())
            };



            /*

            let hash_hex = format!("{:016x}", hash); // 16 chars, lowercase, zero-padded


            let first_char = hash_hex.chars().next().unwrap();
            let pic_ext = self.medias_source_image_codec_to_ext(codec);
            let pic_filename = format!("{}.{}", &hash_hex.to_string(),pic_ext);
            let pic_filename_small = format!("{}.tb.{}", &hash_hex.to_string(),pic_ext);

            let location = format!("{}{}/{}/{}", self.rel_cache_path(), "img", first_char, pic_filename);
            let pic_path_str = format!("{}{}/{}/", self.cache_path(), "img", first_char);
            let pic_path = Path::new(&pic_path_str);

            let pic_full_path = pic_path.join(&pic_filename);
            let tb_full_path = pic_path.join(&pic_filename_small);
            */

            let cache_path = self.cache_path();
            let pic_path_str = media_source_picture.path(self.cache_path());
            let pic_full_path = media_source_picture.pic_full_path(self.cache_path());
            let tb_full_path = media_source_picture.tb_full_path(self.cache_path());
            fs::create_dir_all(pic_path_str.clone())?;

            let pic_full_path_exists = pic_full_path.exists();
            if !pic_full_path_exists {
                let file = File::create(pic_full_path)?;
                let mut writer = BufWriter::new(file);
                writer.write_all(&pic.data())?;
                writer.flush()?;  // Ensure all data is written
            }

            if !tb_full_path.exists() && pic_full_path_exists{
                resize_image_bytes_to_file(&pic.data(), &tb_full_path, 128, 128);
            }

            pics.push(media_source_picture);
        }

        Ok(pics)
    }


    fn map_encoding(&self, p0: Option<&MimeType>) -> MediaSourceImageCodec {
        if p0.is_some() && let Some(mime_type) = p0 {
            return match mime_type {
                MimeType::Png => MediaSourceImageCodec::Png,
                MimeType::Jpeg => MediaSourceImageCodec::Jpeg,
                MimeType::Tiff => MediaSourceImageCodec::Tiff,
                MimeType::Bmp => MediaSourceImageCodec::Bmp,
                MimeType::Gif => MediaSourceImageCodec::Gif,
                _ => MediaSourceImageCodec::Unknown
            }
        }
        MediaSourceImageCodec::Unknown
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

fn resize_image_bytes_to_file(
    image_bytes: &[u8],
    output_path: &Path,
    max_width: u32,
    max_height: u32
) -> Result<(), Box<dyn std::error::Error>> {
    let img = load_from_memory(image_bytes)?;

    let (width, height) = img.dimensions();
    if width <= max_width && height <= max_height {
        img.save(output_path);
        return Ok(());
    }

    let aspect = width as f32 / height as f32;
    let target_width = (max_height as f32 * aspect).min(max_width as f32) as u32;
    let target_height = (max_width as f32 / aspect).min(max_height as f32) as u32;

    let resized = img.resize(target_width, target_height, FilterType::Lanczos3);
    resized.save(output_path)?;

    Ok(())
}

/*
fn process_image_bytes(
    input_bytes: &[u8],
    max_width: u32,
    max_height: u32
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let resized = resize_image_bytes(input_bytes, max_width, max_height)?;

    // Save to bytes (PNG format)
    let mut output_bytes = Vec::new();
    resized.write_to(&mut output_bytes, image::ImageOutputFormat::Png)?;

    Ok(output_bytes)
}
*/

fn resize_image_bytes(
    image_bytes: &[u8],
    max_width: u32,
    max_height: u32
) -> Result<DynamicImage, image::ImageError> {
    let img = load_from_memory(image_bytes)?;

    let (width, height) = img.dimensions();
    if width <= max_width && height <= max_height {
        return Ok(img);
    }

    let aspect = width as f32 / height as f32;
    let target_width = (max_height as f32 * aspect).min(max_width as f32) as u32;
    let target_height = (max_width as f32 / aspect).min(max_height as f32) as u32;

    Ok(img.resize(target_width, target_height, FilterType::Lanczos3))
}

fn resize_keep_aspect(image: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
    let (width, height) = image.dimensions();
    let aspect = width as f32 / height as f32;

    let new_width = (max_height as f32 * aspect) as u32;
    let new_height = (max_width as f32 / aspect) as u32;

    // Choose dimensions that fit within bounds
    let (target_width, target_height) = if new_width <= max_width {
        (new_width, max_height)
    } else {
        (max_width, new_height)
    };

    image.resize_exact(target_width, target_height, FilterType::Lanczos3)
}

fn mime_to_codec( mime_type_opt: Option<&MimeType>) -> MediaSourceImageCodec {
    let unknown_ext = String::from("dat");
    if let Some(mime_type) = mime_type_opt {
        let result = match mime_type {
            MimeType::Png => MediaSourceImageCodec::Png,
            MimeType::Jpeg => MediaSourceImageCodec::Jpeg,
            MimeType::Tiff => MediaSourceImageCodec::Tiff,
            MimeType::Bmp => MediaSourceImageCodec::Jpeg,
            MimeType::Gif => MediaSourceImageCodec::Gif,
            _ => MediaSourceImageCodec::Unknown
        };
        return result;
    }
    MediaSourceImageCodec::Unknown
}