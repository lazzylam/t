use teloxide::prelude::*;
use crate::database::Database;
use regex::Regex;
use std::collections::HashMap;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

// Gunakan tokio::sync::Mutex untuk async context
static LAST_MESSAGES: Lazy<Mutex<HashMap<i64, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Kompilasi regex sekali saja
static MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@[\w\d_]{5,}").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+|t\.me/\S+|wa\.me/\S+|bit\.ly/\S+").unwrap());
static EMOJI_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\u{1F600}-\u{1F64F}\u{2700}-\u{27BF}\u{1F680}-\u{1F6FF}\u{1F300}-\u{1F5FF}]").unwrap());

const SUSPICIOUS_KEYWORDS: [&str; 4] = ["tmo", "vcs", "vcan", "vcs-an"];

pub async fn handle_message(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let message_id = msg.id;

    // Early return jika tidak ada text
    let text = match msg.text() {
        Some(t) if !t.trim().is_empty() => t.to_lowercase(),
        _ => return Ok(()),
    };

    // Paralel database queries menggunakan tokio::join!
    let (is_enabled, blacklist, whitelist) = tokio::join!(
        db.is_enabled(chat_id),
        db.list_blacklist(chat_id),
        db.list_whitelist(chat_id)
    );

    if !is_enabled {
        return Ok(());
    }

    // Quick whitelist check terlebih dahulu
    let whitelisted = whitelist.iter().any(|kw| text.contains(&kw.to_lowercase()));
    if whitelisted {
        return Ok(());
    }

    // Deteksi duplikat dengan non-blocking lock
    let is_duplicate = {
        let mut map = LAST_MESSAGES.lock().await;
        if let Some(prev) = map.get(&chat_id) {
            if prev == &text {
                true
            } else {
                map.insert(chat_id, text.clone());
                false
            }
        } else {
            map.insert(chat_id, text.clone());
            false
        }
    };

    // Batching semua checks untuk efisiensi
    let should_delete = is_duplicate
        || SUSPICIOUS_KEYWORDS.iter().any(|kw| text.contains(kw))
        || MENTION_RE.is_match(&text)
        || URL_RE.is_match(&text)
        || EMOJI_RE.find_iter(&text).count() > 5
        || blacklist.iter().any(|kw| text.contains(&kw.to_lowercase()));

    if should_delete {
        // Hapus pesan secara silent (tanpa notifikasi error)
        tokio::spawn(async move {
            let _ = bot.delete_message(msg.chat.id, message_id).await;
        });
    }

    Ok(())
}

// Alternatif dengan caching yang lebih advanced
use std::time::{Duration, Instant};

struct CachedData {
    blacklist: Vec<String>,
    whitelist: Vec<String>,
    is_enabled: bool,
    last_updated: Instant,
}

static CHAT_CACHE: Lazy<Mutex<HashMap<i64, CachedData>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn handle_message_cached(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let message_id = msg.id;

    let text = match msg.text() {
        Some(t) if !t.trim().is_empty() => t.to_lowercase(),
        _ => return Ok(()),
    };

    // Cache database queries selama 5 menit
    let (is_enabled, blacklist, whitelist) = {
        let mut cache = CHAT_CACHE.lock().await;
        
        if let Some(cached) = cache.get(&chat_id) {
            if cached.last_updated.elapsed() < Duration::from_secs(300) {
                (cached.is_enabled, cached.blacklist.clone(), cached.whitelist.clone())
            } else {
                // Refresh cache
                let (enabled, bl, wl) = tokio::join!(
                    db.is_enabled(chat_id),
                    db.list_blacklist(chat_id),
                    db.list_whitelist(chat_id)
                );
                
                cache.insert(chat_id, CachedData {
                    blacklist: bl.clone(),
                    whitelist: wl.clone(),
                    is_enabled: enabled,
                    last_updated: Instant::now(),
                });
                
                (enabled, bl, wl)
            }
        } else {
            // Load dari database pertama kali
            let (enabled, bl, wl) = tokio::join!(
                db.is_enabled(chat_id),
                db.list_blacklist(chat_id),
                db.list_whitelist(chat_id)
            );
            
            cache.insert(chat_id, CachedData {
                blacklist: bl.clone(),
                whitelist: wl.clone(),
                is_enabled: enabled,
                last_updated: Instant::now(),
            });
            
            (enabled, bl, wl)
        }
    };

    if !is_enabled {
        return Ok(());
    }

    // Quick whitelist check
    if whitelist.iter().any(|kw| text.contains(&kw.to_lowercase())) {
        return Ok(());
    }

    // Duplicate check
    let is_duplicate = {
        let mut map = LAST_MESSAGES.lock().await;
        map.get(&chat_id).map_or(false, |prev| prev == &text)
            .then(|| true)
            .unwrap_or_else(|| {
                map.insert(chat_id, text.clone());
                false
            })
    };

    // Kombinasi semua checks
    let should_delete = is_duplicate
        || SUSPICIOUS_KEYWORDS.iter().any(|kw| text.contains(kw))
        || MENTION_RE.is_match(&text)
        || URL_RE.is_match(&text)
        || EMOJI_RE.find_iter(&text).count() > 5
        || blacklist.iter().any(|kw| text.contains(&kw.to_lowercase()));

    if should_delete {
        // Fire-and-forget deletion
        tokio::spawn(async move {
            let _ = bot.delete_message(msg.chat.id, message_id).await;
        });
    }

    Ok(())
}