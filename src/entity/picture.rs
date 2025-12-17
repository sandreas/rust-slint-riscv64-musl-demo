use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pictures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub location: String,  // file path or URL

    pub hash: String,      // unique image hash for deduplication

    pub date_modified: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::items_pictures::Entity")]
    ItemsPictures,
}

impl ActiveModelBehavior for ActiveModel {}
