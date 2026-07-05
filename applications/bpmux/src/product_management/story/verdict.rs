use crate::workflow::StageId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Succeed { step: StageId },
    Fail { step: StageId },
}
