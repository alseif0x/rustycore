//! Item disenchant valuation and represented enchantment catalog access.

use std::sync::Arc;

use crate::session::ItemValuationCatalogsLikeCpp;
use wow_constants::{ItemBondingType, ItemEnchantmentType, ItemFlags};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{
    ItemDisenchantLootStore, SpellEnchantProcEntryLikeCpp, SpellEnchantProcStoreLikeCpp,
};
use wow_data::{ItemRandomEnchantmentTemplateStore, SpellItemEnchantmentStore};
use wow_entities::{
    ApplyEnchantmentEffectRef, ApplyEnchantmentRandomSuffixRef, ApplyEnchantmentTemplateRef,
};

impl crate::session::state::SessionCatalogs {
    /// C++ `Item::GetDisenchantLoot`.
    ///
    /// `can_disenchant_bonus` represents `BonusData::CanDisenchant`, which is
    /// not yet a canonical Rust item-bonus subsystem.
    pub fn item_disenchant_loot_with_catalogs_like_cpp(
        &self,
        catalogs: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
        quality: u32,
        item_level: u32,
        can_disenchant_bonus: bool,
    ) -> Option<(u32, u16)> {
        if !can_disenchant_bonus {
            return None;
        }

        let basic = self.items.store.as_ref()?.get(item_id)?;
        let sparse = self.items.stats_store.as_ref()?.sparse_template(item_id)?;
        let item_flags = sparse.item_flags();

        if item_flags.contains(ItemFlags::CONJURED)
            || item_flags.contains(ItemFlags::NO_DISENCHANT)
            || sparse.bonding == ItemBondingType::Quest as u8
        {
            return None;
        }

        if sparse.zone_bound[0] != 0
            || sparse.zone_bound[1] != 0
            || sparse.instance_bound != 0
            || sparse.max_stack_size() > 1
        {
            return None;
        }

        if self.item_sell_price_with_catalogs_like_cpp(catalogs, item_id, quality, item_level)
            == Some(0)
            && !catalogs.currency_costs.has_item_currency_cost(item_id)
        {
            return None;
        }

        catalogs
            .disenchant_loot
            .find_for_item_like_cpp(
                u32::from(basic.class_id),
                basic.subclass_id as i8,
                quality as u8,
                item_level,
                sparse.required_expansion,
            )
            .map(|entry| (entry.id, entry.skill_required))
    }

    /// Get the item random enchantment template store reference.
    pub fn item_random_enchantment_template_store(
        &self,
    ) -> Option<&Arc<ItemRandomEnchantmentTemplateStore>> {
        self.items.random_enchantment_template_store.as_ref()
    }

    /// Set the item disenchant loot store for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_item_disenchant_loot_store(&mut self, store: Arc<ItemDisenchantLootStore>) {
        self.item_disenchant_loot_store = Some(store);
    }

    /// Resolve C++ `sItemRandomSuffixStore.LookupEntry(abs(RandomPropertiesID))`.
    pub fn apply_enchantment_random_suffix_ref(
        &self,
        random_properties_id: i32,
    ) -> Option<ApplyEnchantmentRandomSuffixRef> {
        let id = random_properties_id.unsigned_abs();
        if id == 0 {
            return None;
        }

        self.items
            .random_suffix_store
            .as_ref()
            .and_then(|store| store.get(id))
            .map(|entry| {
                ApplyEnchantmentRandomSuffixRef::new(
                    entry.id,
                    entry.enchantments,
                    entry.allocation_pct,
                )
            })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_spell_enchant_proc_store(&mut self, store: Arc<SpellEnchantProcStoreLikeCpp>) {
        self.spell_catalogs.spell_enchant_proc_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn spell_enchant_proc_event_like_cpp(
        &self,
        enchantment_id: u32,
    ) -> Option<&SpellEnchantProcEntryLikeCpp> {
        self.spell_catalogs
            .spell_enchant_proc_store
            .as_ref()
            .and_then(|store| store.get_spell_enchant_proc_event_like_cpp(enchantment_id))
    }

    /// C++ `SpellMgr::IsArenaAllowedEnchancment`.
    pub fn is_arena_allowed_enchantment(&self, enchantment_id: u32) -> bool {
        self.spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
            .is_some_and(|store| store.is_arena_allowed_enchantment(enchantment_id))
    }

    /// Build the entity-level `ApplyEnchantment` template from `SpellItemEnchantment.db2`.
    pub fn apply_enchantment_template_ref(
        &self,
        enchantment_id: i32,
        required_skill_value: u16,
        condition_fits: bool,
    ) -> Option<ApplyEnchantmentTemplateRef> {
        let id = u32::try_from(enchantment_id).ok()?;
        self.spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
            .and_then(|store| store.get(id))
            .map(|entry| {
                let mut template = ApplyEnchantmentTemplateRef::new(enchantment_id);
                template.condition_id = u32::from(entry.condition_id);
                template.condition_fits = condition_fits;
                template.min_level = entry.min_level;
                template.required_skill_id = u32::from(entry.required_skill_id);
                template.required_skill_rank = entry.required_skill_rank;
                template.required_skill_value = required_skill_value;
                template
            })
    }

    /// Build the C++ three `SpellItemEnchantmentEntry` effect refs.
    pub fn apply_enchantment_effect_refs(
        &self,
        enchantment_id: u32,
    ) -> Option<[ApplyEnchantmentEffectRef; 3]> {
        self.spell_catalogs
            .spell_item_enchantment_store
            .as_ref()
            .and_then(|store| store.get(enchantment_id))
            .map(|entry| {
                std::array::from_fn(|index| {
                    let amount = entry.effect_points_min[index] as u32;
                    let arg = entry.effect_arg[index];
                    match <ItemEnchantmentType as num_traits::FromPrimitive>::from_u8(
                        entry.effect[index],
                    ) {
                        Some(effect_type) => {
                            ApplyEnchantmentEffectRef::known(effect_type, amount, arg)
                        }
                        None => ApplyEnchantmentEffectRef::unknown(
                            u32::from(entry.effect[index]),
                            amount,
                            arg,
                        ),
                    }
                })
            })
    }
}

impl crate::session::state::SessionCatalogs {
    /// Get the spell item enchantment store reference.
    pub fn spell_item_enchantment_store(&self) -> Option<&Arc<SpellItemEnchantmentStore>> {
        self.spell_catalogs.spell_item_enchantment_store.as_ref()
    }
}
