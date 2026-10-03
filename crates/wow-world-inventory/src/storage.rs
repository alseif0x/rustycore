// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;

use wow_constants::InventoryType;
use wow_constants::item::EnchantmentSlot;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_constants::unit::WeaponAttackType;
use wow_core::ObjectGuid;
use wow_entities::{
    Item, ItemObjectUpdateLikeCpp, Player, PlayerEnchantDuration, PlayerEnchantTimeUpdate,
    PlayerInventoryItem as InventoryItem, PlayerInventoryRuntime, PlayerItemTimeUpdate,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, PLAYER_SLOT_END,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedCombatStatRecalculationLikeCpp;
use wow_world_core::session::{HubMut, HubRef};

/// C++ CombatRating::CR_ARMOR_PENETRATION (Unit.h:329).
pub const CR_ARMOR_PENETRATION_LIKE_CPP: u8 = 24;

impl crate::InventoryState {
    pub fn represented_item_inventory_type_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        item_guid: ObjectGuid,
    ) -> Option<InventoryType> {
        self.resolved_inventory_items_like_cpp(hub)?
            .values()
            .find(|item| item.guid == item_guid)
            .and_then(|item| item.inventory_type)
            .and_then(<InventoryType as num_traits::FromPrimitive>::from_u8)
            .or_else(|| {
                hub.catalogs
                    .items
                    .store
                    .as_ref()
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

    pub(crate) fn restore_inventory_item_enchantment_durations_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        durations: &[wow_entities::PlayerEnchantDuration],
    ) -> bool {
        let updates = durations
            .iter()
            .map(
                |duration| wow_entities::ItemObjectUpdateLikeCpp::SetEnchantmentDuration {
                    slot: duration.slot,
                    duration: duration.left_duration_ms,
                },
            )
            .collect::<Vec<_>>();
        self.apply_inventory_item_object_updates_like_cpp(hub, item_guid, &updates)
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
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
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
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
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
        let inventory_items = self.resolved_inventory_items_like_cpp(hub)?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp(hub)?;
        Some(
            inventory_items
                .values()
                .filter_map(|inventory_item| item_objects.get(&inventory_item.guid))
                .chain(item_objects.values().filter(|item| {
                    !item.container_guid().is_empty()
                        && item_objects.contains_key(&item.container_guid())
                }))
                .filter(|item| !item.is_in_trade())
                .fold(HashMap::new(), |mut counts, item| {
                    let entry_id = item.object().entry();
                    counts
                        .entry(entry_id)
                        .and_modify(|count| *count = count.saturating_add(item.count()))
                        .or_insert(item.count());
                    counts
                }),
        )
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

        if let Some((&slot, item)) = self
            .resolved_inventory_items_like_cpp(hub)?
            .iter()
            .find(|(_, item)| item.guid == item_guid)
        {
            if (slot as usize) < PLAYER_SLOT_END && !wow_entities::is_buyback_slot(slot) {
                return Some((INVENTORY_SLOT_BAG_0, slot, item.clone()));
            }
        }

        let runtime_item = self.resolved_inventory_item_object_like_cpp(hub, item_guid)?;
        if !runtime_item.is_in_bag() {
            return None;
        }

        let bag = runtime_item.bag_slot();
        let slot = runtime_item.slot();
        self.get_inventory_item_by_pos(hub, bag, slot)
            .filter(|item| item.guid == item_guid)
            .map(|item| (bag, slot, item))
    }

    pub fn direct_inventory_player_snapshot(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Player> {
        if let Some(player) = hub.core.with_owned_player_like_cpp(Clone::clone) {
            return Some(player);
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            let player_guid = hub.core.player_guid()?;
            let mut player = Player::new(None, false);
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(player_guid);
            player
                .set_inventory_slot_count(self.resolved_player_inventory_slot_count_like_cpp(hub)?);
            player.set_bank_bag_slot_count(self.resolved_player_bank_bag_slot_count_like_cpp(hub)?);

            let item_objects = self.resolved_inventory_item_objects_like_cpp(hub)?;
            for (&slot, item) in &self.resolved_inventory_items_like_cpp(hub)? {
                if (slot as usize) < PLAYER_SLOT_END && !wow_entities::is_buyback_slot(slot) {
                    let _ = player.store_top_level_item(slot, item.guid);
                    if is_represented_bag_slot(slot)
                        && item_objects.contains_key(&item.guid)
                        && let Some(template) = hub.catalogs.item_storage_template(item.entry_id)
                        && template.container_slots > 0
                    {
                        let _ =
                            player.register_bag_storage(slot, item.guid, template.container_slots);
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
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)
        else {
            return;
        };

        let Some(removed_enchantments) = hub.core.mutate_canonical_player_like_cpp(|player| {
            let removed_enchantments = player.remove_enchantment_durations(&mut item);
            let _ = player.remove_item_durations(&item);
            removed_enchantments
        }) else {
            return;
        };

        if removed_enchantments.is_empty() {
            return;
        }

        let _ = self.restore_inventory_item_enchantment_durations_like_cpp(
            hub,
            item_guid,
            &removed_enchantments,
        );
    }

    pub fn remove_inventory_tradeable_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)
        else {
            return;
        };

        let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
            player.remove_tradeable_item(&item);
        });
    }

    pub fn add_inventory_item_duration_refs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)
        else {
            return;
        };
        let Some((owner_guid, item_update, enchantment_updates)) =
            hub.core.mutate_canonical_player_like_cpp(|player| {
                let item_update = player.add_item_durations(&item);
                let enchantment_updates = player.add_enchantment_durations(&mut item);
                (player.guid(), item_update, enchantment_updates)
            })
        else {
            return;
        };

        self.insert_inventory_item_object(hub, item);
        if let Some(update) = item_update {
            self.send_item_time_update_plan(hub.shared(), &update);
        }
        self.send_item_enchant_time_update_plans(hub.shared(), owner_guid, &enchantment_updates);
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

    pub fn mutate_player_inventory_runtime_like_cpp<R>(
        &mut self,
        hub: &mut HubMut<'_>,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        hub.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let mut update = Some(update);
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
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
            let result =
                update.take().expect("inventory mutation closure runs once")(&mut inventory);
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
            return Some(result);
        }
        let result = hub.core.with_owned_player_mut_like_cpp(|player| {
            update.take().expect("inventory mutation closure runs once")(
                player.inventory_runtime_mut_like_cpp(),
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if result.is_some()
            && let Some(inventory) = hub
                .core
                .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
        {
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
        }
        result
    }

    pub fn resolved_player_inventory_runtime_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<PlayerInventoryRuntime> {
        if let Some(inventory) = hub
            .core
            .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
        {
            return Some(inventory);
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
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
            return Some(inventory);
        }
        None
    }

    pub fn resolved_inventory_items_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_like_cpp(hub)
            .map(|inventory| inventory.inventory_items().clone())
    }

    pub fn resolved_inventory_item_objects_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<HashMap<ObjectGuid, Item>> {
        self.resolved_player_inventory_runtime_like_cpp(hub)
            .map(|inventory| inventory.item_objects().clone())
    }

    pub fn resolved_inventory_item_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: u8,
    ) -> Option<InventoryItem> {
        self.resolved_player_inventory_runtime_like_cpp(hub)?
            .inventory_items()
            .get(&slot)
            .cloned()
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
                EQUIPMENT_SLOT_MAINHAND => Some(WeaponAttackType::BaseAttack),
                EQUIPMENT_SLOT_OFFHAND => Some(WeaponAttackType::OffAttack),
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
