// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::progression` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// XP, talents, glyphs and respec, skills and proficiencies, reputation and rest fixtures.
pub(crate) struct ProgressionState {
    #[cfg(test)]
    pub(in crate::session) championing_faction_like_cpp: u32,
    #[cfg(test)]
    pub(crate) represented_enchanting_skill: u16,
    /// Handle-less test fixture for Player-owned skill values and persistence rows.
    #[cfg(test)]
    pub(in crate::session) player_skill_test_fixture_like_cpp: PlayerSkillTestFixtureLikeCpp,
    #[cfg(test)]
    pub(in crate::session) represented_gray_level_script_overrides_like_cpp: HashMap<u8, u8>,
    /// Handle-less RestMgr and rate-policy fixture; production state belongs to Player.
    #[cfg(test)]
    pub(in crate::session) rest_mgr_test_fixture_like_cpp: RestMgrTestFixtureLikeCpp,
    /// Handle-less test fallback for C++ `Player::_specializationInfo.ResetTalentsCost`.
    #[cfg(test)]
    pub(in crate::session) represented_talent_reset_cost_like_cpp: u32,
    /// Handle-less test fallback for C++ `Player::_specializationInfo.ResetTalentsTime`.
    #[cfg(test)]
    pub(in crate::session) represented_talent_reset_time_secs_like_cpp: u64,
    /// C++ `UF::ActivePlayerData::CharacterPoints`, recalculated by InitTalentForLevel/LearnTalent.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_character_points_like_cpp: i32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_xp: u32,
    /// XP required to reach next level, cached from player_xp_for_level.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_next_level_xp: u32,
    /// Represented C++ ActivePlayerData::CurrentSpecID / GetPrimarySpecialization.
    #[cfg(test)]
    pub(crate) represented_primary_specialization_id_like_cpp: u32,
    /// C++ `Player::m_weaponProficiency`; `Spell::EffectProficiency` ORs into it.
    #[cfg(test)]
    pub(in crate::session) represented_weapon_proficiency_like_cpp: u32,
    /// C++ `Player::m_armorProficiency`; `Spell::EffectProficiency` ORs into it.
    #[cfg(test)]
    pub(in crate::session) represented_armor_proficiency_like_cpp: u32,
    /// Represented accepted talent-respec wipe requests until Player::ResetTalents is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_confirm_respec_wipe_requests_like_cpp:
        Vec<RepresentedConfirmRespecWipeLikeCpp>,
    /// Represented `sScriptMgr->OnPlayerTalentsReset` calls until ScriptMgr is live.
    #[cfg(test)]
    pub(in crate::session) represented_talent_reset_script_hooks_like_cpp:
        Vec<RepresentedTalentResetScriptHookLikeCpp>,
    /// Represented `unit->CastSpell(_player, 14867, true)` after successful talent reset.
    #[cfg(test)]
    pub(in crate::session) represented_talent_respec_visual_spell_casts_like_cpp:
        Vec<RepresentedTalentRespecVisualSpellCastLikeCpp>,
    /// Represented `CriteriaType::MoneySpentOnRespecs` / `TotalRespecs` events.
    #[cfg(test)]
    pub(in crate::session) represented_talent_respec_criteria_events_like_cpp:
        Vec<RepresentedTalentRespecCriteriaEventLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_active_talent_group_like_cpp: u8,
    #[cfg(test)]
    pub(in crate::session) represented_bonus_talent_groups_like_cpp: u8,
    #[cfg(test)]
    pub(in crate::session) represented_talents_like_cpp:
        [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
    #[cfg(test)]
    pub(in crate::session) represented_talents_loaded_like_cpp: bool,
    #[cfg(test)]
    pub(in crate::session) represented_glyphs_like_cpp: [[u16;
        wow_packet::packets::misc::MAX_GLYPH_SLOT_INDEX_LIKE_CPP];
        MAX_SPECIALIZATIONS_LIKE_CPP],
    #[cfg(test)]
    pub(in crate::session) represented_glyphs_loaded_like_cpp: bool,
    /// Fixture-only fallback. Production C++ `ReputationMgr` state is owned by
    /// the generation-checked canonical `Player`.
    #[cfg(test)]
    /// Test-fallback reputation state for a session without a canonical
    /// Player owner; production always uses the Player's own state (#735).
    #[cfg(test)]
    pub(in crate::session) reputation_state_like_cpp: wow_entities::PlayerReputationStateLikeCpp,
    /// C++ `ActivePlayerData::WatchedFactionIndex` represented state.
    #[cfg(test)]
    pub(in crate::session) watched_faction_index_like_cpp: i32,
}
