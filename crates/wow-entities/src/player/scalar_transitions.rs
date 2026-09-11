// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Single-field Player transitions that C++ also keeps on the object itself.
//!
//! Each one is a named C++ member function over one field, and each was reached
//! here by opening the Player's gameplay state from the session instead.

use crate::{Player, PlayerHomebindLikeCpp};
use wow_core::ObjectGuid;

impl Player {
    /// C++ `Player::SetHomebind` (Player.h:2439).
    pub fn set_homebind_like_cpp(&mut self, homebind: PlayerHomebindLikeCpp) {
        self.gameplay_state_mut().homebind = Some(homebind);
    }

    /// C++ `Unit::SetPetGUID` (Unit.h:1142), which writes the pet summon slot.
    pub fn set_pet_guid_like_cpp(&mut self, pet_guid: Option<ObjectGuid>) {
        self.gameplay_state_mut().pet_guid = pet_guid;
    }

    /// C++ `Player::ActivatePvpItemLevels` (Player.h:2500) over
    /// `_usePvpItemLevels` (Player.h:3143).
    pub fn activate_pvp_item_levels_like_cpp(&mut self, activate: bool) {
        self.gameplay_state_mut().using_pvp_item_levels = activate;
    }

    /// C++ `Player::_questRewardedTalentPoints` (Player.h:1120), which the
    /// quest reward accumulates by the quest's skill points
    /// (`Player::RewardQuest`, Player.cpp:14791, and the load-time replay at
    /// Player.cpp:18771) and the talent total reads back (Player.cpp:26358).
    /// C++ adds into a `uint32`; the saturating add preserves the existing
    /// RustyCore behavior at the boundary rather than wrapping.
    pub fn add_quest_rewarded_talent_points_like_cpp(&mut self, points: u32) {
        let rewarded = &mut self.gameplay_state_mut().quest_rewarded_talent_points;
        *rewarded = rewarded.saturating_add(points);
    }
}
