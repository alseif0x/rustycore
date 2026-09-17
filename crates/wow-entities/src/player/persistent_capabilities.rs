// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player-owned persistent capability transitions.
//!
//! These are the character-loaded fields C++ keeps on `Player`: at-login flags
//! and the weapon/armor proficiency masks. Session owns persistence and packet
//! effects; it names one transition here instead of lending the aggregate to a
//! caller closure.

use super::{Player, PlayerPersistentCapabilityStateLikeCpp};

impl Player {
    /// Snapshot the Player-owned capability fields loaded with the character.
    #[must_use]
    pub fn persistent_capability_state_like_cpp(&self) -> PlayerPersistentCapabilityStateLikeCpp {
        self.gameplay_state().persistent_capabilities
    }

    /// C++ `Player::SetAtLoginFlag`/load hydration over `m_atLoginFlags`
    /// (`Player.h:2474`).
    pub fn set_at_login_flags_like_cpp(&mut self, flags: u16) {
        self.gameplay_state_mut()
            .persistent_capabilities
            .at_login_flags = flags;
    }

    /// C++ `Player::RemoveAtLoginFlag` clears the requested bits and reports
    /// whether any bit was present before the transition.
    pub fn remove_at_login_flags_like_cpp(&mut self, flags: u16) -> bool {
        let state = &mut self.gameplay_state_mut().persistent_capabilities;
        let removed = (state.at_login_flags & flags) != 0;
        state.at_login_flags &= !flags;
        removed
    }

    /// C++ `Player::GetWeaponProficiency` (`Player.h:1432`) returns the mask
    /// accumulated by the learned `SPELL_EFFECT_PROFICIENCY` spells.
    #[must_use]
    pub fn weapon_proficiency_like_cpp(&self) -> u32 {
        self.gameplay_state()
            .persistent_capabilities
            .weapon_proficiency
    }

    /// C++ `Player::AddWeaponProficiency` (`Player.h:1433`) ORs one subclass
    /// mask and returns the resulting mask only when it changed.
    pub fn add_weapon_proficiency_like_cpp(&mut self, subclass_mask: u32) -> Option<u32> {
        let state = &mut self.gameplay_state_mut().persistent_capabilities;
        if subclass_mask == 0 || state.weapon_proficiency & subclass_mask != 0 {
            return None;
        }
        state.weapon_proficiency |= subclass_mask;
        Some(state.weapon_proficiency)
    }

    /// C++ `Player::AddArmorProficiency` (`Player.h:1434`) ORs one subclass
    /// mask and returns the resulting mask only when it changed.
    pub fn add_armor_proficiency_like_cpp(&mut self, subclass_mask: u32) -> Option<u32> {
        let state = &mut self.gameplay_state_mut().persistent_capabilities;
        if subclass_mask == 0 || state.armor_proficiency & subclass_mask != 0 {
            return None;
        }
        state.armor_proficiency |= subclass_mask;
        Some(state.armor_proficiency)
    }
}

#[cfg(test)]
mod tests {
    use super::Player;

    #[test]
    fn persistent_capability_transitions_match_cpp_masks() {
        let mut player = Player::new(Some(7), false);

        player.set_at_login_flags_like_cpp(0x24);
        assert!(player.remove_at_login_flags_like_cpp(0x20));
        assert!(!player.remove_at_login_flags_like_cpp(0x20));
        assert_eq!(
            player.persistent_capability_state_like_cpp().at_login_flags,
            0x04
        );

        assert_eq!(player.add_weapon_proficiency_like_cpp(0x10), Some(0x10));
        assert_eq!(player.add_weapon_proficiency_like_cpp(0x10), None);
        assert_eq!(player.add_armor_proficiency_like_cpp(0x20), Some(0x20));
    }

    #[test]
    fn zero_capability_masks_are_stable() {
        let mut player = Player::new(Some(8), false);

        assert_eq!(player.add_weapon_proficiency_like_cpp(0), None);
        assert_eq!(player.add_armor_proficiency_like_cpp(0), None);
        assert!(!player.remove_at_login_flags_like_cpp(0));
    }
}
