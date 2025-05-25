use mongodb::{Client, Collection, options::ClientOptions, bson::doc};
use crate::models::{BlacklistItem, WhitelistItem, GroupSettings};
use futures_util::stream::StreamExt;
use std::env;

#[derive(Clone)]
pub struct Database {
    pub blacklist: Collection<BlacklistItem>,
    pub whitelist: Collection<WhitelistItem>,
    pub settings: Collection<GroupSettings>,
}

impl Database {
    pub async fn init() -> Self {
        dotenv::dotenv().ok();
        let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");

        let client_options = ClientOptions::parse(uri).await.unwrap();
        let client = Client::with_options(client_options).unwrap();
        let db = client.database("antigcast");

        Self {
            blacklist: db.collection("blacklist"),
            whitelist: db.collection("whitelist"),
            settings: db.collection("settings"),
        }
    }

    pub async fn is_enabled(&self, group_id: i64) -> bool {
        match self.settings.find_one(doc! { "group_id": group_id }, None).await {
            Ok(Some(s)) => s.enabled,
            _ => false,
        }
    }

    pub async fn set_enabled(&self, group_id: i64, enable: bool) {
        let _ = self.settings
            .update_one(
                doc! { "group_id": group_id },
                doc! { "$set": { "enabled": enable } },
                mongodb::options::UpdateOptions::builder().upsert(true).build(),
            )
            .await;
    }

    pub async fn add_blacklist(&self, group_id: i64, keyword: String) {
        let item = BlacklistItem { id: None, group_id, keyword };
        let _ = self.blacklist.insert_one(item, None).await;
    }

    pub async fn remove_blacklist(&self, group_id: i64, keyword: String) {
        let _ = self.blacklist
            .delete_one(doc! { "group_id": group_id, "keyword": &keyword }, None)
            .await;
    }

    pub async fn list_blacklist(&self, group_id: i64) -> Vec<String> {
        let mut cursor = self.blacklist
            .find(doc! { "group_id": group_id }, None)
            .await
            .unwrap();
        
        let mut keywords = Vec::new();
        while let Some(result) = cursor.next().await {
            if let Ok(item) = result {
                keywords.push(item.keyword);
            }
        }
        keywords
    }

    pub async fn add_whitelist(&self, group_id: i64, keyword: String) {
        let item = WhitelistItem { id: None, group_id, keyword };
        let _ = self.whitelist.insert_one(item, None).await;
    }

    pub async fn list_whitelist(&self, group_id: i64) -> Vec<String> {
        let mut cursor = self.whitelist
            .find(doc! { "group_id": group_id }, None)
            .await
            .unwrap();
        
        let mut keywords = Vec::new();
        while let Some(result) = cursor.next().await {
            if let Ok(item) = result {
                keywords.push(item.keyword);
            }
        }
        keywords
    }
}