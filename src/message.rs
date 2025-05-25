use teloxide::prelude::*;
use crate::database::Database;

pub async fn handle_message(bot: Bot, db: Database, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    // Lewati jika bot tidak aktif di grup ini
    if !db.is_enabled(chat_id).await {
        return Ok(());
    }

    // Ambil teks dari pesan
    let text = match msg.text() {
        Some(t) => t.to_lowercase(),
        None => return Ok(()), // abaikan jika bukan pesan teks
    };

    // Ambil daftar blacklist & whitelist dari DB
    let blacklist = db.list_blacklist(chat_id).await;
    let whitelist = db.list_whitelist(chat_id).await;

    // Deteksi manual kata mencurigakan (anti-GC)
    let suspicious_keywords = ["tmo", "vcs", "vcan", "vcs-an"];
    let contains_suspicious = suspicious_keywords.iter().any(|kw| text.contains(kw));

    // Deteksi mention
    let contains_mention = text
        .split_whitespace()
        .any(|word| word.starts_with('@') && word.len() > 1 && word.chars().all(|c| c.is_alphanumeric() || c == '_'));

    // Deteksi keyword blacklist
    let contains_blacklist = blacklist.iter().any(|kw| text.contains(&kw.to_lowercase()));
    let whitelisted = whitelist.iter().any(|kw| text.contains(&kw.to_lowercase()));

    // Jika pesan mencurigakan & tidak ada dalam whitelist, hapus
    if (contains_suspicious || contains_mention || contains_blacklist) && !whitelisted {
        if let Some(id) = msg.id {
            let _ = bot.delete_message(msg.chat.id, id).await;
            let _ = bot.send_message(msg.chat.id, "🚫 Pesan mencurigakan dihapus.").await;
        }
    }

    Ok(())
}