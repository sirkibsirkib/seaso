use core::hash::Hash;

use std::collections::HashMap;

type Index = u16;

pub struct Indexer<T: Hash + Eq + Clone> {
    index_to_item: Vec<T>, // Index -> T
    item_to_index: HashMap<T, Index>,
    next_index: Option<Index>,
}

pub enum IndexErr {
    IndexesSpent,
}
impl<T: Hash + Eq + Clone> Default for Indexer<T> {
    fn default() -> Self {
        Self {
            index_to_item: Default::default(),
            item_to_index: Default::default(),
            next_index: Some(0),
        }
    }
}

impl<T: Hash + Eq + Clone> Indexer<T> {
    fn index(&mut self, item: T) -> Result<Index, IndexErr> {
        if let Some(&index) = self.item_to_index.get(&item) {
            Ok(index)
        } else {
            let index = self.next_index.take().ok_or(IndexErr::IndexesSpent)?;
            self.next_index = index.checked_add(1);
            self.item_to_index.insert(item.clone(), index);
            self.index_to_item.push(item);
            Ok(index)
        }
    }
}
