use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlacklistItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: i64,
    pub keyword: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WhitelistItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: i64,
    pub keyword: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupSettings {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: i64,
    pub enabled: bool,
}
