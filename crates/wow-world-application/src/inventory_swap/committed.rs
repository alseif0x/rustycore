// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_data::{ItemStatsStore, ItemStore};
use wow_entities::{INVENTORY_SLOT_BAG_0, ItemObjectUpdateLikeCpp, PlayerInventoryItem};
use wow_world_core::session::OwnedInventoryAccessLikeCpp;
use wow_world_inventory::InventoryState;

/// Applies an already committed swap, retaining each original mutation scope.
pub struct InventoryCommittedSwapCxLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    access: OwnedInventoryAccessLikeCpp<'a>,
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
}

impl<'a> InventoryCommittedSwapCxLikeCpp<'a> {
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

    fn resolved_inventory_items_like_cpp(
        &self,
    ) -> Option<std::collections::HashMap<u8, PlayerInventoryItem>> {
        self.inventory
            .resolved_inventory_items_with_access_like_cpp(&self.access)
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

    pub fn apply_committed_inventory_item_swap_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
    ) -> bool {
        if source_bag == destination_bag && source_slot == destination_slot {
            return false;
        }
        let Some(source) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return false;
        };
        let Some(destination) = self.get_inventory_item_by_pos(destination_bag, destination_slot)
        else {
            return false;
        };
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return false;
        };

        let container_guid = |bag: u8| {
            if bag == INVENTORY_SLOT_BAG_0 {
                Some(ObjectGuid::EMPTY)
            } else {
                inventory_items.get(&bag).map(|item| item.guid)
            }
        };
        let Some(source_container) = container_guid(source_bag) else {
            return false;
        };
        let Some(destination_container) = container_guid(destination_bag) else {
            return false;
        };

        let source_bag_size = self
            .item_storage_template(source.entry_id)
            .map(|template| template.container_slots)
            .filter(|size| *size > 0);
        let destination_bag_size = self
            .item_storage_template(destination.entry_id)
            .map(|template| template.container_slots)
            .filter(|size| *size > 0);
        let source_children = item_objects
            .values()
            .filter(|item| item.container_guid() == source.guid)
            .map(|item| (item.slot(), item.object().guid()))
            .collect::<Vec<_>>();
        let destination_children = item_objects
            .values()
            .filter(|item| item.container_guid() == destination.guid)
            .map(|item| (item.slot(), item.object().guid()))
            .collect::<Vec<_>>();

        if source_bag == INVENTORY_SLOT_BAG_0 {
            self.remove_inventory_item_like_cpp(source_slot);
        }
        if destination_bag == INVENTORY_SLOT_BAG_0 {
            self.remove_inventory_item_like_cpp(destination_slot);
        }
        if destination_bag == INVENTORY_SLOT_BAG_0 {
            self.insert_inventory_item_like_cpp(destination_slot, source.clone());
        }
        if source_bag == INVENTORY_SLOT_BAG_0 {
            self.insert_inventory_item_like_cpp(source_slot, destination.clone());
        }

        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            source.guid,
            &[
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
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            destination.guid,
            &[
                wow_entities::ItemObjectUpdateLikeCpp::SetSlot(source_slot),
                wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                    source_container,
                    source_bag,
                ),
                wow_entities::ItemObjectUpdateLikeCpp::SetContainedIn(
                    if source_container.is_empty() {
                        player_guid
                    } else {
                        source_container
                    },
                ),
            ],
        );
        for (_, child_guid) in &source_children {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                *child_guid,
                &[
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                        source.guid,
                        destination_slot,
                    ),
                ],
            );
        }
        for (_, child_guid) in &destination_children {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                *child_guid,
                &[
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                        destination.guid,
                        source_slot,
                    ),
                ],
            );
        }

        self.access
            .apply_committed_inventory_swap_native_placement_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                source.guid,
                destination.guid,
                source_bag_size,
                destination_bag_size,
                &source_children,
                &destination_children,
            );
        true
    }
}
