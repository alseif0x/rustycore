//! loss for the existing durability owner.

use super::*;

impl WorldSession {
    /// C++ `Player::DurabilityLossAll` (`Player.cpp:4522-4544`).
    ///
    /// Equipment is always processed; `inventory` additionally walks the
    /// backpack slots and every bag's contents, matching C++.
    pub(in crate::session) fn apply_represented_durability_loss_all_like_cpp(
        &mut self,
        percent: f64,
        inventory: bool,
    ) -> usize {
        let targets = self.represented_durability_targets_like_cpp(inventory);
        let mut affected = 0;
        for (guid, slot, equipped) in targets {
            if self.apply_represented_durability_loss_item_like_cpp(guid, slot, equipped, percent) {
                affected += 1;
            }
        }
        affected
    }
    /// C++ `Player::DurabilityPointsLossAll` (`Player.cpp:4566-4588`).
    pub(in crate::session) fn apply_represented_durability_points_loss_all_like_cpp(
        &mut self,
        points: i32,
        inventory: bool,
    ) -> usize {
        let targets = self.represented_durability_targets_like_cpp(inventory);
        let mut affected = 0;
        for (guid, slot, equipped) in targets {
            if self.apply_represented_durability_points_loss_like_cpp(
                guid,
                slot,
                equipped,
                i64::from(points),
            ) {
                affected += 1;
            }
        }
        affected
    }
    /// C++ `Spell::EffectDurabilityDamage` slot branch: one top-level
    /// `INVENTORY_SLOT_BAG_0` slot, resolved through `GetItemByPos`.
    pub(in crate::session) fn apply_represented_durability_points_loss_at_slot_like_cpp(
        &mut self,
        slot: u8,
        points: i32,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_like_cpp(slot) else {
            return false;
        };
        let equipped = is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot));
        self.apply_represented_durability_points_loss_like_cpp(
            item.guid,
            slot,
            equipped,
            i64::from(points),
        )
    }
    /// C++ `Spell::EffectDurabilityDamagePCT` slot branch.
    pub(in crate::session) fn apply_represented_durability_loss_at_slot_like_cpp(
        &mut self,
        slot: u8,
        percent: f64,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_like_cpp(slot) else {
            return false;
        };
        let equipped = is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot));
        self.apply_represented_durability_loss_item_like_cpp(item.guid, slot, equipped, percent)
    }
    /// The `DurabilityLossAll`/`DurabilityPointsLossAll` target set.
    fn represented_durability_targets_like_cpp(
        &self,
        inventory: bool,
    ) -> Vec<(ObjectGuid, u8, bool)> {
        let Some(items) = self.resolved_inventory_item_objects_like_cpp() else {
            return Vec::new();
        };
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(
                self.resolved_player_inventory_slot_count_like_cpp()
                    .unwrap_or(0),
            )
            .min(INVENTORY_SLOT_ITEM_END);
        items
            .iter()
            .filter_map(|(guid, item)| {
                let slot = item.slot();
                if item.container_guid().is_empty() {
                    if is_equipment_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)) {
                        return Some((*guid, slot, true));
                    }
                    if inventory && (INVENTORY_SLOT_ITEM_START..inventory_end).contains(&slot) {
                        return Some((*guid, slot, false));
                    }
                    return None;
                }
                inventory.then_some((*guid, slot, false))
            })
            .collect()
    }
    /// C++ `Player::DurabilityLoss` (`Player.cpp:4546-4562`).
    fn apply_represented_durability_loss_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        equipped: bool,
        percent: f64,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        let max_durability = item.data().max_durability;
        if max_durability == 0 {
            return false;
        }
        let percent =
            percent / f64::from(self.represented_durability_loss_aura_multiplier_like_cpp());
        let points = ((f64::from(max_durability) * percent) as u32).max(1);
        self.apply_represented_durability_points_loss_like_cpp(
            item_guid,
            slot,
            equipped,
            i64::from(points),
        )
    }
    /// C++ `Player::DurabilityPointsLoss` (`Player.cpp:4590-4620`).
    ///
    /// Mods are removed *before* the durability write so the represented
    /// `_ApplyItemMods` gate still sees the item as unbroken, then restored
    /// after, exactly like C++.
    fn apply_represented_durability_points_loss_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        equipped: bool,
        points: i64,
    ) -> bool {
        if self.represented_prevent_durability_loss_like_cpp() {
            return false;
        }
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return false;
        };
        let max_durability = i64::from(item.data().max_durability);
        let old_durability = i64::from(item.data().durability);
        let new_durability = (old_durability - points).clamp(0, max_durability);
        if old_durability == new_durability {
            return false;
        }
        if new_durability == 0 && old_durability > 0 && equipped {
            self.record_represented_item_mods_like_cpp(item_guid, slot, false);
        }
        let updated = self.apply_inventory_item_object_updates_like_cpp(
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetDurability(
                u32::try_from(new_durability).unwrap_or(0),
            )],
        );
        if new_durability > 0 && old_durability == 0 && equipped {
            self.record_represented_item_mods_like_cpp(item_guid, slot, true);
        }
        updated
    }
}
