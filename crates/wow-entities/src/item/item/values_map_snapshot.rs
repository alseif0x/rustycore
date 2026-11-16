use std::collections::HashMap;

use wow_core::ObjectGuid;

use crate::ItemValuesSnapshot;

use super::Item;

/// Ordered immutable Item facts captured from one owner's Item map.
#[derive(Debug, Clone)]
pub struct ItemValuesMapSnapshot {
    ordered_items: Vec<(ObjectGuid, ItemValuesSnapshot)>,
    indices: HashMap<ObjectGuid, usize>,
}

impl ItemValuesMapSnapshot {
    pub fn capture(items: &HashMap<ObjectGuid, Item>) -> Self {
        let mut ordered_items = Vec::with_capacity(items.len());
        let mut indices = HashMap::with_capacity(items.len());
        for (&guid, item) in items {
            let index = ordered_items.len();
            ordered_items.push((guid, ItemValuesSnapshot::capture(item)));
            indices.insert(guid, index);
        }
        Self {
            ordered_items,
            indices,
        }
    }

    pub fn get(&self, guid: &ObjectGuid) -> Option<&ItemValuesSnapshot> {
        self.indices
            .get(guid)
            .map(|&index| &self.ordered_items[index].1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ObjectGuid, &ItemValuesSnapshot)> {
        self.ordered_items.iter().map(|(guid, item)| (guid, item))
    }

    pub fn values(&self) -> impl Iterator<Item = &ItemValuesSnapshot> {
        self.ordered_items.iter().map(|(_, item)| item)
    }

    pub fn is_empty(&self) -> bool {
        self.ordered_items.is_empty()
    }
}

#[cfg(test)]
#[path = "values_map_snapshot/tests.rs"]
mod tests;
