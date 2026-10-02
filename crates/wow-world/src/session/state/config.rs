// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::config` sub-state (#1241 F2): moved fields, no logic.

use super::*;
use super::{
    LegacyCreatureAggroConfigLikeCpp, LootDropRatesLikeCpp, MMapRuntimeConfigLikeCpp,
    ReputationRatesLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    CreatureClassificationHealthRatesLikeCpp,
};

/// Immutable world configuration and rate values (C++ `sWorld` config subsets) and the script
/// dispatchers injected at composition.
pub(in crate::session) struct SessionWorldConfig {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) characters_per_realm_like_cpp: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) declined_names_used_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) feature_system_bpay_store_enabled_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) feature_system_character_undelete_enabled_like_cpp: bool,

    pub(in crate::session) legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) give_player_xp_script_dispatcher_like_cpp:
        Option<GivePlayerXpScriptDispatcherLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp,

    // Characters confirmed for this account

    // Pending async packets to process

    // ── ConnectTo flow ──────────────────────────────────────────

    // ── Logout ──────────────────────────────────────────────────────
    /// C++ `CONFIG_MAX_PLAYER_LEVEL`. `RestMgr::SetRestBonus` reads this value
    /// directly; `Player::IsMaxLevel` reads the expansion-bounded active field.
    pub(in crate::session) max_player_level_config_like_cpp: u32,
    /// C++ `CONFIG_MAX_PRIMARY_TRADE_SKILL`, kept independent from talent
    /// `CharacterPoints` and from the two physical profession associations.
    pub(in crate::session) max_primary_trade_skills_like_cpp: u8,
    /// C++ `CONFIG_CAST_UNSTUCK` immutable world policy injected into spell effects.
    pub(in crate::session) represented_cast_unstuck_enabled_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) exploration_xp_rate_like_cpp: f32,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) min_discovered_scaled_xp_ratio_like_cpp: u32,
    /// C++ `sWorld->getRate(...)` subset used by represented loot generation.
    pub(in crate::session) loot_drop_rates: LootDropRatesLikeCpp,
    /// C++ `sWorld->getRate(...)` subset used by represented reputation gain.
    pub(in crate::session) reputation_rates: ReputationRatesLikeCpp,
    /// C++ `sWorld->getRate(RATE_REPAIRCOST)` represented value.
    pub(in crate::session) repair_cost_rate_like_cpp: f32,
    /// C++ `sWorld->getRate(RATE_DURABILITY_LOSS_ON_DEATH)` fraction
    /// (`DurabilityLoss.OnDeath / 100`).
    pub(in crate::session) durability_loss_on_death_rate_like_cpp: f32,
    /// C++ `CONFIG_STATS_LIMITS_*` subset consumed by the represented
    /// `Player::UpdateBlockPercentage`/`UpdateDodgePercentage`/
    /// `UpdateParryPercentage`/`UpdateCritPercentage` caps.
    pub(in crate::session) stats_limits_like_cpp: wow_data::StatsLimitsLikeCpp,
    /// C++ `CONFIG_RESET_SCHEDULE_{HOUR,WEEK_DAY}` consumed by `InstanceLockMgr::GetNextResetTime`.
    pub(in crate::session) reset_schedule_like_cpp: wow_instances::ResetSchedule,

    /// C++ `CONFIG_VMAP_INDOOR_CHECK` represented switch.
    pub(in crate::session) vmap_indoor_check_like_cpp: bool,
    /// C++ `CONFIG_ENABLE_AE_LOOT` represented switch.
    pub(in crate::session) enable_ae_loot_like_cpp: bool,
    /// C++ `CONFIG_ADDON_CHANNEL` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) addon_channel_like_cpp: bool,
    /// C++ `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING` represented switch for chat validation.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_fake_message_preventing_like_cpp: bool,
    /// C++ `CONFIG_CHAT_PARTY_RAID_WARNINGS` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) party_raid_warnings_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_GM_GROUP` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) allow_gm_group_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) allow_two_side_interaction_group_like_cpp: bool,
    /// C++ `CONFIG_PARTY_LEVEL_REQ` represented gate.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) party_level_req_like_cpp: u32,
    /// C++ `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_strict_link_checking_kick_like_cpp: bool,
    /// C++ `CONFIG_CHAT_*_LEVEL_REQ` represented chat level gates.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp,
    /// C++ `CONFIG_LISTEN_RANGE_*` represented nearby-chat ranges.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp,
    /// C++ `CONFIG_CHATFLOOD_*` represented chat spam protection.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) chat_flood_config_like_cpp: ChatFloodConfigLikeCpp,
    /// C++ `CONFIG_ENABLE_MMAPS` + `DataDir` represented until map lifecycle owns real mmaps.
    pub(in crate::session) mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp,
}

impl Default for SessionWorldConfig {
    fn default() -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            characters_per_realm_like_cpp: 60,
            #[cfg(any(test, feature = "test-fixtures"))]
            declined_names_used_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            feature_system_bpay_store_enabled_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            feature_system_character_undelete_enabled_like_cpp: false,

            legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            give_player_xp_script_dispatcher_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp::default(),

            max_player_level_config_like_cpp: 80,
            max_primary_trade_skills_like_cpp:
                crate::profession::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP,
            represented_cast_unstuck_enabled_like_cpp: true,
            #[cfg(any(test, feature = "test-fixtures"))]
            exploration_xp_rate_like_cpp: 1.0,

            #[cfg(any(test, feature = "test-fixtures"))]
            min_discovered_scaled_xp_ratio_like_cpp: 0,
            loot_drop_rates: LootDropRatesLikeCpp::default(),
            reputation_rates: ReputationRatesLikeCpp::default(),
            repair_cost_rate_like_cpp: 1.0,
            durability_loss_on_death_rate_like_cpp: 0.1,
            stats_limits_like_cpp: wow_data::StatsLimitsLikeCpp::default(),
            reset_schedule_like_cpp: wow_instances::ResetSchedule::default(),

            vmap_indoor_check_like_cpp: false,
            enable_ae_loot_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            addon_channel_like_cpp: true,
            #[cfg(any(test, feature = "test-fixtures"))]
            chat_fake_message_preventing_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            party_raid_warnings_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            allow_gm_group_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            allow_two_side_interaction_group_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            party_level_req_like_cpp: 1,
            #[cfg(any(test, feature = "test-fixtures"))]
            chat_strict_link_checking_kick_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            chat_flood_config_like_cpp: ChatFloodConfigLikeCpp::default(),
            mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp::default(),
        }
    }
}
