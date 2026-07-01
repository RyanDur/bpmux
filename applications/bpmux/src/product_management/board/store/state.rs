use std::collections::HashMap;

use crate::product_management::story::{Story, StoryKey};

#[derive(Default)]
pub struct BoardState {
    pub stories: HashMap<StoryKey, Story>,
}
