// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Client-visible GUIDs and update-field publication.

use super::*;

const ACTIVE_PLAYER_DATA_LOCAL_FLAGS_PARENT_BIT: usize = 38;
const ACTIVE_PLAYER_DATA_LOCAL_FLAGS_BIT: usize = 69;

impl Player {
    /// Player.h:2760 / UpdateField.h:262 marks LocalFlags even on a no-op.
    pub fn set_player_local_flags_like_cpp(&mut self, flags: u32) {
        let previous = self.gameplay_state.active_local_flags;
        self.gameplay_state.active_local_flags |= flags;
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_LOCAL_FLAGS_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_LOCAL_FLAGS_BIT);
        if self.gameplay_state.active_local_flags != previous {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }

    pub fn remove_player_local_flags_like_cpp(&mut self, flags: u32) {
        let previous = self.gameplay_state.active_local_flags;
        self.gameplay_state.active_local_flags &= !flags;
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_LOCAL_FLAGS_PARENT_BIT);
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_LOCAL_FLAGS_BIT);
        if self.gameplay_state.active_local_flags != previous {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }

    /// C++ RemovePlayerFlag marks the native UpdateField even for an absent flag.
    pub fn remove_player_flags_like_cpp(&mut self, flags: u32) {
        let previous = self.data.player_flags;
        self.data.player_flags &= !flags;
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(PLAYER_DATA_FLAGS_BIT);
        if self.data.player_flags != previous {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }

    pub fn sync_farsight_visibility_detection_like_cpp(&mut self) {
        let farsight_object = self.active_data.farsight_object;
        self.unit_mut()
            .set_seer_can_always_see_target_guid_like_cpp(farsight_object);
    }

    pub fn values_update(&self, include_active_player: bool) -> PlayerValuesUpdate {
        let unit_update = self.unit.values_update();
        PlayerValuesUpdate {
            changed_object_type_mask: self.changed_object_type_mask(include_active_player),
            object_data: unit_update.object_data,
            unit_data: unit_update.unit_data,
            player_data: self
                .player_data_changes
                .is_any_set()
                .then(|| PlayerDataUpdate {
                    mask: self.player_data_changes.clone(),
                    values: self.data,
                    quest_log: self.quest_log_rows_for_update_like_cpp(),
                }),
            active_player_data: (include_active_player
                && self.active_player_data_changes.is_any_set())
            .then(|| ActivePlayerDataUpdate {
                mask: self.active_player_data_changes.clone(),
                values: self.active_data.clone(),
                local_flags: self.gameplay_state.active_local_flags,
                rest_info_change_masks: self.rest_info_change_masks,
            }),
            derived_stats: self.derived_stats_values_update(include_active_player),
        }
    }
}
