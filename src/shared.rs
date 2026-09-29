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

pub async fn save_data(data: &Data) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(&data.users)?;
    tokio::fs::write("data.json", json).await?;
    Ok(())
}

pub async fn load_data() -> Data {
    match tokio::fs::read_to_string("data.json").await {
        Ok(json) => {
            let users: DashMap<UserId, UserMetadata> =
                serde_json::from_str(&json).unwrap_or_default();
            Data { users }
        }
        Err(_) => Data::default(),
    }
}
