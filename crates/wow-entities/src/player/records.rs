// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player lifecycle and gameplay-load records and plans.

use crate::PlayerGameplayState;
use wow_constants::{Gender, PowerType};
use wow_core::{ObjectGuid, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerLifecyclePower {
    pub power: PowerType,
    pub current: i32,
    pub max: i32,
}

impl PlayerLifecyclePower {
    pub const fn new(power: PowerType, current: i32, max: i32) -> Self {
        Self {
            power,
            current,
            max,
        }
    }
}

/// Represented subset of TrinityCore `Player::Create` input.
///
/// Appearance validation, player-info starter spells/items/actions, skills, inventory item
/// creation and threat/combat subsystem startup remain deferred until their canonical systems are
/// ported. This record only carries fields currently owned by `wow-entities`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCreateLifecycleRecord {
    pub guid: ObjectGuid,
    pub name: String,
    pub race: u8,
    pub class_id: u8,
    pub gender: Gender,
    pub level: u8,
    pub xp: i32,
    pub money: u64,
    pub inventory_slot_count: u8,
    pub bank_bag_slot_count: u8,
    pub map_id: u32,
    pub position: Position,
    pub max_health: u64,
    pub health: u64,
    pub powers: Vec<PlayerLifecyclePower>,
    pub display_power: PowerType,
    pub faction_template: Option<u32>,
    pub display_id: Option<u32>,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub extra_flags: u32,
    pub create_time: Option<u64>,
    pub create_mode: Option<u8>,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub active_talent_group: Option<u8>,
}

