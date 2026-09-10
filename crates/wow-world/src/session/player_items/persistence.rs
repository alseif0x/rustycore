//! Save and load plans for represented item and inventory state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

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
        self.update_inventory_item_object_like_cpp(inventory_item.guid, |item| {
            item.set_count(moved_count);
            item.set_slot(destination_slot);
            item.set_container_guid_and_slot(destination_container, destination_bag);
            item.set_contained_in(if destination_container.is_empty() {
                player_guid
            } else {
                destination_container
            });
        });

        // A bag's children keep the bag item GUID in the database, but the
        // runtime also caches the bag's current top-level slot.
        let child_guids: Vec<_> = item_objects
            .values()
            .filter(|item| item.container_guid() == inventory_item.guid)
            .map(|item| item.object().guid())
            .collect();
        for child_guid in child_guids {
            self.update_inventory_item_object_like_cpp(child_guid, |item| {
                item.set_container_guid_and_slot(inventory_item.guid, destination_slot);
            });
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
        self.update_inventory_item_object_like_cpp(source.guid, |item| {
            item.set_slot(destination_slot);
            item.set_container_guid_and_slot(destination_container, destination_bag);
            item.set_contained_in(if destination_container.is_empty() {
                player_guid
            } else {
                destination_container
            });
        });
        self.update_inventory_item_object_like_cpp(destination.guid, |item| {
            item.set_slot(source_slot);
            item.set_container_guid_and_slot(source_container, source_bag);
            item.set_contained_in(if source_container.is_empty() {
                player_guid
            } else {
                source_container
            });
        });
        for (_, child_guid) in &source_children {
            self.update_inventory_item_object_like_cpp(*child_guid, |item| {
                item.set_container_guid_and_slot(source.guid, destination_slot);
            });
        }
        for (_, child_guid) in &destination_children {
            self.update_inventory_item_object_like_cpp(*child_guid, |item| {
                item.set_container_guid_and_slot(destination.guid, source_slot);
            });
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
    pub fn set_stored_item_money_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.stored_item_money = Some(port);
    }
    pub(crate) fn stored_item_money_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .stored_item_money
            .clone()
    }
    pub fn set_item_template_addon_catalog_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .catalogs
            .item_template_addon_catalog = Some(port);
    }
    pub(crate) fn item_template_addon_catalog_persistence_port_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .catalogs
            .item_template_addon_catalog
            .clone()
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
    /// C++ `Player::SwapItem` preflight (child redirects, life state,
    /// `CanUnequipItem`, and bag-in-bag guards) for live session positions.
    pub(crate) fn plan_inventory_swap_preflight_like_cpp(
        &self,
        src: u16,
        dst: u16,
    ) -> Option<SwapItemPreflightPlan> {
        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        let source = self.get_inventory_item_by_pos(src_bag, src_slot);
        let destination = self.get_inventory_item_by_pos(dst_bag, dst_slot);

        let preflight_item = |item: &InventoryItem, bag: u8, slot: u8, swap: bool| {
            let runtime_item = self.resolved_inventory_item_object_like_cpp(item.guid);
            let proto = self.item_storage_template(item.entry_id);
            let is_bag = proto
                .as_ref()
                .is_some_and(|template| template.container_slots > 0);
            let parent_pos = runtime_item
                .as_ref()
                .filter(|item| item.has_item_flag(ItemFieldFlags::CHILD))
                .and_then(|item| self.get_inventory_item_by_guid_like_cpp(item.data().creator))
                .map(|(bag, slot, _)| make_item_pos(bag, slot));
            SwapItemPreflightItem {
                is_bag,
                is_empty_bag: is_bag && !self.direct_item_contains_items(item.guid),
                is_child: runtime_item
                    .as_ref()
                    .is_some_and(|item| item.has_item_flag(ItemFieldFlags::CHILD)),
                parent_pos,
                can_unequip_result: self.can_unequip_inventory_item_at_like_cpp(
                    bag,
                    slot,
                    swap,
                    runtime_item.as_ref(),
                    proto.as_ref(),
                    self.direct_item_contains_items(item.guid),
                ),
            }
        };

        let source_is_bag_pos = is_bag_pos(src);
        let destination_is_bag_pos = is_bag_pos(dst);
        let source_is_bag = source.as_ref().is_some_and(|item| {
            self.item_storage_template(item.entry_id)
                .is_some_and(|template| template.container_slots > 0)
        });
        let destination_is_empty_bag = destination.as_ref().is_some_and(|item| {
            self.item_storage_template(item.entry_id)
                .is_some_and(|template| template.container_slots > 0)
                && !self.direct_item_contains_items(item.guid)
        });
        let source_is_empty_bag = source_is_bag
            && source
                .as_ref()
                .is_some_and(|item| !self.direct_item_contains_items(item.guid));
        let source_swap = !source_is_bag_pos || destination_is_bag_pos || destination_is_empty_bag;
        let destination_swap = !destination_is_bag_pos || source_is_bag_pos || source_is_empty_bag;
        let source_preflight = source
            .as_ref()
            .map(|item| preflight_item(item, src_bag, src_slot, source_swap));
        let destination_preflight = destination
            .as_ref()
            .map(|item| preflight_item(item, dst_bag, dst_slot, destination_swap));

        let player = self.direct_inventory_player_snapshot()?;
        let player_is_alive = self.resolved_player_is_alive_like_cpp()?;
        Some(player.swap_item_preflight_plan(
            src,
            dst,
            player_is_alive,
            source_preflight,
            destination_preflight,
        ))
    }
    pub fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_new_direct_inventory_item_at(entry_id, count, NULL_BAG, NULL_SLOT)
    }
    pub fn plan_store_new_direct_inventory_item_at(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_direct_inventory_item_like_cpp(
            entry_id,
            count,
            bag,
            slot,
            None,
            false,
            &[],
            &[],
        )
    }
    pub(crate) fn plan_store_new_direct_inventory_item_with_overlays_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        overlays: &[DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_direct_inventory_item_like_cpp(
            entry_id,
            count,
            NULL_BAG,
            NULL_SLOT,
            None,
            false,
            overlays,
            vacated_positions,
        )
    }
    pub(crate) fn plan_store_existing_direct_inventory_item_like_cpp(
        &self,
        source_slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_existing_inventory_item_like_cpp(INVENTORY_SLOT_BAG_0, source_slot)
    }
    pub(crate) fn plan_store_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        self.plan_store_existing_inventory_item_at_like_cpp(
            source_bag,
            source_slot,
            NULL_BAG,
            NULL_SLOT,
            false,
        )
    }
    pub(crate) fn plan_store_existing_inventory_item_at_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        let inventory_item = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)?;
        self.plan_store_direct_inventory_item_like_cpp(
            inventory_item.entry_id,
            source_item.count(),
            destination_bag,
            destination_slot,
            Some(&source_item),
            swap,
            &[],
            &[],
        )
    }
    fn plan_store_direct_inventory_item_like_cpp(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
        source_item: Option<&Item>,
        swap: bool,
        overlays: &[DirectInventoryStorageOverlayLikeCpp],
        vacated_positions: &[(u8, u8)],
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        let mut player = self.direct_inventory_player_snapshot()?;
        // C++ processes every valid deposit (including recursive bag
        // contents) before it calls CanStoreNewItem for withdrawals. Remove
        // those detached positions from this planning snapshot in the same
        // child-before-parent order.
        for &(vacated_bag, vacated_slot) in vacated_positions {
            if vacated_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.remove_top_level_item(vacated_slot);
            } else {
                let _ = player.remove_bag_item(vacated_bag, vacated_slot);
            }
        }
        let proto = self.item_storage_template(entry_id);
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        let mut overlay_items = Vec::with_capacity(overlays.len());
        for (index, overlay) in overlays.iter().enumerate() {
            let mut item = self
                .get_inventory_item_by_pos(overlay.bag, overlay.slot)
                .and_then(|inventory_item| item_objects.get(&inventory_item.guid))
                .cloned()
                .unwrap_or_else(|| {
                    let mut item = Item::default();
                    let placeholder_counter = i64::MAX.saturating_sub(index as i64);
                    item.object_mut().create(ObjectGuid::create_item(
                        self.realm_id(),
                        placeholder_counter,
                    ));
                    item.object_mut().set_entry(overlay.entry_id);
                    item
                });
            // An overlay can reuse a slot vacated by an earlier deposit, so
            // its planned entry is authoritative over the stale runtime item
            // that still occupies the slot until the transaction commits.
            item.object_mut().set_entry(overlay.entry_id);
            item.set_count(overlay.count);
            item.set_slot(overlay.slot);
            overlay_items.push((overlay.bag, overlay.slot, item));
        }
        let mut template_cache = HashMap::new();
        for item in item_objects.values() {
            let entry_id = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) = template_cache.entry(entry_id)
            {
                if let Some(template) = self.item_storage_template(entry_id) {
                    entry.insert(template);
                }
            }
        }
        for (_, _, item) in &overlay_items {
            let entry_id = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) = template_cache.entry(entry_id)
                && let Some(template) = self.item_storage_template(entry_id)
            {
                entry.insert(template);
            }
        }

        let mut represented_bag_slots_by_guid = HashMap::new();
        let mut bag_templates = Vec::new();
        for (&slot, item) in &inventory_items {
            if crate::session_rules::is_buyback_slot(slot) {
                continue;
            }
            if vacated_positions.contains(&(INVENTORY_SLOT_BAG_0, slot)) {
                continue;
            }
            if is_represented_bag_slot(slot) && item_objects.contains_key(&item.guid) {
                represented_bag_slots_by_guid.insert(item.guid, slot);
                if let Some(template) = template_cache.get(&item.entry_id)
                    && template.container_slots > 0
                {
                    bag_templates.push(BagTemplateRef::new(slot, template));
                }
            }
        }
        for (index, (bag, slot, item)) in overlay_items.iter().enumerate() {
            if *bag != INVENTORY_SLOT_BAG_0 || !is_represented_bag_slot(*slot) {
                continue;
            }
            let Some(template) = template_cache.get(&item.object().entry()) else {
                continue;
            };
            if template.container_slots == 0 {
                continue;
            }
            if vacated_positions.contains(&(*bag, *slot))
                || self.get_inventory_item_by_pos(*bag, *slot).is_none()
            {
                let placeholder_counter = i64::MAX.saturating_sub(index as i64);
                let placeholder_guid =
                    ObjectGuid::create_item(self.realm_id(), placeholder_counter);
                let _ = player.store_top_level_item(*slot, placeholder_guid);
                let _ =
                    player.register_bag_storage(*slot, placeholder_guid, template.container_slots);
            }
            if !bag_templates.iter().any(|bag| bag.bag == *slot) {
                bag_templates.push(BagTemplateRef::new(*slot, template));
            }
        }

        let mut slot_items = Vec::new();
        let mut stored_items = Vec::new();
        for (&slot, inventory_item) in &inventory_items {
            if crate::session_rules::is_buyback_slot(slot) {
                continue;
            }
            if overlays
                .iter()
                .any(|overlay| overlay.bag == INVENTORY_SLOT_BAG_0 && overlay.slot == slot)
                || vacated_positions.contains(&(INVENTORY_SLOT_BAG_0, slot))
            {
                continue;
            }
            let Some(item) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            slot_items.push(ItemSlotRef::new(INVENTORY_SLOT_BAG_0, slot, item));
            stored_items.push(ItemStorageRef::new(
                INVENTORY_SLOT_BAG_0,
                slot,
                item,
                template_cache.get(&inventory_item.entry_id),
            ));
        }
        for item in item_objects.values() {
            let container_guid = item.container_guid();
            if container_guid.is_empty() {
                continue;
            }
            let Some(&bag_slot) = represented_bag_slots_by_guid.get(&container_guid) else {
                continue;
            };
            if overlays
                .iter()
                .any(|overlay| overlay.bag == bag_slot && overlay.slot == item.slot())
                || vacated_positions.contains(&(bag_slot, item.slot()))
            {
                continue;
            }
            let entry_id = item.object().entry();
            slot_items.push(ItemSlotRef::new(bag_slot, item.slot(), item));
            stored_items.push(ItemStorageRef::new(
                bag_slot,
                item.slot(),
                item,
                template_cache.get(&entry_id),
            ));
        }
        for (bag, slot, item) in &overlay_items {
            let entry_id = item.object().entry();
            slot_items.push(ItemSlotRef::new(*bag, *slot, item));
            stored_items.push(ItemStorageRef::new(
                *bag,
                *slot,
                item,
                template_cache.get(&entry_id),
            ));
        }

        let limit_category = proto.as_ref().and_then(|proto| {
            self.item_limit_category_template_like_cpp(proto.item_limit_category)
        });
        let mut dest = Vec::new();
        let outcome = player.can_store_item(
            &mut dest,
            CanStoreItemArgs {
                bag,
                slot,
                entry: entry_id,
                count,
                proto: proto.as_ref(),
                source_item,
                source_is_not_empty_bag: source_item
                    .is_some_and(|item| self.direct_item_contains_items(item.object().guid())),
                source_bop_trade_allowed_for_player: false,
                swap,
                limit_category: limit_category.as_ref(),
                slot_items: &slot_items,
                stored_items: &stored_items,
                bag_templates: &bag_templates,
            },
        );

        Some((outcome.result, dest, outcome.no_space_count))
    }
    pub(crate) fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> DurableItemLootPersistenceGuardLikeCpp {
        self.durable_item_loot_persistence_like_cpp.begin_like_cpp()
    }
    pub(crate) async fn wait_for_durable_item_loot_persistence_like_cpp(&self) {
        self.durable_item_loot_persistence_like_cpp
            .wait_until_idle_like_cpp()
            .await;
    }
    pub(crate) fn take_durable_item_loot_completions_like_cpp(
        &self,
    ) -> Vec<DurableItemLootCompletionLikeCpp> {
        self.durable_item_loot_persistence_like_cpp
            .take_completions_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_loot_item_store_test_commit_gate_like_cpp(
        &mut self,
        gate: Arc<tokio::sync::Notify>,
    ) {
        self.loot_item_store_test_commit_gate_like_cpp = Some(gate);
    }
    pub fn send_new_item_plan(&self, plan: &SendNewItemPlan) {
        let packet = crate::session_rules::item_push_result_from_send_new_item_plan(plan);
        if plan.delivery == SendNewItemDelivery::GroupBroadcast {
            use wow_packet::ServerPacket;

            if self.broadcast_item_push_result_to_group(packet.to_bytes()) {
                return;
            }
        }

        // C++ `Player::SendNewItem` uses `SendDirectMessage`; opcode routing
        // places `SMSG_ITEM_PUSH_RESULT` on CONNECTION_TYPE_REALM even while
        // the inventory object updates remain on the instance connection.
        self.send_packet_realm(&packet);
    }
    pub fn send_item_time_update_plan(&self, update: &PlayerItemTimeUpdate) {
        self.send_packet(&ItemTimeUpdate {
            item_guid: update.item_guid,
            duration_left: update.expiration,
        });
    }
    pub fn send_item_time_update_plans(&self, updates: &[PlayerItemTimeUpdate]) {
        for update in updates {
            self.send_item_time_update_plan(update);
        }
    }
}
