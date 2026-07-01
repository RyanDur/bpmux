#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PollKind {
    Ci,
    Ingest,
    Hydration,
}
