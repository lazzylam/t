use teloxide::prelude::*;
use crate::database::Database;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static LAST_MESSAGES: Lazy<Mutex<HashMap<i64, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn handle_message(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    if !db.is_enabled(chat_id).await {
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) => t.to_lowercase(),
        None => return Ok(()),
    };

    if text.trim().is_empty() {
        return Ok(());
    }

    let blacklist = db.list_blacklist(chat_id).await;
    let whitelist = db.list_whitelist(chat_id).await;

    const SUSPICIOUS_KEYWORDS: [&str; 4] = ["tmo", "vcs", "vcan", "vcs-an"];
    let contains_suspicious = SUSPICIOUS_KEYWORDS.iter().any(|kw| text.contains(kw));

    let mention_re = Regex::new(r"@[\w\d_]{5,}").unwrap();
    let contains_mention = mention_re.is_match(&text);

    let url_re = Regex::new(r"https?://\S+|t\.me/\S+|wa\.me/\S+|bit\.ly/\S+").unwrap();
    let contains_link = url_re.is_match(&text);

    let emoji_re = Regex::new(r"[\u{1F600}-\u{1F64F}\u{2700}-\u{27BF}\u{1F680}-\u{1F6FF}\u{1F300}-\u{1F5FF}]").unwrap();
    let emoji_count = emoji_re.find_iter(&text).count();
    let too_many_emojis = emoji_count > 5;

    let contains_blacklist = blacklist.iter().any(|kw| text.contains(&kw.to_lowercase()));
    let whitelisted = whitelist.iter().any(|kw| text.contains(&kw.to_lowercase()));

    // Deteksi duplikat di grup yang sama
    let is_duplicate = {
        let mut map = LAST_MESSAGES.lock().unwrap();
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

    if (contains_suspicious
        || contains_mention
        || contains_link
        || contains_blacklist
        || too_many_emojis
        || is_duplicate)
        && !whitelisted
    {
        if let Some(id) = msg.id {
            let _ = bot.delete_message(msg.chat.id, id).await;
            let _ = bot
                .send_message(msg.chat.id, "🚫 Pesan mencurigakan dihapus.")
                .await;
        }
    }

    Ok(())
}