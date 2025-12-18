use sea_orm::entity::prelude::*;
use sea_orm::ActiveEnum;
use chrono::NaiveDateTime;

// Enum stored as INTEGER in SQLite
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum MediaType {
    #[sea_orm(num_value = 0)]
    Audio,
    #[sea_orm(num_value = 1)]
    Video,
    #[sea_orm(num_value = 2)]
    Image,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "items")]   // plural table name
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub media_type: MediaType,

    pub name: String,

    pub date_modified: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::entity::items_metadata::Entity")]
    ItemsMetadata,

    #[sea_orm(has_many = "crate::entity::items_json_metadata::Entity")]
    ItemsJsonMetadata,

    #[sea_orm(has_many = "crate::entity::items_pictures::Entity")]
    ItemsPictures,

    #[sea_orm(has_many = "crate::entity::items_progress::Entity")]
    ItemsProgress,
}


impl ActiveModelBehavior for ActiveModel {}
