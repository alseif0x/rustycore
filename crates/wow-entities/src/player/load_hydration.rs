// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-load hydration of Player-owned CUF profiles and customizations.
//!
//! Like the mail and achievement hydration beside it, these are named as
//! hydration so the difference from a gameplay transition stays visible. The
//! CUF profile slot is the one exception that C++ also reaches in-game, and it
//! keeps its own `Player::SaveCUFProfile` name for that reason.

use crate::{Player, PlayerCufProfile, PlayerCustomizationChoice};

/// C++ `MAX_CUF_PROFILES` (CUFProfile.h:26). C++ holds the profiles in a fixed
/// `std::array` (Player.h:3059); the Vec here is normalized to the same length
/// before a slot is written. Kept private: `wow_packet` already owns the
/// wire-side copy of the same C++ constant, and the Player needs only its own
/// array length.
const MAX_CUF_PROFILES_LIKE_CPP: usize = 5;

impl Player {
    /// C++ `Player::_LoadCUFProfiles` (Player.h:2879, called from the login
    /// holder at Player.cpp:17886): the load completed, including a load that
    /// found no rows.
    pub fn mark_cuf_profiles_loaded_like_cpp(&mut self) {
        self.gameplay_state_mut().cuf_profiles_loaded = true;
    }

    /// Reset the Player's CUF profiles to the constructed state: C++ empties
    /// every slot in the Player constructor (Player.cpp:342), and the load
    /// marker goes back to unloaded with them.
    pub fn reset_cuf_profiles_like_cpp(&mut self) {
        let state = self.gameplay_state_mut();
        state.cuf_profiles = vec![None; MAX_CUF_PROFILES_LIKE_CPP];
        state.cuf_profiles_loaded = false;
    }

    /// C++ `Player::SaveCUFProfile` (Player.h:1635): replace the profile at
    /// position 0-4, `None` for the overload that empties it (Player.h:1634).
    /// Returns whether the index is one of the five slots.
    pub fn save_cuf_profile_like_cpp(
        &mut self,
        index: usize,
        profile: Option<PlayerCufProfile>,
    ) -> bool {
        if index >= MAX_CUF_PROFILES_LIKE_CPP {
            return false;
        }
        let profiles = &mut self.gameplay_state_mut().cuf_profiles;
        if profiles.len() != MAX_CUF_PROFILES_LIKE_CPP {
            *profiles = vec![None; MAX_CUF_PROFILES_LIKE_CPP];
        }
        profiles[index] = profile;
        true
    }

    /// C++ `Player::SetCustomizations` (Player.h:2698) as the character load
    /// calls it (Player.cpp:17318, `markChanged = false`): install the loaded
    /// appearance choices without marking the update field changed.
    pub fn hydrate_customizations_like_cpp(
        &mut self,
        customizations: Vec<PlayerCustomizationChoice>,
    ) {
        self.gameplay_state_mut().customizations = customizations;
    }
}
