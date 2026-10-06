// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `SessionFixtures::combat` sub-state (#1241 F2): moved fields, no logic.

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::persistence_capabilities::CharacterPowerSnapshotLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PlayerResurrectionRequestLikeCpp;

/// Combat target and flags, vitals and powers, GM and immunity flags, PvP flags and timers, death
/// and resurrection.
pub struct CombatState {
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_player_powers_like_cpp: CharacterPowerSnapshotLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_player_max_powers_like_cpp: CharacterPowerSnapshotLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_player_base_mana_like_cpp: i32,
    /// Currently selected target GUID (SetSelection).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub selection_guid: Option<wow_core::ObjectGuid>,

    // ── Combat state ─────────────────────────────────────────────
    /// Current auto-attack target (None if not in combat).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub combat_target: Option<wow_core::ObjectGuid>,
    /// True when the player is engaged in combat.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub in_combat: bool,
    /// Test-only legacy fixture for sessions without an installed Player owner.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_alive_like_cpp: bool,
    /// Handle-less fixture for C++ `Player::IsGameMaster()`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_game_master_like_cpp: bool,
    /// Represented `CHEAT_GOD` movement/fall guard.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_cheat_god_like_cpp: bool,
    /// Represented `IsImmunedToDamage(SPELL_SCHOOL_MASK_NORMAL)` fall guard.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_normal_damage_immune_like_cpp: bool,
    /// Represented `IsImmuneToEnvironmentalDamage()` guard inside EnvironmentalDamage.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_environmental_damage_immune_like_cpp: bool,
    /// Test-only legacy health fixture for sessions without a Player handle.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_health_like_cpp: u32,
    /// Test-only legacy max-health fixture for sessions without a Player handle.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_max_health_like_cpp: u32,
    /// C++ `Player::_areaSpiritHealerGUID`, represented until battleground/player resurrection owns it.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub area_spirit_healer_guid_like_cpp: ObjectGuid,
    /// Represented `pvpInfo.IsHostile` branch for Honorless Target after taxi landing.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_pvp_hostile_like_cpp: bool,
    /// Represented `Player::IsPvP()` branch for friendly-area near teleport handling.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_pvp_enabled_like_cpp: bool,
    /// Represented `PLAYER_FLAGS_IN_PVP` branch for friendly-area near teleport handling.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_in_pvp_flag_like_cpp: bool,
    /// Represented `pvpInfo.EndTimer` consumed by `Player::UpdatePvPFlag`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_pvp_end_timer_like_cpp: Option<i64>,
    /// C++ `Player::m_contestedPvPTimer`, reset by `Player::ResetContestedPvP`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub player_contested_pvp_timer_like_cpp: u32,
    /// C++ `Player::_resurrectionData`, represented until real Player/Spell
    /// resurrection request ownership exists.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_resurrection_request_like_cpp: Option<PlayerResurrectionRequestLikeCpp>,
    /// C++ `DELAYED_RESURRECT_PLAYER`, represented for resurrection requests
    /// that initiate teleport and must apply after WorldPortResponse.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_delayed_resurrection_after_teleport_like_cpp:
        Option<PlayerResurrectionRequestLikeCpp>,
    /// C++ `Player::GetDeathTimer()` represented for `Spell::EffectStuck`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_death_timer_active_like_cpp: bool,
    /// Count of represented `Player::RepopAtGraveyard` calls from rejected pending binds.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_repop_at_graveyard_count: u32,
}
