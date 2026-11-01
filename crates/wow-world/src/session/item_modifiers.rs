// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item modifiers: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ShieldBlockRegularGameTableLikeCpp;
use super::WorldSession;
use super::{Arc, BANK_SLOT_BAG_START};
use super::{BANK_SLOT_BAG_END, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START};
use super::{PlayerStatsStore, REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START};
#[cfg(test)]
pub(crate) use wow_world_inventory::RepresentedItemSetAuraRefreshEventLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_inventory::RepresentedItemSetSpellEventLikeCpp;
#[cfg(test)]
pub(crate) use wow_world_inventory::{
    RepresentedCombatStatRecalculationLikeCpp, RepresentedItemModsReapplyEventLikeCpp,
};

pub(in crate::session) use wow_world_inventory::ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP;

#[cfg(test)]
pub(crate) type RepresentedItemSetEffectLikeCpp = wow_entities::PlayerItemSetEffectLikeCpp;

#[cfg(test)]
pub(crate) type RepresentedItemBonusStateLikeCpp = wow_entities::PlayerItemBonusStateLikeCpp;

#[cfg(test)]
pub(crate) fn void_withdrawal_post_store_item_values_update_like_cpp(
    item: &wow_entities::Item,
    create_dynamic_flags: u32,
) -> Option<wow_entities::ItemValuesUpdate> {
    let mut item_data_mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    let mut has_parent_field = false;
    if !item.data().creator.is_empty() {
        item_data_mask.set(wow_entities::ITEM_DATA_CREATOR_BIT);
        has_parent_field = true;
    }
    if item.data().dynamic_flags != create_dynamic_flags {
        item_data_mask.set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS_BIT);
        has_parent_field = true;
    }
    if item.data().property_seed != 0 {
        item_data_mask.set(wow_entities::ITEM_DATA_PROPERTY_SEED_BIT);
        has_parent_field = true;
    }
    if item.data().random_properties_id != 0 {
        item_data_mask.set(wow_entities::ITEM_DATA_RANDOM_PROPERTIES_ID_BIT);
        has_parent_field = true;
    }
    if has_parent_field {
        item_data_mask.set(wow_entities::ITEM_DATA_PARENT_BIT);
    }
    for (index, enchantment) in item.data().enchantments.iter().enumerate() {
        if *enchantment != wow_entities::ItemEnchantment::default() {
            item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT);
            item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT + index);
        }
    }
    if !item_data_mask.is_any_set() {
        return None;
    }
    Some(wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask: item_data_mask,
            values: item.data().clone(),
        }),
    })
}

pub(crate) use wow_world_inventory::item_storage_fields_values_update_like_cpp;
pub(in crate::session) use wow_world_inventory::represented_player_stat_changes_like_cpp;

pub(in crate::session) use wow_world_core::session::RepresentedScalingStatContextLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::player_class_mask_for_transmog_like_cpp;

impl WorldSession {
    pub fn set_shield_block_regular_game_table(
        &mut self,
        table: Arc<ShieldBlockRegularGameTableLikeCpp>,
    ) {
        self.catalogs.shield_block_regular_game_table = Some(table);
    }

    /// Set the player stats store for this session.
    pub fn set_player_stats(&mut self, store: Arc<PlayerStatsStore>) {
        self.catalogs.player_stats = Some(store);
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/item_modifiers/f3_shims.rs"]
mod f3_shims;
