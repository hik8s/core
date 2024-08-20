use uuid::Uuid;

use super::item::Item;

#[derive(Clone)]
pub struct Class {
    pub items: Vec<Item>,
    pub count: i32,
    pub length: usize,
    pub id: String,
}
impl Class {
    pub fn new(texts: Vec<String>) -> Self {
        let items: Vec<Item> = texts.into_iter().map(Item::Fix).collect();
        Self {
            length: items.len().clone(),
            items,
            count: 1,
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn mask_items(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| match item {
                Item::Fix(text) => text.clone(),
                _ => "".to_string(),
            })
            .collect()
    }

    pub fn update_items(&mut self, texts: &Vec<String>) {
        for (item, text) in self.items.iter_mut().zip(texts) {
            if let Item::Fix(item_str) = item {
                if text != item_str {
                    *item = Item::Var;
                }
            }
        }
    }
}
