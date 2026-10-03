//! 02245dcd DB2StorageBase::WriteRecord and seven Item* / GemProperties Meta.
//! External IDs are excluded. Parent fields are serialized even when physically
//! stored in a DB2 relationship. No cached byte mirror or optional-data waiver.
use crate::forever_birth::{
    item_records::{
        ITEM_EFFECT_HASH, ITEM_HASH, ITEM_RELATION_HASH, ITEM_SPARSE_HASH, ItemCatalog,
        ItemEffectRecord, ItemRecord,
    },
    item_sparse::SparseItemRecord,
    item_specs::{GEM_PROPERTIES_HASH, ITEM_SPEC_HASH, ITEM_SPEC_OVERRIDE_HASH, ItemSpecCatalog},
};
use std::sync::Arc;

pub const ITEM_TABLE_HASHES: [u32; 7] = [
    ITEM_HASH,
    ITEM_SPARSE_HASH,
    ITEM_EFFECT_HASH,
    ITEM_RELATION_HASH,
    ITEM_SPEC_HASH,
    ITEM_SPEC_OVERRIDE_HASH,
    GEM_PROPERTIES_HASH,
];

pub(super) struct ItemHotfixStores {
    pub data: Arc<ItemCatalog>,
    pub specs: Arc<ItemSpecCatalog>,
}
impl ItemHotfixStores {
    pub fn record(&self, hash: u32, id: u32, locale: u8) -> Option<Vec<u8>> {
        Some(match hash {
            ITEM_HASH => basic(self.data.item(id)?),
            ITEM_SPARSE_HASH => sparse(self.data.sparse(id)?, locale),
            ITEM_EFFECT_HASH => effect(self.data.effect(id)?),
            ITEM_RELATION_HASH => {
                let r = self.data.relation(id)?;
                [r.effect.to_le_bytes(), r.item.to_le_bytes()].concat()
            }
            ITEM_SPEC_HASH => {
                let r = self.specs.spec(id)?;
                let mut b = vec![
                    r.min_level,
                    r.max_level,
                    r.item_type,
                    r.primary,
                    r.secondary,
                ];
                b.extend_from_slice(&r.specialization.to_le_bytes());
                b
            }
            ITEM_SPEC_OVERRIDE_HASH => {
                let r = self.specs.override_record(id)?;
                [
                    r.specialization.to_le_bytes().as_slice(),
                    r.item.to_le_bytes().as_slice(),
                ]
                .concat()
            }
            GEM_PROPERTIES_HASH => {
                let r = self.specs.gem(id)?;
                [
                    r.enchantment.to_le_bytes().as_slice(),
                    r.kind.to_le_bytes().as_slice(),
                ]
                .concat()
            }
            _ => return None,
        })
    }
}

// Primitive values, including signed and nonfinite float bits, are copied as
// Source uint8/uint16/uint32 writes; no validation/normalization at wire time.
macro_rules! fields {
    ($b:ident; $($value:expr),+ $(,)?) => {$( $b.extend_from_slice(&$value.to_le_bytes()); )+};
}
macro_rules! array {
    ($b:ident; $values:expr) => {
        for value in $values {
            $b.extend_from_slice(&value.to_le_bytes());
        }
    };
}
fn basic(r: &ItemRecord) -> Vec<u8> {
    let mut b = Vec::with_capacity(43);
    fields!(b; r.class, r.subclass, r.material, r.inventory_type, r.sheathe,
        r.pet_food, r.sound_override, r.icon_file, r.group_sounds, r.content_tuning,
        r.modified_crafting_reagent, r.unknown_1200, r.crafting_quality, r.squish_era,
        r.recraft_reagent_percentage, r.order_source);
    b
}
fn effect(r: &ItemEffectRecord) -> Vec<u8> {
    let mut b = Vec::with_capacity(24);
    fields!(b; r.legacy_slot, r.trigger, r.charges, r.cooldown, r.category_cooldown,
        r.spell_category, r.spell, r.specialization, r.player_condition);
    b
}
fn sparse(r: &SparseItemRecord, locale: u8) -> Vec<u8> {
    let mut b = Vec::with_capacity(307);
    for field in 0..5 {
        let text = r.strings.field(locale, field);
        let end = text
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(text.len());
        b.extend_from_slice(&text[..end]);
        b.push(0);
    }
    fields!(b; r.expansion, r.damage_variance, r.limit_category, r.duration,
        r.quality_modifier, r.bag_family, r.start_quest, r.language, r.item_range);
    array!(b; r.socket_percentage);
    array!(b; r.stat_percent);
    array!(b; r.stat_bonus);
    fields!(b; r.stackable, r.max_count, r.min_reputation, r.required_ability,
        r.allowable_race, r.sell_price, r.buy_price, r.vendor_stack, r.price_variance,
        r.price_random);
    array!(b; r.flags);
    fields!(b; r.faction_related, r.modified_crafting_reagent, r.content_tuning,
        r.player_level_curve, r.item_level_offset_curve, r.item_level_offset, r.squish_era,
        r.name_description, r.transmog_holiday, r.holiday, r.gem_properties,
        r.socket_enchantment, r.totem_category, r.instance_bound);
    array!(b; r.zone_bound);
    fields!(b; r.item_set, r.lock, r.page, r.delay, r.min_faction, r.required_skill_rank,
        r.required_skill, r.item_level, r.allowable_class, r.artifact, r.spell_weight,
        r.spell_weight_category);
    array!(b; r.socket_type);
    fields!(b; r.sheathe, r.material, r.page_material, r.bonding, r.damage_type,
        r.container_slots, r.required_pvp_medal, r.required_pvp_rank, r.required_level,
        r.inventory_type, r.quality, r.ammunition);
    b
}

#[cfg(test)]
mod tests;
