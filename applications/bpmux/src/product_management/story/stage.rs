use workflow::StageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Dev,
    DeskCheck,
}

impl Stage {
    pub fn id(self) -> StageId {
        match self {
            Stage::Dev => StageId::from("in-progress"),
            Stage::DeskCheck => StageId::from("desk-check"),
        }
    }

    pub fn from_id(id: &StageId) -> Option<Stage> {
        match id.as_str() {
            "in-progress" => Some(Stage::Dev),
            "desk-check" => Some(Stage::DeskCheck),
            _ => None,
        }
    }
}
