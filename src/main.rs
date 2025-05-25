mod models;
mod database;
mod handlers;
mod utils;

use teloxide::prelude::*;
use database::MongoDb;
use handlers::{admin::AdminCommand, message::handle_message};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting Anti-Spam Bot with MongoDB...");

    // Initialize MongoDB
    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set");
    let db = MongoDb::new(&mongodb_uri, "antispam_bot")
        .await
        .expect("Failed to connect to MongoDB");

    let bot = Bot::from_env();
    let db_arc = Arc::new(Mutex::new(db));

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<AdminCommand>()
                .endpoint(handlers::admin::admin_command_handler)
        )
        .branch(
            dptree::filter(|msg: Message| msg.chat.is_group() || msg.chat.is_supergroup())
                .endpoint(handlers::message::handle_message),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db_arc])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}