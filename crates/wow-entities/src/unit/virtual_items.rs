// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure Unit virtual-item value and update-mask rules.

use super::{
    MAX_ATTACK, UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT, UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT, Unit,
    UnitDataValues, VisibleItemValues,
};
use crate::UpdateMask;

pub(super) fn set_virtual_item_values_like_cpp(
    data: &mut UnitDataValues,
    unit_data_changes: &mut UpdateMask,
    index: usize,
    visible: Option<VisibleItemValues>,
) -> bool {
    if index >= MAX_ATTACK {
        return false;
    }

    let value = visible.unwrap_or_default();
    if data.virtual_items[index] == value {
        return false;
    }

    data.virtual_items[index] = value;
    mark_virtual_item_mask(unit_data_changes, index);
    true
}

pub(super) fn mark_virtual_item_changed_like_cpp(
    unit_data_changes: &mut UpdateMask,
    index: usize,
) -> bool {
    if index >= MAX_ATTACK {
        return false;
    }

    mark_virtual_item_mask(unit_data_changes, index);
    true
}

fn mark_virtual_item_mask(unit_data_changes: &mut UpdateMask, index: usize) {
    unit_data_changes.set(UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT);
    unit_data_changes.set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + index);
}

impl Unit {
    pub(super) fn apply_virtual_item_update_request(&mut self, update_requested: bool) {
        if update_requested {
            self.world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }
}
