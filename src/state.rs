use std::time::Duration;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

type ReactionType = String;
type UserId = u64;

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
    pub fn new() -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            users: DashMap::new(),
        })
    }
}