/// Represented subset of TrinityCore `Player::LoadFromDB` base `characters` row.
///
/// Ownership/coordinate validation and subsystem loads (spells, items, quests, guild, auras,
/// action buttons, reputation, currencies, achievements) are deliberately not faked here; callers
/// should layer those bridges when the relevant systems exist.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDbLoadLifecycleRecord {
    pub guid: ObjectGuid,
    pub account_id: u32,
    pub name: String,
    pub race: u8,
    pub class_id: u8,
    pub gender: Gender,
    pub level: u8,
    pub xp: i32,
    pub money: u64,
    pub inventory_slot_count: u8,
    pub bank_bag_slot_count: u8,
    pub map_id: u32,
    pub position: Position,
    pub max_health: u64,
    pub health: u64,
    pub powers: Vec<PlayerLifecyclePower>,
    pub display_power: PowerType,
    pub faction_template: Option<u32>,
    pub display_id: Option<u32>,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub extra_flags: u32,
    pub create_time: Option<u64>,
    pub create_mode: Option<u8>,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub active_talent_group: Option<u8>,
    pub zone_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerLifecycleMetadata {
    pub account_id: Option<u32>,
    pub create_time: Option<u64>,
    pub create_mode: Option<u8>,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub active_talent_group: Option<u8>,
    pub zone_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginLifecycleStep {
    LoadFromDb,
    LoadAccountData,
    SendTutorialData,
    SendFeatureSystemStatus,
    SendTimeZoneInformation,
    SendMotd,
    SendPvpSeasonInfo,
    SendInitialPacketsBeforeAddToMap,
    PlayFirstLoginCinematic,
    AddPlayerToMap,
    RegisterObjectAccessor,
    RestoreGuildAndAuras,
    SendInitialPacketsAfterAddToMap,
    BootstrapVisibility,
    SendZoneWorldStates,
    SendCompactUnitFrameProfiles,
    ApplyLoginAuraEffects,
    SendMovementCompoundState,
    MarkOnline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLoginLifecyclePlan {
    steps: Vec<PlayerLoginLifecycleStep>,
}

impl PlayerLoginLifecyclePlan {
    pub fn trinity_handle_player_login() -> Self {
        Self {
            steps: vec![
                PlayerLoginLifecycleStep::LoadFromDb,
                PlayerLoginLifecycleStep::LoadAccountData,
                PlayerLoginLifecycleStep::SendTutorialData,
                PlayerLoginLifecycleStep::SendFeatureSystemStatus,
                PlayerLoginLifecycleStep::SendTimeZoneInformation,
                PlayerLoginLifecycleStep::SendMotd,
                PlayerLoginLifecycleStep::SendPvpSeasonInfo,
                PlayerLoginLifecycleStep::SendInitialPacketsBeforeAddToMap,
                PlayerLoginLifecycleStep::PlayFirstLoginCinematic,
                PlayerLoginLifecycleStep::AddPlayerToMap,
                PlayerLoginLifecycleStep::RegisterObjectAccessor,
                PlayerLoginLifecycleStep::RestoreGuildAndAuras,
                PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap,
                PlayerLoginLifecycleStep::BootstrapVisibility,
                PlayerLoginLifecycleStep::SendZoneWorldStates,
                PlayerLoginLifecycleStep::SendCompactUnitFrameProfiles,
                PlayerLoginLifecycleStep::ApplyLoginAuraEffects,
                PlayerLoginLifecycleStep::SendMovementCompoundState,
                PlayerLoginLifecycleStep::MarkOnline,
            ],
        }
    }

    pub fn steps(&self) -> &[PlayerLoginLifecycleStep] {
        &self.steps
    }

    pub fn position_of(&self, step: PlayerLoginLifecycleStep) -> Option<usize> {
        self.steps.iter().position(|candidate| *candidate == step)
    }

    pub fn occurs_before(
        &self,
        before: PlayerLoginLifecycleStep,
        after: PlayerLoginLifecycleStep,
    ) -> bool {
        match (self.position_of(before), self.position_of(after)) {
            (Some(before_index), Some(after_index)) => before_index < after_index,
            _ => false,
        }
    }
}

impl Default for PlayerLoginLifecyclePlan {
    fn default() -> Self {
        Self::trinity_handle_player_login()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerWorldInsertionState {
    pub added_to_map: bool,
    pub object_accessor_registered: bool,
    pub visibility_bootstrapped: bool,
    pub worldstates_sent: bool,
}

impl PlayerWorldInsertionState {
    pub fn from_completed_steps(steps: &[PlayerLoginLifecycleStep]) -> Self {
        Self {
            added_to_map: steps.contains(&PlayerLoginLifecycleStep::AddPlayerToMap),
            object_accessor_registered: steps
                .contains(&PlayerLoginLifecycleStep::RegisterObjectAccessor),
            visibility_bootstrapped: steps.contains(&PlayerLoginLifecycleStep::BootstrapVisibility),
            worldstates_sent: steps.contains(&PlayerLoginLifecycleStep::SendZoneWorldStates),
        }
    }
}

/// TrinityCore `Player::LoadFromDB` gameplay subsystem load order, represented as a bridge plan.
///
/// These steps deliberately describe ordering and owned entity-state buckets only. They are not a
/// DB loader, packet delivery pipeline, spell runtime, manager implementation, or session queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerGameplayLoadStep {
    LoadAchievementsAndQuestCriteria,
    LoadHomeBind,
    InitializeSkillFields,
    LoadGroup,
    LoadCurrency,
    LoadInstanceLocks,
    LoadBattlegroundData,
    LoadTaxiMaskAndDestinations,
    InitTaxiNodesForLevel,
    InitStatsForLevel,
    ApplyRestBonus,
    LoadSkills,
    UpdateSkillsForLevel,
    LoadTalents,
    LoadSpells,
    LoadCollectionsGlyphsAndAuras,
    LoadQuestStatus,
    LoadQuestObjectives,
    LoadRewardedQuests,
    LoadDailyWeeklyMonthlySeasonalQuests,
    LoadRandomBattleground,
    LearnDefaultSkills,
    LearnCustomSpells,
    LoadTraits,
    LoadReputation,
    LoadInventory,
    LoadVoidStorage,
    LoadActionButtons,
    LoadMail,
    LoadSocial,
    FinalRelocate,
    LoadSpellCooldownsAndCharges,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerGameplayLoadPlan {
    steps: Vec<PlayerGameplayLoadStep>,
}

impl PlayerGameplayLoadPlan {
    pub fn trinity_load_from_db() -> Self {
        Self {
            steps: vec![
                PlayerGameplayLoadStep::LoadAchievementsAndQuestCriteria,
                PlayerGameplayLoadStep::LoadHomeBind,
                PlayerGameplayLoadStep::InitializeSkillFields,
                PlayerGameplayLoadStep::LoadGroup,
                PlayerGameplayLoadStep::LoadCurrency,
                PlayerGameplayLoadStep::LoadInstanceLocks,
                PlayerGameplayLoadStep::LoadBattlegroundData,
                PlayerGameplayLoadStep::LoadTaxiMaskAndDestinations,
                PlayerGameplayLoadStep::InitTaxiNodesForLevel,
                PlayerGameplayLoadStep::InitStatsForLevel,
                PlayerGameplayLoadStep::ApplyRestBonus,
                PlayerGameplayLoadStep::LoadSkills,
                PlayerGameplayLoadStep::UpdateSkillsForLevel,
                PlayerGameplayLoadStep::LoadTalents,
                PlayerGameplayLoadStep::LoadSpells,
                PlayerGameplayLoadStep::LoadCollectionsGlyphsAndAuras,
                PlayerGameplayLoadStep::LoadQuestStatus,
                PlayerGameplayLoadStep::LoadQuestObjectives,
                PlayerGameplayLoadStep::LoadRewardedQuests,
                PlayerGameplayLoadStep::LoadDailyWeeklyMonthlySeasonalQuests,
                PlayerGameplayLoadStep::LoadRandomBattleground,
                PlayerGameplayLoadStep::LearnDefaultSkills,
                PlayerGameplayLoadStep::LearnCustomSpells,
                PlayerGameplayLoadStep::LoadTraits,
                PlayerGameplayLoadStep::LoadReputation,
                PlayerGameplayLoadStep::LoadInventory,
                PlayerGameplayLoadStep::LoadVoidStorage,
                PlayerGameplayLoadStep::LoadActionButtons,
                PlayerGameplayLoadStep::LoadMail,
                PlayerGameplayLoadStep::LoadSocial,
                PlayerGameplayLoadStep::FinalRelocate,
                PlayerGameplayLoadStep::LoadSpellCooldownsAndCharges,
            ],
        }
    }

    pub fn steps(&self) -> &[PlayerGameplayLoadStep] {
        &self.steps
    }

    pub fn position_of(&self, step: PlayerGameplayLoadStep) -> Option<usize> {
        self.steps.iter().position(|candidate| *candidate == step)
    }

    pub fn occurs_before(
        &self,
        before: PlayerGameplayLoadStep,
        after: PlayerGameplayLoadStep,
    ) -> bool {
        match (self.position_of(before), self.position_of(after)) {
            (Some(before_index), Some(after_index)) => before_index < after_index,
            _ => false,
        }
    }
}

impl Default for PlayerGameplayLoadPlan {
    fn default() -> Self {
        Self::trinity_load_from_db()
    }
}

/// Canonical `wow-entities` bridge snapshot for gameplay data loaded by TrinityCore
/// `Player::LoadFromDB` after the base `characters` row.
///
/// This state is intentionally independent from update masks. Runtime managers, DB loaders,
/// packet serializers/delivery, spell/aura execution, social/mail managers and session queues
/// remain separate layers and should consume/produce these buckets explicitly.
#[derive(Debug, PartialEq, Default)]
pub struct PlayerGameplayLoadRecord {
    pub state: PlayerGameplayState,
}
