use std::time::Duration;

use dashmap::DashMap;
use poise::serenity_prelude::{ReactionType, UserId};
use serde::{Deserialize, Serialize};

use crate::Error;

const SAVE_FILE: &str = "data.json";
const HTTP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserMetadata {
    pub reaction: Option<ReactionType>,
    pub fantasy_team_name: Option<String>,
}

pub struct AppState {
    pub http: reqwest::Client,
    pub users: DashMap<UserId, UserMetadata>,
}
impl AppState {
    pub fn new() -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(HTTP_USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            users: DashMap::new(),
        })
    }

    pub async fn save(&self) -> Result<(), Error> {
        let json = serde_json::to_string_pretty(&self.users)?;
        tokio::fs::write(SAVE_FILE, json).await?;
        Ok(())
    }

    pub async fn load(&mut self) {
        match tokio::fs::read_to_string(SAVE_FILE).await {
            Ok(json) => {
                self.users = serde_json::from_str(&json).unwrap_or_default();
            }
            Err(_) => {}
        }
    }
}
