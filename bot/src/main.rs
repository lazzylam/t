use teloxide::{prelude::*, dispatching::UpdateFilterExt, utils::command::BotCommands};
use teloxide::types::ChatId;
use bot::{admin::{AdminCommand, handle_command}, message::handle_message, database::Database};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    log::info!("Bot anti-gcast dimulai...");

    // Init ML model sekali saat startup
    ml::init_model().await;

    let bot = Bot::from_env();
    let db = Database::init().await;

    let db_message = db.clone();

    Dispatcher::builder(bot.clone(), Update::filter_message())
        .branch(
            dptree::entry()
                .filter_command::<AdminCommand>()
                .endpoint(move |bot: Bot, msg: Message, cmd: AdminCommand| {
                    let db = db.clone();
                    async move { handle_command(bot, db, msg, cmd).await }
                }),
        )
        .default_handler(move |bot: Bot, msg: Message| {
            let db = db_message.clone();
            async move { handle_message(bot, db, msg).await.unwrap_or(()) }
        })
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}