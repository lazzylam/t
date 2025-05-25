use mongodb::{Client, Collection, options::{ClientOptions, FindOptions}, bson::doc};
use crate::models::{BlacklistItem, WhitelistItem, GroupSettings};
use futures_util::stream::StreamExt;
use std::env;
use dashmap::DashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;

#[derive(Clone)]
struct CacheEntry {
    data: Vec<String>,
    last_updated: Instant,
}

#[derive(Clone)]
struct SettingsCache {
    enabled: bool,
    last_updated: Instant,
}

#[derive(Clone)]
pub struct Database {
    pub blacklist: Collection<BlacklistItem>,
    pub whitelist: Collection<WhitelistItem>,
    pub settings: Collection<GroupSettings>,
    // High-performance concurrent caches
    blacklist_cache: Arc<DashMap<i64, CacheEntry>>,
    whitelist_cache: Arc<DashMap<i64, CacheEntry>>,
    settings_cache: Arc<DashMap<i64, SettingsCache>>,
}

impl Database {
    pub async fn init() -> Self {
        dotenv::dotenv().ok();
        let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");

        let mut client_options = ClientOptions::parse(uri).await.unwrap();
        // Optimasi koneksi MongoDB
        client_options.max_pool_size = Some(20);
        client_options.min_pool_size = Some(5);
        client_options.max_idle_time = Some(Duration::from_secs(30));
        client_options.server_selection_timeout = Some(Duration::from_secs(5));

        let client = Client::with_options(client_options).unwrap();
        let db = client.database("antigcast");

        Self {
            blacklist: db.collection("blacklist"),
            whitelist: db.collection("whitelist"),
            settings: db.collection("settings"),
            blacklist_cache: Arc::new(DashMap::new()),
            whitelist_cache: Arc::new(DashMap::new()),
            settings_cache: Arc::new(DashMap::new()),
        }
    }

    pub async fn is_enabled(&self, group_id: i64) -> bool {
        // Check cache first
        if let Some(cached) = self.settings_cache.get(&group_id) {
            if cached.last_updated.elapsed() < Duration::from_secs(300) { // 5 menit cache
                return cached.enabled;
            }
        }

        // Load from database if not cached or expired
        let enabled = match self.settings.find_one(doc! { "group_id": group_id }, None).await {
            Ok(Some(s)) => s.enabled,
            _ => false,
        };

        // Update cache
        self.settings_cache.insert(group_id, SettingsCache {
            enabled,
            last_updated: Instant::now(),
        });

        enabled
    }

    pub async fn set_enabled(&self, group_id: i64, enable: bool) {
        // Update database
        let _ = self.settings
            .update_one(
                doc! { "group_id": group_id },
                doc! { "$set": { "enabled": enable } },
                mongodb::options::UpdateOptions::builder().upsert(true).build(),
            )
            .await;

        // Update cache immediately
        self.settings_cache.insert(group_id, SettingsCache {
            enabled: enable,
            last_updated: Instant::now(),
        });
    }

    pub async fn add_blacklist(&self, group_id: i64, keyword: String) {
        let item = BlacklistItem { id: None, group_id, keyword: keyword.clone() };
        let _ = self.blacklist.insert_one(item, None).await;

        // Invalidate cache untuk refresh
        self.blacklist_cache.remove(&group_id);
    }

    pub async fn remove_blacklist(&self, group_id: i64, keyword: String) {
        let _ = self.blacklist
            .delete_one(doc! { "group_id": group_id, "keyword": &keyword }, None)
            .await;

        // Invalidate cache
        self.blacklist_cache.remove(&group_id);
    }

    pub async fn list_blacklist(&self, group_id: i64) -> Vec<String> {
        // Check cache first
        if let Some(cached) = self.blacklist_cache.get(&group_id) {
            if cached.last_updated.elapsed() < Duration::from_secs(300) {
                return cached.data.clone();
            }
        }

        // Load from database with optimized query
        let find_options = FindOptions::builder()
            .projection(doc! { "keyword": 1, "_id": 0 })
            .build();

        let mut cursor = match self.blacklist
            .find(doc! { "group_id": group_id }, find_options)
            .await {
                Ok(cursor) => cursor,
                Err(_) => return Vec::new(),
            };

        let mut keywords = Vec::new();
        while let Some(result) = cursor.next().await {
            if let Ok(item) = result {
                keywords.push(item.keyword);
            }
        }

        // Update cache
        self.blacklist_cache.insert(group_id, CacheEntry {
            data: keywords.clone(),
            last_updated: Instant::now(),
        });

        keywords
    }

    pub async fn add_whitelist(&self, group_id: i64, keyword: String) {
        let item = WhitelistItem { id: None, group_id, keyword: keyword.clone() };
        let _ = self.whitelist.insert_one(item, None).await;

        // Invalidate cache
        self.whitelist_cache.remove(&group_id);
    }

    pub async fn list_whitelist(&self, group_id: i64) -> Vec<String> {
        // Check cache first
        if let Some(cached) = self.whitelist_cache.get(&group_id) {
            if cached.last_updated.elapsed() < Duration::from_secs(300) {
                return cached.data.clone();
            }
        }

        // Load from database with optimized query
        let find_options = FindOptions::builder()
            .projection(doc! { "keyword": 1, "_id": 0 })
            .build();

        let mut cursor = match self.whitelist
            .find(doc! { "group_id": group_id }, find_options)
            .await {
                Ok(cursor) => cursor,
                Err(_) => return Vec::new(),
            };

        let mut keywords = Vec::new();
        while let Some(result) = cursor.next().await {
            if let Ok(item) = result {
                keywords.push(item.keyword);
            }
        }

        // Update cache
        self.whitelist_cache.insert(group_id, CacheEntry {
            data: keywords.clone(),
            last_updated: Instant::now(),
        });

        keywords
    }

    // Batch operations untuk performa yang lebih baik
    pub async fn get_chat_data(&self, group_id: i64) -> (bool, Vec<String>, Vec<String>) {
        tokio::join!(
            self.is_enabled(group_id),
            self.list_blacklist(group_id),
            self.list_whitelist(group_id)
        )
    }
}