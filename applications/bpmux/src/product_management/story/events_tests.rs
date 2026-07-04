#![allow(clippy::unwrap_used)]

use chrono::{TimeZone, Utc};

use super::events::{EventMeta, event_wins};
use crate::product_management::story::peer::PeerId;

fn meta(actor: &str, ts_year: i32, peer: &str) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        ts: Utc.with_ymd_and_hms(ts_year, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named(peer),
    }
}

#[test]
fn event_wins_later_ts_beats_earlier() {
    let earlier = meta("a", 2025, "local");
    let later = meta("b", 2026, "local");
    assert!(event_wins(&later, &earlier));
    assert!(!event_wins(&earlier, &later));
}

#[test]
fn event_wins_tiebreak_by_peer_id_lexical_when_ts_equal() {
    let a = meta("x", 2026, "aaa");
    let b = meta("y", 2026, "zzz");
    assert!(event_wins(&b, &a));
    assert!(!event_wins(&a, &b));
}
