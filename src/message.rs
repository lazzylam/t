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

    // Periksa apakah teks mengandung keyword blacklist
    for keyword in blacklist {
        if text.contains(&keyword.to_lowercase()) {
            // Abaikan jika keyword juga masuk whitelist
            if whitelist.iter().any(|w| text.contains(&w.to_lowercase())) {
                return Ok(());
            }

            // Hapus pesan yang mencurigakan
            if let Some(id) = msg.id {
                let _ = bot.delete_message(msg.chat.id, id).await;
                let _ = bot.send_message(msg.chat.id, "🚫 Pesan otomatis dihapus karena mengandung kata terlarang.").await;
            }
            break;
        }
    }

    Ok(())
}