//! publication for the existing inventory_moves owner.

use super::*;

impl WorldSession {

    pub(crate) fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        let mut unique_positions = positions.to_vec();
        unique_positions.sort_unstable();
        unique_positions.dedup();
        let mut top_level_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut gear_changed = false;

        for (bag, slot) in unique_positions {
            if bag != INVENTORY_SLOT_BAG_0 {
                self.send_bag_slot_values_update_like_cpp(bag, slot);
                continue;
            }
            let item = self.get_inventory_item_by_pos(bag, slot);
            top_level_changes.push((
                slot,
                item.as_ref().map_or(ObjectGuid::EMPTY, |item| item.guid),
            ));
            if slot < 19 {
                gear_changed = true;
                visible_item_changes.push((
                    slot,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((
                    slot - 15,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
        }
        if !top_level_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &top_level_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if gear_changed {
            self.send_stat_update();
        }
    }
}
