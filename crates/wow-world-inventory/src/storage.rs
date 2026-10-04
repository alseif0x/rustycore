// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;
use std::sync::Arc;

use wow_constants::InventoryType;
use wow_constants::item::EnchantmentSlot;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::unit::WeaponAttackType;
use wow_core::ObjectGuid;
use wow_data::{ItemStatsStore, ItemStore};
use wow_entities::{
    Item, ItemObjectUpdateLikeCpp, PlayerEnchantDuration, PlayerEnchantTimeUpdate,
    PlayerInventoryItem as InventoryItem, PlayerInventoryRuntime, PlayerItemTimeUpdate,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, PLAYER_SLOT_END,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedCombatStatRecalculationLikeCpp;
use wow_world_core::session::{
    HubMut, HubRef, InventoryPlayerProjectionLikeCpp, OwnedInventoryAccessLikeCpp,
};

/// Detached inventory state already reserved by an earlier operation in the
/// same atomic storage plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectInventoryStorageOverlayLikeCpp {
    pub bag: u8,
    pub slot: u8,
    pub entry_id: u32,
    pub count: u32,
}

/// C++ CombatRating::CR_ARMOR_PENETRATION (Unit.h:329).
pub const CR_ARMOR_PENETRATION_LIKE_CPP: u8 = 24;

