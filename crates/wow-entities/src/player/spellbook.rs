// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spells, auras, cooldowns and talents.

use super::*;

impl Player {
    pub const fn titan_grip_penalty_spell_id(&self) -> u32 {
        self.titan_grip_penalty_spell_id
    }

    /// C++ `MAX_ACTION_BUTTONS` for the 3.4.3 client.
    pub const ACTION_BUTTON_COUNT_LIKE_CPP: usize = 180;

    /// C++ `Player::AddActionButton` storage mutation after the caller has
    /// performed the store-backed validation that still lives above the
    /// entity boundary.
    pub fn set_action_button_like_cpp(
        &mut self,
        button: u8,
        action_id: u32,
        action_type: u8,
    ) -> bool {
        if usize::from(button) >= Self::ACTION_BUTTON_COUNT_LIKE_CPP {
            return false;
        }

        self.gameplay_state
            .action_buttons
            .retain(|record| record.button != button);
        if action_id != 0 {
            self.gameplay_state
                .action_buttons
                .push(PlayerActionButtonRecord {
                    button,
                    action_id,
                    action_type,
                });
            self.gameplay_state
                .action_buttons
                .sort_unstable_by_key(|record| record.button);
        }
        true
    }

    /// C++ `_LoadActionButtons` starts from an empty `m_actionButtons` map.
    pub fn reset_action_buttons_for_load_like_cpp(&mut self) {
        self.gameplay_state.action_buttons.clear();
        self.gameplay_state.action_buttons_loaded = false;
    }

    pub fn mark_action_buttons_loaded_like_cpp(&mut self) {
        self.gameplay_state.action_buttons_loaded = true;
    }

    pub fn action_buttons_loaded_like_cpp(&self) -> bool {
        self.gameplay_state.action_buttons_loaded
    }

    pub fn action_buttons_snapshot_like_cpp(&self) -> [u32; Self::ACTION_BUTTON_COUNT_LIKE_CPP] {
        let mut buttons = [0; Self::ACTION_BUTTON_COUNT_LIKE_CPP];
        for record in &self.gameplay_state.action_buttons {
            let Some(slot) = buttons.get_mut(usize::from(record.button)) else {
                continue;
            };
            *slot = record.action_id | (u32::from(record.action_type) << 24);
        }
        buttons
    }

    pub fn action_button_like_cpp(&self, button: u8) -> Option<u32> {
        (usize::from(button) < Self::ACTION_BUTTON_COUNT_LIKE_CPP).then(|| {
            self.gameplay_state
                .action_buttons
                .iter()
                .find(|record| record.button == button)
                .map(|record| record.action_id | (u32::from(record.action_type) << 24))
                .unwrap_or(0)
        })
    }
}
