// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashSet;

use wow_core::ObjectGuid;
use wow_entities::{
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, PLAYER_SLOT_END,
    PlayerInventoryItem as InventoryItem,
};
use wow_world_core::session::{HubMut, HubRef};

pub fn is_represented_bag_slot(slot: u8) -> bool {
    wow_entities::is_bag_pos(wow_entities::make_item_pos(
        wow_entities::INVENTORY_SLOT_BAG_0,
        slot,
    ))
}

impl crate::InventoryState {
    pub fn represented_direct_inventory_slot_by_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<(u8, InventoryItem)> {
        let (bag, slot, item) = self.get_inventory_item_by_guid_like_cpp(hub, guid)?;
        (bag == INVENTORY_SLOT_BAG_0).then_some((slot, item))
    }

    pub fn set_inventory_item_object_slot(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        slot: u8,
    ) {
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            hub,
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetSlot(slot)],
        );
    }

    pub fn represented_empty_inventory_positions_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Vec<(u8, u8)>> {
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.resolved_player_inventory_slot_count_like_cpp(hub)?)
            .min(INVENTORY_SLOT_ITEM_END);
        let mut positions = (INVENTORY_SLOT_ITEM_START..inventory_end)
            .filter(|slot| {
                self.get_inventory_item_by_pos(hub, INVENTORY_SLOT_BAG_0, *slot)
                    .is_none()
            })
            .map(|slot| (INVENTORY_SLOT_BAG_0, slot))
            .collect::<Vec<_>>();

        for bag in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
            let Some(bag_item) = self.resolved_inventory_item_like_cpp(hub, bag) else {
                continue;
            };
            let Some(template) = hub.catalogs.item_storage_template(bag_item.entry_id) else {
                continue;
            };
            positions.extend(
                (0..template.container_slots)
                    .filter(|slot| self.get_inventory_item_by_pos(hub, bag, *slot).is_none())
                    .map(|slot| (bag, slot)),
            );
        }
        Some(positions)
    }

    /// Resolve an inventory item by (bag, slot) following C++ Player::GetItemByPos.
    ///
    /// - `bag == INVENTORY_SLOT_BAG_0`       → top-level direct inventory (buyback excluded).
    /// - `bag` in carried/bank/reagent range → search nested runtime items inside the bag.
    pub fn get_inventory_item_by_pos(
        &self,
        hub: HubRef<'_>,
        bag: u8,
        slot: u8,
    ) -> Option<InventoryItem> {
        if bag == INVENTORY_SLOT_BAG_0 {
            if (slot as usize) >= PLAYER_SLOT_END || wow_entities::is_buyback_slot(slot) {
                return None;
            }
            self.resolved_inventory_item_like_cpp(hub, slot)
        } else if is_represented_bag_slot(bag) {
            let bag_item = self.resolved_inventory_item_like_cpp(hub, bag)?;
            let bag_guid = bag_item.guid;
            let item_objects = self.resolved_inventory_item_objects_like_cpp(hub)?;
            let nested = item_objects
                .values()
                .find(|item| item.container_guid() == bag_guid && item.slot() == slot)?;
            let guid = nested.object().guid();
            let entry_id = nested.object().entry();
            Some(InventoryItem {
                guid,
                entry_id,
                db_guid: guid.counter() as u64,
                inventory_type: hub.item_template_inventory_type(entry_id),
            })
        } else {
            None
        }
    }

    /// C++ `Player::IsValidPos` against the session-owned inventory snapshot.
    pub fn is_valid_inventory_pos_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag: u8,
        slot: u8,
        explicit_pos: bool,
    ) -> bool {
        self.direct_inventory_player_snapshot(hub)
            .is_some_and(|player| player.is_valid_pos(bag, slot, explicit_pos))
    }

    /// Return every runtime item contained by `container_guid`, deepest first.
    /// C++ `Player::DestroyItem` recursively destroys bag contents before the
    /// container itself. The normal client disallows nested bags, but keeping
    /// this traversal recursive also makes corrupted/runtime-only graphs safe.
    pub fn represented_inventory_descendants_postorder_like_cpp(
        &self,
        hub: HubRef<'_>,
        container_guid: ObjectGuid,
    ) -> Option<Vec<(u8, u8, InventoryItem)>> {
        let mut candidates = self
            .resolved_inventory_item_objects_like_cpp(hub)?
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
                            inventory_type: hub.item_template_inventory_type(entry_id),
                        },
                    )
                })
                .collect(),
        )
    }

    pub fn resolved_player_inventory_slot_count_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u8> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_player_inventory_slot_count_with_access_like_cpp(&access)
    }

    pub fn resolved_player_inventory_slot_count_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<u8> {
        let canonical = access.player_inventory_slot_count_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .player_inventory_slot_count_like_cpp,
            );
        }
        canonical
    }
}

impl crate::InventoryState {
    pub fn set_player_inventory_slot_count_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        count: u8,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(count))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            self.player_item_test_fixture_like_cpp
                .player_inventory_slot_count_like_cpp = count;
        }
        canonical || cfg!(any(test, feature = "test-fixtures")) && hub.core.player_handle_like_cpp.is_none()
    }
}
