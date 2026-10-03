//! DB2Store.cpp:127-133, DB2DatabaseLoader.cpp:27-174, final removal pass
//! in DB2Stores.cpp:1539-1548 at 02245dcd. Not a full ItemTemplate owner.
use super::{ItemEffectRecord, ItemRecord, ItemRecords};
use crate::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{
        item_quantities::{
            ItemClassQuantityRecord, ItemEffectQuantityRecord, ItemEffectRelationRecord,
            ItemQuantitySources,
        },
        item_sparse::SparseItemRecord,
    },
};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub const ITEM_HASH: u32 = 0x5023_8EC2;
pub const ITEM_SPARSE_HASH: u32 = 0x919B_E54E;
pub const ITEM_EFFECT_HASH: u32 = 0x4002_A5B1;
pub const ITEM_RELATION_HASH: u32 = 0x00CB_674F;

pub struct ItemCatalog {
    items: BTreeMap<u32, ItemRecord>,
    sparse: BTreeMap<u32, SparseItemRecord>,
    effects: BTreeMap<u32, ItemEffectRecord>,
    relations: BTreeMap<u32, ItemEffectRelationRecord>,
    unknown_baseline_records: [usize; 4],
}

impl ItemRecords {
    /// Overlays keep observed query order, including legal repeated IDs.
    /// No SQL producer/transactional snapshot/native acceptance is implied.
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<ItemCatalog> {
        Ok(ItemCatalog {
            items: compose(
                self.items,
                official.items,
                custom.items,
                ITEM_HASH,
                removals,
                |r| r.id,
            )?,
            sparse: compose_sparse(self.sparse, official.sparse, custom.sparse, removals)?,
            effects: compose(
                self.effects,
                official.effects,
                custom.effects,
                ITEM_EFFECT_HASH,
                removals,
                |r| r.id,
            )?,
            relations: compose(
                self.relations,
                official.relations,
                custom.relations,
                ITEM_RELATION_HASH,
                removals,
                |r| r.id,
            )?,
            unknown_baseline_records: self.unknown_baseline_records,
        })
    }
}

impl ItemCatalog {
    /// Called after numeric/enUS batches, before final publication. Missing
    /// locale IDs do not create a record (DB2DatabaseLoader::LoadStrings).
    pub fn with_sparse_locale(
        mut self,
        locale: u8,
        official: impl IntoIterator<Item = (u32, [Vec<u8>; 5])>,
        custom: impl IntoIterator<Item = (u32, [Vec<u8>; 5])>,
    ) -> Self {
        for (id, fields) in official.into_iter().chain(custom) {
            if let Some(row) = self.sparse.get_mut(&id) {
                row.strings.overlay(locale, fields);
            }
        }
        self
    }
    pub fn sparse_records(&self) -> impl Iterator<Item = &SparseItemRecord> {
        self.sparse.values()
    }
    pub fn item(&self, id: u32) -> Option<&ItemRecord> {
        self.items.get(&id)
    }
    pub fn sparse(&self, id: u32) -> Option<&SparseItemRecord> {
        self.sparse.get(&id)
    }
    pub fn effect(&self, id: u32) -> Option<&ItemEffectRecord> {
        self.effects.get(&id)
    }
    pub fn relations(&self) -> impl Iterator<Item = &ItemEffectRelationRecord> {
        self.relations.values()
    }
    pub fn relation(&self, id: u32) -> Option<&ItemEffectRelationRecord> {
        self.relations.get(&id)
    }

    /// Derived transient quantity view of final records, not another mutable
    /// record authority, template, inventory or permission to create an item.
    pub fn quantity_projection(&self) -> Result<ItemQuantitySources> {
        ItemQuantitySources::from_effective_records(
            self.items
                .values()
                .map(|r| ItemClassQuantityRecord {
                    id: r.id,
                    class: r.class,
                    subclass: r.subclass,
                })
                .collect(),
            self.sparse
                .values()
                .map(SparseItemRecord::quantity_projection)
                .collect(),
            self.effects
                .values()
                .map(|r| ItemEffectQuantityRecord {
                    id: r.id,
                    legacy_slot: r.legacy_slot,
                    spell_category: r.spell_category,
                })
                .collect(),
            self.relations.values().copied().collect(),
        )
    }

    pub fn counts(&self) -> [usize; 4] {
        [
            self.items.len(),
            self.sparse.len(),
            self.effects.len(),
            self.relations.len(),
        ]
    }
    pub fn unknown_baseline_records(&self) -> [usize; 4] {
        self.unknown_baseline_records
    }
}

fn compose_sparse(
    baseline: Vec<SparseItemRecord>,
    official: Vec<SparseItemRecord>,
    custom: Vec<SparseItemRecord>,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<BTreeMap<u32, SparseItemRecord>> {
    let mut rows = BTreeMap::new();
    for row in baseline {
        ensure!(
            rows.insert(row.id, row).is_none(),
            "Duplicate sparse item baseline ID"
        );
    }
    for batch in [official, custom] {
        // Source inserts new indices only at the end of each main SQL batch.
        // Repeated new IDs each start with empty strings, whereas existing IDs
        // update the same slots in place; do not accumulate new-ID text.
        let existing: std::collections::BTreeSet<_> = rows.keys().copied().collect();
        for mut row in batch {
            if existing.contains(&row.id) {
                let mut strings = rows.remove(&row.id).unwrap().strings;
                strings.overlay_en_us(row.strings);
                row.strings = strings;
            }
            rows.insert(row.id, row);
        }
    }
    rows.retain(|&id, _| !removals.contains_like_cpp(ITEM_SPARSE_HASH, id as i32));
    Ok(rows)
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
    for row in baseline {
        ensure!(
            rows.insert(id(&row), row).is_none(),
            "Duplicate item baseline ID"
        );
    }
    for row in official.into_iter().chain(custom) {
        rows.insert(id(&row), row);
    }
    rows.retain(|&id, _| !removals.contains_like_cpp(hash, id as i32));
    Ok(rows)
}

#[cfg(test)]
mod tests;
