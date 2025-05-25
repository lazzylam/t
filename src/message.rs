use teloxide::prelude::*;
use mongodb::bson::doc;
use regex::Regex;
use lazy_static::lazy_static;
use crate::{models::*, database::MongoDb};
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static! {
    static ref SPAM_REGEX: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(tmo|vcs|call|wa\.me|telegram\.me)\b").unwrap(),
        Regex::new(r"(?:\+?\d{1,3}[-. ]?)?\(?\d{3}\)?[-. ]?\d{3}[-. ]?\d{4}").unwrap(),
        Regex::new(r"(?i)(https?://|www\.)(telegram|whatsapp|vcs|tmo)\.(me|com|org)/[^\s]+").unwrap(),
    ];
}

pub struct SpamDetector {
    db: Arc<Mutex<MongoDb>>,
}

impl SpamDetector {
    pub fn new(db: Arc<Mutex<MongoDb>>) -> Self {
        Self { db }
    }

    pub async fn detect_spam(&self, msg: &Message) -> Result<SpamDetectionResult, Box<dyn std::error::Error + Send + Sync>> {
        // Skip if message is from bot itself
        if msg.from().map(|u| u.is_bot).unwrap_or(false) {
            return Ok(SpamDetectionResult {
                is_spam: false,
                reason: None,
            });
        }

        let chat_id = msg.chat.id.0;
        let user = msg.from().unwrap();
        let user_id = user.id.0;

        // Check if user is whitelisted
        let db = self.db.lock().await;
        if db.is_user_whitelisted(chat_id, user_id).await? {
            return Ok(SpamDetectionResult {
                is_spam: false,
                reason: None,
            });
        }

        // Check if user is blacklisted
        if db.is_user_blacklisted(chat_id, user_id).await? {
            return Ok(SpamDetectionResult {
                is_spam: true,
                reason: Some("User is blacklisted".to_string()),
            });
        }

        // Check message content
        if let Some(text) = msg.text() {
            // Check against default spam patterns
            for pattern in SPAM_REGEX.iter() {
                if pattern.is_match(text) {
                    return Ok(SpamDetectionResult {
                        is_spam: true,
                        reason: Some(format!("Matched spam pattern: {}", pattern.as_str())),
                    });
                }
            }

            // Check against custom blacklisted texts
            if let Some(pattern) = db.find_matching_blacklisted_text(chat_id, text).await? {
                return Ok(SpamDetectionResult {
                    is_spam: true,
                    reason: Some(format!("Matched blacklisted text: {}", pattern)),
                });
            }

            // Check for mass mentions
            if Self::is_mass_mention(msg) {
                return Ok(SpamDetectionResult {
                    is_spam: true,
                    reason: Some("Mass mention detected".to_string()),
                });
            }
        }

        Ok(SpamDetectionResult {
            is_spam: false,
            reason: None,
        })
    }

    fn is_mass_mention(msg: &Message) -> bool {
        msg.entities()
            .map(|entities| {
                entities.iter()
                    .filter(|e| e.kind == MessageEntityKind::Mention)
                    .count() > 3
            })
            .unwrap_or(false)
    }
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    db: Arc<Mutex<MongoDb>>,
    detector: Arc<SpamDetector>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check if bot is active in this group
    let chat_id = msg.chat.id.0;
    let db = db.lock().await;
    if !db.is_group_active(chat_id).await? {
        return Ok(());
    }

    // Detect spam
    let detection_result = detector.detect_spam(&msg).await?;

    if detection_result.is_spam {
        handle_spam_message(&bot, &msg, detection_result.reason.unwrap_or_default()).await?;
    }

    Ok(())
}

async fn handle_spam_message(
    bot: &Bot,
    msg: &Message,
    reason: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(user) = msg.from() {
        let username = user.username.as_deref().unwrap_or("unknown");
        
        log::warn!("SPAM detected from {} ({}): {}", username, user.id, reason);
        
        // Delete the message
        if let Err(e) = bot.delete_message(msg.chat.id, msg.id).await {
            log::error!("Failed to delete message: {}", e);
        }
        
        // Send warning
        let warning = format!(
            "⚠️ @{} {} Pesan telah dihapus.",
            username, reason
        );
        
        bot.send_message(msg.chat.id, warning).await?;
    }
    
    Ok(())
}