use chrono::Utc;
use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "pictures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub location: String,  // file path or URL

    pub hash: String, // unique image hash for deduplication

    pub date_modified: chrono::DateTime<Utc>,

    #[sea_orm(has_many, via = "items_pictures")]
    pub items: HasMany<super::item::Entity>,

}

impl ActiveModelBehavior for ActiveModel {}
