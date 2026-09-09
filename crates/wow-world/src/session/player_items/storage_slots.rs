//! Slot resolution for represented inventory storage: free space, position search and fit checks.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_direct_inventory_slot_by_guid_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<(u8, InventoryItem)> {
        let (bag, slot, item) = self.get_inventory_item_by_guid_like_cpp(guid)?;
        (bag == INVENTORY_SLOT_BAG_0).then_some((slot, item))
    }
    pub(crate) fn move_represented_direct_inventory_item_to_pos_like_cpp(
        &mut self,
        src: u8,
        dst_bag: u8,
        dst_slot: u8,
    ) -> bool {
        if dst_bag == INVENTORY_SLOT_BAG_0 {
            return self.move_represented_direct_inventory_item_like_cpp(src, dst_slot);
        }
        if !is_represented_bag_slot(dst_bag)
            || self.get_inventory_item_by_pos(dst_bag, dst_slot).is_some()
        {
            return false;
        }

        let Some(src_item) = self.resolved_inventory_item_like_cpp(src) else {
            return false;
        };
        let Some(bag_item) = self.resolved_inventory_item_like_cpp(dst_bag) else {
            return false;
        };
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return false;
        };
        if !item_objects.contains_key(&src_item.guid) || !item_objects.contains_key(&bag_item.guid)
        {
            return false;
        }

        self.remove_inventory_item_like_cpp(src);
        self.update_inventory_item_object_like_cpp(src_item.guid, |item| {
            item.set_contained_in(bag_item.guid);
            item.set_slot(dst_slot);
            item.set_container_guid_and_slot(bag_item.guid, dst_bag);
        })
    }
    pub(crate) fn set_inventory_item_object_slot(&mut self, item_guid: ObjectGuid, slot: u8) {
        self.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_slot(slot);
        });
    }
    pub(crate) fn represented_empty_inventory_positions_like_cpp(&self) -> Option<Vec<(u8, u8)>> {
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.resolved_player_inventory_slot_count_like_cpp()?)
            .min(INVENTORY_SLOT_ITEM_END);
        let mut positions = (INVENTORY_SLOT_ITEM_START..inventory_end)
            .filter(|slot| {
                self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, *slot)
                    .is_none()
            })
            .map(|slot| (INVENTORY_SLOT_BAG_0, slot))
            .collect::<Vec<_>>();

        for bag in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
            let Some(bag_item) = self.resolved_inventory_item_like_cpp(bag) else {
                continue;
            };
            let Some(template) = self.item_storage_template(bag_item.entry_id) else {
                continue;
            };
            positions.extend(
                (0..template.container_slots)
                    .filter(|slot| self.get_inventory_item_by_pos(bag, *slot).is_none())
                    .map(|slot| (bag, slot)),
            );
        }
        Some(positions)
    }
    /// Resolve an inventory item by (bag, slot) following C++ Player::GetItemByPos.
    ///
    /// - `bag == INVENTORY_SLOT_BAG_0`       → top-level direct inventory (buyback excluded).
    /// - `bag` in carried/bank/reagent range → search nested runtime items inside the bag.
    pub(crate) fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        if bag == INVENTORY_SLOT_BAG_0 {
            if (slot as usize) >= PLAYER_SLOT_END || Self::is_buyback_slot(slot) {
                return None;
            }
            self.resolved_inventory_item_like_cpp(slot)
        } else if is_represented_bag_slot(bag) {
            let bag_item = self.resolved_inventory_item_like_cpp(bag)?;
            let bag_guid = bag_item.guid;
            let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
            let nested = item_objects
                .values()
                .find(|item| item.container_guid() == bag_guid && item.slot() == slot)?;
            let guid = nested.object().guid();
            let entry_id = nested.object().entry();
            Some(InventoryItem {
                guid,
                entry_id,
                db_guid: guid.counter() as u64,
                inventory_type: self.item_template_inventory_type(entry_id),
            })
        } else {
            None
        }
    }
    /// C++ `Player::IsValidPos` against the session-owned inventory snapshot.
    pub(crate) fn is_valid_inventory_pos_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        explicit_pos: bool,
    ) -> bool {
        self.direct_inventory_player_snapshot()
            .is_some_and(|player| player.is_valid_pos(bag, slot, explicit_pos))
    }
    /// Return every runtime item contained by `container_guid`, deepest first.
    /// C++ `Player::DestroyItem` recursively destroys bag contents before the
    /// container itself. The normal client disallows nested bags, but keeping
    /// this traversal recursive also makes corrupted/runtime-only graphs safe.
    pub(crate) fn represented_inventory_descendants_postorder_like_cpp(
        &self,
        container_guid: ObjectGuid,
    ) -> Option<Vec<(u8, u8, InventoryItem)>> {
        let mut candidates = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .map(|item| {
                (
                    item.container_guid(),
                    item.bag_slot(),
                    item.slot(),
                    item.object().guid(),
                    item.object().entry(),
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(_, bag, slot, guid, _)| (*bag, *slot, guid.counter()));

        fn visit(
            container_guid: ObjectGuid,
            candidates: &[(ObjectGuid, u8, u8, ObjectGuid, u32)],
            visited: &mut HashSet<ObjectGuid>,
            descendants: &mut Vec<(u8, u8, ObjectGuid, u32)>,
        ) {
            for &(parent, bag, slot, guid, entry_id) in candidates {
                if parent != container_guid || !visited.insert(guid) {
                    continue;
                }
                visit(guid, candidates, visited, descendants);
                descendants.push((bag, slot, guid, entry_id));
            }
        }

        let mut descendants = Vec::new();
        visit(
            container_guid,
            &candidates,
            &mut HashSet::new(),
            &mut descendants,
        );
        Some(
            descendants
                .into_iter()
                .map(|(bag, slot, guid, entry_id)| {
                    (
                        bag,
                        slot,
                        InventoryItem {
                            guid,
                            entry_id,
                            db_guid: guid.counter() as u64,
                            inventory_type: self.item_template_inventory_type(entry_id),
                        },
                    )
                })
                .collect(),
        )
    }
    pub(crate) fn set_player_inventory_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(count))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_inventory_slot_count_like_cpp = count;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn resolved_player_inventory_slot_count_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(Player::inventory_slot_count);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_inventory_slot_count_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_inventory_slot_count_like_cpp(&self) -> u8 {
        self.resolved_player_inventory_slot_count_like_cpp()
            .expect("test Player inventory-slot owner must resolve")
    }
}
