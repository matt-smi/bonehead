use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::time::{MissedTickBehavior, interval};

const POLL_SECS: u64 = 10;
const MAX_SEND_ATTEMPTS: u32 = 5;
const HEARTBEAT_EVERY: u64 = 30; // polls (~5 minutes)

// ---- data types (unchanged) ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayByPlay {
    game_state: String,
    away_team: Team,
    home_team: Team,
    #[serde(default)]
    plays: Vec<Play>,
    #[serde(default)]
    roster_spots: Vec<RosterSpot>,
}

#[derive(Deserialize)]
struct Team {
    id: u64,
    abbrev: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Play {
    event_id: u64,
    type_desc_key: String,
    time_in_period: String,
    period_descriptor: PeriodDescriptor,
    details: Option<Details>,
}

#[derive(Deserialize)]
struct PeriodDescriptor {
    number: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Details {
    event_owner_team_id: Option<u64>,
    scoring_player_id: Option<u64>,
    assist1_player_id: Option<u64>,
    assist2_player_id: Option<u64>,
    home_score: Option<u32>,
    away_score: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RosterSpot {
    player_id: u64,
    first_name: Localized,
    last_name: Localized,
}

#[derive(Deserialize)]
struct Localized {
    default: String,
}

// ---- helpers ----

/// Returns None only when the scorer isn't populated yet (retry next poll).
/// Unknown player names fall back to "#<id>" so a goal is never stuck forever.
fn goal_message(play: &Play, pbp: &PlayByPlay, names: &HashMap<u64, String>) -> Option<String> {
    let d = play.details.as_ref()?;
    let scorer_id = d.scoring_player_id?;

    let name = |id: u64| names.get(&id).cloned().unwrap_or_else(|| format!("#{id}"));

    let team = if d.event_owner_team_id == Some(pbp.home_team.id) {
        &pbp.home_team.abbrev
    } else if d.event_owner_team_id == Some(pbp.away_team.id) {
        &pbp.away_team.abbrev
    } else {
        "???"
    };

    let assists: Vec<String> = [d.assist1_player_id, d.assist2_player_id]
        .into_iter()
        .flatten()
        .map(name)
        .collect();
    let assists = if assists.is_empty() {
        "unassisted".to_string()
    } else {
        assists.join(", ")
    };

    Some(format!(
        "🚨 **GOAL {team}!** {} ({assists})\n{} {} - {} {} · P{} {}",
        name(scorer_id),
        pbp.away_team.abbrev,
        d.away_score.unwrap_or(0),
        d.home_score.unwrap_or(0),
        pbp.home_team.abbrev,
        play.period_descriptor.number,
        play.time_in_period,
    ))
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<PlayByPlay, reqwest::Error> {
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

// ---- the poller ----

pub async fn poll_game(
    http: Arc<serenity::Http>,
    client: reqwest::Client,
    game_id: u64,
    channel: serenity::ChannelId,
) {
    let url = format!("https://api-web.nhle.com/v1/gamecenter/{game_id}/play-by-play");

    let mut announced: HashSet<u64> = HashSet::new(); // goal event IDs handled
    let mut send_failures: HashMap<u64, u32> = HashMap::new(); // goal event ID -> failed sends
    let mut last_type: HashMap<u64, String> = HashMap::new(); // event ID -> last typeDescKey
    let mut seeded = false;
    let mut last_state = String::new();
    let mut consecutive_failures: u32 = 0;
    let mut polls: u64 = 0;

    let mut ticker = interval(Duration::from_secs(POLL_SECS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    println!("[game {game_id}] starting play-by-play poller");

    loop {
        ticker.tick().await;

        let pbp = match fetch(&client, &url).await {
            Ok(p) => {
                if consecutive_failures > 0 {
                    println!(
                        "[game {game_id}] fetch recovered after {consecutive_failures} failures"
                    );
                }
                consecutive_failures = 0;
                p
            }
            Err(e) => {
                consecutive_failures += 1;
                // first failure, then about once a minute, to avoid spam
                if consecutive_failures == 1 || consecutive_failures % 6 == 0 {
                    eprintln!(
                        "[game {game_id}] fetch failed ({consecutive_failures} in a row): {e}"
                    );
                }
                continue;
            }
        };

        polls += 1;
        if polls % HEARTBEAT_EVERY == 0 {
            println!(
                "[game {game_id}] alive: state={}, plays={}, goals_handled={}",
                pbp.game_state,
                pbp.plays.len(),
                announced.len()
            );
        }

        if pbp.game_state != last_state {
            println!(
                "[game {game_id}] state changed: '{last_state}' -> '{}'",
                pbp.game_state
            );
            last_state = pbp.game_state.clone();
        }

        let names: HashMap<u64, String> = pbp
            .roster_spots
            .iter()
            .map(|r| {
                (
                    r.player_id,
                    format!("{} {}", r.first_name.default, r.last_name.default),
                )
            })
            .collect();

        // Log in-place type changes (e.g. a shot reclassified as a goal)
        for play in &pbp.plays {
            if let Some(prev) = last_type.insert(play.event_id, play.type_desc_key.clone()) {
                if prev != play.type_desc_key {
                    println!(
                        "[game {game_id}] event {} changed type: {prev} -> {}",
                        play.event_id, play.type_desc_key
                    );
                }
            }
        }

        // Goals only, re-scanned every poll
        for play in pbp.plays.iter().filter(|p| p.type_desc_key == "goal") {
            if announced.contains(&play.event_id) {
                continue;
            }

            if !seeded {
                println!(
                    "[game {game_id}] seeding existing goal (event {}, P{} {})",
                    play.event_id, play.period_descriptor.number, play.time_in_period
                );
                announced.insert(play.event_id);
                continue;
            }

            match goal_message(play, &pbp, &names) {
                Some(msg) => {
                    println!(
                        "[game {game_id}] goal detected (event {}), sending message",
                        play.event_id
                    );
                    match channel.say(&http, msg).await {
                        Ok(_) => {
                            announced.insert(play.event_id);
                        }
                        Err(e) => {
                            let n = send_failures.entry(play.event_id).or_insert(0);
                            *n += 1;
                            eprintln!(
                                "[game {game_id}] failed to send goal message (event {}, attempt {n}/{MAX_SEND_ATTEMPTS}): {e}",
                                play.event_id
                            );
                            if *n >= MAX_SEND_ATTEMPTS {
                                eprintln!("[game {game_id}] giving up on event {}", play.event_id);
                                announced.insert(play.event_id);
                            }
                        }
                    }
                }
                None => {
                    println!(
                        "[game {game_id}] goal (event {}) details not populated yet, will retry",
                        play.event_id
                    );
                }
            }
        }

        if !seeded {
            println!(
                "[game {game_id}] seeded with {} existing goals, not announcing them",
                announced.len()
            );
            seeded = true;
        }

        if matches!(pbp.game_state.as_str(), "OFF" | "FINAL") {
            println!("[game {game_id}] game over, stopping poller");
            break;
        }
    }
}
