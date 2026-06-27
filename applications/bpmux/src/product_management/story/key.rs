use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryKey {
    Pending(Uuid),
    Saved(u64),
}
