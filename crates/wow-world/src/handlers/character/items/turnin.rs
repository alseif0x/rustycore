//! turnin for the existing items owner.

use super::*;

impl WorldSession {

    pub(in crate::handlers::character) fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
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
            let Some(item) = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
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

    pub(in crate::handlers) fn plan_destroy_item_count_direct_inventory(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        if count == 0 {
            return Some(Vec::new());
        }

        let inventory_items = self.resolved_inventory_items_like_cpp()?;
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
            let Some(item) = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
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

    pub(in crate::handlers) fn apply_item_turnin_changes(
        &mut self,
        _player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        let mut cleared_slots = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut send_stat_update = false;

        for change in changes {
            match *change {
                ExtendedCostItemTurninChange::Update {
                    item_guid,
                    new_count,
                    ..
                } => {
                    let _ = self.apply_inventory_item_object_updates_like_cpp(
                        item_guid,
                        &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
                    );
                    self.send_packet(&UpdateObject::item_stack_count_update(
                        item_guid, map_id, new_count,
                    ));
                }
                ExtendedCostItemTurninChange::Delete {
                    slot, item_guid, ..
                } => {
                    self.remove_inventory_item_like_cpp(slot);
                    self.remove_inventory_item_object(item_guid);
                    cleared_slots.push((slot, ObjectGuid::EMPTY));
                    if (slot as usize) < 19 {
                        visible_item_changes.push((slot, 0i32, 0u16, 0u16));
                        send_stat_update = true;
                    }
                    if (15..=17).contains(&slot) {
                        virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
                    }
                }
            }
        }

        if !cleared_slots.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &cleared_slots,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if send_stat_update {
            self.send_stat_update();
        }
    }
}
