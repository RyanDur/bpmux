use super::CiSnapshot;

#[derive(Clone, Debug)]
pub enum CiAction {
    StartPolling,
    StopPolling,
    Updated(CiSnapshot),
}
