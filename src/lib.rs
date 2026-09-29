pub mod cron;
pub mod edit_distance;
pub mod fantrax;
pub mod leaderboard;
pub mod nhl;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