impl crate::InventoryState {
    pub fn represented_item_inventory_type_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        item_guid: ObjectGuid,
    ) -> Option<InventoryType> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.represented_item_inventory_type_with_access_like_cpp(
            &access,
            hub.catalogs.items.store.as_ref(),
            item_entry,
            item_guid,
        )
    }

    pub(crate) fn represented_item_inventory_type_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        item_store: Option<&Arc<ItemStore>>,
        item_entry: u32,
        item_guid: ObjectGuid,
    ) -> Option<InventoryType> {
        let inventory_items = self
            .resolved_player_inventory_runtime_with_access_like_cpp(access)?
            .inventory_items()
            .clone();
        inventory_items
            .values()
            .find(|item| item.guid == item_guid)
            .and_then(|item| item.inventory_type)
            .and_then(<InventoryType as num_traits::FromPrimitive>::from_u8)
            .or_else(|| {
                item_store
                    .and_then(|store| store.get(item_entry))
                    .and_then(|record| {
                        <InventoryType as num_traits::FromPrimitive>::from_i8(record.inventory_type)
                    })
            })
    }

    pub fn insert_inventory_item_object(
        &mut self,
        hub: &mut HubMut<'_>,
        item: Item,
    ) -> Option<Item> {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.store_item_object_like_cpp(item)
        })
        .flatten()
    }

    pub fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.apply_item_object_updates_like_cpp(item_guid, updates)
        })
        .unwrap_or(false)
    }

    pub fn transform_inventory_wrapped_gift_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        entry: u32,
        flags: u32,
        max_durability: u32,
    ) -> Option<u32> {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.transform_wrapped_gift_item_like_cpp(item_guid, entry, flags, max_durability)
        })
        .flatten()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn update_inventory_item_object_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        update: impl FnOnce(&mut Item),
    ) -> bool {
        let mut update = Some(update);
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            let Some(item) = inventory.item_objects_mut().get_mut(&item_guid) else {
                return false;
            };
            if let Some(update) = update.take() {
                update(item);
            }
            true
        })
        .unwrap_or(false)
    }

    pub fn clear_inventory_item_equipped_state_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        cleared_enchantments: &[EnchantmentSlot],
    ) -> bool {
        let mut updates = vec![ItemObjectUpdateLikeCpp::SetEquipped(false)];
        updates.extend(
            cleared_enchantments
                .iter()
                .copied()
                .map(ItemObjectUpdateLikeCpp::ClearEnchantment),
        );
        self.apply_inventory_item_object_updates_like_cpp(hub, item_guid, &updates)
    }

    pub fn set_inventory_item_equipped_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        equipped: bool,
    ) -> bool {
        self.apply_inventory_item_object_updates_like_cpp(
            hub,
            item_guid,
            &[ItemObjectUpdateLikeCpp::SetEquipped(equipped)],
        )
    }

    pub fn remove_inventory_item_object(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) -> Option<Item> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.remove_inventory_item_object_with_access_like_cpp(&access, item_guid)
    }

    pub fn remove_inventory_item_object_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) -> Option<Item> {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.remove_item_object_like_cpp(item_guid)
        })
        .flatten()
    }

    pub fn clear_inventory_items_and_objects_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.clear_items_and_objects_like_cpp();
        });
    }

    pub fn clear_all_inventory_runtime_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            *inventory = PlayerInventoryRuntime::default();
        });
        self.reset_represented_item_bonus_runtime_like_cpp(hub);
    }

    pub fn insert_inventory_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.store_item_in_slot_like_cpp(slot, item)
        })
        .flatten()
    }

    pub fn remove_inventory_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
    ) -> Option<InventoryItem> {
        self.remove_inventory_item_with_access_like_cpp(&hub.core.owned_inventory_access_like_cpp(), slot)
    }

    pub fn remove_inventory_item_with_access_like_cpp(
        &mut self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        slot: u8,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.remove_item_from_slot_like_cpp(slot)
        })
        .flatten()
    }

    pub fn update_inventory_item_metadata_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
        item_guid: ObjectGuid,
        entry_id: u32,
        inventory_type: Option<u8>,
    ) -> bool {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.update_slot_item_metadata_like_cpp(slot, item_guid, entry_id, inventory_type)
        })
        .unwrap_or(false)
    }

    pub fn represented_inventory_item_counts_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<u32, u32>> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.represented_inventory_item_counts_with_access_like_cpp(&access)
    }

    /// Resolve an inventory item by GUID following C++ `Player::GetItemByGuid`.
    ///
    /// C++ iterates direct inventory and represented bags. Rust still models
    /// nested bag contents through runtime `Item` objects, so this returns the
    /// effective `(bag, slot, item)` tuple needed by `DestroyItem`.
    pub fn get_inventory_item_by_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)> {
        if item_guid.is_empty() {
            return None;
        }

        let access = hub.core.owned_inventory_access_like_cpp();
        self.get_inventory_item_by_guid_with_access_like_cpp(
            &access,
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
            item_guid,
        )
    }

    /// Resolve the same GUID owner when the caller already has the operation's
    /// selected inventory access and catalog stores.
    pub(crate) fn get_inventory_item_by_guid_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        item_store: Option<&Arc<ItemStore>>,
        item_stats_store: Option<&Arc<ItemStatsStore>>,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)> {
        if item_guid.is_empty() {
            return None;
        }

        if let Some((&slot, item)) = self
            .resolved_inventory_items_with_access_like_cpp(access)?
            .iter()
            .find(|(_, item)| item.guid == item_guid)
        {
            if (slot as usize) < PLAYER_SLOT_END && !wow_entities::is_buyback_slot(slot) {
                return Some((INVENTORY_SLOT_BAG_0, slot, item.clone()));
            }
        }

        let runtime_item =
            self.resolved_inventory_item_object_with_access_like_cpp(access, item_guid)?;
        if !runtime_item.is_in_bag() {
            return None;
        }

        let bag = runtime_item.bag_slot();
        let slot = runtime_item.slot();
        self.get_inventory_item_by_pos_with_access_like_cpp(
            access,
            item_store,
            item_stats_store,
            bag,
            slot,
        )
        .filter(|item| item.guid == item_guid)
        .map(|item| (bag, slot, item))
    }

    pub fn direct_inventory_player_snapshot(
        &self,
        hub: HubRef<'_>,
    ) -> Option<InventoryPlayerProjectionLikeCpp> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.direct_inventory_player_snapshot_with_access_like_cpp(
            &access,
            hub.catalogs.items.store.as_ref(),
            hub.catalogs.items.stats_store.as_ref(),
        )
    }

    pub fn direct_inventory_player_snapshot_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        _item_store: Option<&Arc<ItemStore>>,
        _item_stats_store: Option<&Arc<ItemStatsStore>>,
    ) -> Option<InventoryPlayerProjectionLikeCpp> {
        if let Some(player) = access.inventory_player_projection_snapshot_like_cpp() {
            return Some(player);
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            let player_guid = access.player_guid_like_cpp()?;
            let mut player = InventoryPlayerProjectionLikeCpp::new_like_cpp(player_guid);
            player.set_inventory_slot_count_like_cpp(
                self.resolved_player_inventory_slot_count_with_access_like_cpp(access)?,
            );
            player.set_bank_bag_slot_count_like_cpp(
                self.resolved_player_bank_bag_slot_count_with_access_like_cpp(access)?,
            );

            let item_objects = self.resolved_inventory_item_objects_with_access_like_cpp(access)?;
            for (&slot, item) in &self.resolved_inventory_items_with_access_like_cpp(access)? {
                if (slot as usize) < PLAYER_SLOT_END && !wow_entities::is_buyback_slot(slot) {
                    let _ = player.store_top_level_item_like_cpp(slot, item.guid);
                    if crate::is_represented_bag_slot(slot)
                        && item_objects.contains_key(&item.guid)
                        && let Some(template) =
                            wow_world_core::catalogs::item::item_storage_template_like_cpp(
                                _item_store,
                                _item_stats_store,
                                item.entry_id,
                            )
                        && template.container_slots > 0
                    {
                        let _ = player.register_bag_storage_like_cpp(
                            slot,
                            item.guid,
                            template.container_slots,
                        );
                    }
                }
            }
            return Some(player);
        }

        None
    }

    pub fn remove_inventory_item_duration_refs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        self.remove_inventory_item_duration_refs_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(), item_guid,
        );
    }

    pub fn remove_inventory_item_duration_refs_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_player_inventory_item_object_with_access_like_cpp(access, item_guid)
        else {
            return;
        };

        let Some(removed_enchantments) = access.remove_item_duration_refs_like_cpp(&mut item) else {
            return;
        };

        if removed_enchantments.is_empty() {
            return;
        }

        let updates = removed_enchantments.iter().map(|duration| {
            ItemObjectUpdateLikeCpp::SetEnchantmentDuration {
                slot: duration.slot,
                duration: duration.left_duration_ms,
            }
        }).collect::<Vec<_>>();
        let _ = self.apply_inventory_item_object_updates_with_access_like_cpp(access, item_guid, &updates);
    }

    pub fn remove_inventory_tradeable_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        self.remove_inventory_tradeable_item_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(), item_guid,
        );
    }

    pub fn remove_inventory_tradeable_item_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(item) = self.resolved_player_inventory_item_object_with_access_like_cpp(access, item_guid)
        else {
            return;
        };

        access.remove_tradeable_item_like_cpp(&item);
    }

    pub fn add_inventory_item_duration_refs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        self.add_inventory_item_duration_refs_with_access_like_cpp(
            &hub.core.owned_inventory_access_like_cpp(),
            &hub.core.packet_publication_access_like_cpp(), item_guid,
        );
    }

    pub fn add_inventory_item_duration_refs_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        publication: &wow_world_core::session::PacketPublicationAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_player_inventory_item_object_with_access_like_cpp(access, item_guid)
        else {
            return;
        };
        let Some((owner_guid, item_update, enchantment_updates)) =
            access.add_item_duration_refs_like_cpp(&mut item)
        else {
            return;
        };

        let _ = self.mutate_player_inventory_runtime_with_access_like_cpp(access, |inventory| {
            inventory.store_item_object_like_cpp(item)
        });
        if let Some(update) = item_update {
            publication.send_packet(&wow_packet::packets::item::ItemTimeUpdate {
                item_guid: update.item_guid,
                duration_left: update.expiration,
            });
        }
        for update in &enchantment_updates {
            publication.send_packet(&wow_packet::packets::item::ItemEnchantTimeUpdate {
                owner_guid,
                item_guid: update.item_guid,
                slot: update.slot as u32,
                duration_left: update.duration_secs,
            });
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn mirror_player_inventory_runtime_to_legacy_like_cpp(
        &mut self,
        inventory: &PlayerInventoryRuntime,
    ) {
        self.player_item_test_fixture_like_cpp.inventory_items =
            inventory.inventory_items().clone();
        self.player_item_test_fixture_like_cpp.buyback_items = inventory.buyback_items().clone();
        self.player_item_test_fixture_like_cpp.buyback_price = *inventory.buyback_price();
        self.player_item_test_fixture_like_cpp.buyback_timestamp = *inventory.buyback_timestamp();
        self.player_item_test_fixture_like_cpp.current_buyback_slot =
            inventory.current_buyback_slot();
        self.inventory_item_objects = inventory.item_objects().clone();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn represented_player_inventory_runtime_like_cpp(&self) -> PlayerInventoryRuntime {
        let mut inventory = PlayerInventoryRuntime::default();
        inventory.inventory_items_mut().extend(
            self.player_item_test_fixture_like_cpp
                .inventory_items
                .clone(),
        );
        inventory
            .buyback_items_mut()
            .extend(self.player_item_test_fixture_like_cpp.buyback_items.clone());
        *inventory.buyback_price_mut() = self.player_item_test_fixture_like_cpp.buyback_price;
        *inventory.buyback_timestamp_mut() =
            self.player_item_test_fixture_like_cpp.buyback_timestamp;
        inventory.set_current_buyback_slot(
            self.player_item_test_fixture_like_cpp.current_buyback_slot,
        );
        inventory
            .item_objects_mut()
            .extend(self.inventory_item_objects.clone());
        inventory
    }

    pub fn mutate_player_inventory_runtime_like_cpp<R>(
        &mut self,
        hub: &mut HubMut<'_>,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.mutate_player_inventory_runtime_with_access_like_cpp(&access, update)
    }

    pub(crate) fn mutate_player_inventory_runtime_with_access_like_cpp<R>(
        &mut self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        access.invalidate_spell_hit_aura_authority_for_inventory_mutation_like_cpp();
        let mut update = Some(update);
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            let mut inventory = self.represented_player_inventory_runtime_like_cpp();
            let result =
                update.take().expect("inventory mutation closure runs once")(&mut inventory);
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
            return Some(result);
        }
        let result = access.with_inventory_runtime_mut_like_cpp(|inventory| {
            update.take().expect("inventory mutation closure runs once")(inventory)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if result.is_some()
            && let Some(inventory) = access.inventory_runtime_snapshot_like_cpp()
        {
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
        }
        result
    }

    pub fn resolved_player_inventory_runtime_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<PlayerInventoryRuntime> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_player_inventory_runtime_with_access_like_cpp(&access)
    }

    pub(crate) fn resolved_player_inventory_runtime_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<PlayerInventoryRuntime> {
        if let Some(inventory) = access.inventory_runtime_snapshot_like_cpp() {
            return Some(inventory);
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            return Some(self.represented_player_inventory_runtime_like_cpp());
        }
        None
    }

    pub fn resolved_inventory_items_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<u8, InventoryItem>> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_inventory_items_with_access_like_cpp(&access)
    }

    pub fn resolved_inventory_items_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)
            .map(|inventory| inventory.inventory_items().clone())
    }

    pub fn resolved_inventory_item_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        slot: u8,
    ) -> Option<InventoryItem> {
        let canonical = access.inventory_item_snapshot_like_cpp(slot);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            return self.inventory_items_like_cpp().get(&slot).cloned();
        }
        canonical
    }

    pub fn resolved_inventory_item_objects_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<ObjectGuid, Item>> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_inventory_item_objects_with_access_like_cpp(&access)
    }

    pub fn resolved_inventory_item_objects_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<HashMap<ObjectGuid, Item>> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)
            .map(|inventory| inventory.item_objects().clone())
    }

    pub fn resolved_inventory_item_object_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
        guid: ObjectGuid,
    ) -> Option<Item> {
        let canonical = access.inventory_item_object_snapshot_like_cpp(guid);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            return self.inventory_item_object_for_test_like_cpp(&guid).cloned();
        }
        canonical
    }

    pub fn resolved_inventory_item_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: u8,
    ) -> Option<InventoryItem> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.quest_reward_inventory_item_from_runtime_with_access_like_cpp(&access, slot)
    }

    pub fn resolved_inventory_item_object_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<Item> {
        self.resolved_player_inventory_runtime_like_cpp(hub)?
            .item_objects()
            .get(&guid)
            .cloned()
    }
}

