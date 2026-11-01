// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::{RepresentedAutoUnequipOffhandLikeCpp, RepresentedAutoUnequipOffhandReasonLikeCpp};
use wow_constants::{InventoryType, ItemClass, ItemFlags3, ItemSubClassWeapon};
use wow_core::ObjectGuid;
use wow_entities::{EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, INVENTORY_SLOT_BAG_0};
use wow_world_core::session::{HubMut, HubRef};

impl crate::InventoryState {
    pub fn represented_auto_unequip_offhand_reason_like_cpp(
        &self,
        hub: HubRef<'_>,
        force: bool,
    ) -> Option<RepresentedAutoUnequipOffhandReasonLikeCpp> {
        let offhand_item = self.resolved_inventory_item_like_cpp(hub, EQUIPMENT_SLOT_OFFHAND)?;
        let offhand_template = hub.catalogs.item_storage_template(offhand_item.entry_id)?;
        let mainhand_template = self
            .resolved_inventory_item_like_cpp(hub, EQUIPMENT_SLOT_MAINHAND)
            .and_then(|item| hub.catalogs.item_storage_template(item.entry_id));
        let (can_dual_wield, can_titan_grip) = self.inventory_equip_capabilities_like_cpp(hub)?;

        let always_allow_dual_wield = hub
            .catalogs
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

    pub fn clear_represented_offhand_equipped_flag_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            hub,
            item_guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetEquipped(false)],
        );
    }

    pub fn send_auto_unequip_offhand_values_update_like_cpp(
        &self,
        hub: HubRef<'_>,
        stored_destination: Option<(u8, u8)>,
        item_guid: ObjectGuid,
    ) {
        let mut inv_slot_changes = vec![(EQUIPMENT_SLOT_OFFHAND, ObjectGuid::EMPTY)];
        if let Some((INVENTORY_SLOT_BAG_0, slot)) = stored_destination {
            inv_slot_changes.push((slot, item_guid));
        }

        self.send_player_values_update_from_entity_bridge(
            hub,
            &inv_slot_changes,
            &[(EQUIPMENT_SLOT_OFFHAND, 0, 0, 0)],
            &[],
            &[],
            None,
        );
    }
}

impl crate::InventoryState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_auto_unequip_offhand_requests_like_cpp(
        &self,
    ) -> &[RepresentedAutoUnequipOffhandLikeCpp] {
        &self.represented_auto_unequip_offhand_requests_like_cpp
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_auto_unequip_offhand_request_like_cpp(
        &mut self,
        request: RepresentedAutoUnequipOffhandLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_auto_unequip_offhand_requests_like_cpp
            .push(request);
    }
}
