use serde::{Deserialize, Serialize};

use workflow::StageId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Succeed { step: StageId },
    Fail { step: StageId },
}

impl Verdict {
    pub fn is_fail(&self) -> bool {
        matches!(self, Verdict::Fail { .. })
    }
}
