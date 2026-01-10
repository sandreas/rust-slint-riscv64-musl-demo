use crate::media_source::media_source_image_codec::MediaSourceImageCodec;

#[derive(Debug, Clone)]
pub struct MediaSourcePicture {
    pub cache_dir: String,
    pub hash: String,
    pub codec: MediaSourceImageCodec,
}



impl MediaSourcePicture {
    pub fn path(&self) -> String {
        if self.hash.is_empty() {
            return String::from("");
        }
        let mut chars = self.hash.chars();

        let first_char = chars.next().unwrap();
        let second_char = chars.next().unwrap();
        format!("{}/{}/{}/{}/", self.cache_dir.trim_end_matches('/'), "img", first_char, second_char)
    }

    pub fn pic_full_path(&self, ext: String) -> String {
        self.internal_file(String::from(""), ext)
    }

    pub fn tb_full_path(&self, ext: String) -> String {
        self.internal_file(String::from("tb."), ext)
    }

    fn internal_file(&self, suffix: String, pic_ext: String) -> String {
        if self.hash.is_empty() {
            return String::from("");
        }
        let path = self.path();

        let pic_filename = format!("{}.{}{}", &self.hash.to_string(), suffix, pic_ext);

        format!("{}{}", path, pic_filename)
    }
}
