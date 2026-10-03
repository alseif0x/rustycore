// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_world_core::session::HubRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedCostItemTurninChange {
    Update {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
        new_count: u32,
    },
    Delete {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
    },
}

impl crate::InventoryState {
    pub fn inventory_container_db_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag: u8,
    ) -> Option<u64> {
        if bag == INVENTORY_SLOT_BAG_0 {
            Some(0)
        } else {
            self.resolved_inventory_item_like_cpp(hub, bag)
                .map(|item| item.db_guid)
        }
    }

    pub fn has_item_count_direct_inventory(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        count: u32,
    ) -> bool {
        if count == 0 {
            return true;
        }

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp(hub) else {
            return false;
        };
        let mut current_count = 0_u32;
        let mut slots: Vec<_> = inventory_items.iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (_, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }
            current_count = current_count.saturating_add(item.count());
            if current_count >= count {
                return true;
            }
        }

        false
    }

    pub fn plan_destroy_item_count_direct_inventory(
        &self,
        hub: HubRef<'_>,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        if count == 0 {
            return Some(Vec::new());
        }

        let inventory_items = self.resolved_inventory_items_like_cpp(hub)?;
        let mut remaining = count;
        let mut changes = Vec::new();
        let mut slots: Vec<_> = inventory_items.iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (&slot, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }

            let item_count = item.count();
            if item_count <= remaining {
                remaining -= item_count;
                changes.push(ExtendedCostItemTurninChange::Delete {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                });
            } else {
                changes.push(ExtendedCostItemTurninChange::Update {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                    new_count: item_count - remaining,
                });
                remaining = 0;
            }

            if remaining == 0 {
                return Some(changes);
            }
        }

        None
    }
}
