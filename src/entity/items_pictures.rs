// src/entity/items_pictures.rs
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "items_pictures")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub item_id: i32,
    pub picture_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::entity::item::Entity",
        from = "Column::ItemId",
        to = "crate::entity::item::Column::Id"
    )]
    Item,

    #[sea_orm(
        belongs_to = "crate::entity::picture::Entity",
        from = "Column::PictureId",
        to = "crate::entity::picture::Column::Id"
    )]
    Picture,
}

impl Related<crate::entity::item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl Related<crate::entity::picture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Picture.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
