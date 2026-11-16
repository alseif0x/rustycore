// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Actor-free Unit update values captured from canonical Unit state.

use super::{Unit, UnitDataUpdate, UnitDataValues, UnitValuesUpdate};
use crate::{ObjectDataUpdate, UpdateMask, VisibleItemValues};
use crate::{
    unit_subsystems::AuraStateValuesSnapshot,
    update_fields::{TYPEID_OBJECT, TYPEID_UNIT},
};
use wow_core::ObjectGuid;

pub(crate) struct UnitValuesSnapshot {
    owner_guid: ObjectGuid,
    object_data: Option<ObjectDataUpdate>,
    unit_data: UnitDataValues,
    unit_data_changes: UpdateMask,
    aura_state: AuraStateValuesSnapshot,
}

impl UnitValuesSnapshot {
    pub(crate) fn capture(unit: &Unit) -> Self {
        let object_data = unit.world.object().values_update().object_data;
        Self {
            owner_guid: unit.world.object().guid(),
            object_data,
            unit_data: unit.data,
            unit_data_changes: unit.unit_data_changes.clone(),
            aura_state: AuraStateValuesSnapshot::capture(&unit.subsystems.auras),
        }
    }

    pub(crate) fn values_update(&self) -> UnitValuesUpdate {
        let has_unit_data = self.unit_data_changes.is_any_set();
        let changed_object_type_mask = (if self.object_data.is_some() {
            1 << TYPEID_OBJECT
        } else {
            0
        }) | (if has_unit_data { 1 << TYPEID_UNIT } else { 0 });

        UnitValuesUpdate {
            changed_object_type_mask,
            object_data: self.object_data.clone(),
            unit_data: has_unit_data.then(|| UnitDataUpdate {
                mask: self.unit_data_changes.clone(),
                values: self.unit_data,
                aura_state: self
                    .aura_state
                    .build_aura_state_update_for_target_like_cpp(self.owner_guid),
                aura_state_per_caster: self
                    .aura_state
                    .aura_state_per_caster_contributions_like_cpp(),
            }),
        }
    }

    pub(crate) fn unit_data_changes_mask(&self) -> &UpdateMask {
        &self.unit_data_changes
    }

    pub(crate) fn clear_unit_data_changes(&mut self) {
        self.unit_data_changes.reset_all();
    }

    pub(crate) fn clear_object_data_changes(&mut self) {
        self.object_data = None;
    }

    pub(crate) fn set_virtual_item(&mut self, index: usize, visible: Option<VisibleItemValues>) {
        let _ = super::virtual_items::set_virtual_item_values_like_cpp(
            &mut self.unit_data,
            &mut self.unit_data_changes,
            index,
            visible,
        );
    }

    pub(crate) fn mark_virtual_item_changed(&mut self, index: usize) {
        let _ = super::virtual_items::mark_virtual_item_changed_like_cpp(
            &mut self.unit_data_changes,
            index,
        );
    }
}
