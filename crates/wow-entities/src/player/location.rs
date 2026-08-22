// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player position, map/zone and bind points.

use super::*;

impl Player {
    pub fn bind_session(&mut self, session_id: Option<u64>) {
        self.session_id = session_id;
    }

    /// C++ `Player::AddExploredZones(pos, mask)`.
    pub fn add_explored_zones_like_cpp(&mut self, index: usize, mask: u64) -> bool {
        let Some(target) = self.active_data.explored_zones.get_mut(index) else {
            return false;
        };
        let new_value = *target | mask;
        if *target == new_value {
            return false;
        }

        *target = new_value;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT,
            ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT,
            index,
        );
        true
    }

    pub fn set_explored_zones_block_like_cpp(&mut self, index: usize, value: u64) -> bool {
        let Some(target) = self.active_data.explored_zones.get_mut(index) else {
            return false;
        };
        if *target == value {
            return false;
        }

        *target = value;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT,
            ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT,
            index,
        );
        true
    }

    pub fn load_explored_zones_string_like_cpp(&mut self, input: &str) -> usize {
        let blocks = parse_explored_zones_db_string_like_cpp(input);
        self.set_explored_zones_blocks_like_cpp(&blocks)
    }

    pub fn set_explored_zones_blocks_like_cpp(
        &mut self,
        blocks: &[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    ) -> usize {
        blocks
            .iter()
            .copied()
            .enumerate()
            .filter(|(index, value)| self.set_explored_zones_block_like_cpp(*index, *value))
            .count()
    }

    pub fn explored_zones_block_like_cpp(&self, index: usize) -> Option<u64> {
        self.active_data.explored_zones.get(index).copied()
    }

    pub fn explored_zones_blocks_like_cpp(&self) -> &[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP] {
        &self.active_data.explored_zones
    }

    pub fn explored_zones_db_string_like_cpp(&self) -> String {
        explored_zones_db_string_from_blocks_like_cpp(&self.active_data.explored_zones)
    }
}
