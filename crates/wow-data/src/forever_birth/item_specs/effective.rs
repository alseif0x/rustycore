//! Final DB2 records -> item override ID index; 02245dcd DB2Stores.cpp:1380-1381.
use super::{GemPropertiesRecord, ItemSpecOverrideRecord, ItemSpecRecord, ItemSpecRecords};
use crate::Db2HotfixRemovalStoreLikeCpp;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;
pub const ITEM_SPEC_HASH: u32 = 0x08DA_6E2A;
pub const ITEM_SPEC_OVERRIDE_HASH: u32 = 0x149A_AE79;
pub const GEM_PROPERTIES_HASH: u32 = 0x9C00_EA6D;

pub struct ItemSpecCatalog {
    specs: BTreeMap<u32, ItemSpecRecord>,
    overrides: BTreeMap<u32, ItemSpecOverrideRecord>,
    gems: BTreeMap<u32, GemPropertiesRecord>,
    overrides_by_item: BTreeMap<u32, Vec<u32>>,
}
impl ItemSpecRecords {
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<ItemSpecCatalog> {
        let specs = compose(
            self.specs,
            official.specs,
            custom.specs,
            ITEM_SPEC_HASH,
            removals,
            |r| r.id,
        )?;
        let overrides = compose(
            self.overrides,
            official.overrides,
            custom.overrides,
            ITEM_SPEC_OVERRIDE_HASH,
            removals,
            |r| r.id,
        )?;
        let gems = compose(
            self.gems,
            official.gems,
            custom.gems,
            GEM_PROPERTIES_HASH,
            removals,
            |r| r.id,
        )?;
        let mut overrides_by_item = BTreeMap::<u32, Vec<u32>>::new();
        for r in overrides.values() {
            overrides_by_item.entry(r.item).or_default().push(r.id);
        }
        Ok(ItemSpecCatalog {
            specs,
            overrides,
            gems,
            overrides_by_item,
        })
    }
}
impl ItemSpecCatalog {
    pub fn spec(&self, id: u32) -> Option<&ItemSpecRecord> {
        self.specs.get(&id)
    }
    pub fn override_record(&self, id: u32) -> Option<&ItemSpecOverrideRecord> {
        self.overrides.get(&id)
    }
    pub fn specs(&self) -> impl Iterator<Item = &ItemSpecRecord> {
        self.specs.values()
    }
    /// Some with unresolved specialization references still selects the source
    /// override branch; never turn it into fallback ItemSpec matching.
    pub fn overrides(&self, item: u32) -> Option<impl Iterator<Item = &ItemSpecOverrideRecord>> {
        let ids = self.overrides_by_item.get(&item)?;
        Some(ids.iter().map(|id| &self.overrides[id]))
    }
    pub fn gem(&self, id: u32) -> Option<&GemPropertiesRecord> {
        self.gems.get(&id)
    }
    pub fn counts(&self) -> [usize; 3] {
        [self.specs.len(), self.overrides.len(), self.gems.len()]
    }
}
fn compose<T>(
    baseline: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    hash: u32,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    id: impl Fn(&T) -> u32,
) -> Result<BTreeMap<u32, T>> {
    let mut rows = BTreeMap::new();
    for r in baseline {
        ensure!(
            rows.insert(id(&r), r).is_none(),
            "Duplicate item spec baseline ID"
        );
    }
    for r in official.into_iter().chain(custom) {
        rows.insert(id(&r), r);
    }
    rows.retain(|&id, _| !removals.contains_like_cpp(hash, id as i32));
    Ok(rows)
}

#[cfg(test)]
mod tests;
