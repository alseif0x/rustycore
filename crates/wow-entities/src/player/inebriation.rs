use super::Player;
use crate::ObjectNotifyFlags;

const INVISIBILITY_DRUNK: usize = 6;

impl Player {
    /// C++ `Player::GetDrunkenstateByValue` thresholds.
    pub const fn drunken_state_by_value_like_cpp(value: u8) -> u8 {
        if value >= 90 {
            3
        } else if value >= 50 {
            2
        } else if value > 0 {
            1
        } else {
            0
        }
    }

    /// Apply C++ `Player::SetDrunkValue` and return whether the drunkenness
    /// threshold changed. The owning application supplies fake-Aura facts
    /// read from this Player's canonical applications.
    pub fn set_drunk_value_like_cpp(
        &mut self,
        requested_value: u8,
        fake_inebriation_modifier: i32,
        has_fake_inebriate_aura: bool,
    ) -> bool {
        let old_value = self.inebriation_like_cpp();
        let is_sobering = requested_value < old_value;
        let new_value = requested_value.min(100);
        let old_state = Self::drunken_state_by_value_like_cpp(old_value);
        let new_state = Self::drunken_state_by_value_like_cpp(new_value);
        let drunk_percent = i32::from(new_value).max(fake_inebriation_modifier);

        if drunk_percent > 0 {
            self.unit
                .set_invisibility_detect_like_cpp(INVISIBILITY_DRUNK, drunk_percent);
        } else if !has_fake_inebriate_aura && new_value == 0 {
            self.unit
                .clear_invisibility_detect_flag_like_cpp(INVISIBILITY_DRUNK);
        }

        self.set_inebriation_like_cpp(new_value);
        // Player.cpp:894 / UpdateField.h:262 marks this assignment even for sober zero.
        self.mark_player_data(super::PLAYER_DATA_INEBRIATION_BIT);
        if self.unit.world().object().is_in_world() {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        }
        if !is_sobering {
            self.drunk_timer = 0;
        }

        old_state != new_state
    }
}

#[cfg(test)]
#[path = "../player_tests/inebriation.rs"]
mod tests;
