use dashmap::DashMap;
use poise::serenity_prelude::{ReactionType, UserId};
use serde::{Deserialize, Serialize};

pub const GUILD_ID: u64 = 239205378406088704;
pub const GENERAL_CHAT: u64 = 239205378406088704;

pub type Ctx<'a> = poise::Context<'a, Data, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserMetadata {
    pub reaction: Option<ReactionType>,
    pub fantasy_team_name: Option<String>,
}

#[derive(Default)]
pub struct Data {
    pub users: DashMap<UserId, UserMetadata>,
}
