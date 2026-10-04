#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub timezone: Tz,
    pub guild_id: u64,
    pub channels: Channels,
    #[serde(default)]
    pub roles: Roles,
    #[serde(default)]
    pub nhl: Nhl,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channels {
    pub default: u64,
    pub hockey: u64,
}
