// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed Player value and update-mask operations used by inventory projections.

use super::{
    ACTIVE_PLAYER_DATA_COINAGE_BIT, ACTIVE_PLAYER_DATA_PARENT_BIT, ActivePlayerDataValues,
    PLAYER_DATA_FLAGS_BIT, PLAYER_DATA_PARENT_BIT, Player, PlayerDataValues,
};
use crate::UpdateMask;

pub(super) struct InventoryValuesMut<'a> {
    player_data: &'a mut PlayerDataValues,
    active_player_data: &'a mut ActivePlayerDataValues,
    player_data_changes: &'a mut UpdateMask,
    active_player_data_changes: &'a mut UpdateMask,
}

impl<'a> InventoryValuesMut<'a> {
    fn new(
        player_data: &'a mut PlayerDataValues,
        active_player_data: &'a mut ActivePlayerDataValues,
        player_data_changes: &'a mut UpdateMask,
        active_player_data_changes: &'a mut UpdateMask,
    ) -> Self {
        Self {
            player_data,
            active_player_data,
            player_data_changes,
            active_player_data_changes,
        }
    }

    pub(super) fn replace_all_player_flags(&mut self, flags: u32) -> bool {
        if self.player_data.player_flags == flags {
            return false;
        }

        self.player_data.player_flags = flags;
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(PLAYER_DATA_FLAGS_BIT);
        true
    }

    pub(super) fn set_money(&mut self, value: u64) -> bool {
        if self.active_player_data.coinage == value {
            return false;
        }

        self.active_player_data.coinage = value;
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_COINAGE_BIT);
        true
    }

    pub(super) fn mark_money_changed(&mut self) -> bool {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_COINAGE_BIT);
        true
    }
}

impl Player {
    pub(super) fn inventory_values_mut(&mut self) -> InventoryValuesMut<'_> {
        InventoryValuesMut::new(
            &mut self.data,
            &mut self.active_data,
            &mut self.player_data_changes,
            &mut self.active_player_data_changes,
        )
    }

    pub(super) fn apply_inventory_value_update_request(&mut self, update_requested: bool) {
        if update_requested {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }
}
