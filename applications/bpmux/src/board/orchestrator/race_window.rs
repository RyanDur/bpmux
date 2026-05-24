pub struct RaceWindowBarrier {
    target: usize,
    arrival_barrier: tokio::sync::Barrier,
    leader_done_tx: tokio::sync::watch::Sender<bool>,
    leader_done_rx: tokio::sync::watch::Receiver<bool>,
}

impl RaceWindowBarrier {
    pub fn new(target: usize) -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        Self {
            target,
            arrival_barrier: tokio::sync::Barrier::new(target.max(1)),
            leader_done_tx: tx,
            leader_done_rx: rx,
        }
    }

    pub async fn arrive(&self) {
        if self.target == 0 {
            return;
        }
        let is_leader = self.arrival_barrier.wait().await.is_leader();
        if is_leader {
            return;
        }
        let mut rx = self.leader_done_rx.clone();
        loop {
            if *rx.borrow_and_update() {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    }

    pub fn complete(&self) {
        if self.target == 0 {
            return;
        }
        let _ = self.leader_done_tx.send(true);
    }
}
