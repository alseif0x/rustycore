// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical Player and map facts consumed by instance applications.

use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

pub struct InstancePlayerAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn instance_player_access_like_cpp(&self) -> InstancePlayerAccessLikeCpp<'_> {
        InstancePlayerAccessLikeCpp { core: self }
    }
}

impl InstancePlayerAccessLikeCpp<'_> {
    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn difficulty_preferences_like_cpp(&self) -> Option<(u32, u32, u32)> {
        self.core
            .with_owned_player_like_cpp(|player| player.difficulty_preferences_like_cpp())
    }

    pub fn set_dungeon_difficulty_like_cpp(&self, difficulty_id: u32) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_dungeon_difficulty_id_like_cpp(difficulty_id);
            })
            .is_some()
    }

    pub fn set_raid_difficulty_like_cpp(&self, difficulty_id: u32) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_raid_difficulty_id_like_cpp(difficulty_id);
            })
            .is_some()
    }

    pub fn set_legacy_raid_difficulty_like_cpp(&self, difficulty_id: u32) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_legacy_raid_difficulty_id_like_cpp(difficulty_id);
            })
            .is_some()
    }

    pub fn game_master_with_fixture_like_cpp(&self, fixture_value: &bool) -> Option<bool> {
        self.core.player_is_game_master_with_fixture_like_cpp(fixture_value)
    }

    pub fn canonical_game_master_like_cpp(&self) -> Option<bool> {
        self.core.player_is_game_master_canonical_like_cpp()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    pub fn canonical_map_difficulty_like_cpp(
        &self,
        map_id: u32,
        instance_id: u32,
    ) -> Option<u8> {
        let manager = self.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_id, instance_id)
            .map(|managed| managed.map().difficulty())
    }
}
