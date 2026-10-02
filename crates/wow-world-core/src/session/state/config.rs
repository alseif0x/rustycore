// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared immutable world configuration consumed by the World session shell.

use crate::session::MMapRuntimeConfigLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
};
use crate::session_policy::{LootDropRatesLikeCpp, ReputationRatesLikeCpp};
use crate::session::creature_aggro_contracts::LegacyCreatureAggroConfigLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::CreatureClassificationHealthRatesLikeCpp;
use wow_data::StatsLimitsLikeCpp;
use wow_instances::ResetSchedule;

#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
pub type GivePlayerXpScriptDispatcherLikeCpp =
    Arc<dyn Fn(wow_script::player::GivePlayerXpContextLikeCpp, &mut u32) + Send + Sync>;

/// Immutable world configuration and rate values (C++ `sWorld` config subsets) and the script
/// dispatchers injected at composition.
pub struct SessionWorldConfig {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub characters_per_realm_like_cpp: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub declined_names_used_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub feature_system_bpay_store_enabled_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub feature_system_character_undelete_enabled_like_cpp: bool,

    pub legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub give_player_xp_script_dispatcher_like_cpp: Option<GivePlayerXpScriptDispatcherLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp,

    // Characters confirmed for this account

    // Pending async packets to process

    // ── ConnectTo flow ──────────────────────────────────────────

    // ── Logout ──────────────────────────────────────────────────────
    /// C++ `CONFIG_MAX_PLAYER_LEVEL`. `RestMgr::SetRestBonus` reads this value
    /// directly; `Player::IsMaxLevel` reads the expansion-bounded active field.
    pub max_player_level_config_like_cpp: u32,
    /// C++ `CONFIG_MAX_PRIMARY_TRADE_SKILL`, kept independent from talent
    /// `CharacterPoints` and from the two physical profession associations.
    pub max_primary_trade_skills_like_cpp: u8,
    /// C++ `CONFIG_CAST_UNSTUCK` immutable world policy injected into spell effects.
    pub represented_cast_unstuck_enabled_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub exploration_xp_rate_like_cpp: f32,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub min_discovered_scaled_xp_ratio_like_cpp: u32,
    /// C++ `sWorld->getRate(...)` subset used by represented loot generation.
    pub loot_drop_rates: LootDropRatesLikeCpp,
    /// C++ `sWorld->getRate(...)` subset used by represented reputation gain.
    pub reputation_rates: ReputationRatesLikeCpp,
    /// C++ `sWorld->getRate(RATE_REPAIRCOST)` represented value.
    pub repair_cost_rate_like_cpp: f32,
    /// C++ `sWorld->getRate(RATE_DURABILITY_LOSS_ON_DEATH)` fraction
    /// (`DurabilityLoss.OnDeath / 100`).
    pub durability_loss_on_death_rate_like_cpp: f32,
    /// C++ `CONFIG_STATS_LIMITS_*` subset consumed by the represented
    /// `Player::UpdateBlockPercentage`/`UpdateDodgePercentage`/
    /// `UpdateParryPercentage`/`UpdateCritPercentage` caps.
    pub stats_limits_like_cpp: StatsLimitsLikeCpp,
    /// C++ `CONFIG_RESET_SCHEDULE_{HOUR,WEEK_DAY}` consumed by `InstanceLockMgr::GetNextResetTime`.
    pub reset_schedule_like_cpp: ResetSchedule,

    /// C++ `CONFIG_VMAP_INDOOR_CHECK` represented switch.
    pub vmap_indoor_check_like_cpp: bool,
    /// C++ `CONFIG_ENABLE_AE_LOOT` represented switch.
    pub enable_ae_loot_like_cpp: bool,
    /// C++ `CONFIG_ADDON_CHANNEL` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub addon_channel_like_cpp: bool,
    /// C++ `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING` represented switch for chat validation.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub chat_fake_message_preventing_like_cpp: bool,
    /// C++ `CONFIG_CHAT_PARTY_RAID_WARNINGS` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub party_raid_warnings_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_GM_GROUP` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub allow_gm_group_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub allow_two_side_interaction_group_like_cpp: bool,
    /// C++ `CONFIG_PARTY_LEVEL_REQ` represented gate.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub party_level_req_like_cpp: u32,
    /// C++ `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK` represented switch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub chat_strict_link_checking_kick_like_cpp: bool,
    /// C++ `CONFIG_CHAT_*_LEVEL_REQ` represented chat level gates.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp,
    /// C++ `CONFIG_LISTEN_RANGE_*` represented nearby-chat ranges.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp,
    /// C++ `CONFIG_CHATFLOOD_*` represented chat spam protection.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub chat_flood_config_like_cpp: ChatFloodConfigLikeCpp,
    /// C++ `CONFIG_ENABLE_MMAPS` + `DataDir` represented until map lifecycle owns real mmaps.
    pub mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp,
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
            max_primary_trade_skills_like_cpp: wow_config::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP,
            represented_cast_unstuck_enabled_like_cpp: true,
            #[cfg(any(test, feature = "test-fixtures"))]
            exploration_xp_rate_like_cpp: 1.0,

            #[cfg(any(test, feature = "test-fixtures"))]
            min_discovered_scaled_xp_ratio_like_cpp: 0,
            loot_drop_rates: LootDropRatesLikeCpp::default(),
            reputation_rates: ReputationRatesLikeCpp::default(),
            repair_cost_rate_like_cpp: 1.0,
            durability_loss_on_death_rate_like_cpp: 0.1,
            stats_limits_like_cpp: StatsLimitsLikeCpp::default(),
            reset_schedule_like_cpp: ResetSchedule::default(),

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
