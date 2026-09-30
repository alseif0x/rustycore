// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's own PvP-state transitions.
//!
//! In C++ these are Player methods — `Player::UpdatePvP` (Player.cpp:22663),
//! the timer half of `Player::UpdatePvPFlag` (Player.cpp:20816) and the
//! clearing half of contested PvP (Player.cpp:20765 and its reset) — so the
//! flag, the player flag and the timer always move together and no caller can
//! move one without the others.

use crate::Player;
use wow_constants::{UnitPvpFlags, UnitState};

/// C++ `PLAYER_FLAGS_PVP_TIMER`, the same bit the session tree already uses.
const PLAYER_FLAGS_PVP_TIMER_LIKE_CPP: u32 = 0x0004_0000;
/// C++ `PLAYER_FLAGS_CONTESTED_PVP`, likewise.
const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0x0000_0100;

impl Player {
    /// Calculate the represented final `pvpInfo.IsHostile` value for an area.
    ///
    /// This preserves the current Rust branch order and inputs. The lazy
    /// resolver is called only for an uncontested enemies-PvP-flagged area,
    /// where the caller resolves the area faction mask and the Player's
    /// faction-template groups. Zone overrides, battleground state, PvP world
    /// states and PvP-forcing quests remain outside this represented rule.
    #[allow(clippy::too_many_arguments)]
    pub fn represented_hostile_area_state_like_cpp(
        sanctuary: bool,
        free_for_all: bool,
        combat_zone: bool,
        enemies_pvp_flagged: bool,
        contested: bool,
        war_mode_active: bool,
        is_pvp_realm: bool,
        resolve_faction_groups: impl FnOnce() -> (u8, Option<(u8, u8)>),
    ) -> bool {
        let zone_hostile = if sanctuary {
            false
        } else if free_for_all || combat_zone {
            true
        } else if enemies_pvp_flagged {
            if contested {
                war_mode_active
            } else {
                let (faction_group_mask, faction_template_groups) = resolve_faction_groups();
                faction_template_groups.is_some_and(|(friend_group, enemy_group)| {
                    if (friend_group & faction_group_mask) != 0 {
                        false
                    } else if (enemy_group & faction_group_mask) != 0 {
                        true
                    } else {
                        is_pvp_realm
                    }
                })
            }
        } else {
            false
        };

        // Preserve the current represented behavior, including War Mode's
        // final override after the Sanctuary branch.
        zone_hostile || war_mode_active
    }

    /// C++ `Player::UpdatePvP(bool state, bool _override)` (Player.cpp:22663).
    ///
    /// The branch order follows Classic: clearing sets the flag before zeroing
    /// the timer, and arming stamps the timer before setting the flag. Only the
    /// flag is published, so the order is not observable on the wire; it is
    /// kept because this is the C++ transition, not a re-derivation of it.
    pub fn update_pvp_like_cpp(&mut self, state: bool, now_secs: i64, override_state: bool) {
        if !state || override_state {
            self.set_pvp_state_like_cpp(state);
            self.gameplay_state_mut().world_local.pvp_end_timer = None;
        } else {
            self.gameplay_state_mut().world_local.pvp_end_timer = Some(now_secs);
            self.set_pvp_state_like_cpp(state);
        }
    }

    /// C++ `SetPvP(state)` as `Player::UpdatePvP` reaches it.
    fn set_pvp_state_like_cpp(&mut self, state: bool) {
        if state {
            self.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        } else {
            self.unit_mut().remove_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        }
    }

    /// The expiry half of C++ `Player::UpdatePvPFlag` (Player.cpp:20816).
    ///
    /// The caller owns the guards Classic checks before reaching it — being
    /// PvP, holding a timer, the five-minute grace and not being hostile.
    pub fn expire_pvp_timer_like_cpp(&mut self) {
        self.gameplay_state_mut().world_local.pvp_end_timer = None;
        self.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
    }

    /// Clear contested PvP, the inverse of C++ `Player::SetContestedPvP`
    /// (Player.cpp:20765): the unit state, the player flag and the timer leave
    /// together.
    pub fn clear_contested_pvp_like_cpp(&mut self) {
        self.unit_mut()
            .clear_unit_state(UnitState::ATTACK_PLAYER.bits());
        self.remove_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
        self.gameplay_state_mut().world_local.contested_pvp_timer = 0;
    }
}

#[cfg(test)]
#[path = "pvp/tests.rs"]
mod tests;
