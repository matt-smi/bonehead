use std::sync::Arc;

use crate::state::AppState;

pub mod cron;
pub mod edit_distance;
pub mod fantrax;
pub mod goal;
pub mod leaderboard;
pub mod register;
pub mod state;

pub type Ctx<'a> = poise::Context<'a, Arc<AppState>, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub const GUILD_ID: u64 = 239205378406088704;
pub const GENERAL_CHAT: u64 = 239205378406088704;
pub const HOCKEY_PLAY_BY_PLAY: u64 = 1554669317931667546;
