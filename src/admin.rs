use teloxide::{prelude::*, utils::command::BotCommands};
use crate::database::Database;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Command yang tersedia:")]
pub enum AdminCommand {
    #[command(description = "Aktifkan anti-gcast.")]
    On,
    #[command(description = "Nonaktifkan anti-gcast.")]
    Off,
    #[command(description = "Tambah keyword blacklist.")]
    Addbl(String),
    #[command(description = "Hapus keyword blacklist.")]
    Delbl(String),
    #[command(description = "Lihat semua blacklist.")]
    Listbl,
    #[command(description = "Tambah keyword whitelist.")]
    Addwhite(String),
    #[command(description = "Lihat semua whitelist.")]
    Listwhite,
    #[command(description = "Tampilkan bantuan.")]
    Help,
}

async fn is_user_admin(bot: &Bot, chat_id: i64, user_id: i64) -> bool {
    match bot.get_chat_administrators(ChatId(chat_id)).await {
        Ok(admins) => admins.iter().any(|admin| admin.user.id.0 == user_id),
        Err(_) => false,
    }
}

pub async fn handle_command(
    bot: Bot,
    db: Database,
    msg: Message,
    cmd: AdminCommand,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let user_id = match msg.from() {
        Some(u) => u.id.0,
        None => {
            bot.send_message(chat_id, "⚠️ Tidak dapat verifikasi pengguna.").await?;
            return Ok(());
        }
    };

    if !is_user_admin(&bot, chat_id, user_id).await {
        bot.send_message(chat_id, "⚠️ Hanya admin yang dapat menggunakan perintah ini.").await?;
        return Ok(());
    }

    match cmd {
        AdminCommand::On => {
            db.set_enabled(chat_id, true).await;
            bot.send_message(msg.chat.id, "✅ Anti-GCast diaktifkan.").await?;
        }
        AdminCommand::Off => {
            db.set_enabled(chat_id, false).await;
            bot.send_message(msg.chat.id, "⛔ Anti-GCast dinonaktifkan.").await?;
        }
        AdminCommand::Addbl(word) => {
            db.add_blacklist(chat_id, word.clone()).await;
            bot.send_message(msg.chat.id, format!("✔️ Ditambahkan ke blacklist: `{}`", word)).await?;
        }
        AdminCommand::Delbl(word) => {
            db.remove_blacklist(chat_id, word.clone()).await;
            bot.send_message(msg.chat.id, format!("🗑️ Dihapus dari blacklist: `{}`", word)).await?;
        }
        AdminCommand::Listbl => {
            let list = db.list_blacklist(chat_id).await;
            let text = if list.is_empty() {
                "⚠️ Blacklist kosong.".to_string()
            } else {
                format!("🛑 *Blacklist:*\n{}", list.iter().map(|x| format!("- {}", x)).collect::<Vec<_>>().join("\n"))
            };
            bot.send_message(msg.chat.id, text).parse_mode(teloxide::types::ParseMode::Markdown).await?;
        }
        AdminCommand::Addwhite(word) => {
            db.add_whitelist(chat_id, word.clone()).await;
            bot.send_message(msg.chat.id, format!("✔️ Ditambahkan ke whitelist: `{}`", word)).await?;
        }
        AdminCommand::Listwhite => {
            let list = db.list_whitelist(chat_id).await;
            let text = if list.is_empty() {
                "⚠️ Whitelist kosong.".to_string()
            } else {
                format!("✅ *Whitelist:*\n{}", list.iter().map(|x| format!("- {}", x)).collect::<Vec<_>>().join("\n"))
            };
            bot.send_message(msg.chat.id, text).parse_mode(teloxide::types::ParseMode::Markdown).await?;
        }
        AdminCommand::Help => {
            bot.send_message(msg.chat.id, AdminCommand::descriptions().to_string()).await?;
        }
    }

    Ok(())
}