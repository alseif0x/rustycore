//! Target template numeric rules, not strings/wire/use/equip/item instances.
//! 02245dcd ObjectMgr.cpp:3020-3442. Raw rows have one nested immutable owner;
//! derived templates/effects contain IDs/metadata, not duplicated item records.
mod durability;
mod spec_stats;
mod specializations;
#[cfg(test)]
mod tests;
use std::collections::BTreeMap;
use std::sync::Arc;
use wow_data::forever_birth::{
    item_quantities::ItemQuantitySources,
    item_records::{ItemCatalog, ItemEffectRecord, ItemRecord},
    item_sparse::SparseItemRecord,
    item_specs::ItemSpecCatalog,
};
use wow_data::forever_initialization::InitializationCatalog;
use wow_persistence::forever::item_specs::ItemAddonRow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemTemplateError {
    InvalidDurabilityQuality,
    InvalidDurabilityWeapon,
    InvalidSpecialization,
    InvalidQuantityProjection,
}

pub struct ItemTemplateMetadata {
    pub max_durability: u32,
    pub specialization_class_mask: u32,
    /// Exactly MAX_CLASSES(16)*MAX_SPECIALIZATIONS(5) source bits per range.
    /// Remaining high u128 bits are always zero, including all-spec fallback.
    pub specializations: [u128; 3],
    pub flags: u32,
    pub food: u8,
    pub min_money: u32,
    pub max_money: u32,
    pub spell_ppm: f32,
    pub random_bonus_template: u32,
    pub quest_log_item: i32,
}
pub struct ItemTemplateNumericView<'a> {
    pub basic: &'a ItemRecord,
    pub extended: &'a SparseItemRecord,
    pub metadata: &'a ItemTemplateMetadata,
}
pub struct NumericItemTemplates {
    data: Arc<ItemCatalog>,
    metadata: BTreeMap<u32, ItemTemplateMetadata>,
    effects: BTreeMap<u32, Vec<u32>>,
}
impl NumericItemTemplates {
    pub fn load(
        data: impl Into<Arc<ItemCatalog>>,
        specs: &ItemSpecCatalog,
        initialization: &InitializationCatalog,
        addons: Vec<ItemAddonRow>,
    ) -> Result<Self, ItemTemplateError> {
        let data = data.into();
        let mut metadata = BTreeMap::new();
        for sparse in data.sparse_records() {
            let Some(basic) = data.item(sparse.id) else {
                continue;
            };
            let (specialization_class_mask, specializations) =
                specializations::sets(basic, sparse, specs, |id| {
                    initialization.specialization_by_id(id)
                })?;
            metadata.insert(
                sparse.id,
                ItemTemplateMetadata {
                    max_durability: durability::maximum(
                        basic.class as u32,
                        basic.subclass as u32,
                        sparse.inventory_type as i32 as u32,
                        sparse.quality as i32 as u32,
                        u32::from(sparse.item_level),
                    )?,
                    specialization_class_mask,
                    specializations,
                    flags: 0,
                    food: 0,
                    min_money: 0,
                    max_money: 0,
                    spell_ppm: 0.0,
                    random_bonus_template: 0,
                    quest_log_item: 0,
                },
            );
        }
        let mut effects = BTreeMap::<u32, Vec<u32>>::new();
        for relation in data.relations() {
            if !metadata.contains_key(&relation.item) {
                continue;
            }
            let Some(effect) = data.effect(relation.effect as u32) else {
                continue;
            };
            let ids = effects.entry(relation.item).or_default();
            let at = ids
                .partition_point(|id| data.effect(*id).unwrap().legacy_slot < effect.legacy_slot);
            ids.insert(at, effect.id); // Source lower_bound inserts before equal slots.
        }
        for addon in addons {
            let Some(row) = metadata.get_mut(&addon.id) else {
                continue;
            };
            row.flags = addon.flags;
            row.food = addon.food;
            row.min_money = addon.min_money.min(addon.max_money);
            row.max_money = addon.max_money.max(addon.min_money);
            row.spell_ppm = addon.spell_ppm;
            row.random_bonus_template = addon.random_bonus_template;
            row.quest_log_item = addon.quest_log_item;
        }
        Ok(Self {
            data,
            metadata,
            effects,
        })
    }
    pub fn template(&self, id: u32) -> Option<ItemTemplateNumericView<'_>> {
        Some(ItemTemplateNumericView {
            basic: self.data.item(id)?,
            extended: self.data.sparse(id)?,
            metadata: self.metadata.get(&id)?,
        })
    }
    pub fn effects(&self, item: u32) -> impl Iterator<Item = &ItemEffectRecord> {
        self.effects
            .get(&item)
            .into_iter()
            .flatten()
            .map(|id| self.data.effect(*id).unwrap())
    }
    pub fn quantity_projection(&self) -> Result<ItemQuantitySources, ItemTemplateError> {
        self.data
            .quantity_projection()
            .map_err(|_| ItemTemplateError::InvalidQuantityProjection)
    }
    pub fn counts(&self) -> [usize; 4] {
        self.data.counts()
    }
    pub fn template_count(&self) -> usize {
        self.metadata.len()
    }
    pub fn unknown_baseline_records(&self) -> [usize; 4] {
        self.data.unknown_baseline_records()
    }
}
