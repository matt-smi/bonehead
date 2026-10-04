use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::{str::FromStr, time::Duration};

#[derive(Clone, Debug)]
pub enum Schedule {
    Every(Duration),
    Cron(cron::Schedule),
}
impl Schedule {
    pub fn every(period: Duration) -> Self {
        Schedule::Every(period)
    }

    pub fn cron(expr: &str) -> Result<Self, cron::error::Error> {
        let parsed = cron::Schedule::from_str(expr)?;
        Ok(Schedule::Cron(parsed))
    }
}

//
// impl Schedule {
//     pub fn every(period: Duration) -> Self {
//         Schedule::Every(period)
//     }
//
//     pub fn cron(expr: &str) -> Result<Self> {
//         let parsed = cron::Schedule::from_str(expr)?; // todo: better error handling
//         Ok(Schedule::Cron(parsed))
//     }
//
//     pub fn next_fire(&self, from: DateTime<Utc>, tz: Tz) -> Option<NextFire> {
//         match self {
//             Schedule::Every(d) => Some(NextFire::After(*d)),
//             Schedule::Cron(c) => c
//                 .after(&from.with_timezone(&tz))
//                 .next()
//                 .map(|dt| NextFire::At(dt.with_timezone(&Utc))),
//         }
//     }
// }
