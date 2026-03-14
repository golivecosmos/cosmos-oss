// Commands module - organizes all Tauri command handlers
pub mod app_installation;
pub mod audio;
pub mod config_commands;
pub mod debug;
pub mod drive_commands;
pub mod file_ops;
pub mod indexing;
pub mod job_management;
pub mod logging;
pub mod migration_commands;
pub mod models;
pub mod search;
pub mod system;
pub mod video;

// Re-export all commands for easy importing in main.rs
pub use app_installation::*;
pub use audio::*;
pub use config_commands::{get_config_info, set_custom_db_path};
pub use debug::*;
pub use drive_commands::*;
pub use file_ops::*;
pub use indexing::*;
pub use job_management::*;
pub use logging::*;
pub use migration_commands::*;
pub use models::*;
pub use search::*;
pub use system::*;
pub use video::*;

#[cfg(test)]
mod tests;
