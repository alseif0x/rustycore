//! Represented automatic offhand unequip, its recorded requests and published state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_auto_unequip_offhand_requests_like_cpp(
        &self,
    ) -> &[RepresentedAutoUnequipOffhandLikeCpp] {
        &self.represented_auto_unequip_offhand_requests_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auto_unequip_offhand_request_like_cpp(
        &mut self,
        request: RepresentedAutoUnequipOffhandLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auto_unequip_offhand_requests_like_cpp
            .push(request);
    }
    pub(crate) fn represented_auto_unequip_offhand_reason_like_cpp(
        &self,
        force: bool,
    ) -> Option<RepresentedAutoUnequipOffhandReasonLikeCpp> {
        let offhand_item = self.resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)?;
        let offhand_template = self.item_storage_template(offhand_item.entry_id)?;
        let mainhand_template = self
            .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_MAINHAND)
            .and_then(|item| self.item_storage_template(item.entry_id));
        let (can_dual_wield, can_titan_grip) = self.inventory_equip_capabilities_like_cpp()?;

        let always_allow_dual_wield = self
            .item_template_flags3(offhand_item.entry_id)
            .is_some_and(|flags| (flags & ItemFlags3::AlwaysAllowDualWield as u32) != 0);
        let lost_dual_wield = !can_dual_wield
            && ((offhand_template.inventory_type == InventoryType::WeaponOffhand
                && !always_allow_dual_wield)
                || offhand_template.inventory_type == InventoryType::Weapon);
        let is_two_hand_used = mainhand_template.is_some_and(|template| {
            (template.inventory_type == InventoryType::Weapon2Hand && !can_titan_grip)
                || template.inventory_type == InventoryType::Ranged
                || (template.inventory_type == InventoryType::RangedRight
                    && template.class_id == ItemClass::Weapon
                    && template.subclass_id != ItemSubClassWeapon::Wand as u32)
        });

        if force {
            Some(RepresentedAutoUnequipOffhandReasonLikeCpp::Forced)
        } else if lost_dual_wield {
            Some(RepresentedAutoUnequipOffhandReasonLikeCpp::LostDualWield)
        } else if !can_titan_grip
            && (offhand_template.inventory_type == InventoryType::Weapon2Hand || is_two_hand_used)
        {
            Some(RepresentedAutoUnequipOffhandReasonLikeCpp::InvalidTwoHandState)
        } else {
            None
        }
    }
    pub(in crate::session) fn represented_auto_unequip_offhand_if_need_like_cpp(
        &mut self,
        force: bool,
    ) -> bool {
        let Some(offhand_item) = self.resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)
        else {
            return false;
        };

        let Some(reason) = self.represented_auto_unequip_offhand_reason_like_cpp(force) else {
            return false;
        };
        #[cfg(not(test))]
        let _ = reason;

        self.remove_inventory_item_duration_refs_like_cpp(offhand_item.guid);
        self.clear_represented_offhand_equipped_flag_like_cpp(offhand_item.guid);
        self.remove_inventory_tradeable_item_like_cpp(offhand_item.guid);
        let _item_set_changed = self.record_direct_inventory_item_set_remove_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_OFFHAND,
            offhand_item.guid,
        );
        let item_mods_changed =
            self.record_represented_offhand_item_mod_remove_like_cpp(offhand_item.guid);
        self.record_inventory_item_combat_stat_recalculations_like_cpp(EQUIPMENT_SLOT_OFFHAND);

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
                self.sync_canonical_direct_inventory_move_like_cpp(
                    EQUIPMENT_SLOT_OFFHAND,
                    bag,
                    slot,
                    offhand_item.guid,
                );
                self.send_auto_unequip_offhand_values_update_like_cpp(
                    Some((bag, slot)),
                    offhand_item.guid,
                );
                self.send_item_contained_in_values_update_like_cpp(offhand_item.guid);
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
            self.sync_canonical_direct_inventory_remove_like_cpp(EQUIPMENT_SLOT_OFFHAND);
            self.send_auto_unequip_offhand_values_update_like_cpp(None, offhand_item.guid);
            self.update_inventory_item_object_like_cpp(offhand_item.guid, |item| {
                item.set_contained_in(ObjectGuid::EMPTY);
                item.set_container_guid(ObjectGuid::EMPTY);
                item.set_slot(NULL_SLOT);
            });
            self.send_item_contained_in_values_update_like_cpp(offhand_item.guid);
            if item_mods_changed {
                self.send_represented_item_bonus_player_stat_update_like_cpp();
            }
        }

        self.record_represented_titan_grip_penalty_action_like_cpp();
        self.record_represented_avg_equipped_item_level_update_like_cpp();

        #[cfg(test)]
        self.represented_auto_unequip_offhand_requests_like_cpp
            .push(RepresentedAutoUnequipOffhandLikeCpp {
                item_guid: offhand_item.guid,
                item_entry: offhand_item.entry_id,
                reason,
                stored_destination,
                needs_mail_fallback,
            });
        true
    }
    fn clear_represented_offhand_equipped_flag_like_cpp(&mut self, item_guid: ObjectGuid) {
        self.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.remove_item_flag2(ItemFieldFlags2::EQUIPPED);
        });
    }
    fn send_auto_unequip_offhand_values_update_like_cpp(
        &self,
        stored_destination: Option<(u8, u8)>,
        item_guid: ObjectGuid,
    ) {
        let mut inv_slot_changes = vec![(EQUIPMENT_SLOT_OFFHAND, ObjectGuid::EMPTY)];
        if let Some((INVENTORY_SLOT_BAG_0, slot)) = stored_destination {
            inv_slot_changes.push((slot, item_guid));
        }

        self.send_player_values_update_from_entity_bridge(
            &inv_slot_changes,
            &[(EQUIPMENT_SLOT_OFFHAND, 0, 0, 0)],
            &[],
            &[],
            None,
        );
    }
}
