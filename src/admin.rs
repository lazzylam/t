use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use crate::{models::*, database::MongoDb};

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "Admin commands:")]
pub enum AdminCommand {
    #[command(description = "Aktifkan bot anti-spam")]
    On,
    #[command(description = "Nonaktifkan bot anti-spam")]
    Off,
    #[command(description = "Tambahkan teks ke blacklist", parse_with = "split")]
    Addbltext(String),
    // ... command lainnya
}

pub async fn admin_command_handler(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
    db: MongoDb,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Verifikasi admin
    if !is_admin(&bot, &msg).await {
        bot.send_message(msg.chat.id, "❌ Hanya admin yang bisa menggunakan command ini!")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }

    match cmd {
        AdminCommand::On => {
            db.set_group_active(msg.chat.id.0, true).await?;
            bot.send_message(msg.chat.id, "✅ Bot anti-spam diaktifkan!").await?;
        }
        // Implementasi command lainnya
        _ => {}
    }

    Ok(())
}

async fn is_admin(bot: &Bot, msg: &Message) -> bool {
    // Implementasi verifikasi admin
    true
}