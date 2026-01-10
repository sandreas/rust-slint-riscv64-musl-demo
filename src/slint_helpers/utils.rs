use std::path::Path;
use std::rc::Rc;
use slint::{ModelRc, Rgb8Pixel, SharedPixelBuffer, SharedString, ToSharedString, VecModel};
use crate::display::utils;
use crate::media_source::media_source::{MediaSourceItem, MediaType};
use crate::media_source::media_source_picture::MediaSourcePicture;
use crate::{ SlintMediaSourceChapter, SlintMediaSourceItem, SlintPreferences};
use crate::slint_helpers::load_cover_result::LoadCoverResult;

pub fn sync_preferences(pref: SlintPreferences) {
    let new_brightness = pref.get_brightness();
    let brightness_target_value = utils::brightness_percent_to_target_value(new_brightness);
    utils::update_brightness(brightness_target_value);

    println!("brightness: {}", brightness_target_value);

    // dark / light
    println!("color-scheme: {}", pref.get_color_scheme());
}

pub fn option_to_slint_string(option: &Option<String>) -> SharedString {
    if option.is_some() {
        option.as_ref().unwrap().to_shared_string()
    } else {
        SharedString::from("")
    }
}


pub fn option_to_slint_cover(option: &Option<MediaSourcePicture>) -> (SharedString, SharedString) {
    if option.is_some() {
        let media_source_picture = option.as_ref().unwrap();
        (
            media_source_picture
                .pic_full_path(String::from("jpg"))
                .to_shared_string(),
            media_source_picture
                .tb_full_path(String::from("jpg"))
                .to_shared_string(),
        )
    } else {
        (SharedString::from(""), SharedString::from(""))
    }
}


pub fn load_cover_with_fallback(
    cover_path: &str,
    media_type: &MediaType,
) -> (slint::Image, LoadCoverResult) {
    let cover_result = slint::Image::load_from_path(Path::new(cover_path));

    if let Ok(cover) = cover_result {
        return (cover, LoadCoverResult::Image);
    }

    // todo: implement fallback image
    let fallback_image_result = match media_type {
        MediaType::Audiobook => slint::Image::load_from_svg_data(include_bytes!(
            "../../ui/images/icons/home/audiobooks.png"
        )),
        _ => slint::Image::load_from_svg_data(include_bytes!("../../ui/images/icons/home/music.png")),
    };
    if let Ok(fallback_image) = fallback_image_result {
        return (fallback_image, LoadCoverResult::Placeholder);
    }
    empty_cover_result()
}

pub fn empty_cover_result() -> (slint::Image, LoadCoverResult) {
    (
        slint::Image::from_rgb8(SharedPixelBuffer::<Rgb8Pixel>::new(1, 1)),
        LoadCoverResult::None,
    )
}

pub fn rust_items_to_slint_model(
    rust_items: Vec<MediaSourceItem>,
    details: bool,
) -> ModelRc<SlintMediaSourceItem> {
    // Create VecModel directly
    let model = VecModel::<SlintMediaSourceItem>::from(
        rust_items
            .into_iter()
            .map(|rust_item| {
                let (cover_path, thumbnail_path) = option_to_slint_cover(&rust_item.metadata.cover);

                let (thumbnail, thumbnail_type) =
                    load_cover_with_fallback(&thumbnail_path, &rust_item.media_type);

                let (cover, cover_type) = if details {
                    load_cover_with_fallback(&cover_path, &rust_item.media_type)
                } else {
                    empty_cover_result()
                };

                let mut slint_chapters_vec = VecModel::default();
                for chapter in &rust_item.metadata.chapters {
                    let start: i64 = chapter
                        .start
                        .as_millis()
                        .try_into()
                        .expect("Duration too long for u64");
                    let duration: i64 = chapter
                        .duration
                        .as_millis()
                        .try_into()
                        .expect("Duration too long for u64");

                    let slint_chapter = SlintMediaSourceChapter {
                        name: chapter.name.to_shared_string(),
                        start,
                        duration,
                    };

                    slint_chapters_vec.push(slint_chapter);
                }

                let chapters_model = ModelRc::new(slint_chapters_vec);

                SlintMediaSourceItem {
                    id: rust_item.id.clone().into(),
                    media_type: crate::media_source::utils::convert_media_type_to_int(&rust_item.media_type),
                    name: rust_item.title.clone().into(),
                    genre: option_to_slint_string(&rust_item.metadata.genre),
                    artist: option_to_slint_string(&rust_item.metadata.artist),
                    album: option_to_slint_string(&rust_item.metadata.album),
                    composer: option_to_slint_string(&rust_item.metadata.composer),
                    series: option_to_slint_string(&rust_item.metadata.series),
                    part: option_to_slint_string(&rust_item.metadata.part),
                    has_cover: cover_type != LoadCoverResult::None,
                    cover,
                    has_thumbnail: thumbnail_type != LoadCoverResult::None,
                    thumbnail,
                    chapters: chapters_model,
                }
            })
            .collect::<Vec<_>>(),
    );

    // Explicitly wrap in ModelRc if needed (usually not)
    ModelRc::from(Rc::new(model))
}