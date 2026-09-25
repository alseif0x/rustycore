// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Action bar adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Player, WorldSession};

pub(in crate::session) fn rounded_median_u32(sorted_values: &[u32]) -> u32 {
    debug_assert!(!sorted_values.is_empty());
    let mid = sorted_values.len() / 2;
    if sorted_values.len() % 2 == 1 {
        sorted_values[mid]
    } else {
        ((f64::from(sorted_values[mid - 1]) + f64::from(sorted_values[mid])) / 2.0).round() as u32
    }
}

pub(in crate::session) fn set_active_player_update_bit_like_cpp(mask: &mut [u32; 48], bit: usize) {
    mask[bit / 32] |= 1 << (bit % 32);
}

pub(in crate::session) fn action_button_action_like_cpp(packed: u32) -> u32 {
    packed & 0x00FF_FFFF
}

pub(in crate::session) fn action_button_type_like_cpp(packed: u32) -> u8 {
    ((packed & 0xFF00_0000) >> 24) as u8
}

pub(in crate::session) fn make_action_button_like_cpp(action: u32, action_type: u8) -> u32 {
    action_button_action_like_cpp(action) | ((action_type as u32) << 24)
}

impl WorldSession {
    pub(crate) fn set_tutorial_int_like_cpp(&mut self, index: usize, value: u32) -> bool {
        let Some(current) = self.tutorials_like_cpp.get_mut(index) else {
            return false;
        };

        if *current != value {
            *current = value;
            self.tutorials_changed_like_cpp = true;
        }
        true
    }

    pub(crate) fn apply_tutorial_action_like_cpp(
        &mut self,
        action: u8,
        tutorial_bit: Option<u32>,
    ) -> bool {
        match action {
            wow_packet::packets::misc::TUTORIAL_ACTION_UPDATE_LIKE_CPP => {
                let Some(tutorial_bit) = tutorial_bit else {
                    return false;
                };
                let index = (tutorial_bit >> 5) as usize;
                if index >= self.tutorials_like_cpp.len() {
                    return false;
                }
                let flag = self.tutorials_like_cpp[index] | (1u32 << (tutorial_bit & 0x1F));
                self.set_tutorial_int_like_cpp(index, flag)
            }
            wow_packet::packets::misc::TUTORIAL_ACTION_CLEAR_LIKE_CPP => {
                for index in 0..self.tutorials_like_cpp.len() {
                    self.set_tutorial_int_like_cpp(index, u32::MAX);
                }
                true
            }
            wow_packet::packets::misc::TUTORIAL_ACTION_RESET_LIKE_CPP => {
                for index in 0..self.tutorials_like_cpp.len() {
                    self.set_tutorial_int_like_cpp(index, 0);
                }
                true
            }
            _ => false,
        }
    }

    pub(in crate::session) fn active_player_update_state_like_cpp(&self) -> Option<(u32, i32, u8)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            (
                state.active_local_flags,
                state.active_transport_server_time,
                state.multi_action_bars,
            )
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.active_player_local_flags_like_cpp,
                self.active_player_transport_server_time_like_cpp,
                self.active_player_multi_action_bars_like_cpp,
            ));
        }
        canonical
    }

    pub(in crate::session) fn mutate_active_player_update_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerGameplayState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut state =
                wow_entities::PlayerGameplayState::with_active_player_update_fields_like_cpp(
                    self.active_player_local_flags_like_cpp,
                    self.active_player_transport_server_time_like_cpp,
                    self.active_player_multi_action_bars_like_cpp,
                );
            let result = mutate(&mut state);
            self.active_player_local_flags_like_cpp = state.active_local_flags;
            self.active_player_transport_server_time_like_cpp = state.active_transport_server_time;
            self.active_player_multi_action_bars_like_cpp = state.multi_action_bars;
            return Some(result);
        }
        self.with_owned_player_mut_like_cpp(|player| mutate(player.gameplay_state_mut()))
    }

    #[cfg(test)]
    pub(crate) fn active_player_local_flags_like_cpp(&self) -> u32 {
        self.active_player_update_state_like_cpp()
            .expect("test active Player owner must resolve")
            .0
    }

    #[cfg(test)]
    pub(crate) fn set_active_player_local_flags_like_cpp(&mut self, flags: u32) {
        let _ = self.mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags = flags;
        });
        self.sync_current_player_session_visibility_detection_like_cpp();
    }

    pub(crate) fn represented_set_action_bar_toggles_like_cpp(&mut self, mask: u8) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        if self
            .mutate_active_player_update_state_like_cpp(|state| state.multi_action_bars = mask)
            .is_none()
        {
            return false;
        }
        self.send_active_player_multi_action_bars_update_like_cpp(guid);
        true
    }

    #[cfg(test)]
    pub(crate) fn active_player_multi_action_bars_like_cpp(&self) -> u8 {
        self.active_player_update_state_like_cpp()
            .expect("test active Player owner must resolve")
            .2
    }

    pub(crate) fn represented_set_action_button_like_cpp(
        &mut self,
        index: u8,
        packed_action: u32,
    ) -> bool {
        let action = action_button_action_like_cpp(packed_action);
        let action_type = action_button_type_like_cpp(packed_action);

        // C++ delegates deeper validation to `Player::AddActionButton`
        // (SpellMgr/ObjectMgr/Mount/BattlePet stores). Those runtime stores are
        // not unified here yet, so this represented path preserves the exact
        // packed action/type split and slot bounds while leaving store-backed
        // validation explicit.
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_action_button_like_cpp(index, action, action_type)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let Some(button) = self
                .represented_action_buttons_like_cpp
                .get_mut(usize::from(index))
            else {
                return false;
            };
            *button = make_action_button_like_cpp(action, action_type);
            return true;
        }
        canonical
    }

    pub(crate) fn reset_represented_action_buttons_like_cpp(&mut self) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(Player::reset_action_buttons_for_load_like_cpp)
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_action_buttons_like_cpp =
                [0; wow_packet::packets::misc::MAX_ACTION_BUTTONS];
            self.represented_action_buttons_loaded_like_cpp = false;
        }
    }

    pub(crate) fn represented_action_buttons_snapshot_like_cpp(
        &self,
    ) -> Option<[u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS]> {
        let canonical = self.with_owned_player_like_cpp(Player::action_buttons_snapshot_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_action_buttons_like_cpp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn represented_action_button_like_cpp(&self, index: u8) -> Option<u32> {
        if let Some(canonical) =
            self.with_owned_player_like_cpp(|player| player.action_button_like_cpp(index))
        {
            return canonical;
        }
        self.player_handle_like_cpp
            .is_none()
            .then(|| {
                self.represented_action_buttons_like_cpp
                    .get(usize::from(index))
                    .copied()
            })
            .flatten()
    }
}
