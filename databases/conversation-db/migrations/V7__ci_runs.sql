-- the delivery ledger: every conclusive CI outcome observed, keyed by run
CREATE TABLE ci_runs (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id      INTEGER NOT NULL,
    state       TEXT NOT NULL,
    observed_at INTEGER NOT NULL
) STRICT;
CREATE INDEX idx_ci_runs_observed_at ON ci_runs(observed_at);
