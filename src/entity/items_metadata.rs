use chrono::Utc;
use sea_orm::entity::prelude::*;

// TagField enum stored as INTEGER
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TagField {
    #[sea_orm(num_value = 0)]
    Title,
    #[sea_orm(num_value = 1)]
    Artist,
    #[sea_orm(num_value = 2)]
    Album,
    #[sea_orm(num_value = 3)]
    Genre,
    // extend as needed
}

#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq)]
#[sea_orm(table_name = "items_metadata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub item_id: i32,

    pub tag_field: TagField,

    pub value: String,

    pub date_modified: chrono::DateTime<Utc>,

    #[sea_orm(belongs_to, from = "item_id", to = "id")]
    pub item: HasOne<super::item::Entity>,

}

impl ActiveModelBehavior for ActiveModel {}
