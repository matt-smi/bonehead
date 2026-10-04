use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

use crate::{outbound::Outbound, scheduler::Scheduler, state::AppState};

pub enum Outcome {
    Continue,
    Done,
}

#[derive(Clone)]
pub struct JobContext {
    pub state: Arc<AppState>,
    pub out: mpsc::Sender<Outbound>,
    pub scheduler: Scheduler,
}

#[derive(Clone, Debug)]
pub struct JobOptions {
    pub timeout: Duration,     // if the run takes longer than this, cancel it
    pub run_immediately: bool, // run on registration/start-up
}
impl Default for JobOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            run_immediately: false,
        }
    }
}

#[async_trait]
pub trait Job {
    fn name(&self) -> &str;
    async fn run(&mut self, ctx: &JobContext) -> Outcome;

    // Implementations can override this method to customize JobOptions
    fn options(&self) -> JobOptions {
        JobOptions::default()
    }
}
