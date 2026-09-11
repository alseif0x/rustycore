// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player talent, glyph and specialization state.
//!
//! Separated from the `player/mod.rs` root under #752, which also made the
//! fields private: every transition C++ performs on `_talents` and
//! `_specializationInfo` is a named operation here.

use std::collections::BTreeMap;

use super::{PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP, PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP};

/// Exact mutable owner for C++ `Player::_specializationInfo.Talents` and
/// `Player::_specializationInfo.Glyphs` (`Player.h:1039-1040`).
///
/// The load flags preserve the distinction between an authoritative empty DB
/// result and state that has not been hydrated, so persistence never fabricates
/// an empty replacement when the canonical owner is unavailable.
///
/// Its fields are private: every transition C++ performs on `_specializationInfo`
/// is a named operation here, so a caller cannot write a group index, glyph slot
/// or load flag the owner has not checked (#752).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerTalentRuntimeState {
    talent_groups: [BTreeMap<u32, u8>; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP],
    talents_loaded: bool,
    glyph_groups: [[u16; PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP]; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP],
    glyphs_loaded: bool,
    /// C++ `Player::_specializationInfo.{ActiveGroup,BonusGroups,ResetTalentsCost,ResetTalentsTime}`.
    active_group: u8,
    bonus_groups: u8,
    reset_talents_cost: u32,
    reset_talents_time_secs: u64,
}

impl PlayerTalentRuntimeState {
    /// C++ `Player::GetActiveTalentGroup`.
    #[must_use]
    pub fn active_group_like_cpp(&self) -> u8 {
        self.active_group
    }

    /// C++ `Player::ActivateTalentGroup` selecting the group to play.
    ///
    /// C++ never addresses a group outside `MAX_SPECIALIZATIONS`; an
    /// out-of-range request is clamped to the last group, as the session did
    /// before this owner existed. Returns the group actually in effect.
    pub fn set_active_group_like_cpp(&mut self, group: u8) -> u8 {
        self.active_group = group.min(Self::LAST_GROUP_INDEX_LIKE_CPP);
        self.active_group
    }

    /// C++ `Player::_specializationInfo.BonusGroups`.
    #[must_use]
    pub fn bonus_groups_like_cpp(&self) -> u8 {
        self.bonus_groups
    }

    pub fn set_bonus_groups_like_cpp(&mut self, bonus_groups: u8) -> u8 {
        self.bonus_groups = bonus_groups.min(Self::LAST_GROUP_INDEX_LIKE_CPP);
        self.bonus_groups
    }

    /// Whether `Player::_LoadTalents` has run for this Player.
    #[must_use]
    pub fn talents_loaded_like_cpp(&self) -> bool {
        self.talents_loaded
    }

    /// C++ `Player::_LoadTalents` completing (`Player.cpp:26623`).
    pub fn mark_talents_loaded_like_cpp(&mut self) {
        self.talents_loaded = true;
    }

    /// Whether `Player::_LoadGlyphs` has run for this Player.
    #[must_use]
    pub fn glyphs_loaded_like_cpp(&self) -> bool {
        self.glyphs_loaded
    }

    /// C++ `Player::_LoadGlyphs` completing (`Player.cpp:26573`).
    pub fn mark_glyphs_loaded_like_cpp(&mut self) {
        self.glyphs_loaded = true;
    }

    /// C++ `Player::GetPlayerTalentMap(group)`.
    #[must_use]
    pub fn talent_group_like_cpp(&self, group: u8) -> Option<&BTreeMap<u32, u8>> {
        self.talent_groups.get(usize::from(group))
    }

    /// Every talent group in group order, as the save projection reads them.
    pub fn talent_groups_like_cpp(&self) -> impl Iterator<Item = &BTreeMap<u32, u8>> {
        self.talent_groups.iter()
    }

    /// C++ `Player::AddTalent` storing the rank in one group's map
    /// (`Player.cpp:2644`, `talentMap[talent->ID] = { ..., rank }`).
    ///
    /// The catalog validation, spell learning and override handling C++ does
    /// around that write stay with the caller; this owns the stored row.
    pub fn add_talent_like_cpp(&mut self, group: u8, talent_id: u32, rank: u8) -> bool {
        let Some(talents) = self.talent_groups.get_mut(usize::from(group)) else {
            return false;
        };
        talents.insert(talent_id, rank);
        true
    }

