//! committed runtime for the existing persistence owner.

use super::*;

impl WorldSession {
    /// Apply a previously validated and committed C++ `RemoveItem` +
    /// `StoreItem`/`BankItem` relocation to the session-owned inventory.
    ///
    /// `character_inventory.bag` persistence is handled by the caller. This
    /// method deliberately has no fallible database work so memory cannot move
    /// ahead of the transaction.
    pub(crate) fn apply_committed_inventory_item_relocation_like_cpp(
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
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(source_slot);
            } else {
                let _ = player.remove_bag_item(source_bag, source_slot);
            }
            if destination_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(destination_slot, item_guid);
                if is_represented_bag_slot(destination_slot)
                    && let Some(bag_size) = moved_bag_size
                    && player
                        .register_bag_storage(destination_slot, item_guid, bag_size)
                        .is_ok()
                {
                    for &(child_slot, child_guid) in &moved_bag_children {
                        let _ = player.store_bag_item(destination_slot, child_slot, child_guid);
                    }
                }
            } else {
                let _ = player.store_bag_item(destination_bag, destination_slot, item_guid);
            }
        });
        true
    }
    /// Publish a committed C++ real swap after both database positions were
    /// replaced in one transaction. Both positions must still contain the
    /// pre-commit items; all runtime mutation is therefore infallible after
    /// this preflight.
    pub(crate) fn apply_committed_inventory_item_swap_like_cpp(
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

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(source_slot);
            } else {
                let _ = player.remove_bag_item(source_bag, source_slot);
            }
            if destination_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(destination_slot);
            } else {
                let _ = player.remove_bag_item(destination_bag, destination_slot);
            }

            if destination_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(destination_slot, source.guid);
                if is_represented_bag_slot(destination_slot)
                    && let Some(size) = source_bag_size
                    && player
                        .register_bag_storage(destination_slot, source.guid, size)
                        .is_ok()
                {
                    for &(slot, guid) in &source_children {
                        let _ = player.store_bag_item(destination_slot, slot, guid);
                    }
                }
            } else {
                let _ = player.store_bag_item(destination_bag, destination_slot, source.guid);
            }

            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(source_slot, destination.guid);
                if is_represented_bag_slot(source_slot)
                    && let Some(size) = destination_bag_size
                    && player
                        .register_bag_storage(source_slot, destination.guid, size)
                        .is_ok()
                {
                    for &(slot, guid) in &destination_children {
                        let _ = player.store_bag_item(source_slot, slot, guid);
                    }
                }
            } else {
                let _ = player.store_bag_item(source_bag, source_slot, destination.guid);
            }
        });
        true
    }
    /// Remove a source item after its complete stack was merged into existing
    /// destination stacks by a committed storage transaction.
    pub(crate) fn apply_committed_inventory_item_removal_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(source) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return false;
        };
        if source.guid != item_guid {
            return false;
        }
        if source_bag == INVENTORY_SLOT_BAG_0 {
            self.remove_inventory_item_like_cpp(source_slot);
        }
        self.remove_inventory_item_object(item_guid);
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            if source_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(source_slot);
            } else {
                let _ = player.remove_bag_item(source_bag, source_slot);
            }
        });
        true
    }
    /// Publish one new item whose CharacterDB rows already committed.
    pub(crate) fn apply_committed_new_inventory_item_at_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        mut item_object: Item,
    ) -> bool {
        if self.get_inventory_item_by_pos(bag, slot).is_some() {
            return false;
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            item_object.set_container_guid(ObjectGuid::EMPTY);
            item_object.set_slot(slot);
            self.insert_inventory_item_like_cpp(slot, inventory_item.clone());
        } else {
            let Some(bag_item) = self.resolved_inventory_item_like_cpp(bag) else {
                return false;
            };
            item_object.set_contained_in(bag_item.guid);
            item_object.set_slot(slot);
            item_object.set_container_guid_and_slot(bag_item.guid, bag);
        }

        let item_guid = inventory_item.guid;
        let new_bag_size = (bag == INVENTORY_SLOT_BAG_0 && is_represented_bag_slot(slot))
            .then(|| self.item_storage_template(inventory_item.entry_id))
            .flatten()
            .map(|template| template.container_slots)
            .filter(|size| *size > 0);
        self.insert_inventory_item_object(item_object);
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            if bag == INVENTORY_SLOT_BAG_0 {
                let stored = player.store_top_level_item(slot, item_guid);
                debug_assert!(stored.is_ok());
                if let Some(bag_size) = new_bag_size {
                    // C++ installs the new `Bag` object in m_items before a
                    // later sequential StoreNewItem can address its children.
                    // The canonical Rust Player keeps bag contents in a
                    // separate registry, so establish it at the same point.
                    let registered = player.register_bag_storage(slot, item_guid, bag_size);
                    debug_assert!(registered.is_ok());
                }
            } else {
                let stored = player.store_bag_item(bag, slot, item_guid);
                debug_assert!(stored.is_ok());
            }
        });
        true
    }
}
