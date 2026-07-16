use super::CiSnapshot;

#[derive(Clone, Debug)]
pub enum CiAction {
    StartPolling,
    PollingStarted,
    StopPolling,
    PollingStopped,
    Updated(CiSnapshot),
    PollSucceeded { at: String, updated: bool },
}
