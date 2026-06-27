use std::collections::HashMap;

use crate::product_management::story::Story;

#[derive(Default)]
pub struct BoardState {
    pub stories: HashMap<u64, Story>,
}
