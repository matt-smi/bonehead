use chrono::{DateTime, Duration as ChronoDuration, Utc};
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
const MAX_POLL_DURATION: Duration = Duration::from_secs(6 * 60 * 60); // safety net per game

/// How long after announcing a goal we keep checking for updated details
/// (late assists) or for the goal disappearing (overturned on review).
const EDIT_WINDOW: Duration = Duration::from_secs(10 * 60);

const SCHEDULE_URL: &str = "https://api-web.nhle.com/v1/club-schedule-season/VAN/now";

/// A game that started longer ago than this has to be over.
fn live_grace() -> ChronoDuration {
    ChronoDuration::hours(4)
}

/// Only spawn pollers for games starting within this window.
fn spawn_horizon() -> ChronoDuration {
    ChronoDuration::days(7)
}

// ---- data types ----

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

// ---- schedule types ----

#[derive(Deserialize)]
struct Schedule {
    games: Vec<ScheduleGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleGame {
    id: u64,
    #[serde(rename = "startTimeUTC")] // overrides the camelCase rename
    start_time_utc: DateTime<Utc>,
    game_state: String,
    #[serde(default)]
    game_schedule_state: String,
}

// ---- sent-message tracking ----

/// A goal message we posted, kept around so it can be edited later.
struct SentGoal {
    message_id: serenity::MessageId,
    /// What the message currently says in Discord.
    content: String,
    /// The most recent "live goal" text, used to build the waved-off version
    /// and to restore it if the goal reappears in the feed.
    goal_text: String,
    sent_at: tokio::time::Instant,
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

/// Strikes through each line of the original goal message and flags it as disallowed.
fn waved_off_message(goal_text: &str) -> String {
    let struck = goal_text
        .lines()
        .map(|l| format!("~~{l}~~"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("❌ **Goal disallowed**\n{struck}")
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

async fn fetch_schedule(client: &reqwest::Client) -> Result<Schedule, reqwest::Error> {
    client
        .get(SCHEDULE_URL)
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
    let mut sent: HashMap<u64, SentGoal> = HashMap::new(); // goal event ID -> message we sent
    let mut seeded = false;
    let mut last_state = String::new();
    let mut consecutive_failures: u32 = 0;
    let mut polls: u64 = 0;

    let mut ticker = interval(Duration::from_secs(POLL_SECS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let started = tokio::time::Instant::now();

    println!("[game {game_id}] starting play-by-play poller");

    loop {
        ticker.tick().await;

        if started.elapsed() > MAX_POLL_DURATION {
            eprintln!("[game {game_id}] exceeded max poll duration, stopping");
            break;
        }

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
                    match channel.say(&http, msg.clone()).await {
                        Ok(m) => {
                            announced.insert(play.event_id);
                            sent.insert(
                                play.event_id,
                                SentGoal {
                                    message_id: m.id,
                                    content: msg.clone(),
                                    goal_text: msg,
                                    sent_at: tokio::time::Instant::now(),
                                },
                            );
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

        // Re-check recently announced goals: fill in late assists, and mark goals
        // that disappear from the feed (overturned on review) as disallowed.
        sent.retain(|_, g| g.sent_at.elapsed() < EDIT_WINDOW);

        // An empty plays list is more likely an API hiccup than every goal being
        // overturned, so don't treat it as "all goals disappeared".
        if !pbp.plays.is_empty() {
            let current: HashMap<u64, &Play> = pbp
                .plays
                .iter()
                .filter(|p| p.type_desc_key == "goal")
                .map(|p| (p.event_id, p))
                .collect();

            for (event_id, g) in sent.iter_mut() {
                let (desired, is_live) = match current.get(event_id) {
                    Some(play) => match goal_message(play, &pbp, &names) {
                        Some(m) => (m, true),
                        None => continue,
                    },
                    None => (waved_off_message(&g.goal_text), false),
                };

                if desired == g.content {
                    continue;
                }

                match channel
                    .edit_message(
                        &http,
                        g.message_id,
                        serenity::EditMessage::new().content(desired.clone()),
                    )
                    .await
                {
                    Ok(_) => {
                        if is_live {
                            println!("[game {game_id}] updated goal message (event {event_id})");
                            g.goal_text = desired.clone();
                        } else {
                            println!(
                                "[game {game_id}] goal (event {event_id}) no longer in feed, marked disallowed"
                            );
                        }
                        g.content = desired;
                    }
                    Err(e) => eprintln!(
                        "[game {game_id}] failed to edit goal message (event {event_id}): {e}"
                    ),
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

// ---- scheduling ----

/// Sleeps until the scheduled start, then runs the normal poller.
/// If the start time has already passed (e.g. bot restarted mid-game),
/// it starts immediately; poll_game's seeding skips goals already scored.
async fn poll_game_at(
    start: DateTime<Utc>,
    http: Arc<serenity::Http>,
    client: reqwest::Client,
    game_id: u64,
    channel: serenity::ChannelId,
) {
    let wait = (start - Utc::now()).to_std().unwrap_or(Duration::ZERO);
    if !wait.is_zero() {
        println!(
            "[game {game_id}] waiting {}s until start ({start})",
            wait.as_secs()
        );
        tokio::time::sleep(wait).await;
    }
    poll_game(http, client, game_id, channel).await;
}

/// Spawns a (sleeping) poller for every unfinished game we aren't tracking yet
/// that starts within the horizon and isn't already long over.
/// Returns how many new pollers were spawned.
pub async fn spawn_missing_pollers(
    http: &Arc<serenity::Http>,
    client: &reqwest::Client,
    channel: serenity::ChannelId,
    tracked: &mut HashSet<u64>,
) -> Result<usize, reqwest::Error> {
    let schedule = fetch_schedule(client).await?;
    let now = Utc::now();
    let mut spawned = 0;

    for game in schedule.games {
        if tracked.contains(&game.id) {
            continue;
        }
        // Too far out: leave it untracked so a later run picks it up
        if game.start_time_utc > now + spawn_horizon() {
            continue;
        }
        // Past games: skip anything that started more than the grace window ago
        if game.start_time_utc < now - live_grace() {
            continue;
        }
        if matches!(game.game_state.as_str(), "OFF" | "FINAL") {
            continue;
        }
        // Postponed/cancelled: leave untracked so a reschedule gets picked up later
        if matches!(game.game_schedule_state.as_str(), "PPD" | "CNCL") {
            continue;
        }

        tracked.insert(game.id);
        tokio::spawn(poll_game_at(
            game.start_time_utc,
            http.clone(),
            client.clone(),
            game.id,
            channel,
        ));
        spawned += 1;
    }

    Ok(spawned)
}
