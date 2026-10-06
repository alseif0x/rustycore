//! Represented automatic offhand unequip, its recorded requests and published state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auto_unequip_offhand_request_like_cpp(
        &mut self,
        request: RepresentedAutoUnequipOffhandLikeCpp,
    ) {
        self.inventory
            .record_represented_auto_unequip_offhand_request_like_cpp(request)
    }
    pub(in crate::session) fn represented_auto_unequip_offhand_if_need_like_cpp(
        &mut self,
        force: bool,
    ) -> bool {
        let Some(offhand_item) = self.resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)
        else {
            return false;
        };

        let Some(reason) = ({
            let (s, h) = crate::session::split_inventory_ref(self);
            s.represented_auto_unequip_offhand_reason_like_cpp(h, force)
        }) else {
            return false;
        };
        #[cfg(not(test))]
        let _ = reason;

        self.remove_inventory_item_duration_refs_like_cpp(offhand_item.guid);
        {
            let (s, mut h) = crate::session::split_inventory_mut(self);
            s.clear_represented_offhand_equipped_flag_like_cpp(&mut h, offhand_item.guid)
        };
        self.remove_inventory_tradeable_item_like_cpp(offhand_item.guid);
        let _item_set_changed = self.record_direct_inventory_item_set_remove_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_OFFHAND,
            offhand_item.guid,
        );
        let item_mods_changed =
            self.record_represented_offhand_item_mod_remove_like_cpp(offhand_item.guid);
        self.inventory
            .record_inventory_item_combat_stat_recalculations_like_cpp(EQUIPMENT_SLOT_OFFHAND);

        #[cfg(test)]
        let mut stored_destination = None;
        let mut needs_mail_fallback = true;
        if let Some((InventoryResult::Ok, destinations, _)) =
            self.plan_store_existing_direct_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)
            && let Some(destination) = destinations.first()
        {
            let [bag, slot] = destination.pos.to_be_bytes();
            if self.move_represented_direct_inventory_item_to_pos_like_cpp(
                EQUIPMENT_SLOT_OFFHAND,
                bag,
                slot,
            ) {
                {
                    let (s, mut h) = crate::session::split_inventory_mut(self);
                    s.sync_canonical_direct_inventory_move_like_cpp(
                        &mut h,
                        EQUIPMENT_SLOT_OFFHAND,
                        bag,
                        slot,
                        offhand_item.guid,
                    )
                };
                {
                    let (s, h) = crate::session::split_inventory_ref(self);
                    s.send_auto_unequip_offhand_values_update_like_cpp(
                        h,
                        Some((bag, slot)),
                        offhand_item.guid,
                    )
                };
                {
                    let (s, h) = crate::session::split_inventory_ref(self);
                    s.send_item_contained_in_values_update_like_cpp(h, offhand_item.guid)
                };
                if bag != INVENTORY_SLOT_BAG_0 {
                    self.send_bag_slot_values_update_like_cpp(bag, slot);
                }
                if item_mods_changed {
                    self.send_represented_item_bonus_player_stat_update_like_cpp();
                }
                #[cfg(test)]
                {
                    stored_destination = Some((bag, slot));
                }
                needs_mail_fallback = false;
            }
        }
        if needs_mail_fallback {
            self.remove_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND);
            {
                let (s, mut h) = crate::session::split_inventory_mut(self);
                s.sync_canonical_direct_inventory_remove_like_cpp(&mut h, EQUIPMENT_SLOT_OFFHAND)
            };
            {
                let (s, h) = crate::session::split_inventory_ref(self);
                s.send_auto_unequip_offhand_values_update_like_cpp(h, None, offhand_item.guid)
            };
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                offhand_item.guid,
                &[
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainedIn(ObjectGuid::EMPTY),
                    wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuid(ObjectGuid::EMPTY),
                    wow_entities::ItemObjectUpdateLikeCpp::SetSlot(NULL_SLOT),
                ],
            );
            {
                let (s, h) = crate::session::split_inventory_ref(self);
                s.send_item_contained_in_values_update_like_cpp(h, offhand_item.guid)
            };
            if item_mods_changed {
                self.send_represented_item_bonus_player_stat_update_like_cpp();
            }
        }

        self.record_represented_titan_grip_penalty_action_like_cpp();
        self.record_represented_avg_equipped_item_level_update_like_cpp();

        #[cfg(test)]
        self.record_represented_auto_unequip_offhand_request_like_cpp(
            RepresentedAutoUnequipOffhandLikeCpp {
                item_guid: offhand_item.guid,
                item_entry: offhand_item.entry_id,
                reason,
                stored_destination,
                needs_mail_fallback,
            },
        );
        true
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/offhand/f3_shims.rs"]
mod f3_shims;
