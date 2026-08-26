// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl Player {
    pub fn set_party_type_like_cpp(&mut self, category: u8, party_type: u8) -> bool {
        let index = usize::from(category);
        if index >= self.data.party_type.len() {
            return false;
        }

        if self.data.party_type[index] != party_type {
            self.data.party_type[index] = party_type;
            self.mark_player_data_array(
                PLAYER_DATA_PARTY_TYPE_PARENT_BIT,
                PLAYER_DATA_PARTY_TYPE_FIRST_BIT,
                index,
            );
        }
        true
    }
}
