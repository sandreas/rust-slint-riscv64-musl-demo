use chrono::Utc;
use sea_orm::entity::prelude::*;

use sea_orm::{ActiveModelBehavior, DeriveEntityModel};
#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq)]
#[sea_orm(table_name = "items_pictures")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub picture_id: i32,
    #[sea_orm(belongs_to, from = "item_id", to = "id")]
    pub item: Option<super::item::Entity>,
    #[sea_orm(belongs_to, from = "picture_id", to = "id")]
    pub picture: Option<super::picture::Entity>,
    
    pub date_modified: chrono::DateTime<Utc>,
}

impl ActiveModelBehavior for crate::entity::items_pictures::ActiveModel {}