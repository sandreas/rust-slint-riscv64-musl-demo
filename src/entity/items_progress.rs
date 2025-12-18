use sea_orm::entity::prelude::*;
use chrono::NaiveDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "items_progress")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    // Foreign key to items.id
    pub item_id: i32,

    pub session_key: String,

    // playback position in seconds, percentage, etc.
    pub position: f32,

    pub date_modified: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::entity::item::Entity",
        from = "Column::ItemId",
        to = "crate::entity::item::Column::Id"
    )]
    Item,
}

impl Related<crate::entity::item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
