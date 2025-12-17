use sea_orm::DatabaseConnection;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use serde::{Serialize, Deserialize};
use serde_json;
use chrono::Utc;

pub struct SettingsManager {
    db: DatabaseConnection,
}

impl SettingsManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    // If you specifically want a setter-style API:
    pub fn set_db(&mut self, db: DatabaseConnection) {
        self.db = db;
    }

    pub async fn get<T>(
        &self,
        key: &str,
        default_value: T,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        use crate::entity::setting;

        if let Some(model) = setting::Entity::find()
            .filter(setting::Column::Key.eq(key.to_string()))
            .one(&self.db)
            .await?
        {
            let value: T = serde_json::from_str(&model.value)?;
            Ok(value)
        } else {
            Ok(default_value)
        }
    }

    pub async fn get_optional<T>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        use crate::entity::setting;

        let setting = setting::Entity::find()
            .filter(setting::Column::Key.eq(key.to_string()))
            .one(&self.db)
            .await?;

        if let Some(model) = setting {
            let value: T = serde_json::from_str(&model.value)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub async fn set<T>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize,
    {
        use crate::entity::setting;

        let json_str = serde_json::to_string(value)?;

        if let Some(model) = setting::Entity::find()
            .filter(setting::Column::Key.eq(key.to_string()))
            .one(&self.db)
            .await?
        {
            let mut active: setting::ActiveModel = model.into();
            active.value = Set(json_str);
            active.date_modified = Set(Utc::now().naive_utc());
            active.update(&self.db).await?;
        } else {
            let active = setting::ActiveModel {
                id: Default::default(),
                key: Set(key.to_string()),
                value: Set(json_str),
                date_modified: Set(Utc::now().naive_utc()),
            };
            active.insert(&self.db).await?;
        }

        Ok(())
    }
}
