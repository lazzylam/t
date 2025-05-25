use teloxide::prelude::*;
use crate::{database::Database, ml::is_spam_or_toxic};  // tambahkan import ml

pub async fn handle_message(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    if !db.is_enabled(chat_id).await {
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) => t.to_lowercase(),
        None => return Ok(()),
    };

    let blacklist = db.list_blacklist(chat_id).await;
    let whitelist = db.list_whitelist(chat_id).await;

    let suspicious_keywords = ["tmo", "vcs", "vcan", "vcs-an"];
    let contains_suspicious = suspicious_keywords.iter().any(|kw| text.contains(kw));

    let contains_mention = text
        .split_whitespace()
        .any(|word| word.starts_with('@') && word.len() > 1 && word.chars().all(|c| c.is_alphanumeric() || c == '_'));

    let contains_blacklist = blacklist.iter().any(|kw| text.contains(&kw.to_lowercase()));
    let whitelisted = whitelist.iter().any(|kw| text.contains(&kw.to_lowercase()));

    // Panggil ML deteksi toxic/spam
    let detected_by_ml = is_spam_or_toxic(&text).await;

    if (contains_suspicious || contains_mention || contains_blacklist || detected_by_ml) && !whitelisted {
        if let Some(id) = msg.id {
            let _ = bot.delete_message(msg.chat.id, id).await;
            let _ = bot.send_message(msg.chat.id, "🚫 Pesan mencurigakan dihapus (deteksi ML & keyword).").await;
        }
    }

    Ok(())
}