use chrono::Utc;
use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum ImageCodec {
    #[sea_orm(num_value = 0)]
    Png,
    #[sea_orm(num_value = 1)]
    Jpeg,
    #[sea_orm(num_value = 2)]
    Tiff,
    #[sea_orm(num_value = 3)]
    Bmp,
    #[sea_orm(num_value = 4)]
    Gif,
}

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pictures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub location: String,  // file path or URL

    pub hash: u64,

    pub encoding: ImageCodec,

    pub date_modified: chrono::DateTime<Utc>,

    #[sea_orm(has_many, via = "items_pictures")]
    pub items: HasMany<super::item::Entity>,

}

impl ActiveModelBehavior for ActiveModel {}
