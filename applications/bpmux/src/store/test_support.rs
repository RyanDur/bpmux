#![allow(clippy::panic)]

use std::time::Duration;

pub(crate) async fn eventually(label: &str, mut condition: impl FnMut() -> bool) {
    for _ in 0..400 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("eventually({label}): condition not satisfied within 2s");
}
