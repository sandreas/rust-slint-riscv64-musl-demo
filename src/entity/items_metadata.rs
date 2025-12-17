use sea_orm::entity::prelude::*;
use sea_orm::ActiveEnum;
use chrono::NaiveDateTime;

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

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "items_metadata")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    // Foreign key to items.id
    pub item_id: i32,

    pub tag_field: TagField,

    pub value: String,

    pub date_modified: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::item::Entity",
        from = "Column::ItemId",
        to = "super::item::Column::Id"
    )]
    Item,
}

impl Related<super::item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
