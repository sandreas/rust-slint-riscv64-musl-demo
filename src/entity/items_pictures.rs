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
    pub item: Option<super::cake::Entity>,
    #[sea_orm(belongs_to, from = "picture_id", to = "id")]
    pub picture: Option<super::filling::Entity>,
}

impl ActiveModelBehavior for crate::entity::picture::ActiveModel {}
