use sea_orm::entity::prelude::*;
use sea_orm::ActiveEnum;
use chrono::NaiveDateTime;

// JSON-specific tags (separate namespace)
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum JsonTagField {
    #[sea_orm(num_value = 0)]
    Chapters,
    #[sea_orm(num_value = 1)]
    Lyrics,
    #[sea_orm(num_value = 2)]
    ChaptersMarkers,
    #[sea_orm(num_value = 3)]
    EmbeddedImages,
    // JSON-specific complex data
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "items_json_metadata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub item_id: i32,

    pub tag_field: JsonTagField,  // Now uses JsonTagField

    pub value: String,  // JSON string

    pub date_modified: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::super::item::Entity",
        from = "Column::ItemId",
        to = "super::super::item::Column::Id"
    )]
    Item,
}

impl Related<super::super::item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
