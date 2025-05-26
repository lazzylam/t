use teloxide::{prelude::*, dispatching::UpdateFilterExt};
use teloxide::error_handlers::ErrorHandler;
use futures_util::future::BoxFuture;
use std::sync::Arc;

mod admin;
mod message;
mod database;
mod models;

use admin::{AdminCommand};
use message::{cleanup_old_messages, handle_message_predictive}; // Added predictive handler
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
    let db_predictive = db.clone(); // Clone untuk predictive handler

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<AdminCommand>()
                .endpoint(move |bot: Bot, msg: Message, cmd: AdminCommand| {
                    let db = db_admin.clone();
                    async move { 
                        // Wrap dengan error handling yang tidak mengganggu performa
                        match admin::handle_command(bot, db, msg, cmd).await {
                            Ok(_) => Ok::<(), teloxide::RequestError>(()),
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
                    let db_msg = db_message.clone();
                    let db_pred = db_predictive.clone();
                    async move {
                        // Jalankan kedua handler secara paralel untuk performa optimal
                        let (result1, result2) = tokio::join!(
                            message::handle_message(bot.clone(), db_msg, msg.clone()),
                            handle_message_predictive(bot, db_pred, msg)
                        );

                        // Handle errors dari kedua handler
                        if let Err(e) = result1 {
                            log::debug!("Message handling error: {:?}", e);
                        }
                        if let Err(e) = result2 {
                            log::debug!("Predictive handling error: {:?}", e);
                        }

                        Ok::<(), teloxide::RequestError>(())
                    }
                })
        );

    // Optimized dispatcher dengan custom error handler
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .error_handler(Arc::new(LoggingErrorHandler::new()))
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
    E: Debug + Send + 'static,
{
    fn handle_error(self: std::sync::Arc<Self>, error: E) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            log::debug!("Dispatcher error: {:?}", error);
        })
    }
}