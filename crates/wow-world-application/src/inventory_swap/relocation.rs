// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_data::{ItemStatsStore, ItemStore};
use wow_entities::{INVENTORY_SLOT_BAG_0, ItemObjectUpdateLikeCpp, PlayerInventoryItem};
use wow_world_core::session::OwnedInventoryAccessLikeCpp;
use wow_world_inventory::InventoryState;

/// Applies a committed relocation without combining the original mutation scopes.
pub struct InventoryCommittedRelocationCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    access: OwnedInventoryAccessLikeCpp<'a>,
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
}
impl<'a> InventoryCommittedRelocationCxLikeCpp<'a> {
    pub fn new(
        inventory: &'a mut InventoryState,
        access: OwnedInventoryAccessLikeCpp<'a>,
        item_store: Option<&'a Arc<ItemStore>>,
        item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    ) -> Self {
        Self {
            inventory,
            access,
            item_store,
            item_stats_store,
        }
    }
    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<PlayerInventoryItem> {
        self.inventory
            .get_inventory_item_by_pos_with_access_like_cpp(
                &self.access,
                self.item_store,
                self.item_stats_store,
                bag,
                slot,
            )
    }
    fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<PlayerInventoryItem> {
        self.inventory
            .quest_reward_inventory_item_from_runtime_with_access_like_cpp(&self.access, slot)
    }
    fn resolved_inventory_item_objects_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<ObjectGuid, wow_entities::Item>> {
        self.inventory
            .resolved_inventory_item_objects_with_access_like_cpp(&self.access)
    }
    fn item_storage_template(&self, entry: u32) -> Option<wow_entities::ItemStorageTemplate> {
        wow_world_core::catalogs::item::item_storage_template_like_cpp(
            self.item_store,
            self.item_stats_store,
            entry,
        )
    }
    fn remove_inventory_item_like_cpp(&mut self, slot: u8) {
        let _ = self
            .inventory
            .remove_inventory_item_with_access_like_cpp(&self.access, slot);
    }
    fn insert_inventory_item_like_cpp(&mut self, slot: u8, item: PlayerInventoryItem) {
        let _ = self
            .inventory
            .insert_quest_reward_inventory_item_with_access_like_cpp(&self.access, slot, item);
    }
    fn player_guid(&self) -> Option<ObjectGuid> {
        self.access.player_guid_like_cpp()
    }
    fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        self.inventory
            .apply_inventory_item_object_updates_with_access_like_cpp(&self.access, guid, updates)
    }
    pub fn apply_committed_inventory_item_relocation_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        moved_count: u32,
    ) -> bool {
        if source_bag == destination_bag && source_slot == destination_slot {
            return false;
        }
        if self
            .get_inventory_item_by_pos(destination_bag, destination_slot)
            .is_some()
        {
            return false;
        }

        let Some(inventory_item) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return false;
        };
        let destination_container = if destination_bag == INVENTORY_SLOT_BAG_0 {
            None
        } else {
            self.resolved_inventory_item_like_cpp(destination_bag)
                .map(|bag| bag.guid)
        };
        if destination_bag != INVENTORY_SLOT_BAG_0 && destination_container.is_none() {
            return false;
        }

        if source_bag == INVENTORY_SLOT_BAG_0 {
            self.remove_inventory_item_like_cpp(source_slot);
        }
        if destination_bag == INVENTORY_SLOT_BAG_0 {
            self.insert_inventory_item_like_cpp(destination_slot, inventory_item.clone());
        }

        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let destination_container = destination_container.unwrap_or(ObjectGuid::EMPTY);
        let moved_bag_size = self
            .item_storage_template(inventory_item.entry_id)
            .map(|template| template.container_slots)
            .filter(|size| *size > 0);
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return false;
        };
        let moved_bag_children: Vec<_> = item_objects
            .values()
            .filter(|item| item.container_guid() == inventory_item.guid)
            .map(|item| (item.slot(), item.object().guid()))
            .collect();
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            inventory_item.guid,
            &[
                wow_entities::ItemObjectUpdateLikeCpp::SetCount(moved_count),
                wow_entities::ItemObjectUpdateLikeCpp::SetSlot(destination_slot),
                wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                    destination_container,
                    destination_bag,
                ),
                wow_entities::ItemObjectUpdateLikeCpp::SetContainedIn(
                    if destination_container.is_empty() {
                        player_guid
                    } else {
                        destination_container
                    },
                ),
            ],
        );

        // A bag's children keep the bag item GUID in the database, but the
        // runtime also caches the bag's current top-level slot.
        let child_guids: Vec<_> = item_objects
            .values()
            .filter(|item| item.container_guid() == inventory_item.guid)
            .map(|item| item.object().guid())
            .collect();
        for child_guid in child_guids {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                child_guid,
                &[
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                        inventory_item.guid,
                        destination_slot,
                    ),
                ],
            );
        }

        let item_guid = inventory_item.guid;
        self.access
            .apply_committed_inventory_relocation_native_placement_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                item_guid,
                moved_bag_size,
                &moved_bag_children,
            );
        true
    }
}
