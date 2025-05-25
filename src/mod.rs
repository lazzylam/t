mod admin;
mod message;

pub use admin::AdminCommand;
pub use admin::admin_command_handler;
pub use message::handle_message;
pub use message::SpamDetector;