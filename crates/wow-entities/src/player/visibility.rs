// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Client-visible GUIDs and update-field publication.

use super::*;

impl Player {
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
                }),
            active_player_data: (include_active_player
                && self.active_player_data_changes.is_any_set())
            .then(|| ActivePlayerDataUpdate {
                mask: self.active_player_data_changes.clone(),
                values: self.active_data.clone(),
                rest_info_change_masks: self.rest_info_change_masks,
            }),
        }
    }
}
