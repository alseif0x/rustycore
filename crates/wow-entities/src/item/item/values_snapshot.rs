use std::collections::HashSet;

use wow_constants::{ItemBondingType, ItemUpdateState};
use wow_core::ObjectGuid;

use crate::{ItemDataValues, ItemReadView, ObjectValuesSnapshot, UpdateMask};

use super::Item;

/// Complete immutable Item/Object values captured from one admitted owner read.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemValuesSnapshot {
    object: ObjectValuesSnapshot,
    data: ItemDataValues,
    item_data_changes: UpdateMask,
    slot: u8,
    container: ObjectGuid,
    container_slot: u8,
    bonding: ItemBondingType,
    update_state: ItemUpdateState,
    queue_pos: i16,
    loot_generated: bool,
    in_trade: bool,
    last_played_time_update: i64,
    refund_recipient: ObjectGuid,
    paid_money: u64,
    paid_extended_cost: u32,
    soulbound_trade_allowed_guids: HashSet<ObjectGuid>,
    text: String,
}

impl ItemValuesSnapshot {
    pub fn capture(item: &Item) -> Self {
        Self {
            object: item.object.values_snapshot(),
            data: item.data.clone(),
            item_data_changes: item.item_data_changes.clone(),
            slot: item.slot,
            container: item.container,
            container_slot: item.container_slot,
            bonding: item.bonding,
            update_state: item.update_state,
            queue_pos: item.queue_pos,
            loot_generated: item.loot_generated,
            in_trade: item.in_trade,
            last_played_time_update: item.last_played_time_update,
            refund_recipient: item.refund_recipient,
            paid_money: item.paid_money,
            paid_extended_cost: item.paid_extended_cost,
            soulbound_trade_allowed_guids: item.soulbound_trade_allowed_guids.clone(),
            text: item.text.clone(),
        }
    }

    pub const fn object(&self) -> &ObjectValuesSnapshot {
        &self.object
    }

    pub const fn data(&self) -> &ItemDataValues {
        &self.data
    }

    pub const fn item_data_changes_mask(&self) -> &UpdateMask {
        &self.item_data_changes
    }

    pub const fn slot(&self) -> u8 {
        self.slot
    }

    pub const fn container_guid(&self) -> ObjectGuid {
        self.container
    }

    pub const fn container_slot(&self) -> u8 {
        self.container_slot
    }

    pub const fn bonding(&self) -> ItemBondingType {
        self.bonding
    }

    pub const fn update_state(&self) -> ItemUpdateState {
        self.update_state
    }

    pub const fn queue_pos(&self) -> i16 {
        self.queue_pos
    }

    pub const fn loot_generated(&self) -> bool {
        self.loot_generated
    }

    pub const fn is_in_trade(&self) -> bool {
        self.in_trade
    }

    pub const fn last_played_time_update(&self) -> i64 {
        self.last_played_time_update
    }

    pub const fn refund_recipient(&self) -> ObjectGuid {
        self.refund_recipient
    }

    pub const fn paid_money(&self) -> u64 {
        self.paid_money
    }

    pub const fn paid_extended_cost(&self) -> u32 {
        self.paid_extended_cost
    }

    pub const fn soulbound_trade_allowed_guids(&self) -> &HashSet<ObjectGuid> {
        &self.soulbound_trade_allowed_guids
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn read_view(&self) -> ItemReadView<'_> {
        ItemReadView::from_snapshot(self)
    }

    pub fn changed_object_type_mask(&self) -> u32 {
        self.read_view().changed_object_type_mask()
    }

    pub fn values_update(&self) -> crate::ItemValuesUpdate {
        self.read_view().values_update()
    }
}

impl Item {
    pub fn values_snapshot(&self) -> ItemValuesSnapshot {
        ItemValuesSnapshot::capture(self)
    }
}

#[cfg(test)]
#[path = "values_snapshot/tests.rs"]
mod tests;
