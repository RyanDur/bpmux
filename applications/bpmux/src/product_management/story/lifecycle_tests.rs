use super::enums::{Acceptor, BoardState};
use super::lifecycle::{Conclusion, Lifecycle, LifecycleFields, OnBoard};
use super::stage::Stage;

fn all_lifecycles() -> Vec<Lifecycle> {
    vec![
        Lifecycle::Backlog,
        Lifecycle::OnBoard(OnBoard::Ready),
        Lifecycle::OnBoard(OnBoard::InProgress(Stage::Dev.id())),
        Lifecycle::OnBoard(OnBoard::InProgress(Stage::DeskCheck.id())),
        Lifecycle::OnBoard(OnBoard::Acceptance(Acceptor::Team)),
        Lifecycle::OnBoard(OnBoard::Acceptance(Acceptor::Owner)),
        Lifecycle::OnBoard(OnBoard::Acceptance(Acceptor::Dev)),
        Lifecycle::Concluded(Conclusion::Accepted(Acceptor::Team)),
        Lifecycle::Concluded(Conclusion::Accepted(Acceptor::Owner)),
        Lifecycle::Concluded(Conclusion::Accepted(Acceptor::Dev)),
        Lifecycle::Concluded(Conclusion::Retired),
    ]
}

fn canonical_fields() -> Vec<LifecycleFields> {
    vec![
        LifecycleFields {
            state: BoardState::Icebox,
            is_closed: false,
            placement: None,
            acceptor: None,
        },
        LifecycleFields {
            state: BoardState::Ready,
            is_closed: false,
            placement: None,
            acceptor: None,
        },
        LifecycleFields {
            state: BoardState::InProgress,
            is_closed: false,
            placement: Some(Stage::Dev.id()),
            acceptor: None,
        },
        LifecycleFields {
            state: BoardState::InProgress,
            is_closed: false,
            placement: Some(Stage::DeskCheck.id()),
            acceptor: None,
        },
        LifecycleFields {
            state: BoardState::Acceptance,
            is_closed: false,
            placement: None,
            acceptor: Some(Acceptor::Owner),
        },
        LifecycleFields {
            state: BoardState::Accepted,
            is_closed: true,
            placement: None,
            acceptor: Some(Acceptor::Team),
        },
        LifecycleFields {
            state: BoardState::Retired,
            is_closed: true,
            placement: None,
            acceptor: None,
        },
    ]
}

#[test]
fn every_lifecycle_value_round_trips_through_its_fields() {
    for lc in all_lifecycles() {
        let f = lc.to_fields();
        let back = Lifecycle::from_fields(&f.state, f.is_closed, f.placement.clone(), f.acceptor);
        assert_eq!(
            back, lc,
            "lifecycle must survive projection to fields and back: {lc:?}"
        );
    }
}

#[test]
fn canonical_fields_round_trip_through_the_lifecycle() {
    for f in canonical_fields() {
        let lc = Lifecycle::from_fields(&f.state, f.is_closed, f.placement.clone(), f.acceptor);
        assert_eq!(
            lc.to_fields(),
            f,
            "canonical fields must survive projection to lifecycle and back: {f:?}"
        );
    }
}

#[test]
fn a_closed_non_terminal_story_projects_to_retired() {
    let lc = Lifecycle::from_fields(&BoardState::InProgress, true, Some(Stage::Dev.id()), None);
    assert_eq!(
        lc,
        Lifecycle::Concluded(Conclusion::Retired),
        "a closed in-progress story is retired — is_closed folds a non-terminal position to Retired"
    );
}

#[test]
fn an_unplaced_in_progress_story_holds_the_first_middle_stage() {
    let lc = Lifecycle::from_fields(&BoardState::InProgress, false, None, None);
    assert_eq!(
        lc,
        Lifecycle::OnBoard(OnBoard::InProgress(Stage::Dev.id())),
        "in-progress with no placement holds the first middle stage"
    );
}

#[test]
fn an_acceptance_story_without_an_acceptor_normalizes_to_team() {
    let lc = Lifecycle::from_fields(&BoardState::Acceptance, false, None, None);
    assert_eq!(
        lc,
        Lifecycle::OnBoard(OnBoard::Acceptance(Acceptor::Team)),
        "acceptance with no assigned acceptor normalizes to team — mirrors gate_class_from_acceptor(None)"
    );
}
