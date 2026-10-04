// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;
use wow_constants::{InventoryResult, ItemFieldFlags};
use wow_constants::item::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_data::{ItemStore, ItemStatsStore, ItemEffectStore, SpellItemEnchantmentStore};
use wow_entities::{INVENTORY_SLOT_BAG_0, PlayerInventoryItem};
use wow_world_core::session::{OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp,
    OwnedItemSetAccessLikeCpp, PacketPublicationAccessLikeCpp, PlayerStatsAccessLikeCpp,
    InventoryValuationAccessLikeCpp, PlayerRegistrySyncAccessLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::InventoryValuationCatalogViewLikeCpp;
use wow_world_inventory::{InventoryState, ItemModsCatalogsViewLikeCpp};
use wow_world_lifecycle::SessionLifecycleState;
use tracing::warn;
use super::equip_contracts::{bind_inventory_item_for_destination_like_cpp,
    item_dynamic_flags_changed_like_cpp, item_storage_mutable_persistence_like_cpp};

pub struct InventoryEquipCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    lifecycle: &'a SessionLifecycleState,
    access: OwnedInventoryAccessLikeCpp<'a>,
    modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
    item_sets: OwnedItemSetAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    stats: PlayerStatsAccessLikeCpp<'a>,
    valuation: InventoryValuationAccessLikeCpp<'a>,
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    item_effect_store: Option<&'a Arc<ItemEffectStore>>,
    enchantment_store: Option<&'a SpellItemEnchantmentStore>,
    mods_catalogs: ItemModsCatalogsViewLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    valuation_catalogs: InventoryValuationCatalogViewLikeCpp<'a>,
    loot: &'a wow_world_loot::LootState,
    registry: PlayerRegistrySyncAccessLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: InventoryEquipFixtureRefsLikeCpp<'a>,
    consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    min_item_level: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    max_item_level: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    registry_hydration: Option<crate::registry_sync::PlayerRegistryHydrationContext<'a>>,
}
#[cfg(any(test, feature = "test-fixtures"))]
pub struct InventoryEquipFixtureRefsLikeCpp<'a> {
    level: &'a u8,
    form: &'a u32,
    visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
}
#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> InventoryEquipFixtureRefsLikeCpp<'a> {
    pub fn new(level: &'a u8, form: &'a u32,
        visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>) -> Self {
        Self { level, form, visible_auras }
    }
}
impl<'a> InventoryEquipCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(inventory: &'a mut InventoryState, lifecycle: &'a SessionLifecycleState,
        access: OwnedInventoryAccessLikeCpp<'a>, modifiers: OwnedItemModifiersAccessLikeCpp<'a>,
        item_sets: OwnedItemSetAccessLikeCpp<'a>, publication: PacketPublicationAccessLikeCpp<'a>,
        stats: PlayerStatsAccessLikeCpp<'a>, valuation: InventoryValuationAccessLikeCpp<'a>,
        item_store: Option<&'a Arc<ItemStore>>, item_stats_store: Option<&'a Arc<ItemStatsStore>>,
        item_effect_store: Option<&'a Arc<ItemEffectStore>>, enchantment_store: Option<&'a SpellItemEnchantmentStore>,
        mods_catalogs: ItemModsCatalogsViewLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] valuation_catalogs: InventoryValuationCatalogViewLikeCpp<'a>,
        loot: &'a wow_world_loot::LootState,
        registry: PlayerRegistrySyncAccessLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: InventoryEquipFixtureRefsLikeCpp<'a>,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] min_item_level: u32,
        #[cfg(any(test, feature = "test-fixtures"))] max_item_level: u32,
        #[cfg(any(test, feature = "test-fixtures"))] registry_hydration: Option<crate::registry_sync::PlayerRegistryHydrationContext<'a>>,
    ) -> Self {
        Self { inventory, lifecycle, access, modifiers, item_sets, publication, stats, valuation,
            item_store, item_stats_store, item_effect_store, enchantment_store, mods_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] valuation_catalogs,
            loot, registry,
            #[cfg(any(test, feature = "test-fixtures"))] fixtures,
            consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))] min_item_level,
            #[cfg(any(test, feature = "test-fixtures"))] max_item_level,
            #[cfg(any(test, feature = "test-fixtures"))] registry_hydration }
    }
    fn player_guid(&self) -> Option<ObjectGuid> { self.access.player_guid_like_cpp() }
    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<PlayerInventoryItem> {
        self.inventory.get_inventory_item_by_pos_with_access_like_cpp(&self.access, self.item_store, self.item_stats_store, bag, slot)
    }
    fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<wow_entities::Item> {
        self.inventory.resolved_player_inventory_item_object_with_access_like_cpp(&self.access, guid)
    }
    fn inventory_remove_enchantment_persistence_like_cpp(&self, guid: ObjectGuid, clear: bool) -> Option<(String, Vec<EnchantmentSlot>)> {
        self.inventory.inventory_remove_enchantment_persistence_with_access_like_cpp(&self.access, self.enchantment_store, guid, clear)
    }
    fn inventory_container_db_guid_like_cpp(&self, bag: u8) -> Option<u64> {
        self.inventory.inventory_container_db_guid_with_access_like_cpp(&self.access, bag)
    }
    fn item_effect_count_like_cpp(&self, entry: u32) -> usize {
        self.item_effect_store.map(|store| store.item_effects_for_item_id_like_cpp(entry).len().min(wow_entities::MAX_ITEM_SPELLS)).unwrap_or(0)
    }
    fn send_equip_error(&self, result: InventoryResult, item1: Option<ObjectGuid>, item2: Option<ObjectGuid>, level: u32, category: u32) {
        self.access.send_equip_error_like_cpp(result, item1, item2, level, category);
    }
    fn apply_inventory_item_remove_side_effects_like_cpp(&mut self, bag: u8, slot: u8, guid: ObjectGuid, cleared: &[EnchantmentSlot]) -> bool {
        super::effects::InventorySwapEffectsCxLikeCpp::new(
            self.inventory, self.access.reborrow_like_cpp(), self.modifiers.reborrow_like_cpp(),
            self.item_sets.reborrow_like_cpp(), self.publication.reborrow_like_cpp(), self.mods_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] self.fixtures.level,
            #[cfg(any(test, feature = "test-fixtures"))] self.fixtures.form,
            self.consumer_test,
        ).remove_item_effects_like_cpp(bag, slot, guid, cleared)
    }
    fn apply_inventory_item_store_side_effects_like_cpp(&mut self, bag: u8, slot: u8, guid: ObjectGuid) -> bool {
        super::effects::InventorySwapEffectsCxLikeCpp::new(
            self.inventory, self.access.reborrow_like_cpp(), self.modifiers.reborrow_like_cpp(),
            self.item_sets.reborrow_like_cpp(), self.publication.reborrow_like_cpp(), self.mods_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] self.fixtures.level,
            #[cfg(any(test, feature = "test-fixtures"))] self.fixtures.form,
            self.consumer_test,
        ).store_item_effects_like_cpp(bag, slot, guid)
    }
    fn apply_committed_inventory_item_relocation_like_cpp(&mut self, src_bag: u8, src_slot: u8, dst_bag: u8, dst_slot: u8, count: u32) -> bool {
        super::relocation::InventoryCommittedRelocationCxLikeCpp::new(self.inventory,
            self.access.reborrow_like_cpp(), self.item_store, self.item_stats_store,
        ).apply_committed_inventory_item_relocation_like_cpp(src_bag, src_slot, dst_bag, dst_slot, count)
    }
    fn apply_inventory_item_object_updates_like_cpp(&mut self, guid: ObjectGuid, updates: &[wow_entities::ItemObjectUpdateLikeCpp]) -> bool {
        self.inventory.apply_inventory_item_object_updates_with_access_like_cpp(&self.access, guid, updates)
    }
    fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        super::positions::InventoryPositionPublicationCxLikeCpp::new(self.inventory,
            self.access.reborrow_like_cpp(), self.publication.reborrow_like_cpp(),
            self.item_store, self.item_stats_store, self.stats.reborrow_like_cpp(),
        ).publish_inventory_position_changes_like_cpp(positions);
    }
    fn send_item_relocation_values_update_like_cpp(&self, guid: ObjectGuid, flags2: bool, cleared: &[EnchantmentSlot]) {
        self.inventory.send_item_relocation_values_update_with_access_like_cpp(&self.access, &self.publication, guid, flags2, cleared);
    }
    fn send_item_dynamic_flags_values_update_like_cpp(&self, guid: ObjectGuid) {
        self.inventory.send_item_dynamic_flags_values_update_with_access_like_cpp(&self.access, &self.publication, guid);
    }
    fn send_represented_item_bonus_player_stat_update_like_cpp(&mut self) -> bool {
        if crate::stats::CharacterStatsApplicationCxLikeCpp::new(self.stats.reborrow_like_cpp(),
            self.inventory, self.publication.reborrow_like_cpp()).send_stat_update_like_cpp() {
            return true;
        }
        let Some(update) = self.inventory.represented_item_bonus_player_stat_update_object_with_access_like_cpp(&self.modifiers, &self.valuation) else { return false; };
        self.publication.send_packet(&update);
        true
    }
    fn record_represented_titan_grip_penalty_action_like_cpp(&mut self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test {
            let main_template = self.inventory.quest_reward_inventory_item_from_runtime_with_access_like_cpp(&self.access, wow_entities::EQUIPMENT_SLOT_MAINHAND)
                .and_then(|item| wow_world_core::catalogs::item::item_storage_template_like_cpp(self.item_store, self.item_stats_store, item.entry_id));
            let off_template = self.inventory.quest_reward_inventory_item_from_runtime_with_access_like_cpp(&self.access, wow_entities::EQUIPMENT_SLOT_OFFHAND)
                .and_then(|item| wow_world_core::catalogs::item::item_storage_template_like_cpp(self.item_store, self.item_stats_store, item.entry_id));
            let using_two_handed = wow_entities::player_rules::is_using_two_handed_weapon_in_one_hand_template(main_template.as_ref(), off_template.as_ref());
            let Some(action) = self.access.titan_grip_penalty_action_for_inventory_like_cpp(using_two_handed, self.fixtures.visible_auras) else { return; };
            if action != wow_entities::TitanGripPenaltyAction::None {
                self.inventory.record_represented_titan_grip_penalty_action_for_test_like_cpp(action);
            }
        }
    }
    fn record_represented_avg_equipped_item_level_update_like_cpp(&mut self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test {
            let Some(avg) = self.inventory.represented_avg_equipped_item_level_with_access_like_cpp(
                &self.access, &self.valuation, &self.modifiers, &self.valuation_catalogs,
                self.fixtures.level, self.min_item_level, self.max_item_level,
            ) else { return; };
            self.inventory.record_represented_avg_equipped_item_level_update_for_test_like_cpp(avg);
        }
    }
    fn sync_player_registry_state_like_cpp(&mut self) {
        let Some(control) = self.registry.control_binding_if_available_like_cpp() else { return; };
        let sync = crate::registry_sync::PlayerRegistrySyncContext::new(self.registry.reborrow_like_cpp(), control, self.loot);
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(hydration) = self.registry_hydration.as_ref() {
            sync.sync_with_fixture_hydration_like_cpp(hydration);
            return;
        }
        sync.sync();
    }

    pub async fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) {
        let [destination_bag, destination_slot] = destination.to_be_bytes();
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(source) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return;
        };
        let Some(runtime_item) = self.resolved_inventory_item_object_like_cpp(source.guid) else {
            return;
        };
        let Some((enchantments, cleared_enchantments)) = self
            .inventory_remove_enchantment_persistence_like_cpp(
                source.guid,
                source_bag == INVENTORY_SLOT_BAG_0
                    && source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        else {
            return;
        };
        let mut planned_item = runtime_item.clone();
        bind_inventory_item_for_destination_like_cpp(&mut planned_item, destination);
        let dynamic_flags_changed =
            item_dynamic_flags_changed_like_cpp(&runtime_item, &planned_item);
        for slot in &cleared_enchantments {
            planned_item.clear_enchantment(*slot);
        }
        let mutable = item_storage_mutable_persistence_like_cpp(
            source.db_guid,
            &planned_item,
            planned_item.count(),
            planned_item.item_flags_bits(),
            enchantments,
            self.item_effect_count_like_cpp(source.entry_id),
        );
        let Some(container_db_guid) = self.inventory_container_db_guid_like_cpp(destination_bag)
        else {
            return;
        };
        let Some(inventory_port) = self.lifecycle.player_inventory_persistence_port_like_cpp()
        else {
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                None,
                0,
                0,
            );
            return;
        };
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::Equip(
            wow_persistence::InventoryEquipPersistenceLikeCpp {
                mutable_item: mutable,
                delete_source_link_owner_guid: player_guid.counter() as u64,
                delete_source_link_item_guid: source.db_guid,
                destination_link: wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: container_db_guid,
                    slot: destination_slot,
                    item_guid: source.db_guid,
                },
            },
        );
        let outcome = inventory_port
            .persist_inventory_mutation_like_cpp(request)
            .await;
        if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
        | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
        {
            warn!(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                error = %reason,
                "inventory equip transaction failed; runtime left unchanged"
            );
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                None,
                0,
                0,
            );
            return;
        }

        let removed_mods = self.apply_inventory_item_remove_side_effects_like_cpp(
            source_bag,
            source_slot,
            source.guid,
            &cleared_enchantments,
        );
        let relocated = self.apply_committed_inventory_item_relocation_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            runtime_item.count(),
        );
        debug_assert!(relocated);
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            source.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::ReplaceAllItemFlags(
                ItemFieldFlags::from_bits_retain(planned_item.item_flags_bits()),
            )],
        );
        let added_mods = self.apply_inventory_item_store_side_effects_like_cpp(
            destination_bag,
            destination_slot,
            source.guid,
        );
        self.publish_inventory_position_changes_like_cpp(&[
            (source_bag, source_slot),
            (destination_bag, destination_slot),
        ]);
        self.send_item_relocation_values_update_like_cpp(source.guid, true, &cleared_enchantments);
        if dynamic_flags_changed {
            // C++ VisualizeItem dirties ITEM_DATA_DYNAMIC_FLAGS when the
            // destination applies OnEquip/OnAcquire binding.
            self.send_item_dynamic_flags_values_update_like_cpp(source.guid);
        }
        if removed_mods || added_mods {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        self.record_represented_titan_grip_penalty_action_like_cpp();
        self.record_represented_avg_equipped_item_level_update_like_cpp();
        self.sync_player_registry_state_like_cpp();
    }
}