impl crate::InventoryState {
    pub fn sync_canonical_direct_inventory_move_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        src: u8,
        dst_bag: u8,
        dst_slot: u8,
        item_guid: ObjectGuid,
    ) {
        let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
            let _ = player.remove_top_level_item(src);
            if dst_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(dst_slot, item_guid);
            } else {
                let _ = player.store_bag_item(dst_bag, dst_slot, item_guid);
            }
        });
    }

    pub fn sync_canonical_direct_inventory_remove_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        src: u8,
    ) {
        let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
            let _ = player.remove_top_level_item(src);
        });
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_inventory_item_combat_stat_recalculations_like_cpp(&mut self, slot: u8) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let attack = match slot {
                wow_entities::EQUIPMENT_SLOT_MAINHAND => Some(WeaponAttackType::BaseAttack),
                wow_entities::EQUIPMENT_SLOT_OFFHAND => Some(WeaponAttackType::OffAttack),
                _ => None,
            };
            if let Some(attack) = attack {
                self.player_item_test_fixture_like_cpp
                    .represented_combat_stat_recalculations_like_cpp
                    .push(RepresentedCombatStatRecalculationLikeCpp::Expertise { attack });
                self.player_item_test_fixture_like_cpp
                    .represented_combat_stat_recalculations_like_cpp
                    .push(RepresentedCombatStatRecalculationLikeCpp::Rating {
                        combat_rating: CR_ARMOR_PENETRATION_LIKE_CPP,
                    });
            }
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        &self.player_item_test_fixture_like_cpp.inventory_items
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_objects_like_cpp(&self) -> &HashMap<ObjectGuid, Item> {
        &self.inventory_item_objects
    }
}
