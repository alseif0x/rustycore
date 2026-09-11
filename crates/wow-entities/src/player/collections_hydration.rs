// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login hydration of the Player-owned mail and achievement collections.
//!
//! These are deliberately not gameplay transitions and are named so that the
//! difference stays visible: each one replaces a whole collection from the
//! character load, exactly as its C++ loader does, and neither is reachable as
//! an in-game state change. Gameplay mutations of the same collections belong
//! in their own owners, not here.

use crate::{Player, PlayerAchievementRecord, PlayerMailRecord};

impl Player {
    /// C++ `Player::_LoadMail` (Player.cpp:18560): the character load installs
    /// the Player's mail collection wholesale.
    pub fn hydrate_mails_like_cpp(&mut self, mails: Vec<PlayerMailRecord>) {
        self.gameplay_state_mut().mails = mails;
    }

    /// C++ `PlayerAchievementMgr::LoadFromDB` (AchievementMgr.cpp:243), whose
    /// manager the Player owns: the character load installs the completed
    /// achievements wholesale.
    pub fn hydrate_completed_achievements_like_cpp(
        &mut self,
        achievements: Vec<PlayerAchievementRecord>,
    ) {
        self.gameplay_state_mut().achievements = achievements;
    }
}
