// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item modifiers: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::TitanGripPenaltyAction;
use super::{Arc, BANK_SLOT_BAG_START};
use super::{BANK_SLOT_BAG_END, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START};
#[cfg(test)]
use super::{EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND};
use super::ItemSubClassArmor;
use super::{PlayerStatsStore, REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START};
use super::ShieldBlockRegularGameTableLikeCpp;
use super::{WorldSession, two_handed_in_one_hand_like_cpp};
pub(crate) use wow_world_inventory::{
    RepresentedItemBonusActionLikeCpp, RepresentedItemSetAuraRefreshEventLikeCpp,
    RepresentedItemSetSpellEventLikeCpp,
};
#[cfg(test)]
pub(crate) use wow_world_inventory::{
    RepresentedCombatStatRecalculationLikeCpp, RepresentedItemModsReapplyEventLikeCpp,
};

pub(in crate::session) use wow_world_inventory::ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP;

pub(crate) type RepresentedItemSetEffectLikeCpp = wow_entities::PlayerItemSetEffectLikeCpp;

pub(crate) type RepresentedItemBonusStateLikeCpp = wow_entities::PlayerItemBonusStateLikeCpp;

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

pub(crate) use wow_world_inventory::LoadedEquippedItemEnchantmentsOutcomeLikeCpp;

#[derive(Debug, Clone, Default)]
pub(crate) struct InitialLoadedItemModsOutcomeLikeCpp {
    pub item_set_auras: usize,
    pub item_equip_auras: usize,
    pub enchantments: LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
}

pub(in crate::session) use wow_world_inventory::is_represented_bag_slot;

pub(crate) use wow_world_core::session::player_class_mask_for_transmog_like_cpp;

pub(in crate::session) fn player_class_by_armor_subclass_like_cpp(subclass: u32) -> u32 {
    match subclass {
        x if x == ItemSubClassArmor::Miscellaneous as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Cloth as u32 => {
            (1 << (5 - 1)) | (1 << (8 - 1)) | (1 << (9 - 1))
        }
        x if x == ItemSubClassArmor::Leather as u32 => {
            (1 << (4 - 1)) | (1 << (10 - 1)) | (1 << (11 - 1)) | (1 << (12 - 1))
        }
        x if x == ItemSubClassArmor::Mail as u32 => (1 << (3 - 1)) | (1 << (7 - 1)),
        x if x == ItemSubClassArmor::Plate as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (6 - 1))
        }
        x if x == ItemSubClassArmor::Cosmetic as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Shield as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (7 - 1))
        }
        x if x == ItemSubClassArmor::Libram as u32 => 1 << (2 - 1),
        x if x == ItemSubClassArmor::Idol as u32 => 1 << (11 - 1),
        x if x == ItemSubClassArmor::Totem as u32 => 1 << (7 - 1),
        x if x == ItemSubClassArmor::Sigil as u32 => 1 << (6 - 1),
        x if x == ItemSubClassArmor::Relic as u32 => {
            (1 << (2 - 1)) | (1 << (6 - 1)) | (1 << (7 - 1)) | (1 << (11 - 1))
        }
        _ => 0,
    }
}

impl WorldSession {
    /// C++ `sImportPriceQualityStore.LookupEntry(quality + 1)`.
    #[cfg(test)]
    pub fn import_price_quality_factor_like_cpp(&self, quality: u32) -> Option<f32> {
        self.item_valuation_catalogs_for_test_like_cpp()
            .import_prices
            .quality
            .get(quality + 1)
            .map(|entry| entry.data)
    }

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

    pub fn player_stats(&self) -> Option<&Arc<PlayerStatsStore>> {
        self.catalogs.player_stats()
    }

    pub(crate) fn record_represented_titan_grip_penalty_action_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let main_template = self
                .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_MAINHAND)
                .and_then(|item| self.item_storage_template(item.entry_id));
            let off_template = self
                .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)
                .and_then(|item| self.item_storage_template(item.entry_id));
            let using_two_handed_weapon_in_one_hand =
                two_handed_in_one_hand_like_cpp(main_template.as_ref(), off_template.as_ref());

            let Some(action) = self.core.canonical_player_snapshot_like_cpp(|player| {
                let penalty_spell_id = player.titan_grip_penalty_spell_id();
                let has_penalty_aura = penalty_spell_id > 0
                    && self
                        .fixtures
                        .auras
                        .visible_auras
                        .values()
                        .any(|aura| aura.spell_id == penalty_spell_id as i32);

                player.check_titan_grip_penalty_action(
                    using_two_handed_weapon_in_one_hand,
                    has_penalty_aura,
                )
            }) else {
                return;
            };

            if action != TitanGripPenaltyAction::None {
                self.inventory
                    .record_represented_titan_grip_penalty_action_for_test_like_cpp(action);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/item_modifiers/f3_shims.rs"]
mod f3_shims;
