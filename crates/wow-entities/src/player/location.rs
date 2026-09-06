// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player position, map/zone and bind points.

use super::*;
use crate::PlayerTeleportStateLikeCpp;

impl Player {
    pub fn bind_session(&mut self, session_id: Option<u64>) {
        self.session_id = session_id;
    }

    pub fn teleport_state_like_cpp(&self) -> &PlayerTeleportStateLikeCpp {
        &self.gameplay_state.teleport
    }

    pub fn teleport_state_mut_like_cpp(&mut self) -> &mut PlayerTeleportStateLikeCpp {
        &mut self.gameplay_state.teleport
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_owns_teleport_lifecycle_like_cpp() {
        let mut player = Player::new(Some(1), false);
        let destination = Position::new(11.0, 22.0, 33.0, 1.5);

        *player.teleport_state_mut_like_cpp() = PlayerTeleportStateLikeCpp {
            recovery: Default::default(),
            can_delay: true,
            has_delayed: true,
            near_pending: true,
            far_pending: false,
            near_destination: Some((571, destination)),
            delayed: Some((571, destination, 0x10)),
            near_destination_zone_area: Some((20, 21)),
        };

        assert_eq!(
            *player.teleport_state_like_cpp(),
            PlayerTeleportStateLikeCpp {
                recovery: Default::default(),
                can_delay: true,
                has_delayed: true,
                near_pending: true,
                far_pending: false,
                near_destination: Some((571, destination)),
                delayed: Some((571, destination, 0x10)),
                near_destination_zone_area: Some((20, 21)),
            }
        );
    }
}
