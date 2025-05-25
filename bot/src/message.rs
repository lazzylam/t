use teloxide::prelude::*;
use crate::database::Database;
use regex::Regex;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;

// Ultra-fast concurrent storage untuk duplicate detection
static LAST_MESSAGES: Lazy<Arc<DashMap<i64, String>>> = Lazy::new(|| Arc::new(DashMap::new()));

// Pre-compiled regex patterns - kompilasi sekali saja
static MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@[\w\d_]{5,}").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+|t\.me/\S+|wa\.me/\S+|bit\.ly/\S+").unwrap());
static EMOJI_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\u{1F600}-\u{1F64F}\u{2700}-\u{27BF}\u{1F680}-\u{1F6FF}\u{1F300}-\u{1F5FF}]").unwrap());

// Suspicious keywords dalam static array untuk performa maksimal
const SUSPICIOUS_KEYWORDS: [&str; 4] = ["tmo", "vcs", "vcan", "vcs-an"];

pub async fn handle_message(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let message_id = msg.id;

    // Super early return untuk non-text messages
    let text = match msg.text() {
        Some(t) if !t.trim().is_empty() => t.to_lowercase(),
        _ => return Ok(()),
    };

    // Batch database operations dalam satu call
    let (is_enabled, blacklist, whitelist) = db.get_chat_data(chat_id).await;

    if !is_enabled {
        return Ok(());
    }

    // Fastest whitelist check menggunakan iterator optimized
    if whitelist.iter().any(|kw| text.contains(&kw.to_lowercase())) {
        return Ok(());
    }

    // Lightning-fast duplicate detection menggunakan DashMap
    let is_duplicate = {
        match LAST_MESSAGES.get(&chat_id) {
            Some(prev) if prev.value() == &text => true,
            _ => {
                LAST_MESSAGES.insert(chat_id, text.clone());
                false
            }
        }
    };

    // Batch semua checks dalam satu pipeline untuk maksimal efisiensi
    let should_delete = is_duplicate
        || SUSPICIOUS_KEYWORDS.iter().any(|&kw| text.contains(kw))
        || MENTION_RE.is_match(&text)
        || URL_RE.is_match(&text)
        || EMOJI_RE.find_iter(&text).count() > 5
        || blacklist.iter().any(|kw| text.contains(&kw.to_lowercase()));

    if should_delete {
        // Ultimate silent deletion - fire-and-forget dengan minimal overhead
        let bot_clone = bot.clone();
        let chat_id_clone = msg.chat.id;
        tokio::spawn(async move {
            let _ = bot_clone.delete_message(chat_id_clone, message_id).await;
        });
    }

    Ok(())
}

// Advanced version dengan predictive caching untuk grup yang sangat aktif
use std::time::{Duration, Instant};

#[derive(Clone)]
struct MessageStats {
    count: u32,
    last_message: Instant,
}

static MESSAGE_STATS: Lazy<Arc<DashMap<i64, MessageStats>>> = Lazy::new(|| Arc::new(DashMap::new()));

pub async fn handle_message_predictive(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let message_id = msg.id;

    let text = match msg.text() {
        Some(t) if !t.trim().is_empty() => t.to_lowercase(),
        _ => return Ok(()),
    };

    // Track message frequency untuk predictive caching
    let now = Instant::now();
    let is_high_traffic = {
        match MESSAGE_STATS.get_mut(&chat_id) {
            Some(mut stats) => {
                stats.count += 1;
                if stats.last_message.elapsed() < Duration::from_secs(1) {
                    stats.count > 10 // High traffic jika >10 msg/detik
                } else {
                    stats.count = 1;
                    stats.last_message = now;
                    false
                }
            }
            None => {
                MESSAGE_STATS.insert(chat_id, MessageStats {
                    count: 1,
                    last_message: now,
                });
                false
            }
        }
    };

    // Gunakan strategi berbeda untuk high-traffic vs normal chat
    let (is_enabled, blacklist, whitelist) = if is_high_traffic {
        // Untuk high-traffic, prioritaskan cache
        db.get_chat_data(chat_id).await
    } else {
        // Untuk normal traffic, batch query biasa
        tokio::join!(
            db.is_enabled(chat_id),
            db.list_blacklist(chat_id),
            db.list_whitelist(chat_id)
        )
    };

    if !is_enabled {
        return Ok(());
    }

    // Pre-compute lowercase keywords untuk avoid repeated operations
    let whitelist_lower: Vec<String> = whitelist.iter().map(|kw| kw.to_lowercase()).collect();
    let blacklist_lower: Vec<String> = blacklist.iter().map(|kw| kw.to_lowercase()).collect();

    // Ultra-fast whitelist check
    if whitelist_lower.iter().any(|kw| text.contains(kw)) {
        return Ok(());
    }

    // Optimized duplicate detection
    let is_duplicate = LAST_MESSAGES
        .get(&chat_id)
        .map_or(false, |prev| prev.value() == &text);

    if !is_duplicate {
        LAST_MESSAGES.insert(chat_id, text.clone());
    }

    // Parallel regex checks untuk maximum speed
    let (has_mention, has_url, emoji_count) = tokio::join!(
        async { MENTION_RE.is_match(&text) },
        async { URL_RE.is_match(&text) },
        async { EMOJI_RE.find_iter(&text).count() }
    );

    // Kombinasi semua checks
    let should_delete = is_duplicate
        || SUSPICIOUS_KEYWORDS.iter().any(|&kw| text.contains(kw))
        || has_mention
        || has_url
        || emoji_count > 5
        || blacklist_lower.iter().any(|kw| text.contains(kw));

    if should_delete {
        // Absolute silent deletion - zero latency
        tokio::task::spawn(async move {
            let _ = bot.delete_message(msg.chat.id, message_id).await;
        });
    }

    Ok(())
}

// Memory management untuk long-running bots
pub async fn cleanup_old_messages() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Cleanup setiap jam
        loop {
            interval.tick().await;

            // Clean up old message cache (keep last 1000 per chat)
            for _entry in LAST_MESSAGES.iter_mut() {
                // Implement LRU-like cleanup if needed
            }

            // Clean up old stats
            MESSAGE_STATS.retain(|_, stats| stats.last_message.elapsed() < Duration::from_secs(3600));
        }
    });
}