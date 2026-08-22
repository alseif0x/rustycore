// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account collections: mounts, toys, heirlooms and appearances.

use super::*;

impl Player {
    pub fn set_current_battle_pet_breed_quality_like_cpp(&mut self, quality: u8) {
        self.set_player_u8(
            PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT,
            quality,
            |data| &mut data.current_battle_pet_breed_quality,
        );
    }

    pub fn set_summoned_battle_pet_guid_like_cpp(&mut self, guid: ObjectGuid) {
        self.set_active_guid(
            ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT,
            guid,
            |data| &mut data.summoned_battle_pet_guid,
        );
    }

    pub fn set_battle_pet_data_like_cpp(&mut self, pet_guid: ObjectGuid, quality: u8, level: u16) {
        self.set_summoned_battle_pet_guid_like_cpp(pet_guid);
        self.set_current_battle_pet_breed_quality_like_cpp(quality);
        self.unit_mut()
            .set_wild_battle_pet_level_like_cpp(u32::from(level));
    }

    pub fn clear_battle_pet_data_like_cpp(&mut self) {
        self.set_battle_pet_data_like_cpp(ObjectGuid::EMPTY, 0, 0);
    }

    /// C++ `Player::AddHeirloom`.
    pub fn add_heirloom_like_cpp(&mut self, item_id: i32, flags: u32) -> usize {
        let index = self.active_data.heirlooms.len();
        self.active_data.heirlooms.push(item_id);
        self.active_data.heirloom_flags.push(flags);
        Self::set_dynamic_update_mask_index(&mut self.active_data.heirlooms_update_mask, index);
        Self::set_dynamic_update_mask_index(
            &mut self.active_data.heirloom_flags_update_mask,
            index,
        );
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT);
        index
    }

    /// C++ `Player::SetHeirloom`.
    pub fn set_heirloom_like_cpp(&mut self, index: usize, item_id: i32) -> bool {
        let Some(slot) = self.active_data.heirlooms.get_mut(index) else {
            return false;
        };

        *slot = item_id;
        Self::set_dynamic_update_mask_index(&mut self.active_data.heirlooms_update_mask, index);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT);
        true
    }

    /// C++ `Player::SetHeirloomFlags`.
    pub fn set_heirloom_flags_like_cpp(&mut self, index: usize, flags: u32) -> bool {
        let Some(slot) = self.active_data.heirloom_flags.get_mut(index) else {
            return false;
        };

        *slot = flags;
        Self::set_dynamic_update_mask_index(
            &mut self.active_data.heirloom_flags_update_mask,
            index,
        );
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT);
        true
    }

    pub fn heirlooms_like_cpp(&self) -> &[i32] {
        &self.active_data.heirlooms
    }

    pub fn heirloom_flags_like_cpp(&self) -> &[u32] {
        &self.active_data.heirloom_flags
    }

    /// C++ `Player::AddToy`.
    pub fn add_toy_like_cpp(&mut self, item_id: i32) -> usize {
        let index = self.active_data.toys.len();
        self.active_data.toys.push(item_id);
        Self::set_dynamic_update_mask_index(&mut self.active_data.toys_update_mask, index);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_TOYS_BIT);
        index
    }

    pub fn toys_like_cpp(&self) -> &[i32] {
        &self.active_data.toys
    }
}
