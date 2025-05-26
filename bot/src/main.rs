use teloxide::{prelude::*, dispatching::UpdateFilterExt};
use teloxide::error_handlers::ErrorHandler;
use futures_util::future::BoxFuture;

mod admin;
mod message;
mod database;
mod models;

use admin::{AdminCommand, handle_command};
use message::{handle_message, cleanup_old_messages};
use database::Database;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    log::info!("🚀 Bot anti-gcast ultra-fast dimulai...");

    let bot = Bot::from_env();
    let db = Database::init().await;

    // Start background cleanup task
    cleanup_old_messages().await;

    // Clone untuk menghindari move issues
    let db_message = db.clone();
    let db_admin = db.clone();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<AdminCommand>()
                .endpoint(move |bot: Bot, msg: Message, cmd: AdminCommand| {
                    let db = db_admin.clone();
                    async move { 
                        // Wrap dengan error handling yang tidak mengganggu performa
                        match handle_command(bot, db, msg, cmd).await {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                log::warn!("Admin command error: {:?}", e);
                                Ok(())
                            }
                        }
                    }
                })
        )
        .branch(
            Update::filter_message()
                .endpoint(move |bot: Bot, msg: Message| {
                    let db = db_message.clone();
                    async move { 
                        // Ultra-fast message handling dengan zero-latency error handling
                        match handle_message(bot, db, msg).await {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                log::debug!("Message handling error: {:?}", e);
                                Ok(())
                            }
                        }
                    }
                })
        );

    // Optimized dispatcher dengan custom error handler
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .error_handler(LoggingErrorHandler::new())
        .build()
        .dispatch()
        .await;
}

// Custom error handler yang tidak memperlambat performa
use std::fmt::Debug;

struct LoggingErrorHandler;

impl LoggingErrorHandler {
    fn new() -> Self {
        Self
    }
}

impl<E> ErrorHandler<E> for LoggingErrorHandler
   where
       E: Debug + Send,
{
    fn handle_error(self: std::sync::Arc<Self>, error: E) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            log::debug!("Dispatcher error: {:?}", error);
        })
    }
}