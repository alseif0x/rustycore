// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;
#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedItemModsReapplyEventLikeCpp;
use crate::RepresentedItemSetSpellEventLikeCpp;
use crate::{RepresentedAuctionRemoveItemLikeCpp, RepresentedAuctionSellItemLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::unit::WeaponAttackType;
use wow_constants::{InventoryResult, ItemClass, ItemFlags, ItemModifier, ItemQuality};
use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::{ObjectGuidGenerator, guid::HighGuid};
use wow_data::SpellEquippedItemsEntry;
use wow_entities::{
    EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_FEET, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD,
    EQUIPMENT_SLOT_LEGS, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_SHOULDERS,
    EQUIPMENT_SLOT_WAIST, EQUIPMENT_SLOT_WRISTS, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END,
    Item, ItemStorageTemplate, PlayerInventoryItem as InventoryItem,
    SpellCastBattlePetItemModifiersLikeCpp,
};
use wow_world_core::session::{HubMut, HubRef, OwnedInventoryAccessLikeCpp};

impl crate::InventoryState {
    pub fn represented_top_level_item_mod_targets_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Vec<(u8, ObjectGuid)>> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.represented_top_level_item_mod_targets_with_access_like_cpp(&access)
    }

    pub fn represented_top_level_item_mod_targets_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<Vec<(u8, ObjectGuid)>> {
        let mut targets = self
            .resolved_inventory_item_objects_with_access_like_cpp(access)?
            .values()
            .filter(|item| {
                item.container_guid().is_empty()
                    && item.slot() < INVENTORY_SLOT_BAG_END
                    && !item.is_broken()
            })
            .map(|item| (item.slot(), item.object().guid()))
            .collect::<Vec<_>>();
        targets.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(targets)
    }

    pub fn insert_buyback_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.store_buyback_item_in_slot_like_cpp(slot, item)
        })
        .flatten()
    }

    pub fn remove_buyback_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.remove_buyback_item_from_slot_like_cpp(slot)
        })
        .flatten()
    }

    pub fn represented_has_item_count_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        count: u32,
    ) -> bool {
        if count == 0 {
            return true;
        }

        self.represented_inventory_item_counts_like_cpp(hub)
            .is_some_and(|counts| counts.get(&item_entry).copied().unwrap_or(0) >= count)
    }

    pub fn can_destroy_direct_item_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: u8,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        self.can_unequip_inventory_item_at_like_cpp(
            hub,
            INVENTORY_SLOT_BAG_0,
            slot,
            false,
            source_item,
            proto,
            source_is_not_empty_bag,
        )
    }

    pub fn direct_item_contains_items(&self, hub: HubRef<'_>, item_guid: ObjectGuid) -> bool {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.direct_item_contains_items_with_access_like_cpp(&access, item_guid)
    }

    pub fn direct_item_contains_items_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) -> bool {
        self.resolved_inventory_item_objects_with_access_like_cpp(access)
            .is_some_and(|items| {
                items
                    .values()
                    .any(|item| item.container_guid() == item_guid)
            })
    }

    pub fn represented_has_item_fit_to_spell_requirements_like_cpp(
        &self,
        hub: HubRef<'_>,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        const SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES_LIKE_CPP: u32 = 0x0010_0000;

        if equipped.equipped_item_class < 0 {
            return true;
        }

        match equipped.equipped_item_class {
            class if class == ItemClass::Weapon as i8 => {
                self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                    hub,
                    EQUIPMENT_SLOT_MAINHAND,
                    equipped,
                ) || self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                    hub,
                    EQUIPMENT_SLOT_OFFHAND,
                    equipped,
                )
            }
            class if class == ItemClass::Armor as i8 => {
                if hub
                    .catalogs
                    .spell_catalogs
                    .spell_store
                    .as_ref()
                    .is_some_and(|store| {
                        store.has_attribute8_like_cpp(
                            equipped.spell_id,
                            SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES_LIKE_CPP,
                        )
                    })
                {
                    [
                        EQUIPMENT_SLOT_HEAD,
                        EQUIPMENT_SLOT_SHOULDERS,
                        EQUIPMENT_SLOT_CHEST,
                        EQUIPMENT_SLOT_WAIST,
                        EQUIPMENT_SLOT_LEGS,
                        EQUIPMENT_SLOT_FEET,
                        EQUIPMENT_SLOT_WRISTS,
                        EQUIPMENT_SLOT_HANDS,
                    ]
                    .into_iter()
                    .all(|slot| {
                        self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                            hub, slot, equipped,
                        )
                    })
                } else {
                    self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                        hub,
                        EQUIPMENT_SLOT_OFFHAND,
                        equipped,
                    ) || (EQUIPMENT_SLOT_HEAD..EQUIPMENT_SLOT_MAINHAND).any(|slot| {
                        self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                            hub, slot, equipped,
                        )
                    })
                }
            }
            _ => false,
        }
    }

    pub fn resolved_buyback_items_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<u8, InventoryItem>> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_buyback_items_with_access_like_cpp(&access)
    }

    pub(crate) fn resolved_buyback_items_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)
            .map(|inventory| inventory.buyback_items().clone())
    }

    pub fn uncage_cast_item_still_matches_like_cpp(
        &self,
        hub: HubRef<'_>,
        cast_item_entry: u32,
        modifiers: SpellCastBattlePetItemModifiersLikeCpp,
    ) -> Option<(u8, u8, InventoryItem)> {
        let (bag, slot, inventory_item) =
            self.get_inventory_item_by_guid_like_cpp(hub, modifiers.source_item_guid)?;
        if inventory_item.entry_id != cast_item_entry {
            return None;
        }
        let item = self.resolved_inventory_item_object_like_cpp(hub, modifiers.source_item_guid)?;
        (item.object().entry() == cast_item_entry
            && item.get_modifier(ItemModifier::BattlePetSpeciesId) == modifiers.species_id
            && item.get_modifier(ItemModifier::BattlePetBreedData) == modifiers.breed_data
            && item.get_modifier(ItemModifier::BattlePetLevel) == u32::from(modifiers.level)
            && item.get_modifier(ItemModifier::BattlePetDisplayId) == modifiers.display_id)
            .then_some((bag, slot, inventory_item))
    }
}

