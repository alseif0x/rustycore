//! DB2Store.cpp:127-133, DB2DatabaseLoader.cpp:27-174, final removal pass
//! in DB2Stores.cpp:1539-1548 at 02245dcd. Not a full ItemTemplate owner.
use super::{
    AzeriteEmpoweredItemRecord, ItemBonusRecord, ItemEffectRecord, ItemOffsetCurveRecord,
    ItemRecord, ItemRecords, ItemScalingConfigRecord, ItemSquishEraRecord,
};
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
pub const ITEM_BONUS_HASH: u32 = 0x8318_900A;
pub const AZERITE_EMPOWERED_ITEM_HASH: u32 = 0xAFB1_B60F;
pub const ITEM_SCALING_CONFIG_HASH: u32 = 0xF6AA_409E;
pub const ITEM_OFFSET_CURVE_HASH: u32 = 0x8993_63A4;
pub const ITEM_SQUISH_ERA_HASH: u32 = 0xACAA_F53B;

pub struct ItemCatalog {
    items: BTreeMap<u32, ItemRecord>,
    sparse: BTreeMap<u32, SparseItemRecord>,
    effects: BTreeMap<u32, ItemEffectRecord>,
    relations: BTreeMap<u32, ItemEffectRelationRecord>,
    item_bonuses: BTreeMap<u32, ItemBonusRecord>,
    item_bonuses_by_list: BTreeMap<u32, Vec<u32>>,
    azerite_empowered_items: BTreeMap<u32, AzeriteEmpoweredItemRecord>,
    azerite_empowered_item_by_item: BTreeMap<u32, u32>,
    item_scaling_configs: BTreeMap<u32, ItemScalingConfigRecord>,
    item_offset_curves: BTreeMap<u32, ItemOffsetCurveRecord>,
    item_squish_eras: BTreeMap<u32, ItemSquishEraRecord>,
    item_squish_era_storage_last_index: u32,
    unknown_baseline_records: [usize; 9],
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
        ensure!(
            self.item_squish_era_sql_index_size.is_none(),
            "File item squish baseline cannot carry a SQL storage observation"
        );
        ensure!(
            official.item_squish_era_file_last_index.is_none()
                && custom.item_squish_era_file_last_index.is_none(),
            "SQL item squish overlays cannot carry a file storage bound"
        );
        ensure!(
            official.unknown_baseline_records == [0; 9]
                && custom.unknown_baseline_records == [0; 9],
            "SQL item overlays cannot assert unknown baseline coverage"
        );
        let item_squish_era_storage_last_index = self
            .item_squish_era_file_last_index
            .or_else(|| self.item_squish_eras.is_empty().then_some(0))
            .ok_or_else(|| anyhow::anyhow!("Nonempty item squish baseline lacks storage bound"))?;
        ensure!(
            item_squish_era_storage_last_index < u32::MAX,
            "Item squish baseline storage bound overflows GetNumRows"
        );
        ensure!(
            self.item_squish_eras
                .iter()
                .all(|row| row.id <= item_squish_era_storage_last_index),
            "Item squish baseline exceeds its storage bound"
        );
        let (item_squish_eras, item_squish_era_storage_last_index) = compose_squish_eras(
            self.item_squish_eras,
            official.item_squish_eras,
            custom.item_squish_eras,
            item_squish_era_storage_last_index,
            official.item_squish_era_sql_index_size,
            custom.item_squish_era_sql_index_size,
            removals,
        )?;
        let item_bonuses = compose(
            self.item_bonuses,
            official.item_bonuses,
            custom.item_bonuses,
            ITEM_BONUS_HASH,
            removals,
            |r| r.id,
        )?;
        let mut item_bonuses_by_list = BTreeMap::<u32, Vec<u32>>::new();
        for row in item_bonuses.values() {
            item_bonuses_by_list
                .entry(u32::from(row.parent))
                .or_default()
                .push(row.id);
        }
        let azerite_empowered_items = compose(
            self.azerite_empowered_items,
            official.azerite_empowered_items,
            custom.azerite_empowered_items,
            AZERITE_EMPOWERED_ITEM_HASH,
            removals,
            |r| r.id,
        )?;
        let mut azerite_empowered_item_by_item = BTreeMap::new();
        for row in azerite_empowered_items.values() {
            azerite_empowered_item_by_item.insert(row.item as u32, row.id);
        }
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
            item_bonuses_by_list,
            item_bonuses,
            azerite_empowered_item_by_item,
            azerite_empowered_items,
            item_scaling_configs: compose(
                self.item_scaling_configs,
                official.item_scaling_configs,
                custom.item_scaling_configs,
                ITEM_SCALING_CONFIG_HASH,
                removals,
                |r| r.id,
            )?,
            item_offset_curves: compose(
                self.item_offset_curves,
                official.item_offset_curves,
                custom.item_offset_curves,
                ITEM_OFFSET_CURVE_HASH,
                removals,
                |r| r.id,
            )?,
            item_squish_eras,
            item_squish_era_storage_last_index,
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
    pub fn item_bonus(&self, id: u32) -> Option<&ItemBonusRecord> {
        self.item_bonuses.get(&id)
    }
    pub fn item_bonuses_for_list(&self, id: u32) -> impl Iterator<Item = &ItemBonusRecord> {
        self.item_bonuses_by_list
            .get(&id)
            .into_iter()
            .flatten()
            .map(|row_id| self.item_bonuses.get(row_id).unwrap())
    }
    pub fn azerite_empowered_item(&self, id: u32) -> Option<&AzeriteEmpoweredItemRecord> {
        self.azerite_empowered_items.get(&id)
    }
    pub fn azerite_empowered_item_for_item(
        &self,
        item: u32,
    ) -> Option<&AzeriteEmpoweredItemRecord> {
        self.azerite_empowered_item_by_item
            .get(&item)
            .and_then(|id| self.azerite_empowered_items.get(id))
    }
    pub fn item_scaling_config(&self, id: u32) -> Option<&ItemScalingConfigRecord> {
        self.item_scaling_configs.get(&id)
    }
    pub fn item_offset_curve(&self, id: u32) -> Option<&ItemOffsetCurveRecord> {
        self.item_offset_curves.get(&id)
    }
    pub fn item_squish_era(&self, id: u32) -> Option<&ItemSquishEraRecord> {
        (id <= self.item_squish_era_storage_last_index)
            .then(|| self.item_squish_eras.get(&id))
            .flatten()
    }
    pub fn item_squish_era_storage_last_index(&self) -> u32 {
        self.item_squish_era_storage_last_index
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

    pub fn counts(&self) -> [usize; 9] {
        [
            self.items.len(),
            self.sparse.len(),
            self.effects.len(),
            self.relations.len(),
            self.item_bonuses.len(),
            self.azerite_empowered_items.len(),
            self.item_scaling_configs.len(),
            self.item_offset_curves.len(),
            self.item_squish_eras.len(),
        ]
    }
    pub fn unknown_baseline_records(&self) -> [usize; 9] {
        self.unknown_baseline_records
    }
}

fn compose_squish_eras(
    baseline: Vec<ItemSquishEraRecord>,
    official: Vec<ItemSquishEraRecord>,
    custom: Vec<ItemSquishEraRecord>,
    file_last_index: u32,
    official_size: Option<u32>,
    custom_size: Option<u32>,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<(BTreeMap<u32, ItemSquishEraRecord>, u32)> {
    let mut rows = BTreeMap::new();
    for row in baseline {
        ensure!(
            row.id <= file_last_index,
            "Item squish baseline exceeds its storage bound"
        );
        ensure!(
            rows.insert(row.id, row).is_none(),
            "Duplicate item squish baseline ID"
        );
    }
    let mut published_size = u64::from(file_last_index) + 1;
    let mut allocated_size = published_size;
    for (batch, observed_size) in [(official, official_size), (custom, custom_size)] {
        if batch.is_empty() {
            ensure!(
                observed_size.is_none(),
                "Empty item squish batch has a storage query"
            );
            continue;
        }
        let observed_size = observed_size
            .ok_or_else(|| anyhow::anyhow!("Nonempty item squish batch lacks MAX(ID)+1"))?;
        let allocation = published_size.max(u64::from(observed_size));
        if allocation > published_size {
            allocated_size = allocation;
        }
        ensure!(
            batch.iter().all(|row| u64::from(row.id) < allocated_size),
            "Item squish SQL row exceeds observed source allocation"
        );
        let adds_record = batch.iter().any(|row| !rows.contains_key(&row.id));
        for row in batch {
            rows.insert(row.id, row);
        }
        if adds_record {
            published_size = allocation;
        }
    }
    rows.retain(|id, _| u64::from(*id) < published_size);
    rows.retain(|&id, _| !removals.contains_like_cpp(ITEM_SQUISH_ERA_HASH, id as i32));
    Ok((rows, (published_size - 1) as u32))
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
