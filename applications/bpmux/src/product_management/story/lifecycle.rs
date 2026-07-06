use crate::workflow::StageId;

use super::enums::{Acceptor, BoardState};
use super::stage::Stage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lifecycle {
    Backlog,
    OnBoard(OnBoard),
    Concluded(Conclusion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnBoard {
    Ready,
    InProgress(StageId),
    Acceptance(Acceptor),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conclusion {
    Accepted(Acceptor),
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleFields {
    pub state: BoardState,
    pub is_closed: bool,
    pub placement: Option<StageId>,
    pub acceptor: Option<Acceptor>,
}

fn effective_position(state: &BoardState, is_closed: bool) -> BoardState {
    if is_closed && !matches!(state, BoardState::Accepted | BoardState::Retired) {
        BoardState::Retired
    } else {
        state.clone()
    }
}

impl Lifecycle {
    pub fn from_fields(
        state: &BoardState,
        is_closed: bool,
        placement: Option<StageId>,
        acceptor: Option<Acceptor>,
    ) -> Lifecycle {
        match effective_position(state, is_closed) {
            BoardState::Icebox => Lifecycle::Backlog,
            BoardState::Ready => Lifecycle::OnBoard(OnBoard::Ready),
            BoardState::InProgress => Lifecycle::OnBoard(OnBoard::InProgress(
                placement.unwrap_or_else(|| Stage::Dev.id()),
            )),
            BoardState::Acceptance => {
                Lifecycle::OnBoard(OnBoard::Acceptance(acceptor.unwrap_or(Acceptor::Team)))
            }
            BoardState::Accepted => {
                Lifecycle::Concluded(Conclusion::Accepted(acceptor.unwrap_or(Acceptor::Team)))
            }
            BoardState::Retired => Lifecycle::Concluded(Conclusion::Retired),
        }
    }

    pub fn to_fields(&self) -> LifecycleFields {
        match self {
            Lifecycle::Backlog => LifecycleFields {
                state: BoardState::Icebox,
                is_closed: false,
                placement: None,
                acceptor: None,
            },
            Lifecycle::OnBoard(OnBoard::Ready) => LifecycleFields {
                state: BoardState::Ready,
                is_closed: false,
                placement: None,
                acceptor: None,
            },
            Lifecycle::OnBoard(OnBoard::InProgress(stage)) => LifecycleFields {
                state: BoardState::InProgress,
                is_closed: false,
                placement: Some(stage.clone()),
                acceptor: None,
            },
            Lifecycle::OnBoard(OnBoard::Acceptance(acceptor)) => LifecycleFields {
                state: BoardState::Acceptance,
                is_closed: false,
                placement: None,
                acceptor: Some(*acceptor),
            },
            Lifecycle::Concluded(Conclusion::Accepted(acceptor)) => LifecycleFields {
                state: BoardState::Accepted,
                is_closed: true,
                placement: None,
                acceptor: Some(*acceptor),
            },
            Lifecycle::Concluded(Conclusion::Retired) => LifecycleFields {
                state: BoardState::Retired,
                is_closed: true,
                placement: None,
                acceptor: None,
            },
        }
    }
}