impl crate::InventoryState {
    pub fn apply_wrapped_gift_row_to_runtime_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        bag: u8,
        item_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        flags: u32,
    ) -> Option<u32> {
        let current_item = self.get_inventory_item_by_pos(hub.shared(), bag, slot)?;
        if current_item.guid != item_guid {
            return None;
        }

        let max_durability = hub.catalogs.item_template_max_durability(entry);
        let inventory_type = hub.shared().item_template_inventory_type(entry);
        let durability = self.transform_inventory_wrapped_gift_item_like_cpp(
            hub,
            item_guid,
            entry,
            flags,
            max_durability,
        )?;

        if bag == INVENTORY_SLOT_BAG_0 {
            self.update_inventory_item_metadata_like_cpp(
                hub,
                slot,
                item_guid,
                entry,
                inventory_type,
            );
        }

        Some(durability)
    }

    pub fn direct_inventory_item_count_like_cpp_representable(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
    ) -> Option<u32> {
        Some(
            self.resolved_inventory_items_like_cpp(hub)?
                .values()
                .filter(|inventory_item| inventory_item.entry_id == item_id)
                .filter_map(|inventory_item| {
                    self.resolved_inventory_item_object_like_cpp(hub, inventory_item.guid)
                })
                .filter(|item| !item.is_in_trade())
                .fold(0_u32, |total, item| total.saturating_add(item.count())),
        )
    }
}

impl crate::InventoryState {
    /// Install the process-wide C++
    /// `sObjectMgr->GetGenerator<HighGuid::Item>()` mirror.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_item_guid_generator_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        generator: Arc<ObjectGuidGenerator>,
    ) {
        assert_eq!(
            generator.high_guid(),
            HighGuid::Item,
            "item GUID allocator must use HighGuid::Item"
        );
        hub.core.item_guid_generator_like_cpp = Some(generator);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn item_guid_generator_like_cpp_for_bridge(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Arc<ObjectGuidGenerator>> {
        hub.core.item_guid_generator_like_cpp.clone()
    }

    pub fn record_represented_remove_items_set_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        item_set: &wow_data::ItemSetEntry,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let item_modifiers = hub.core.owned_item_modifiers_access_like_cpp();
        let shared = hub.shared();
        let item_sets = shared.owned_item_set_access_like_cpp();
        self.record_represented_remove_items_set_item_with_access_like_cpp(
            &item_modifiers,
            &item_sets,
            item_guid,
            item_set,
        )
    }

    pub fn item_drop_rate_like_cpp(&self, hub: HubRef<'_>, item_id: u32) -> f32 {
        let quality = hub
            .catalogs
            .item_template_quality(item_id)
            .and_then(<ItemQuality as num_traits::FromPrimitive>::from_i8);
        match quality {
            Some(ItemQuality::Poor) => hub.config.loot_drop_rates.item_poor,
            Some(ItemQuality::Normal) => hub.config.loot_drop_rates.item_normal,
            Some(ItemQuality::Uncommon) => hub.config.loot_drop_rates.item_uncommon,
            Some(ItemQuality::Rare) => hub.config.loot_drop_rates.item_rare,
            Some(ItemQuality::Epic) => hub.config.loot_drop_rates.item_epic,
            Some(ItemQuality::Legendary) => hub.config.loot_drop_rates.item_legendary,
            Some(ItemQuality::Artifact) => hub.config.loot_drop_rates.item_artifact,
            _ => 1.0,
        }
    }

    /// C++ `Item::IsBoundAccountWide` template-flag predicate.
    pub fn is_item_bound_account_wide(&self, hub: HubRef<'_>, item_id: u32) -> bool {
        hub.catalogs
            .item_template_flags(item_id)
            .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn allocate_item_instance_guids_like_cpp(
        &self,
        hub: HubRef<'_>,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        let generator = hub.core.item_guid_generator_like_cpp.as_deref()?;
        hub.core
            .allocate_item_instance_guids_with_generator_like_cpp(generator, count)
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_auction_remove_item_like_cpp(
        &mut self,
        remove: RepresentedAuctionRemoveItemLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_auction_remove_items_like_cpp.push(remove);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_auction_remove_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionRemoveItemLikeCpp] {
        &self.represented_auction_remove_items_like_cpp
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_auction_sell_item_like_cpp(
        &mut self,
        sell: RepresentedAuctionSellItemLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_auction_sell_items_like_cpp.push(sell);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_auction_sell_items_like_cpp(&self) -> &[RepresentedAuctionSellItemLikeCpp] {
        &self.represented_auction_sell_items_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn buyback_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        &self.player_item_test_fixture_like_cpp.buyback_items
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_item_mod_reapply_events_like_cpp(
        &self,
    ) -> &[RepresentedItemModsReapplyEventLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_mod_reapply_events_like_cpp
    }
}
