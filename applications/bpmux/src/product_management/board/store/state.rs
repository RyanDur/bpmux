use std::collections::HashMap;

use crate::product_management::board::item::ProductItem;
use crate::product_management::story::StoryKey;

#[derive(Default)]
pub struct BoardState {
    pub stories: HashMap<StoryKey, ProductItem>,
}
