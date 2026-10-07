use chrono::{Datelike, TimeZone, Weekday};
use chrono_tz::Tz;
use poise::serenity_prelude as serenity;
use std::collections::HashSet;
use std::sync::Arc;

use crate::Error;
use crate::fantrax::get_standings;
use crate::goal::spawn_missing_pollers;
use crate::leaderboard::{LeaderboardRow, fetch_avatar_bytes, generate_leaderboard_image};
use crate::state::AppState;

const ROSTER_ROLE_ID: u64 = 1554000251579539596;

fn next_occurrence(
    tz: Tz,
    weekday: Option<Weekday>,
    hour: u32,
    minute: u32,
) -> chrono::DateTime<Tz> {
    let now = chrono::Utc::now().with_timezone(&tz);
    let mut date = now.date_naive();

    loop {
        if weekday.map_or(true, |wd| date.weekday() == wd) {
            let naive = date.and_hms_opt(hour, minute, 0).unwrap();
            if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
                if dt > now {
                    return dt;
                }
            }
        }
        date = date.succ_opt().unwrap();
    }
}

pub async fn fantasy_leaderboard(
    http: Arc<serenity::Http>,
    channel: serenity::ChannelId,
    app_state: Arc<AppState>,
) {
    let tz = chrono_tz::America::Los_Angeles;

    loop {
        let next = next_occurrence(tz, Some(Weekday::Sun), 10 + 12, 30); // 10:30pm Sunday
        let now = chrono::Utc::now().with_timezone(&tz);
        let wait = (next - now).to_std().unwrap_or_default();

        tokio::time::sleep(wait).await;

        if let Err(e) = build_and_send_leaderboard(&http, channel, &app_state, true).await {
            eprintln!("monday job failed: {e}");
        }

        // buffer so the same slot can't fire twice if the clock jitters
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub async fn play_by_play(
    http: Arc<serenity::Http>,
    channel: serenity::ChannelId,
    app_state: Arc<AppState>,
) {
    let tz = chrono_tz::America::Vancouver;
    let mut tracked: HashSet<u64> = HashSet::new(); // lives for the whole task

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        match spawn_missing_pollers(&http, &app_state.http, channel, &mut tracked).await {
            Ok(n) => println!(
                "[schedule] spawned {n} new pollers ({} tracked)",
                tracked.len()
            ),
            Err(e) => eprintln!("[schedule] failed to fetch schedule: {e}"),
        }

        let next = next_occurrence(tz, None, 0, 0); // next midnight
        let now = chrono::Utc::now().with_timezone(&tz);
        let wait = (next - now).to_std().unwrap_or_default();
        tokio::time::sleep(wait).await;
    }
}

async fn build_and_send_leaderboard(
    http: &serenity::Http,
    channel: serenity::ChannelId,
    app_state: &Arc<AppState>,
    ping_role: bool,
) -> Result<(), Error> {
    let standings = get_standings().await.unwrap();

    let mut rows = Vec::new();
    for team in &standings {
        let owner_id = app_state
            .users
            .iter()
            .find(|entry| entry.fantasy_team_name.as_deref() == Some(team.team_name.as_str()))
            .map(|entry| *entry.key());

        let (owner_display, avatar_bytes) = match owner_id {
            Some(user_id) => {
                let user = user_id.to_user(http).await?;
                let bytes = fetch_avatar_bytes(&user.face()).await;
                (format!("@{}", user.name), bytes)
            }
            None => ("Unclaimed".to_string(), None),
        };

        rows.push(LeaderboardRow {
            rank: team.rank,
            team_name: team.team_name.clone(),
            record: team.points.clone(),
            owner_display,
            avatar_bytes,
        });
    }

    let png_bytes = generate_leaderboard_image(&rows)?;
    let attachment = serenity::CreateAttachment::bytes(png_bytes, "leaderboard.png");
    let embed = serenity::CreateEmbed::new()
        .title("🏆 Fantasy Standings")
        .image("attachment://leaderboard.png")
        .color(0x5865F2);

    let mut message = serenity::CreateMessage::new()
        .embed(embed)
        .add_file(attachment);

    if ping_role {
        let role = serenity::RoleId::new(ROSTER_ROLE_ID);
        message = message
            .content(format!(
                "<@&{}> Reminder: select your roster for this week in Fantasy",
                role.get()
            ))
            .allowed_mentions(serenity::CreateAllowedMentions::new().roles([role]));
    }

    channel.send_message(http, message).await?;
    Ok(())
}
