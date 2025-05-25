use mongodb::{Client, Database, options::ClientOptions};
use mongodb::bson::doc;
use crate::models::*;
use thiserror::Error;

#[derive(Clone)]
pub struct MongoDb {
    db: Database,
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),
    #[error("Invalid data format: {0}")]
    DataFormat(String),
}

impl MongoDb {
    pub async fn new(uri: &str, db_name: &str) -> Result<Self, DbError> {
        let mut client_options = ClientOptions::parse(uri).await?;
        client_options.app_name = Some("telegram-antispam-bot".to_string());
        client_options.max_pool_size = Some(100);
        client_options.min_pool_size = Some(10);
        client_options.connect_timeout = Some(Duration::from_secs(5));
        client_options.server_selection_timeout = Some(Duration::from_secs(5));
        
        let client = Client::with_options(client_options)?;
        let db = client.database(db_name);
        
        Ok(Self { db })
    }

    pub async fn get_group_settings(&self, chat_id: i64) -> Result<Option<GroupSettings>, DbError> {
        self.db.collection::<GroupSettings>("group_settings")
            .find_one(doc! { "_id": chat_id }, None)
            .await
            .map_err(Into::into)
    }

    pub async fn set_group_active(&self, chat_id: i64, active: bool) -> Result<(), DbError> {
        let settings = GroupSettings {
            chat_id,
            is_active: active,
            updated_at: chrono::Utc::now(),
        };

        self.db.collection::<GroupSettings>("group_settings")
            .update_one(
                doc! { "_id": chat_id },
                doc! { "$set": mongodb::bson::to_bson(&settings)? },
                mongodb::options::UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            )
            .await?;

        Ok(())
    }

    // Implementasi method lainnya untuk blacklist/whitelist dengan pola serupa
    // ...
}