    /// Take one group's talents, as `Player::ActivateTalentGroup` does before
    /// removing their spells (`Player.cpp:26894`). Refuses while the talents
    /// have not been loaded, so an unhydrated Player cannot report an empty
    /// group as authoritative.
    pub fn take_talent_group_like_cpp(&mut self, group: u8) -> Option<BTreeMap<u32, u8>> {
        if !self.talents_loaded {
            return None;
        }
        self.talent_groups
            .get_mut(usize::from(group))
            .map(std::mem::take)
    }

    /// C++ `Player::ResetTalents` emptying every group (`Player.cpp:3505`).
    /// The load flag returns to unhydrated, which is what login expects.
    pub fn clear_talents_like_cpp(&mut self) {
        for talents in &mut self.talent_groups {
            talents.clear();
        }
        self.talents_loaded = false;
    }

    /// Copy every talent group, for a plan that must own them.
    #[must_use]
    pub fn talent_groups_snapshot_like_cpp(
        &self,
    ) -> [BTreeMap<u32, u8>; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP] {
        self.talent_groups.clone()
    }

    /// Install the groups a completed talent reset leaves behind.
    pub fn replace_talent_groups_like_cpp(
        &mut self,
        groups: [BTreeMap<u32, u8>; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP],
    ) {
        self.talent_groups = groups;
    }

    /// Copy every glyph group, for a fixture that must own them.
    #[must_use]
    pub fn glyph_groups_snapshot_like_cpp(
        &self,
    ) -> [[u16; PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP]; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP] {
        self.glyph_groups
    }

    /// Install every glyph group at once, as a hydration fixture does.
    pub fn replace_glyph_groups_like_cpp(
        &mut self,
        groups: [[u16; PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP]; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP],
    ) {
        self.glyph_groups = groups;
    }

    /// C++ `Player::GetGlyph(group, slot)`.
    #[must_use]
    pub fn glyph_like_cpp(&self, group: u8, slot: u8) -> Option<u16> {
        self.glyph_groups
            .get(usize::from(group))?
            .get(usize::from(slot))
            .copied()
    }

    /// Every glyph group in group order, as the save projection reads them.
    pub fn glyph_groups_like_cpp(
        &self,
    ) -> impl Iterator<Item = &[u16; PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP]> {
        self.glyph_groups.iter()
    }

    /// C++ `Player::SetGlyph(slot, glyph)` (`Player.cpp:25477`) for one group.
    pub fn set_glyph_like_cpp(&mut self, group: u8, slot: u8, glyph: u16) -> bool {
        let Some(slots) = self.glyph_groups.get_mut(usize::from(group)) else {
            return false;
        };
        let Some(current) = slots.get_mut(usize::from(slot)) else {
            return false;
        };
        *current = glyph;
        true
    }

    /// Empty every glyph group and return to unhydrated, as login does before
    /// `Player::_LoadGlyphs` runs for the next character.
    pub fn clear_glyphs_like_cpp(&mut self) {
        self.glyph_groups =
            [[0; PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP]; PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP];
        self.glyphs_loaded = false;
    }

    /// C++ `Player::_specializationInfo.ResetTalentsCost`.
    #[must_use]
    pub fn reset_talents_cost_like_cpp(&self) -> u32 {
        self.reset_talents_cost
    }

    /// C++ `Player::_specializationInfo.ResetTalentsTime`.
    #[must_use]
    pub fn reset_talents_time_secs_like_cpp(&self) -> u64 {
        self.reset_talents_time_secs
    }

    /// C++ `Player::ResetTalents` recording the paid cost and its timestamp.
    pub fn set_reset_talents_state_like_cpp(&mut self, cost: u32, time_secs: u64) {
        self.reset_talents_cost = cost;
        self.reset_talents_time_secs = time_secs;
    }

    const LAST_GROUP_INDEX_LIKE_CPP: u8 = (PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8;
}
