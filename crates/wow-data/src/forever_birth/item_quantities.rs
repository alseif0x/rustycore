//! Quantity-only projection of *final effective* target item records.
//! 02245dcd ObjectMgr.cpp:3319-3426,3968-4029; ItemTemplate.h:855,941.
//! Not an ItemTemplate, an equip/use admission check, or an item instance.
//! The producer must complete DB2 baseline/official/custom/removal composition
//! before calling this constructor. No legacy item layout or raw DB2 fallback.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub struct ItemClassQuantityRecord {
    pub id: u32,
    pub class: i32,
    pub subclass: u8,
}

#[derive(Clone, Copy)]
pub struct ItemSparseQuantityRecord {
    pub id: u32,
    pub vendor_stack: u32,
    pub stackable: i32,
}

#[derive(Clone, Copy)]
pub struct ItemEffectQuantityRecord {
    pub id: u32,
    pub legacy_slot: u8,
    pub spell_category: u16,
}

#[derive(Clone, Copy)]
pub struct ItemEffectRelationRecord {
    pub id: u32,
    pub effect: i32,
    pub item: u32,
}

/// A source projection, not proof that all ItemTemplate/equip prerequisites
/// have been loaded. Callers must never use `contains` as CanEquip/CanUse.
pub struct ItemQuantitySources {
    basics: BTreeMap<u32, ItemClassQuantityRecord>,
    sparse: BTreeMap<u32, ItemSparseQuantityRecord>,
    effects: BTreeMap<u32, ItemEffectQuantityRecord>,
    relations: BTreeMap<u32, ItemEffectRelationRecord>,
    effects_by_item: BTreeMap<u32, Vec<u32>>,
}

impl ItemQuantitySources {
    /// Final records have unique IDs. Duplicates here are a producer failure,
    /// unlike legal same-ID SQL overlay rows *before* effective composition.
    pub fn from_effective_records(
        basics: Vec<ItemClassQuantityRecord>,
        sparse: Vec<ItemSparseQuantityRecord>,
        effects: Vec<ItemEffectQuantityRecord>,
        relations: Vec<ItemEffectRelationRecord>,
    ) -> Result<Self> {
        let basics = unique(basics, |row| row.id)?;
        let sparse = unique(sparse, |row| row.id)?;
        let effects = unique(effects, |row| row.id)?;
        let relations = unique(relations, |row| row.id)?;
        let mut effects_by_item = BTreeMap::<u32, Vec<u32>>::new();
        // DB2Storage iteration is ascending relation ID. lower_bound inserts
        // BEFORE equal slots, so a later relation wins the first-effect tie.
        // Neither sorting by effect ID nor deduplicating relations is faithful.
        for relation in relations.values() {
            if !basics.contains_key(&relation.item) || !sparse.contains_key(&relation.item) {
                continue;
            }
            // Source LookupEntry converts the signed foreign key to uint32.
            let Some(effect) = effects.get(&(relation.effect as u32)) else {
                continue;
            };
            let ids = effects_by_item.entry(relation.item).or_default();
            let at = ids.partition_point(|id| effects[id].legacy_slot < effect.legacy_slot);
            ids.insert(at, effect.id);
        }
        Ok(Self {
            basics,
            sparse,
            effects,
            relations,
            effects_by_item,
        })
    }

    /// The source quantity join requires BOTH effective Item and ItemSparse.
    /// This is not full template validation or permission to instantiate it.
    pub fn contains(&self, item: u32) -> bool {
        self.basics.contains_key(&item) && self.sparse.contains_key(&item)
    }

    /// LoadPlayerInfo's loadout amount, not inventory placement/stack splitting.
    /// Food/drink alone are clamped; other vendor counts remain untouched.
    pub fn loadout_quantity(&self, item: u32, class: u8) -> Option<u32> {
        let basic = self.basics.get(&item)?;
        let sparse = self.sparse.get(&item)?;
        let mut count = sparse.vendor_stack.max(1);
        if basic.class == 0 && basic.subclass == 5 {
            match self
                .effects_by_item
                .get(&item)
                .and_then(|ids| ids.first())
                .map(|id| self.effects[id].spell_category)
            {
                Some(11) => count = if class == 6 { 10 } else { 4 },
                Some(59) => count = 2,
                _ => {}
            }
            let max_stack = if sparse.stackable <= 0 || sparse.stackable == i32::MAX {
                0x7FFF_FFFE
            } else {
                sparse.stackable as u32
            };
            count = count.min(max_stack);
        }
        Some(count)
    }

    /// Diagnostic record counts only; no full-template coverage claim.
    pub fn counts(&self) -> [usize; 4] {
        [
            self.basics.len(),
            self.sparse.len(),
            self.effects.len(),
            self.relations.len(),
        ]
    }
}

fn unique<T>(rows: Vec<T>, id: impl Fn(&T) -> u32) -> Result<BTreeMap<u32, T>> {
    let mut result = BTreeMap::new();
    for row in rows {
        ensure!(
            result.insert(id(&row), row).is_none(),
            "Duplicate final item quantity ID"
        );
    }
    Ok(result)
}

#[cfg(test)]
#[path = "item_quantities/tests.rs"]
mod tests;
