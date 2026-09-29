use chrono::{Datelike, TimeZone, Weekday};
use chrono_tz::Tz;
use poise::serenity_prelude as serenity;
use std::sync::Arc;

fn next_occurrence(tz: Tz, weekday: Weekday, hour: u32, minute: u32) -> chrono::DateTime<Tz> {
    let now = chrono::Utc::now().with_timezone(&tz);
    let mut date = now.date_naive();

    loop {
        if date.weekday() == weekday {
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

pub async fn monday_loop(http: Arc<serenity::Http>, channel: serenity::ChannelId) {
    let tz = chrono_tz::America::Los_Angeles;

    loop {
        let next = next_occurrence(tz, Weekday::Mon, 9, 30); // 9:30am Monday
        let now = chrono::Utc::now().with_timezone(&tz);
        let wait = (next - now).to_std().unwrap_or_default();

        tokio::time::sleep(wait).await;
        run_monday_job(&http, channel).await;

        // buffer so the same slot can't fire twice if the clock jitters
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

const ROSTER_ROLE_ID: u64 = 1554000251579539596;

async fn run_monday_job(http: &serenity::Http, channel: serenity::ChannelId) {
    let role = serenity::RoleId::new(ROSTER_ROLE_ID);

    let message = serenity::CreateMessage::new()
        .content(format!(
            "<@&{}> Reminder: please select your roster for this week in fantasy.",
            role.get()
        ))
        .allowed_mentions(serenity::CreateAllowedMentions::new().roles([role]));

    if let Err(e) = channel.send_message(http, message).await {
        eprintln!("monday job failed: {e}");
    }
}
