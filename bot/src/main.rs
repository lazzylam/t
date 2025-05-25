use teloxide::{prelude::*, dispatching::UpdateFilterExt, utils::command::BotCommands};
use crate::{admin::{AdminCommand, handle_command}, message::handle_message, database::Database};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    log::info!("Bot anti-gcast dimulai...");


    let bot = Bot::from_env();
    let db = Database::init().await;

    let db_message = db.clone();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<AdminCommand>()
                .endpoint(move |bot: Bot, msg: Message, cmd: AdminCommand| {
                    let db = db.clone();
                    async move { handle_command(bot, db, msg, cmd).await }
                })
        )
        .branch(
            Update::filter_message()
                .endpoint(move |bot: Bot, msg: Message| {
                    let db = db_message.clone();
                    async move { handle_message(bot, db, msg).await.unwrap_or(()) }
                })
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}