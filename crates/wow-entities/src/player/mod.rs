// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player entity.
//!
//! Issue #226 split the former 9,268-line `player.rs` into private state-family
//! modules. `Player` remains one type with one semantic owner: no storage
//! location, writer, mirror or runtime clock changed.

mod collections;
mod identity;
mod items;
mod location;
mod progression;
mod social;
mod spellbook;
mod visibility;
mod vitals;

use std::collections::{HashMap, HashSet};

use crate::PlayerGameplayState;
use bitflags::bitflags;
use wow_constants::{
    BagFamilyMask, EnchantmentSlot, Gender, InventoryResult, InventoryType, ItemBondingType,
    ItemClass, ItemEnchantmentType, ItemFieldFlags, ItemFieldFlags2, ItemModType, ItemModifier,
    ItemSubClassContainer, ItemSubClassQuiver, ItemSubClassWeapon, ItemSubclassProfession,
    ItemUpdateState, PowerType, Stats, TypeId, TypeMask, WeaponAttackType, spell::SpellSchools,
};
use wow_core::{ObjectGuid, Position};

use crate::{
    BASE_MAXDAMAGE, BASE_MINDAMAGE, Bag, EQUIPMENT_SLOT_BACK, EQUIPMENT_SLOT_BODY,
    EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_END, EQUIPMENT_SLOT_FEET, EQUIPMENT_SLOT_FINGER1,
    EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD, EQUIPMENT_SLOT_LEGS,
    EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_NECK, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_SHOULDERS,
    EQUIPMENT_SLOT_TABARD, EQUIPMENT_SLOT_TRINKET1, EQUIPMENT_SLOT_TRINKET2, EQUIPMENT_SLOT_WAIST,
    EQUIPMENT_SLOT_WRISTS, INVENTORY_SLOT_BAG_0, Item, ItemStorageTemplate, MAX_BAG_SIZE,
    MAX_ENCHANTMENT_SLOT, MAX_POWERS, MAX_POWERS_PER_CLASS, NULL_SLOT, ObjectDataUpdate,
    PROFESSION_SLOT_COOKING_GEAR1, PROFESSION_SLOT_COOKING_TOOL, PROFESSION_SLOT_END,
    PROFESSION_SLOT_FISHING_TOOL, PROFESSION_SLOT_MAX_COUNT, PROFESSION_SLOT_PROFESSION1_GEAR1,
    PROFESSION_SLOT_PROFESSION1_GEAR2, PROFESSION_SLOT_PROFESSION1_TOOL,
    PROFESSION_SLOT_PROFESSION2_GEAR1, PROFESSION_SLOT_PROFESSION2_GEAR2, PROFESSION_SLOT_START,
    Unit, UnitDataUpdate, UpdateMask, item_can_go_into_bag,
    update_fields::{
        ACTIVE_PLAYER_DATA_BITS, PLAYER_DATA_BITS, TYPEID_ACTIVE_PLAYER, TYPEID_PLAYER,
    },
};

pub const PLAYER_EXTRA_GM_ON: u32 = 0x0001;
pub const REPUTATION_FLAG_AT_WAR_LIKE_CPP: u32 = 0x0002;

pub const MAX_MONEY_AMOUNT: u64 = 99_999_999_999;
pub const TEAM_OTHER: u8 = 0;
pub const TEAM_HORDE_ID: u32 = 67;
pub const TEAM_ALLIANCE_ID: u32 = 469;
pub const CLASS_WARRIOR: u8 = 1;
pub const CLASS_PALADIN: u8 = 2;
pub const CLASS_HUNTER: u8 = 3;
pub const CLASS_SHAMAN: u8 = 7;
pub const SKILL_PLATE_MAIL: u32 = 293;
pub const SKILL_MAIL: u32 = 413;
pub const NULL_BAG: u8 = 0;

pub trait PlayerPowerIndexResolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize>;
}

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

#[derive(Debug, Clone, PartialEq)]
struct PlayerLifecycleBase {
    guid: ObjectGuid,
    name: String,
    race: u8,
    class_id: u8,
    gender: Gender,
    level: u8,
    xp: i32,
    money: u64,
    inventory_slot_count: u8,
    bank_bag_slot_count: u8,
    map_id: u32,
    position: Position,
    max_health: u64,
    health: u64,
    powers: Vec<PlayerLifecyclePower>,
    display_power: PowerType,
    faction_template: Option<u32>,
    display_id: Option<u32>,
    player_flags: u32,
    player_flags_ex: u32,
    extra_flags: u32,
    metadata: PlayerLifecycleMetadata,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerQuestGameplayState {
    pub statuses: Vec<PlayerQuestStatusRecord>,
    pub objective_progress: Vec<PlayerQuestObjectiveProgress>,
    pub rewarded_quest_ids: Vec<u32>,
    pub daily_quest_ids: Vec<u32>,
    pub weekly_quest_ids: Vec<u32>,
    pub monthly_quest_ids: Vec<u32>,
    pub seasonal_quest_ids: Vec<u32>,
    pub df_quest_ids: Vec<u32>,
    pub pending_share: Option<(ObjectGuid, u32)>,
    pub objective_counts_by_quest: Vec<(u32, Vec<i32>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestStatusRecord {
    pub quest_id: u32,
    pub status: u8,
    pub explored: bool,
    pub timer_expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestObjectiveProgress {
    pub quest_id: u32,
    pub objective_id: u32,
    pub counter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSkillRecord {
    pub skill_line_id: u32,
    pub current_value: u16,
    pub max_value: u16,
    pub step: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpellLoadState {
    Unchanged,
    New,
    Changed,
    Removed,
    Temporary,
}

impl Default for PlayerSpellLoadState {
    fn default() -> Self {
        Self::Unchanged
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerKnownSpellRecord {
    pub spell_id: u32,
    pub state: PlayerSpellLoadState,
    pub active: bool,
    pub favorite: bool,
    pub dependent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTalentRecord {
    pub talent_id: u32,
    pub spell_id: u32,
    pub rank: u8,
    pub talent_group: u8,
    pub specialization_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerActionButtonRecord {
    pub button: u8,
    pub action_id: u32,
    pub action_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerTaxiState {
    pub known_node_mask: Vec<u8>,
    pub known_node_mask_text: Option<String>,
    pub source_node_id: Option<u32>,
    pub destination_node_id: Option<u32>,
    pub destinations: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerSocialState {
    pub friend_guids: Vec<ObjectGuid>,
    pub ignore_guids: Vec<ObjectGuid>,
    pub auto_reply_msg_like_cpp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCustomizationChoice {
    pub option_id: u32,
    pub choice_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerTransportState {
    pub guid: ObjectGuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub seat: i8,
    pub time: u32,
    pub prev_time: Option<u32>,
    pub vehicle_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMailRecord {
    pub mail_id: u32,
    pub sender: ObjectGuid,
    pub receiver: ObjectGuid,
    pub template_id: Option<u32>,
    pub deliver_time: u64,
    pub expire_time: u64,
    pub checked_flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerGroupState {
    pub group_guid: ObjectGuid,
    pub leader_guid: ObjectGuid,
    pub role_mask: u8,
    pub subgroup: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerGuildState {
    pub guild_id: Option<u64>,
    pub invited_guild_id: Option<u64>,
    pub rank_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerBattlegroundState {
    pub queues: Vec<PlayerBattlegroundQueueRecord>,
    pub current_bg_instance_id: Option<u32>,
    pub current_bg_team: Option<u32>,
    pub random: PlayerRandomBattlegroundState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBattlegroundQueueRecord {
    pub queue_id: u32,
    pub bracket_id: u8,
    pub joined_at: u64,
    pub team_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerRandomBattlegroundState {
    pub reward_claimed_today: bool,
    pub last_reward_time: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerReputationRecord {
    pub faction_id: u32,
    pub standing: i32,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAchievementRecord {
    pub achievement_id: u32,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAchievementCriteriaRecord {
    pub criteria_id: u32,
    pub counter: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCurrencyRecord {
    pub currency_id: u32,
    pub count: u32,
    pub weekly_count: u32,
    pub tracked_quantity: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellCooldownRecord {
    pub spell_id: u32,
    pub item_id: Option<u32>,
    pub category_id: Option<u32>,
    pub cooldown_expires_at: u64,
    pub category_cooldown_expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellChargeRecord {
    pub category_id: u32,
    pub consumed_charges: u8,
    pub recharge_started_at: Option<u64>,
    pub recharge_ends_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerRestState {
    pub rest_xp: u32,
    pub rest_bonus: f32,
    pub rest_honor_bonus: f32,
    pub rest_state: u8,
    pub logout_time: Option<u64>,
    pub logout_was_resting: bool,
    pub is_resting_now: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDuelStateLikeCpp {
    Challenged,
    Countdown,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDuelInfoLikeCpp {
    pub opponent: ObjectGuid,
    pub state: PlayerDuelStateLikeCpp,
}

/// Canonical `wow-entities` bridge snapshot for gameplay data loaded by TrinityCore
/// `Player::LoadFromDB` after the base `characters` row.
///
/// This state is intentionally independent from update masks. Runtime managers, DB loaders,
/// packet serializers/delivery, spell/aura execution, social/mail managers and session queues
/// remain separate layers and should consume/produce these buckets explicitly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerGameplayLoadRecord {
    pub state: PlayerGameplayState,
}

fn representable_power_types() -> [PowerType; MAX_POWERS] {
    [
        PowerType::Mana,
        PowerType::Rage,
        PowerType::Focus,
        PowerType::Energy,
        PowerType::Happiness,
        PowerType::Runes,
        PowerType::RunicPower,
        PowerType::SoulShards,
        PowerType::LunarPower,
        PowerType::HolyPower,
        PowerType::AlternatePower,
        PowerType::Maelstrom,
        PowerType::Chi,
        PowerType::Insanity,
        PowerType::ComboPoints,
        PowerType::DemonicFury,
        PowerType::ArcaneCharges,
        PowerType::Fury,
        PowerType::Pain,
        PowerType::Essence,
        PowerType::RuneBlood,
        PowerType::RuneFrost,
        PowerType::RuneUnholy,
        PowerType::AlternateQuest,
        PowerType::AlternateEncounter,
        PowerType::AlternateMount,
    ]
}

pub const PLAYER_DATA_PARENT_BIT: usize = 0;
pub const PLAYER_DATA_LOOT_TARGET_GUID_BIT: usize = 6;
pub const PLAYER_DATA_FLAGS_BIT: usize = 7;
pub const PLAYER_DATA_FLAGS_EX_BIT: usize = 8;
pub const PLAYER_DATA_PARTY_TYPE_PARENT_BIT: usize = 32;
pub const PLAYER_DATA_PARTY_TYPE_FIRST_BIT: usize = 33;
pub const PLAYER_DATA_NUM_BANK_SLOTS_BIT: usize = 12;
pub const PLAYER_DATA_NATIVE_SEX_BIT: usize = 13;
pub const PLAYER_DATA_INEBRIATION_BIT: usize = 14;
pub const PLAYER_DATA_PLAYER_TITLE_BIT: usize = 21;
pub const PLAYER_DATA_CURRENT_SPEC_ID_BIT: usize = 24;
pub const PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT: usize = 26;
pub const PLAYER_DATA_HONOR_LEVEL_BIT: usize = 27;
pub const PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT: usize = 61;
pub const PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT: usize = 62;

pub const ACTIVE_PLAYER_DATA_PARENT_BIT: usize = 0;
pub const ACTIVE_PLAYER_DATA_FARSIGHT_OBJECT_BIT: usize = 26;
pub const ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT: usize = 27;
pub const ACTIVE_PLAYER_DATA_COINAGE_BIT: usize = 28;
pub const ACTIVE_PLAYER_DATA_XP_BIT: usize = 29;
pub const ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT: usize = 30;
pub const ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT: usize = 70;
pub const ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT: usize = 94;
pub const ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT: usize = 33;
pub const ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT: usize = 7;
pub const ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT: usize = 8;
pub const ACTIVE_PLAYER_DATA_TOYS_BIT: usize = 9;
pub const ACTIVE_PLAYER_DATA_TRANSMOG_BIT: usize = 10;
pub const ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT: usize = 11;
pub const ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT: usize = 102;
pub const ACTIVE_PLAYER_DATA_HONOR_BIT: usize = 109;
pub const ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT: usize = 110;
pub const ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT: usize = 104;
pub const ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT: usize = 124;
pub const ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT: usize = 125;
pub const ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT: usize = 298;
pub const ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT: usize = 299;
pub const ACTIVE_PLAYER_DATA_REST_INFO_PARENT_BIT: usize = 539;
pub const ACTIVE_PLAYER_DATA_REST_INFO_FIRST_BIT: usize = 540;
pub const ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT: usize = 549;
pub const ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT: usize = 550;
pub const ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT: usize = 562;
pub const ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT: usize = 628;
pub const ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT: usize = 629;
pub const ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT: usize = 636;
pub const ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT: usize = 637;
pub const ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT: usize = 92;
pub const QUESTS_COMPLETED_BITS_SIZE: usize = 875;
pub const QUESTS_COMPLETED_BITS_PER_BLOCK: u32 = 64;
pub const PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP: usize = 240;

/// C++ `Player::LoadFromDB` `exploredZones` parser.
///
/// Trinity stores each 64-bit block as two decimal 32-bit words:
/// low half first, then high half. Loading uses `StringTo<uint64>` and
/// shifts by `32 * (token_index % 2)` before OR-ing into the destination
/// block, so malformed tokens become zero and extra tokens are ignored.
pub fn parse_explored_zones_db_string_like_cpp(
    input: &str,
) -> [u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP] {
    let mut blocks = [0u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP];
    for (token_index, token) in input.split_whitespace().enumerate() {
        let block_index = token_index / 2;
        if block_index >= PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP {
            break;
        }

        let value = token.parse::<u64>().unwrap_or(0);
        blocks[block_index] |= value << (32 * (token_index % 2));
    }
    blocks
}

/// C++ `Player::SaveToDB` `exploredZones` serializer.
///
/// The legacy column is a space-separated string with a trailing space after
/// every low/high 32-bit word pair.
pub fn explored_zones_db_string_from_blocks_like_cpp(
    blocks: &[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
) -> String {
    use std::fmt::Write;

    let mut out = String::with_capacity(PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP * 4);
    for block in blocks {
        let _ = write!(
            &mut out,
            "{} {} ",
            (*block & 0xFFFF_FFFF) as u32,
            ((*block >> 32) & 0xFFFF_FFFF) as u32
        );
    }
    out
}

pub const PLAYER_MAX_HONOR_LEVEL_LIKE_CPP: i32 = 500;
pub const PLAYER_LEVEL_MIN_HONOR_LIKE_CPP: u8 = 10;
pub const PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP: i32 = 8_800;
pub const PLAYER_SLOT_END: usize = 141;
pub const INVENTORY_DEFAULT_SIZE: u8 = 16;
pub const INVENTORY_SLOT_BAG_START: u8 = 30;
pub const INVENTORY_SLOT_BAG_END: u8 = 34;
pub const REAGENT_BAG_SLOT_START: u8 = 34;
pub const REAGENT_BAG_SLOT_END: u8 = 35;
pub const INVENTORY_SLOT_ITEM_START: u8 = 35;
pub const INVENTORY_SLOT_ITEM_END: u8 = 59;
pub const BANK_SLOT_ITEM_START: u8 = 59;
pub const BANK_SLOT_ITEM_END: u8 = 87;
pub const BANK_SLOT_BAG_START: u8 = 87;
pub const BANK_SLOT_BAG_END: u8 = 94;
pub const BUYBACK_SLOT_START: u8 = 94;
pub const BUYBACK_SLOT_END: u8 = 106;
pub const BUYBACK_SLOT_COUNT: usize = (BUYBACK_SLOT_END - BUYBACK_SLOT_START) as usize;
pub const KEYRING_SLOT_START: u8 = 106;
pub const KEYRING_SLOT_END: u8 = 138;
pub const CHILD_EQUIPMENT_SLOT_START: u8 = 138;
pub const CHILD_EQUIPMENT_SLOT_END: u8 = 141;
pub const ITEM_LIMIT_CATEGORY_MODE_HAVE: u8 = 0;
pub const ITEM_LIMIT_CATEGORY_MODE_EQUIP: u8 = 1;

const ENCHANTMENT_DURATION_SLOTS: [EnchantmentSlot; MAX_ENCHANTMENT_SLOT] = [
    EnchantmentSlot::EnhancementPermanent,
    EnchantmentSlot::EnhancementTemporary,
    EnchantmentSlot::EnhancementSocket,
    EnchantmentSlot::EnhancementSocket2,
    EnchantmentSlot::EnhancementSocket3,
    EnchantmentSlot::EnhancementSocketBonus,
    EnchantmentSlot::EnhancementSocketPrismatic,
    EnchantmentSlot::EnhancementUse,
    EnchantmentSlot::Property0,
    EnchantmentSlot::Property1,
    EnchantmentSlot::Property2,
    EnchantmentSlot::Property3,
    EnchantmentSlot::Property4,
];

pub const fn make_item_pos(bag: u8, slot: u8) -> u16 {
    u16::from_be_bytes([bag, slot])
}

pub fn is_inventory_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && slot == NULL_SLOT {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0
        && (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END).contains(&slot)
    {
        return true;
    }
    if (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&bag) {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (KEYRING_SLOT_START..KEYRING_SLOT_END).contains(&slot) {
        return true;
    }
    if is_child_equipment_pos(bag, slot) {
        return true;
    }
    false
}

pub fn is_inventory_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_inventory_pos(bag, slot)
}

pub fn is_equipment_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && slot < EQUIPMENT_SLOT_END {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (PROFESSION_SLOT_START..PROFESSION_SLOT_END).contains(&slot) {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0
        && (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
    {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
    {
        return true;
    }
    false
}

pub fn is_equipment_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_equipment_pos(bag, slot)
}

pub fn is_bank_pos(bag: u8, slot: u8) -> bool {
    if bag == INVENTORY_SLOT_BAG_0 && (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END).contains(&slot) {
        return true;
    }
    if bag == INVENTORY_SLOT_BAG_0 && (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot) {
        return true;
    }
    if (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&bag) {
        return true;
    }
    false
}

pub fn is_bank_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_bank_pos(bag, slot)
}

pub fn is_bag_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    bag == INVENTORY_SLOT_BAG_0 && is_bag_storage_slot(slot)
}

pub fn is_child_equipment_pos(bag: u8, slot: u8) -> bool {
    bag == INVENTORY_SLOT_BAG_0
        && (CHILD_EQUIPMENT_SLOT_START..CHILD_EQUIPMENT_SLOT_END).contains(&slot)
}

pub fn is_child_equipment_packed_pos(pos: u16) -> bool {
    let [bag, slot] = pos.to_be_bytes();
    is_child_equipment_pos(bag, slot)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPosCount {
    pub pos: u16,
    pub count: u32,
}

impl ItemPosCount {
    pub const fn new(pos: u16, count: u32) -> Self {
        Self { pos, count }
    }

    pub fn is_contained_in(&self, positions: &[ItemPosCount]) -> bool {
        positions.iter().any(|position| position.pos == self.pos)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemSlotRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: &'a Item,
}

impl<'a> ItemSlotRef<'a> {
    pub const fn new(bag: u8, slot: u8, item: &'a Item) -> Self {
        Self { bag, slot, item }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemStorageRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: &'a Item,
    pub template: Option<&'a ItemStorageTemplate>,
}

impl<'a> ItemStorageRef<'a> {
    pub const fn new(
        bag: u8,
        slot: u8,
        item: &'a Item,
        template: Option<&'a ItemStorageTemplate>,
    ) -> Self {
        Self {
            bag,
            slot,
            item,
            template,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BagTemplateRef<'a> {
    pub bag: u8,
    pub template: &'a ItemStorageTemplate,
}

impl<'a> BagTemplateRef<'a> {
    pub const fn new(bag: u8, template: &'a ItemStorageTemplate) -> Self {
        Self { bag, template }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanStoreItemArgs<'a> {
    pub bag: u8,
    pub slot: u8,
    pub entry: u32,
    pub count: u32,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub source_item: Option<&'a Item>,
    pub source_is_not_empty_bag: bool,
    pub source_bop_trade_allowed_for_player: bool,
    pub swap: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub slot_items: &'a [ItemSlotRef<'a>],
    pub stored_items: &'a [ItemStorageRef<'a>],
    pub bag_templates: &'a [BagTemplateRef<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct CanBankItemArgs<'a> {
    pub bag: u8,
    pub slot: u8,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub source_item: Option<&'a Item>,
    pub source_is_not_empty_bag: bool,
    pub source_is_bag: bool,
    pub source_is_currency_token: bool,
    pub source_bop_trade_allowed_for_player: bool,
    pub swap: bool,
    pub can_use_result: InventoryResult,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub slot_items: &'a [ItemSlotRef<'a>],
    pub stored_items: &'a [ItemStorageRef<'a>],
    pub bag_templates: &'a [BagTemplateRef<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct FindEquipSlotArgs<'a> {
    pub proto: &'a ItemStorageTemplate,
    pub slot: u8,
    pub swap: bool,
    pub can_dual_wield: bool,
    pub can_titan_grip: bool,
    pub is_two_hand_used: bool,
    pub has_required_profession_skill: bool,
    pub profession_slot: Option<u8>,
    pub equipped_items: &'a [ItemSlotRef<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct CanEquipItemArgs<'a> {
    pub slot: u8,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub source_item: Option<&'a Item>,
    pub source_bop_trade_allowed_for_player: bool,
    pub swap: bool,
    pub not_loading: bool,
    pub is_stunned: bool,
    pub is_charmed: bool,
    pub is_in_combat: bool,
    pub is_in_progress_arena: bool,
    pub weapon_change_timer_active: bool,
    pub current_generic_spell_allows_equip: Option<bool>,
    pub current_channeled_spell_allows_equip: Option<bool>,
    pub heirloom_required_level_failed: bool,
    pub can_use_result: InventoryResult,
    pub can_equip_unique_result: InventoryResult,
    pub can_dual_wield: bool,
    pub can_titan_grip: bool,
    pub is_two_hand_used: bool,
    pub proto_always_allow_dual_wield: bool,
    pub has_required_profession_skill: bool,
    pub profession_slot: Option<u8>,
    pub offhand_can_unequip_result: InventoryResult,
    pub offhand_can_store_result: InventoryResult,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub equipped_items: &'a [ItemSlotRef<'a>],
    pub stored_items: &'a [ItemStorageRef<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanEquipItemOutcome {
    pub result: InventoryResult,
    pub dest: u16,
    pub unique_ignore_slot: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipItemObjectOutcome {
    Equipped,
    Merged,
}

#[derive(Debug, Clone, Copy)]
pub struct CanUnequipItemArgs<'a> {
    pub pos: u16,
    pub source_item: Option<&'a Item>,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub swap: bool,
    pub source_is_not_empty_bag: bool,
    pub is_charmed: bool,
    pub is_in_combat: bool,
    pub is_in_progress_arena: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CanUseItemTemplateArgs<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub skip_required_level_check: bool,
    pub player_level: u8,
    pub team: u32,
    pub allowable_class_matches: bool,
    pub allowable_race_matches: bool,
    pub internal_item: bool,
    pub faction_horde: bool,
    pub faction_alliance: bool,
    pub required_skill: u32,
    pub required_skill_rank: u32,
    pub required_skill_value: u32,
    pub required_spell: u32,
    pub has_required_spell: bool,
    pub base_required_level: u8,
    pub holiday_id: u32,
    pub holiday_active: bool,
    pub required_reputation_faction: u32,
    pub required_reputation_rank: u32,
    pub player_reputation_rank: u32,
    pub effect0_spell_id: Option<u32>,
    pub effect1_spell_id: Option<u32>,
    pub has_effect1_spell: bool,
    pub artifact_specialization: Option<u32>,
    pub primary_specialization: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CanUseItemArgs<'a> {
    pub source_item: Option<&'a Item>,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub not_loading: bool,
    pub is_alive: bool,
    pub player_level: u8,
    pub item_required_level: u8,
    pub source_bop_trade_allowed_for_player: bool,
    pub template_args: CanUseItemTemplateArgs<'a>,
    pub item_skill: u32,
    pub item_skill_value: u32,
    pub has_item_skill: bool,
    pub player_class: u8,
    pub proto_is_heirloom: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EquippedGemRef {
    pub slot: u8,
    pub entry: u32,
    pub limit_category: u32,
}

impl EquippedGemRef {
    pub const fn new(slot: u8, entry: u32, limit_category: u32) -> Self {
        Self {
            slot,
            entry,
            limit_category,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanEquipUniqueItemTemplateArgs<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub except_slot: u8,
    pub limit_count: u32,
    pub unique_equippable: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub equipped_items: &'a [ItemStorageRef<'a>],
    pub equipped_gems: &'a [EquippedGemRef],
}

#[derive(Debug, Clone, Copy)]
pub struct SocketedGemUniqueRef<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub unique_equippable: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub source_limit_category_count: u32,
}

impl<'a> SocketedGemUniqueRef<'a> {
    pub const fn new(
        proto: Option<&'a ItemStorageTemplate>,
        unique_equippable: bool,
        limit_category: Option<&'a ItemLimitCategoryTemplate>,
        source_limit_category_count: u32,
    ) -> Self {
        Self {
            proto,
            unique_equippable,
            limit_category,
            source_limit_category_count,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanEquipUniqueItemArgs<'a> {
    pub source_item: Option<&'a Item>,
    pub proto: Option<&'a ItemStorageTemplate>,
    pub except_slot: u8,
    pub limit_count: u32,
    pub unique_equippable: bool,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub equipped_items: &'a [ItemStorageRef<'a>],
    pub equipped_gems: &'a [EquippedGemRef],
    pub socketed_gems: &'a [SocketedGemUniqueRef<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanStoreItemOutcome {
    pub result: InventoryResult,
    pub no_space_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemLimitCategoryTemplate {
    pub id: u32,
    pub quantity: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct CanTakeMoreSimilarItemsArgs<'a> {
    pub proto: Option<&'a ItemStorageTemplate>,
    pub count: u32,
    pub source_item: Option<&'a Item>,
    pub current_item_count: u32,
    pub limit_category: Option<&'a ItemLimitCategoryTemplate>,
    pub current_limit_category_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanTakeMoreSimilarItemsOutcome {
    pub result: InventoryResult,
    pub no_space_count: Option<u32>,
    pub offending_item_id: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct DestroyItemCountItemRef<'a> {
    pub bag: u8,
    pub slot: u8,
    pub item: &'a Item,
    pub can_unequip_result: InventoryResult,
}

impl<'a> DestroyItemCountItemRef<'a> {
    pub const fn new(bag: u8, slot: u8, item: &'a Item) -> Self {
        Self {
            bag,
            slot,
            item,
            can_unequip_result: InventoryResult::Ok,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyItemCountAction {
    pub bag: u8,
    pub slot: u8,
    pub removed_count: u32,
    pub remaining_count: u32,
    pub destroy_stack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestroyItemCountPlan {
    pub removed_count: u32,
    pub actions: Vec<DestroyItemCountAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyFilteredItemRef {
    pub bag: u8,
    pub slot: u8,
    pub should_destroy: bool,
}

impl DestroyFilteredItemRef {
    pub const fn new(bag: u8, slot: u8, should_destroy: bool) -> Self {
        Self {
            bag,
            slot,
            should_destroy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyFilteredItemAction {
    pub bag: u8,
    pub slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemPreflightItem {
    pub is_bag: bool,
    pub is_empty_bag: bool,
    pub is_child: bool,
    pub parent_pos: Option<u16>,
    pub can_unequip_result: InventoryResult,
}

impl SwapItemPreflightItem {
    pub const fn regular() -> Self {
        Self {
            is_bag: false,
            is_empty_bag: false,
            is_child: false,
            parent_pos: None,
            can_unequip_result: InventoryResult::Ok,
        }
    }

    pub const fn bag(is_empty_bag: bool) -> Self {
        Self {
            is_bag: true,
            is_empty_bag,
            is_child: false,
            parent_pos: None,
            can_unequip_result: InventoryResult::Ok,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemPreflightResult {
    NoSource,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
    Error(InventoryResult),
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemPreflightPlan {
    pub result: SwapItemPreflightResult,
    pub src_unequip_swap: Option<bool>,
    pub dst_unequip_swap: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemEmptyDestinationResult {
    OccupiedDestination,
    InvalidDestinationNoop,
    Error(InventoryResult),
    MoveToInventory {
        quest_added_from_bank: bool,
    },
    MoveToBank {
        quest_removed: bool,
    },
    Equip {
        dest: u16,
        auto_unequip_offhand: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemEmptyDestinationPlan {
    pub result: SwapItemEmptyDestinationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemMergeFillResult {
    ContinueToRealSwap,
    InvalidDestinationNoop,
    MoveMergedStackToInventory,
    MoveMergedStackToBank,
    EquipMergedStack {
        dest: u16,
        auto_unequip_offhand: bool,
    },
    PartialFill {
        source_remaining_count: u32,
        destination_count: u32,
        send_updates: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemMergeFillPlan {
    pub result: SwapItemMergeFillResult,
    pub send_refund_info: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemRealSwapValidationSubject {
    Source,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemRealSwapTarget {
    Inventory,
    Bank,
    Equip { dest: u16 },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemRealSwapValidationResult {
    Error {
        result: InventoryResult,
        subject: SwapItemRealSwapValidationSubject,
    },
    Continue {
        source_target: SwapItemRealSwapTarget,
        destination_target: SwapItemRealSwapTarget,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemRealSwapValidationPlan {
    pub result: SwapItemRealSwapValidationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapBagItemRef {
    pub slot: u8,
    pub can_go_into_empty_bag: bool,
}

impl SwapBagItemRef {
    pub const fn new(slot: u8, can_go_into_empty_bag: bool) -> Self {
        Self {
            slot,
            can_go_into_empty_bag,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapBagRef<'a> {
    pub is_empty: bool,
    pub bag_size: u8,
    pub items: &'a [SwapBagItemRef],
}

impl<'a> SwapBagRef<'a> {
    pub const fn new(is_empty: bool, bag_size: u8, items: &'a [SwapBagItemRef]) -> Self {
        Self {
            is_empty,
            bag_size,
            items,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapBagItemMove {
    pub from_slot: u8,
    pub to_slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapItemBagExchangeResult {
    Continue,
    Error(InventoryResult),
    Exchange {
        empty_bag_is_source: bool,
        moves: Vec<SwapBagItemMove>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapItemBagExchangePlan {
    pub result: SwapItemBagExchangeResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapItemRealSwapExecutionPlan {
    pub remove_destination_update: bool,
    pub remove_source_update: bool,
    pub source_target: SwapItemRealSwapTarget,
    pub destination_target: SwapItemRealSwapTarget,
    pub apply_item_dependent_auras: bool,
    pub release_loot: bool,
    pub auto_unequip_offhand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemErrorItemOrder {
    SourceDestination,
    SourceOnly,
    DestinationSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapItemMissingPhase {
    EmptyDestination,
    MergeFill,
    RealSwapValidation,
    BagExchange,
    RealSwapExecution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapItemOrchestrationResult {
    NoSource,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
    Error {
        result: InventoryResult,
        item_order: SwapItemErrorItemOrder,
    },
    EmptyDestination(SwapItemEmptyDestinationPlan),
    MergeFill(SwapItemMergeFillPlan),
    RealSwap {
        bag_exchange: SwapItemBagExchangePlan,
        execution: SwapItemRealSwapExecutionPlan,
    },
    InconsistentRealSwapTargets {
        validation_source_target: SwapItemRealSwapTarget,
        validation_destination_target: SwapItemRealSwapTarget,
        execution_source_target: SwapItemRealSwapTarget,
        execution_destination_target: SwapItemRealSwapTarget,
    },
    MissingPhase(SwapItemMissingPhase),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapItemOrchestrationPlan {
    pub result: SwapItemOrchestrationResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulboundTradeableItemRef {
    pub guid: ObjectGuid,
    pub owner_guid: ObjectGuid,
    pub trade_expired: bool,
}

impl SoulboundTradeableItemRef {
    pub const fn new(guid: ObjectGuid, owner_guid: ObjectGuid, trade_expired: bool) -> Self {
        Self {
            guid,
            owner_guid,
            trade_expired,
        }
    }

    pub fn from_item(item: &Item, owner_total_played_time: u32) -> Self {
        Self {
            guid: item.object().guid(),
            owner_guid: item.owner_guid(),
            trade_expired: item.is_soulbound_trade_expired(owner_total_played_time),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerItemTimeUpdate {
    pub item_guid: ObjectGuid,
    pub expiration: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDurationRef {
    pub guid: ObjectGuid,
    pub expiration: u32,
    pub real_duration: bool,
}

impl ItemDurationRef {
    pub const fn new(guid: ObjectGuid, expiration: u32, real_duration: bool) -> Self {
        Self {
            guid,
            expiration,
            real_duration,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateItemDurationAction {
    MissingItem {
        item_guid: ObjectGuid,
    },
    UpdateExpiration {
        item_guid: ObjectGuid,
        expiration: u32,
    },
    Expire {
        item_guid: ObjectGuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerEnchantDuration {
    pub item_guid: ObjectGuid,
    pub slot: EnchantmentSlot,
    pub left_duration_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerEnchantTimeUpdate {
    pub item_guid: ObjectGuid,
    pub slot: EnchantmentSlot,
    pub duration_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerEnchantDurationItemRef {
    pub item_guid: ObjectGuid,
    pub slot: EnchantmentSlot,
    pub enchantment_id: i32,
}

impl PlayerEnchantDurationItemRef {
    pub const fn new(item_guid: ObjectGuid, slot: EnchantmentSlot, enchantment_id: i32) -> Self {
        Self {
            item_guid,
            slot,
            enchantment_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateEnchantTimeAction {
    RemoveMissingEnchantment {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    },
    ClearExpired {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaEnchantmentItemRef {
    pub guid: ObjectGuid,
    pub bag: u8,
    pub slot: u8,
    pub enchantment_id: i32,
    pub arena_allowed: bool,
}

impl ArenaEnchantmentItemRef {
    pub const fn new(
        guid: ObjectGuid,
        bag: u8,
        slot: u8,
        enchantment_id: i32,
        arena_allowed: bool,
    ) -> Self {
        Self {
            guid,
            bag,
            slot,
            enchantment_id,
            arena_allowed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveArenaEnchantmentAction {
    RemoveDurationReference {
        item_guid: ObjectGuid,
        enchantment_slot: EnchantmentSlot,
    },
    ClearEquippedEnchantment {
        item_guid: ObjectGuid,
        enchantment_slot: EnchantmentSlot,
    },
    ClearInventoryEnchantment {
        item_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        enchantment_slot: EnchantmentSlot,
    },
    MissingInventoryItemRef {
        item_guid: ObjectGuid,
        bag: u8,
        slot: u8,
        enchantment_slot: EnchantmentSlot,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentTemplateRef {
    pub enchantment_id: i32,
    pub condition_id: u32,
    pub condition_fits: bool,
    pub min_level: u8,
    pub required_skill_id: u32,
    pub required_skill_rank: u16,
    pub required_skill_value: u16,
}

impl ApplyEnchantmentTemplateRef {
    pub const fn new(enchantment_id: i32) -> Self {
        Self {
            enchantment_id,
            condition_id: 0,
            condition_fits: true,
            min_level: 0,
            required_skill_id: 0,
            required_skill_rank: 0,
            required_skill_value: 0,
        }
    }

    pub const fn skill_fits(&self) -> bool {
        self.required_skill_id == 0 || self.required_skill_value >= self.required_skill_rank
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentGemRequirementRef {
    pub required_skill_id: u32,
    pub required_skill_rank: u16,
    pub required_skill_value: u16,
}

impl ApplyEnchantmentGemRequirementRef {
    pub const fn new(
        required_skill_id: u32,
        required_skill_rank: u16,
        required_skill_value: u16,
    ) -> Self {
        Self {
            required_skill_id,
            required_skill_rank,
            required_skill_value,
        }
    }

    pub const fn skill_fits(&self) -> bool {
        self.required_skill_id == 0 || self.required_skill_value >= self.required_skill_rank
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentSocketContext {
    pub socket_color: u32,
    pub prismatic_enchantment: Option<ApplyEnchantmentTemplateRef>,
    pub gem_requirement: Option<ApplyEnchantmentGemRequirementRef>,
}

impl ApplyEnchantmentSocketContext {
    pub const fn prismatic(
        prismatic_enchantment: Option<ApplyEnchantmentTemplateRef>,
        gem_requirement: Option<ApplyEnchantmentGemRequirementRef>,
    ) -> Self {
        Self {
            socket_color: 0,
            prismatic_enchantment,
            gem_requirement,
        }
    }

    pub const fn colored(
        socket_color: u32,
        gem_requirement: Option<ApplyEnchantmentGemRequirementRef>,
    ) -> Self {
        Self {
            socket_color,
            prismatic_enchantment: None,
            gem_requirement,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentArgs {
    pub apply: bool,
    pub apply_dur: bool,
    pub ignore_condition: bool,
    pub socket_context: Option<ApplyEnchantmentSocketContext>,
}

impl ApplyEnchantmentArgs {
    pub const fn apply() -> Self {
        Self {
            apply: true,
            apply_dur: true,
            ignore_condition: false,
            socket_context: None,
        }
    }

    pub const fn remove() -> Self {
        Self {
            apply: false,
            apply_dur: true,
            ignore_condition: false,
            socket_context: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentSkipReason {
    MissingItem,
    NotEquipped,
    NoEnchantment,
    MissingEnchantmentTemplate,
    ConditionFailed,
    PlayerLevelTooLow,
    RequiredSkillTooLow,
    MissingPrismaticEnchantment,
    PrismaticRequiredSkillTooLow,
    GemRequiredSkillTooLow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentDurationAction {
    Added(PlayerEnchantTimeUpdate),
    Removed {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentResult {
    Skipped(ApplyEnchantmentSkipReason),
    Applied {
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        enchantment_id: i32,
        apply: bool,
        effects_allowed: bool,
        update_permanent_visible_item: bool,
        duration_action: Option<ApplyEnchantmentDurationAction>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentPlan {
    pub result: ApplyEnchantmentResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentEffectKind {
    Known(ItemEnchantmentType),
    Unknown(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentEffectRef {
    pub effect_kind: ApplyEnchantmentEffectKind,
    pub amount: u32,
    pub arg: u32,
}

impl ApplyEnchantmentEffectRef {
    pub const fn known(effect_type: ItemEnchantmentType, amount: u32, arg: u32) -> Self {
        Self {
            effect_kind: ApplyEnchantmentEffectKind::Known(effect_type),
            amount,
            arg,
        }
    }

    pub const fn unknown(effect_type: u32, amount: u32, arg: u32) -> Self {
        Self {
            effect_kind: ApplyEnchantmentEffectKind::Unknown(effect_type),
            amount,
            arg,
        }
    }
}

pub const APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyEnchantmentRandomSuffixRef {
    pub id: u32,
    pub enchantments: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
    pub allocation_pct: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
}

impl ApplyEnchantmentRandomSuffixRef {
    pub const fn new(
        id: u32,
        enchantments: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
        allocation_pct: [u16; APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS],
    ) -> Self {
        Self {
            id,
            enchantments,
            allocation_pct,
        }
    }

    pub fn amount_for(&self, enchantment_id: i32, property_seed: i32) -> Option<u32> {
        self.enchantments
            .iter()
            .position(|enchantment| i32::from(*enchantment) == enchantment_id)
            .map(|index| {
                ((f64::from(self.allocation_pct[index]) * f64::from(property_seed)) / 10_000.0)
                    as u32
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentEffectAction {
    Noop,
    DeferredCombatSpell,
    DeferredUseSpell,
    UpdateDamageDoneMods {
        attack_type: WeaponAttackType,
        modifier_slot: i16,
    },
    CastEquipSpell {
        spell_id: u32,
        item_guid: ObjectGuid,
    },
    RemoveEquipSpellAura {
        spell_id: u32,
        item_guid: ObjectGuid,
    },
    UnitModifier {
        unit_mod: ApplyEnchantmentUnitMod,
        modifier: ApplyEnchantmentUnitModifier,
        amount: u32,
        apply: bool,
    },
    UpdateStatBuffMod(Stats),
    RatingModifier {
        rating: ApplyEnchantmentCombatRating,
        amount: u32,
        apply: bool,
    },
    ManaRegenBonus {
        amount: u32,
        apply: bool,
    },
    SpellPowerBonus {
        amount: u32,
        apply: bool,
    },
    HealthRegenBonus {
        amount: u32,
        apply: bool,
    },
    SpellPenetrationBonus {
        amount: u32,
        apply: bool,
    },
    BaseModFlatValue {
        base_mod: ApplyEnchantmentBaseMod,
        amount: u32,
        apply: bool,
    },
    SetShieldBlockValue {
        amount: u32,
    },
    SetBaseWeaponDamage {
        attack_type: WeaponAttackType,
        bound: WeaponDamageBoundLikeCpp,
        amount_bits: u32,
    },
    SetBaseAttackTime {
        attack_type: WeaponAttackType,
        time_ms: u32,
    },
    UpdateDamagePhysical {
        attack_type: WeaponAttackType,
    },
    UnhandledStatModifier {
        item_mod: ItemModType,
        amount: u32,
        apply: bool,
    },
    MissingItemTemplateForAttack {
        effect_kind: ApplyEnchantmentEffectKind,
    },
    Unknown {
        effect_type: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponDamageBoundLikeCpp {
    Min,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentUnitModifier {
    BaseValue,
    TotalValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentUnitMod {
    Mana,
    Health,
    Armor,
    StatAgility,
    StatStrength,
    StatIntellect,
    StatSpirit,
    StatStamina,
    AttackPower,
    AttackPowerRanged,
    Resistance(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentCombatRating {
    DefenseSkill,
    Dodge,
    Parry,
    Block,
    HitMelee,
    HitRanged,
    HitSpell,
    CritMelee,
    CritRanged,
    CritSpell,
    HasteMelee,
    HasteRanged,
    HasteSpell,
    Expertise,
    ArmorPenetration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyEnchantmentBaseMod {
    ShieldBlockValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillEnchantmentTemplateRef {
    pub enchantment_id: i32,
    pub required_skill_id: u16,
    pub required_skill_rank: u16,
}

impl SkillEnchantmentTemplateRef {
    pub const fn new(
        enchantment_id: i32,
        required_skill_id: u16,
        required_skill_rank: u16,
    ) -> Self {
        Self {
            enchantment_id,
            required_skill_id,
            required_skill_rank,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillEnchantmentItemRef {
    pub item_guid: ObjectGuid,
    pub inventory_slot: u8,
    pub enchantment_ids: [i32; MAX_ENCHANTMENT_SLOT],
    pub socket_colors: [u32; 3],
}

impl SkillEnchantmentItemRef {
    pub const fn new(
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_ids: [i32; MAX_ENCHANTMENT_SLOT],
        socket_colors: [u32; 3],
    ) -> Self {
        Self {
            item_guid,
            inventory_slot,
            enchantment_ids,
            socket_colors,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSkillEnchantmentReason {
    EnchantmentRequiredSkill,
    PrismaticRequiredSkill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSkillEnchantmentAction {
    Apply {
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
        reason: UpdateSkillEnchantmentReason,
    },
    Remove {
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
        reason: UpdateSkillEnchantmentReason,
    },
    MissingEnchantmentTemplateAbort {
        item_guid: ObjectGuid,
        inventory_slot: u8,
        enchantment_slot: EnchantmentSlot,
        enchantment_id: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendNewItemTemplateRef {
    pub quest_log_item_id: u32,
    pub dont_report_loot_log_to_party: bool,
}

impl SendNewItemTemplateRef {
    pub const fn new(quest_log_item_id: u32, dont_report_loot_log_to_party: bool) -> Self {
        Self {
            quest_log_item_id,
            dont_report_loot_log_to_party,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendNewItemArgs {
    pub quantity: u32,
    pub pushed: bool,
    pub created: bool,
    pub broadcast: bool,
    pub dungeon_encounter_id: u32,
    pub player_in_group: bool,
    pub quantity_in_inventory: u32,
}

impl SendNewItemArgs {
    pub const fn new(quantity: u32, pushed: bool, created: bool) -> Self {
        Self {
            quantity,
            pushed,
            created,
            broadcast: false,
            dungeon_encounter_id: 0,
            player_in_group: false,
            quantity_in_inventory: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendNewItemDisplayText {
    Normal,
    EncounterLoot,
    QuestUpdateAddItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendNewItemDelivery {
    Direct,
    GroupBroadcast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendNewItemModifier {
    pub value: i32,
    pub modifier_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendNewItemInstancePlan {
    pub item_id: u32,
    pub random_properties_seed: i32,
    pub random_properties_id: i32,
    pub modifications: Vec<SendNewItemModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendNewItemPlan {
    pub player_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub item_entry: u32,
    pub item_instance: SendNewItemInstancePlan,
    pub slot: u8,
    pub slot_in_bag: i16,
    pub quest_log_item_id: u32,
    pub quantity: u32,
    pub quantity_in_inventory: u32,
    pub battle_pet_species_id: u32,
    pub battle_pet_breed_id: u32,
    pub battle_pet_breed_quality: u8,
    pub battle_pet_level: u32,
    pub pushed: bool,
    pub created: bool,
    pub display_text: SendNewItemDisplayText,
    pub dungeon_encounter_id: u32,
    pub is_encounter_loot: bool,
    pub delivery: SendNewItemDelivery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitanGripPenaltyAction {
    None,
    Cast(u32),
    Remove(u32),
}

fn item_ref_by_pos<'a>(items: &'a [ItemSlotRef<'a>], bag: u8, slot: u8) -> Option<&'a Item> {
    items
        .iter()
        .find(|slot_item| slot_item.bag == bag && slot_item.slot == slot)
        .map(|slot_item| slot_item.item)
}

fn arena_enchantment_ref_by_guid(
    items: &[ArenaEnchantmentItemRef],
    guid: ObjectGuid,
) -> Option<ArenaEnchantmentItemRef> {
    items.iter().find(|item| item.guid == guid).copied()
}

fn push_arena_inventory_enchantment_action(
    actions: &mut Vec<RemoveArenaEnchantmentAction>,
    items: &[ArenaEnchantmentItemRef],
    item_guid: ObjectGuid,
    bag: u8,
    slot: u8,
    enchantment_slot: EnchantmentSlot,
) {
    match arena_enchantment_ref_by_guid(items, item_guid) {
        Some(item) if item.arena_allowed => {}
        Some(_) => actions.push(RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
            item_guid,
            bag,
            slot,
            enchantment_slot,
        }),
        None => actions.push(RemoveArenaEnchantmentAction::MissingInventoryItemRef {
            item_guid,
            bag,
            slot,
            enchantment_slot,
        }),
    }
}

const fn is_socket_enchantment_slot(slot: EnchantmentSlot) -> bool {
    matches!(
        slot,
        EnchantmentSlot::EnhancementSocket
            | EnchantmentSlot::EnhancementSocket2
            | EnchantmentSlot::EnhancementSocket3
    )
}

fn apply_enchantment_effect_action(
    item: &Item,
    item_template: Option<&ItemStorageTemplate>,
    enchantment_slot: EnchantmentSlot,
    enchantment_id: i32,
    random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
    apply: bool,
    effect: ApplyEnchantmentEffectRef,
) -> Vec<ApplyEnchantmentEffectAction> {
    match effect.effect_kind {
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::None) => {
            vec![ApplyEnchantmentEffectAction::Noop]
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::CombatSpell) => {
            vec![ApplyEnchantmentEffectAction::DeferredCombatSpell]
        }
        ApplyEnchantmentEffectKind::Known(
            kind @ (ItemEnchantmentType::Damage | ItemEnchantmentType::Totem),
        ) => {
            let Some(template) = item_template else {
                return vec![ApplyEnchantmentEffectAction::MissingItemTemplateForAttack {
                    effect_kind: ApplyEnchantmentEffectKind::Known(kind),
                }];
            };
            let attack_type = get_attack_by_slot(item.slot(), template.inventory_type);
            if attack_type == WeaponAttackType::Max {
                vec![ApplyEnchantmentEffectAction::Noop]
            } else {
                vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
                    attack_type,
                    modifier_slot: if apply { -1 } else { enchantment_slot as i16 },
                }]
            }
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::EquipSpell) => {
            if effect.arg == 0 {
                vec![ApplyEnchantmentEffectAction::Noop]
            } else if apply {
                vec![ApplyEnchantmentEffectAction::CastEquipSpell {
                    spell_id: effect.arg,
                    item_guid: item.object().guid(),
                }]
            } else {
                vec![ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
                    spell_id: effect.arg,
                    item_guid: item.object().guid(),
                }]
            }
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Resistance) => {
            let amount =
                resolve_enchantment_effect_amount(item, enchantment_id, random_suffix, effect);
            vec![ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(effect.arg),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            }]
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Stat) => {
            let amount =
                resolve_enchantment_effect_amount(item, enchantment_id, random_suffix, effect);
            apply_enchantment_stat_actions(item_mod_type_from_u32(effect.arg), amount, apply)
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::UseSpell) => {
            vec![ApplyEnchantmentEffectAction::DeferredUseSpell]
        }
        ApplyEnchantmentEffectKind::Known(
            ItemEnchantmentType::PrismaticSocket
            | ItemEnchantmentType::ArtifactPowerBonusRankByType
            | ItemEnchantmentType::ArtifactPowerBonusRankByID
            | ItemEnchantmentType::BonusListID
            | ItemEnchantmentType::BonusListCurve
            | ItemEnchantmentType::ArtifactPowerBonusRankPicker,
        ) => vec![ApplyEnchantmentEffectAction::Noop],
        ApplyEnchantmentEffectKind::Unknown(effect_type) => {
            vec![ApplyEnchantmentEffectAction::Unknown { effect_type }]
        }
    }
}

fn resolve_enchantment_effect_amount(
    item: &Item,
    enchantment_id: i32,
    random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
    effect: ApplyEnchantmentEffectRef,
) -> u32 {
    if effect.amount != 0
        || !matches!(
            effect.effect_kind,
            ApplyEnchantmentEffectKind::Known(
                ItemEnchantmentType::Resistance | ItemEnchantmentType::Stat
            )
        )
    {
        return effect.amount;
    }

    let Some(random_suffix) = random_suffix else {
        return effect.amount;
    };
    if item.data().random_properties_id.unsigned_abs() != random_suffix.id {
        return effect.amount;
    }

    random_suffix
        .amount_for(enchantment_id, item.data().property_seed)
        .unwrap_or(effect.amount)
}

fn apply_enchantment_stat_actions(
    item_mod: ItemModType,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    match item_mod {
        ItemModType::Mana => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Mana,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        )],
        ItemModType::Health => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Health,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        )],
        ItemModType::Agility => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatAgility,
            Stats::Agility,
            amount,
            apply,
        ),
        ItemModType::Strength => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStrength,
            Stats::Strength,
            amount,
            apply,
        ),
        ItemModType::Intellect => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatIntellect,
            Stats::Intellect,
            amount,
            apply,
        ),
        ItemModType::Spirit => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatSpirit,
            Stats::Spirit,
            amount,
            apply,
        ),
        ItemModType::Stamina => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStamina,
            Stats::Stamina,
            amount,
            apply,
        ),
        ItemModType::DefenseSkillRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::DefenseSkill], amount, apply)
        }
        ItemModType::DodgeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Dodge], amount, apply)
        }
        ItemModType::ParryRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Parry], amount, apply)
        }
        ItemModType::BlockRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Block], amount, apply)
        }
        ItemModType::HitMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitMelee], amount, apply)
        }
        ItemModType::HitRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitRanged], amount, apply)
        }
        ItemModType::HitSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitSpell], amount, apply)
        }
        ItemModType::CritMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritMelee], amount, apply)
        }
        ItemModType::CritRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritRanged], amount, apply)
        }
        ItemModType::CritSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritSpell], amount, apply)
        }
        ItemModType::HasteSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteSpell], amount, apply)
        }
        ItemModType::HitRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::HitMelee,
                ApplyEnchantmentCombatRating::HitRanged,
                ApplyEnchantmentCombatRating::HitSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::CritRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::CritMelee,
                ApplyEnchantmentCombatRating::CritRanged,
                ApplyEnchantmentCombatRating::CritSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::HasteRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::HasteMelee,
                ApplyEnchantmentCombatRating::HasteRanged,
                ApplyEnchantmentCombatRating::HasteSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::ExpertiseRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Expertise], amount, apply)
        }
        ItemModType::AttackPower => vec![
            unit_modifier(
                ApplyEnchantmentUnitMod::AttackPower,
                ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            ),
            unit_modifier(
                ApplyEnchantmentUnitMod::AttackPowerRanged,
                ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            ),
        ],
        ItemModType::RangedAttackPower => vec![unit_modifier(
            ApplyEnchantmentUnitMod::AttackPowerRanged,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        )],
        ItemModType::ManaRegeneration => {
            vec![ApplyEnchantmentEffectAction::ManaRegenBonus { amount, apply }]
        }
        ItemModType::ArmorPenetrationRating => rating_actions(
            &[ApplyEnchantmentCombatRating::ArmorPenetration],
            amount,
            apply,
        ),
        ItemModType::SpellPower => {
            vec![ApplyEnchantmentEffectAction::SpellPowerBonus { amount, apply }]
        }
        ItemModType::HealthRegen => {
            vec![ApplyEnchantmentEffectAction::HealthRegenBonus { amount, apply }]
        }
        ItemModType::SpellPenetration => {
            vec![ApplyEnchantmentEffectAction::SpellPenetrationBonus { amount, apply }]
        }
        ItemModType::BlockValue => vec![ApplyEnchantmentEffectAction::BaseModFlatValue {
            base_mod: ApplyEnchantmentBaseMod::ShieldBlockValue,
            amount,
            apply,
        }],
        _ => vec![ApplyEnchantmentEffectAction::UnhandledStatModifier {
            item_mod,
            amount,
            apply,
        }],
    }
}

/// C++ `Player::_ApplyItemBonuses` static ItemSparse stat-loop subset.
pub fn item_stat_bonus_actions_like_cpp(
    stats: &[(i8, i16); 10],
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    let mut actions = Vec::new();
    for &(stat_type, amount) in stats {
        if stat_type == -1 || amount == 0 {
            continue;
        }
        if amount < 0 {
            actions.push(ApplyEnchantmentEffectAction::UnhandledStatModifier {
                item_mod: item_mod_type_from_u32(stat_type as u32),
                amount: amount.unsigned_abs().into(),
                apply,
            });
            continue;
        }
        let item_mod = item_mod_type_from_u32(stat_type as u32);
        actions.extend(item_bonus_stat_actions_like_cpp(
            item_mod,
            amount as u32,
            apply,
        ));
    }
    actions
}

/// C++ `Player::_ApplyItemBonuses` scaling-stat stat loop.
pub fn item_scaling_stat_bonus_actions_like_cpp(
    stat_ids: &[i32; 10],
    bonuses: &[i32; 10],
    ssd_multiplier: i32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    let mut actions = Vec::new();
    for (&stat_type, &bonus) in stat_ids.iter().zip(bonuses.iter()) {
        if stat_type == -1 {
            continue;
        }
        let val = (ssd_multiplier * bonus) / 10_000;
        if val == 0 {
            continue;
        }
        if val < 0 {
            actions.push(ApplyEnchantmentEffectAction::UnhandledStatModifier {
                item_mod: item_mod_type_from_u32(stat_type as u32),
                amount: val.unsigned_abs(),
                apply,
            });
            continue;
        }
        let item_mod = item_mod_type_from_u32(stat_type as u32);
        actions.extend(item_bonus_stat_actions_like_cpp(
            item_mod, val as u32, apply,
        ));
    }
    actions
}

/// C++ `_ApplyItemBonuses` direct `ItemTemplate::GetResistance(school)` loop.
pub fn item_resistance_bonus_actions_like_cpp(
    resistances: &[i16; 7],
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    let mut actions = Vec::new();
    for (school, resistance) in resistances.iter().copied().enumerate() {
        if resistance == 0 {
            continue;
        }
        if resistance < 0 {
            continue;
        }
        actions.push(unit_modifier(
            ApplyEnchantmentUnitMod::Resistance(school as u32),
            ApplyEnchantmentUnitModifier::BaseValue,
            resistance as u32,
            apply,
        ));
    }
    actions
}

/// C++ `_ApplyItemBonuses` direct `ActivePlayerData::ShieldBlock` assignment.
pub fn item_shield_block_bonus_action_like_cpp(
    shield_block_value: i16,
    is_armor_shield: bool,
    apply: bool,
) -> Option<ApplyEnchantmentEffectAction> {
    if !is_armor_shield || shield_block_value <= 0 {
        return None;
    }

    Some(ApplyEnchantmentEffectAction::SetShieldBlockValue {
        amount: if apply { shield_block_value as u32 } else { 0 },
    })
}

/// C++ `Player::_ApplyWeaponDamage` direct non-scaling weapon field actions.
pub fn item_weapon_damage_actions_like_cpp(
    slot: u8,
    inventory_type: InventoryType,
    min_damage: f32,
    max_damage: f32,
    item_delay: u16,
    apply: bool,
    is_in_feral_form: bool,
    can_use_attack_type: bool,
    has_shapeshift_combat_round_time: bool,
    can_modify_stats: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    const BASE_ATTACK_TIME_LIKE_CPP: u32 = 2_000;

    let attack_type = get_attack_by_slot(slot, inventory_type);
    if attack_type == WeaponAttackType::Max || (!is_in_feral_form && apply && !can_use_attack_type)
    {
        return Vec::new();
    }

    let mut actions = Vec::new();
    let mut changed_damage = false;

    if min_damage > 0.0 {
        let amount = if apply { min_damage } else { BASE_MINDAMAGE };
        actions.push(ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
            attack_type,
            bound: WeaponDamageBoundLikeCpp::Min,
            amount_bits: amount.to_bits(),
        });
        changed_damage = true;
    }

    if max_damage > 0.0 {
        let amount = if apply { max_damage } else { BASE_MAXDAMAGE };
        actions.push(ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
            attack_type,
            bound: WeaponDamageBoundLikeCpp::Max,
            amount_bits: amount.to_bits(),
        });
        changed_damage = true;
    }

    if item_delay != 0 && !has_shapeshift_combat_round_time {
        actions.push(ApplyEnchantmentEffectAction::SetBaseAttackTime {
            attack_type,
            time_ms: if apply {
                u32::from(item_delay)
            } else {
                BASE_ATTACK_TIME_LIKE_CPP
            },
        });
    }

    if can_modify_stats && (changed_damage || item_delay != 0) {
        actions.push(ApplyEnchantmentEffectAction::UpdateDamagePhysical { attack_type });
    }

    actions
}

fn item_bonus_stat_actions_like_cpp(
    item_mod: ItemModType,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    match item_mod {
        ItemModType::Agility => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatAgility,
            Stats::Agility,
            amount,
            apply,
        ),
        ItemModType::Strength => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStrength,
            Stats::Strength,
            amount,
            apply,
        ),
        ItemModType::Intellect => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatIntellect,
            Stats::Intellect,
            amount,
            apply,
        ),
        ItemModType::Spirit => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatSpirit,
            Stats::Spirit,
            amount,
            apply,
        ),
        ItemModType::Stamina => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStamina,
            Stats::Stamina,
            amount,
            apply,
        ),
        ItemModType::HasteMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteMelee], amount, apply)
        }
        ItemModType::HasteRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteRanged], amount, apply)
        }
        ItemModType::ExtraArmor => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Armor,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        )],
        ItemModType::FireResistance => {
            item_bonus_resistance_actions(SpellSchools::Fire, amount, apply)
        }
        ItemModType::FrostResistance => {
            item_bonus_resistance_actions(SpellSchools::Frost, amount, apply)
        }
        ItemModType::HolyResistance => {
            item_bonus_resistance_actions(SpellSchools::Holy, amount, apply)
        }
        ItemModType::ShadowResistance => {
            item_bonus_resistance_actions(SpellSchools::Shadow, amount, apply)
        }
        ItemModType::NatureResistance => {
            item_bonus_resistance_actions(SpellSchools::Nature, amount, apply)
        }
        ItemModType::ArcaneResistance => {
            item_bonus_resistance_actions(SpellSchools::Arcane, amount, apply)
        }
        ItemModType::AgiStrInt => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatAgility,
                Stats::Agility,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatStrength,
                Stats::Strength,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatIntellect,
                Stats::Intellect,
                amount,
                apply,
            ),
        ]
        .concat(),
        ItemModType::AgiStr => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatAgility,
                Stats::Agility,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatStrength,
                Stats::Strength,
                amount,
                apply,
            ),
        ]
        .concat(),
        ItemModType::AgiInt => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatAgility,
                Stats::Agility,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatIntellect,
                Stats::Intellect,
                amount,
                apply,
            ),
        ]
        .concat(),
        ItemModType::StrInt => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatStrength,
                Stats::Strength,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatIntellect,
                Stats::Intellect,
                amount,
                apply,
            ),
        ]
        .concat(),
        _ => apply_enchantment_stat_actions(item_mod, amount, apply),
    }
}

fn item_bonus_primary_stat_actions(
    unit_mod: ApplyEnchantmentUnitMod,
    stat: Stats,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    vec![
        unit_modifier(
            unit_mod,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        ),
        ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat),
    ]
}

fn item_bonus_resistance_actions(
    school: SpellSchools,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    vec![unit_modifier(
        ApplyEnchantmentUnitMod::Resistance(school as u32),
        ApplyEnchantmentUnitModifier::BaseValue,
        amount,
        apply,
    )]
}

fn primary_stat_actions(
    unit_mod: ApplyEnchantmentUnitMod,
    stat: Stats,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    vec![
        unit_modifier(
            unit_mod,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        ),
        ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat),
    ]
}

fn unit_modifier(
    unit_mod: ApplyEnchantmentUnitMod,
    modifier: ApplyEnchantmentUnitModifier,
    amount: u32,
    apply: bool,
) -> ApplyEnchantmentEffectAction {
    ApplyEnchantmentEffectAction::UnitModifier {
        unit_mod,
        modifier,
        amount,
        apply,
    }
}

fn rating_actions(
    ratings: &[ApplyEnchantmentCombatRating],
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    ratings
        .iter()
        .map(|rating| ApplyEnchantmentEffectAction::RatingModifier {
            rating: *rating,
            amount,
            apply,
        })
        .collect()
}

fn skill_enchantment_transition(
    curr_value: u16,
    new_value: u16,
    required_skill_rank: u16,
) -> Option<bool> {
    if curr_value < required_skill_rank && new_value >= required_skill_rank {
        Some(true)
    } else if new_value < required_skill_rank && curr_value >= required_skill_rank {
        Some(false)
    } else {
        None
    }
}

fn push_update_skill_enchantment_action(
    actions: &mut Vec<UpdateSkillEnchantmentAction>,
    item: SkillEnchantmentItemRef,
    enchantment_slot: EnchantmentSlot,
    enchantment_id: i32,
    reason: UpdateSkillEnchantmentReason,
    apply: bool,
) {
    let action = if apply {
        UpdateSkillEnchantmentAction::Apply {
            item_guid: item.item_guid,
            inventory_slot: item.inventory_slot,
            enchantment_slot,
            enchantment_id,
            reason,
        }
    } else {
        UpdateSkillEnchantmentAction::Remove {
            item_guid: item.item_guid,
            inventory_slot: item.inventory_slot,
            enchantment_slot,
            enchantment_id,
            reason,
        }
    };
    actions.push(action);
}

const fn get_attack_by_slot(slot: u8, inventory_type: InventoryType) -> WeaponAttackType {
    match slot {
        EQUIPMENT_SLOT_MAINHAND => {
            if matches!(
                inventory_type,
                InventoryType::Ranged | InventoryType::RangedRight
            ) {
                WeaponAttackType::RangedAttack
            } else {
                WeaponAttackType::BaseAttack
            }
        }
        EQUIPMENT_SLOT_OFFHAND => WeaponAttackType::OffAttack,
        _ => WeaponAttackType::Max,
    }
}

const fn item_mod_type_from_u32(value: u32) -> ItemModType {
    match value {
        0 => ItemModType::Mana,
        1 => ItemModType::Health,
        3 => ItemModType::Agility,
        4 => ItemModType::Strength,
        5 => ItemModType::Intellect,
        6 => ItemModType::Spirit,
        7 => ItemModType::Stamina,
        12 => ItemModType::DefenseSkillRating,
        13 => ItemModType::DodgeRating,
        14 => ItemModType::ParryRating,
        15 => ItemModType::BlockRating,
        16 => ItemModType::HitMeleeRating,
        17 => ItemModType::HitRangedRating,
        18 => ItemModType::HitSpellRating,
        19 => ItemModType::CritMeleeRating,
        20 => ItemModType::CritRangedRating,
        21 => ItemModType::CritSpellRating,
        28 => ItemModType::HasteMeleeRating,
        29 => ItemModType::HasteRangedRating,
        30 => ItemModType::HasteSpellRating,
        31 => ItemModType::HitRating,
        32 => ItemModType::CritRating,
        36 => ItemModType::HasteRating,
        37 => ItemModType::ExpertiseRating,
        38 => ItemModType::AttackPower,
        39 => ItemModType::RangedAttackPower,
        43 => ItemModType::ManaRegeneration,
        44 => ItemModType::ArmorPenetrationRating,
        45 => ItemModType::SpellPower,
        46 => ItemModType::HealthRegen,
        47 => ItemModType::SpellPenetration,
        48 => ItemModType::BlockValue,
        50 => ItemModType::ExtraArmor,
        51 => ItemModType::FireResistance,
        52 => ItemModType::FrostResistance,
        53 => ItemModType::HolyResistance,
        54 => ItemModType::ShadowResistance,
        55 => ItemModType::NatureResistance,
        56 => ItemModType::ArcaneResistance,
        71 => ItemModType::AgiStrInt,
        72 => ItemModType::AgiStr,
        73 => ItemModType::AgiInt,
        74 => ItemModType::StrInt,
        _ => ItemModType::None,
    }
}

fn bag_template_by_pos<'a>(
    templates: &'a [BagTemplateRef<'a>],
    bag: u8,
) -> Option<&'a ItemStorageTemplate> {
    templates
        .iter()
        .find(|bag_template| bag_template.bag == bag)
        .map(|bag_template| bag_template.template)
}

fn item_storage_ref_by_guid<'a>(
    items: &[ItemStorageRef<'a>],
    guid: ObjectGuid,
) -> Option<ItemStorageRef<'a>> {
    items
        .iter()
        .find(|stored| stored.item.object().guid() == guid)
        .copied()
}

fn cpp_keyring_family_gate_applies(slot: u8) -> bool {
    let keyring_limit =
        i16::from(KEYRING_SLOT_START) + i16::from(KEYRING_SLOT_START) - i16::from(KEYRING_SLOT_END);
    i16::from(slot) >= i16::from(KEYRING_SLOT_START) && i16::from(slot) < keyring_limit
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemSearchLocation: u8 {
        const EQUIPMENT = 0x01;
        const INVENTORY = 0x02;
        const BANK = 0x04;
        const REAGENT_BANK = 0x08;

        const DEFAULT = Self::EQUIPMENT.bits() | Self::INVENTORY.bits();
        const EVERYWHERE = Self::EQUIPMENT.bits() | Self::INVENTORY.bits()
            | Self::BANK.bits() | Self::REAGENT_BANK.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSearchCallbackResult {
    Stop,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStorageError {
    InvalidPlayerSlot(u8),
    InvalidBagSlot(u8),
    InvalidBagItemSlot(u8),
    UnknownBag(u8),
    EmptyPlayerSlot(u8),
    EmptyBagItemSlot {
        bag: u8,
        slot: u8,
    },
    OccupiedPlayerSlot(u8),
    OccupiedBagItemSlot {
        bag: u8,
        slot: u8,
    },
    MismatchedBagGuid {
        bag: u8,
        expected: ObjectGuid,
        actual: ObjectGuid,
    },
    MismatchedItemGuid {
        slot: u8,
        expected: ObjectGuid,
        actual: ObjectGuid,
    },
    MismatchedBagItemGuid {
        bag: u8,
        slot: u8,
        expected: ObjectGuid,
        actual: ObjectGuid,
    },
    SplitItemLootGenerated,
    InvalidSplitCount {
        available: u32,
        requested: u32,
    },
    TooFewItemsToSplit {
        available: u32,
        requested: u32,
    },
    SplitItemInTrade,
    TopLevelBuybackHiddenFromGetItemByPos(u8),
}

/// Persistent identity and template metadata for one Player-owned item.
///
/// C++ stores the concrete `Item*` directly in `Player::m_items`
/// (`Player.h:2935`). Rust keeps this small record alongside the concrete
/// [`Item`] so database identity and the effective inventory type travel with
/// the same canonical Player lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInventoryItem {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub db_guid: u64,
    pub inventory_type: Option<u8>,
}

/// Concrete item/object runtime owned by the canonical Player.
///
/// This is deliberately a private Player substate rather than a shared lock:
/// MapManager's generation-checked Player handle remains the only route to a
/// mutable owner, including while the Player is detached for a far teleport.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerInventoryRuntime {
    inventory_items: HashMap<u8, PlayerInventoryItem>,
    buyback_items: HashMap<u8, PlayerInventoryItem>,
    buyback_price: [u32; BUYBACK_SLOT_COUNT],
    buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    current_buyback_slot: u8,
    item_objects: HashMap<ObjectGuid, Item>,
}

impl PlayerInventoryRuntime {
    pub fn inventory_items(&self) -> &HashMap<u8, PlayerInventoryItem> {
        &self.inventory_items
    }

    pub fn inventory_items_mut(&mut self) -> &mut HashMap<u8, PlayerInventoryItem> {
        &mut self.inventory_items
    }

    pub fn buyback_items(&self) -> &HashMap<u8, PlayerInventoryItem> {
        &self.buyback_items
    }

    pub fn buyback_items_mut(&mut self) -> &mut HashMap<u8, PlayerInventoryItem> {
        &mut self.buyback_items
    }

    pub const fn buyback_price(&self) -> &[u32; BUYBACK_SLOT_COUNT] {
        &self.buyback_price
    }

    pub fn buyback_price_mut(&mut self) -> &mut [u32; BUYBACK_SLOT_COUNT] {
        &mut self.buyback_price
    }

    pub const fn buyback_timestamp(&self) -> &[i64; BUYBACK_SLOT_COUNT] {
        &self.buyback_timestamp
    }

    pub fn buyback_timestamp_mut(&mut self) -> &mut [i64; BUYBACK_SLOT_COUNT] {
        &mut self.buyback_timestamp
    }

    pub const fn current_buyback_slot(&self) -> u8 {
        self.current_buyback_slot
    }

    pub fn set_current_buyback_slot(&mut self, slot: u8) {
        self.current_buyback_slot = slot;
    }

    pub fn item_objects(&self) -> &HashMap<ObjectGuid, Item> {
        &self.item_objects
    }

    pub fn item_objects_mut(&mut self) -> &mut HashMap<ObjectGuid, Item> {
        &mut self.item_objects
    }
}

impl Default for PlayerInventoryRuntime {
    fn default() -> Self {
        Self {
            inventory_items: HashMap::new(),
            buyback_items: HashMap::new(),
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            current_buyback_slot: BUYBACK_SLOT_START,
            item_objects: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBagStorage {
    pub bag_guid: ObjectGuid,
    pub bag_size: u8,
    pub slots: [Option<ObjectGuid>; MAX_BAG_SIZE],
}

impl PlayerBagStorage {
    pub fn new(bag_guid: ObjectGuid, bag_size: u8) -> Self {
        assert!(bag_size as usize <= MAX_BAG_SIZE);
        Self {
            bag_guid,
            bag_size,
            slots: [None; MAX_BAG_SIZE],
        }
    }

    pub fn item_by_pos(&self, slot: u8) -> Option<ObjectGuid> {
        if slot < self.bag_size {
            self.slots[slot as usize]
        } else {
            None
        }
    }

    pub fn set_item(&mut self, slot: u8, guid: Option<ObjectGuid>) {
        assert!((slot as usize) < MAX_BAG_SIZE);
        self.slots[slot as usize] = guid;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInventoryStorage {
    pub items: [Option<ObjectGuid>; PLAYER_SLOT_END],
    pub bags: [Option<PlayerBagStorage>; PLAYER_SLOT_END],
    pub current_buyback_slot: u8,
}

impl PlayerInventoryStorage {
    pub fn get_item_by_guid_everywhere(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        self.items
            .iter()
            .enumerate()
            .filter(|(slot, _)| !is_buyback_slot(*slot as u8))
            .find_map(|(_, item_guid)| (*item_guid == Some(guid)).then_some(guid))
            .or_else(|| {
                self.bags
                    .iter()
                    .filter_map(|bag| *bag)
                    .flat_map(|bag| bag.slots.into_iter().take(bag.bag_size as usize))
                    .find_map(|item_guid| (item_guid == Some(guid)).then_some(guid))
            })
    }
}

impl Default for PlayerInventoryStorage {
    fn default() -> Self {
        Self {
            items: [None; PLAYER_SLOT_END],
            bags: [None; PLAYER_SLOT_END],
            current_buyback_slot: BUYBACK_SLOT_START,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisibleItemValues {
    pub item_id: i32,
    pub item_appearance_mod_id: u16,
    pub item_visual: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDataValues {
    pub loot_target_guid: ObjectGuid,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub party_type: [u8; 2],
    pub num_bank_slots: u8,
    pub native_sex: u8,
    pub inebriation: u8,
    pub player_title: i32,
    pub current_spec_id: u32,
    pub current_battle_pet_breed_quality: u8,
    pub honor_level: i32,
    pub visible_items: [VisibleItemValues; EQUIPMENT_SLOT_END as usize],
}

impl Default for PlayerDataValues {
    fn default() -> Self {
        Self {
            loot_target_guid: ObjectGuid::EMPTY,
            player_flags: 0,
            player_flags_ex: 0,
            party_type: [0; 2],
            num_bank_slots: 0,
            native_sex: Gender::Male as u8,
            inebriation: 0,
            player_title: 0,
            current_spec_id: 0,
            current_battle_pet_breed_quality: 0,
            honor_level: 0,
            visible_items: [VisibleItemValues::default(); EQUIPMENT_SLOT_END as usize],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePlayerDataValues {
    pub farsight_object: ObjectGuid,
    pub summoned_battle_pet_guid: ObjectGuid,
    pub coinage: u64,
    pub xp: i32,
    pub next_level_xp: i32,
    pub character_points: i32,
    pub honor: i32,
    pub honor_next_level: i32,
    pub watched_faction_index: i32,
    pub scaling_player_level_delta: i32,
    pub num_backpack_slots: u8,
    pub inv_slots: [ObjectGuid; PLAYER_SLOT_END],
    pub explored_zones: [u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    pub rest_info: [PlayerRestInfoValueLikeCpp; 2],
    pub buyback_price: [u32; BUYBACK_SLOT_COUNT],
    pub buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    pub bank_bag_slot_flags: [u32; 7],
    pub heirlooms: Vec<i32>,
    pub heirlooms_update_mask: Option<Vec<u32>>,
    pub heirloom_flags: Vec<u32>,
    pub heirloom_flags_update_mask: Option<Vec<u32>>,
    pub toys: Vec<i32>,
    pub toys_update_mask: Option<Vec<u32>>,
    pub transmog: Vec<u32>,
    pub transmog_update_mask: Option<Vec<u32>>,
    pub conditional_transmog: Vec<i32>,
    pub conditional_transmog_update_mask: Option<Vec<u32>>,
    pub quest_completed: [u64; QUESTS_COMPLETED_BITS_SIZE],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerRestInfoValueLikeCpp {
    pub threshold: u32,
    pub state_id: u8,
}

impl Default for ActivePlayerDataValues {
    fn default() -> Self {
        Self {
            farsight_object: ObjectGuid::EMPTY,
            summoned_battle_pet_guid: ObjectGuid::EMPTY,
            coinage: 0,
            xp: 0,
            next_level_xp: 0,
            character_points: 0,
            honor: 0,
            honor_next_level: 0,
            watched_faction_index: -1,
            scaling_player_level_delta: 0,
            num_backpack_slots: 0,
            inv_slots: [ObjectGuid::EMPTY; PLAYER_SLOT_END],
            explored_zones: [0; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
            rest_info: [PlayerRestInfoValueLikeCpp::default(); 2],
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            bank_bag_slot_flags: [0; 7],
            heirlooms: Vec::new(),
            heirlooms_update_mask: None,
            heirloom_flags: Vec::new(),
            heirloom_flags_update_mask: None,
            toys: Vec::new(),
            toys_update_mask: None,
            transmog: Vec::new(),
            transmog_update_mask: None,
            conditional_transmog: Vec::new(),
            conditional_transmog_update_mask: None,
            quest_completed: [0; QUESTS_COMPLETED_BITS_SIZE],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDataUpdate {
    pub mask: UpdateMask,
    pub values: PlayerDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePlayerDataUpdate {
    pub mask: UpdateMask,
    pub values: ActivePlayerDataValues,
    pub rest_info_change_masks: [u8; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub unit_data: Option<UnitDataUpdate>,
    pub player_data: Option<PlayerDataUpdate>,
    pub active_player_data: Option<ActivePlayerDataUpdate>,
}

impl PlayerValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    unit: Unit,
    session_id: Option<u64>,
    data: PlayerDataValues,
    active_data: ActivePlayerDataValues,
    inventory: Box<PlayerInventoryStorage>,
    inventory_runtime: Box<PlayerInventoryRuntime>,
    gameplay_state: PlayerGameplayState,
    player_data_changes: UpdateMask,
    active_player_data_changes: UpdateMask,
    rest_info_change_masks: [u8; 2],
    mod_melee_hit_chance: f32,
    mod_ranged_hit_chance: f32,
    mod_spell_hit_chance: f32,
    ingame_time: u32,
    shared_quest_id: u32,
    extra_flags: u32,
    team: u8,
    is_active: bool,
    controlled_by_player: bool,
    accept_whispers: bool,
    can_titan_grip: bool,
    titan_grip_penalty_spell_id: u32,
    soulbound_tradeable_items: HashSet<ObjectGuid>,
    item_durations: Vec<ObjectGuid>,
    enchant_durations: Vec<PlayerEnchantDuration>,
    lifecycle_metadata: PlayerLifecycleMetadata,
    duel: Option<PlayerDuelInfoLikeCpp>,
    forced_reaction_faction_ids: HashSet<u32>,
}

impl Player {
    pub fn new(session_id: Option<u64>, can_filter_whispers: bool) -> Self {
        let mut unit = Unit::new(true);
        unit.set_type(
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );

        Self {
            unit,
            session_id,
            data: PlayerDataValues::default(),
            active_data: ActivePlayerDataValues::default(),
            inventory: Box::default(),
            inventory_runtime: Box::default(),
            gameplay_state: PlayerGameplayState::default(),
            player_data_changes: UpdateMask::new(PLAYER_DATA_BITS),
            active_player_data_changes: UpdateMask::new(ACTIVE_PLAYER_DATA_BITS),
            rest_info_change_masks: [0; 2],
            mod_melee_hit_chance: 7.5,
            mod_ranged_hit_chance: 7.5,
            mod_spell_hit_chance: 15.0,
            ingame_time: 0,
            shared_quest_id: 0,
            extra_flags: 0,
            team: TEAM_OTHER,
            is_active: true,
            controlled_by_player: true,
            accept_whispers: !can_filter_whispers,
            can_titan_grip: false,
            titan_grip_penalty_spell_id: 0,
            soulbound_tradeable_items: HashSet::new(),
            item_durations: Vec::new(),
            enchant_durations: Vec::new(),
            lifecycle_metadata: PlayerLifecycleMetadata::default(),
            duel: None,
            forced_reaction_faction_ids: HashSet::new(),
        }
    }

    pub fn create_from_lifecycle(
        session_id: Option<u64>,
        can_filter_whispers: bool,
        record: PlayerCreateLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) -> Self {
        let mut player = Self::new(session_id, can_filter_whispers);
        player.apply_create_lifecycle(record, resolver);
        player
    }

    pub fn load_from_db_lifecycle(
        session_id: Option<u64>,
        can_filter_whispers: bool,
        record: PlayerDbLoadLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) -> Self {
        let mut player = Self::new(session_id, can_filter_whispers);
        player.apply_db_load_lifecycle(record, resolver);
        player
    }

    pub fn apply_create_lifecycle(
        &mut self,
        record: PlayerCreateLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) {
        let metadata = PlayerLifecycleMetadata {
            account_id: None,
            create_time: record.create_time,
            create_mode: record.create_mode,
            played_time_total: record.played_time_total,
            played_time_level: record.played_time_level,
            active_talent_group: record.active_talent_group,
            zone_id: None,
        };

        self.apply_lifecycle_base(
            PlayerLifecycleBase {
                guid: record.guid,
                name: record.name,
                race: record.race,
                class_id: record.class_id,
                gender: record.gender,
                level: record.level,
                xp: record.xp,
                money: record.money,
                inventory_slot_count: record.inventory_slot_count,
                bank_bag_slot_count: record.bank_bag_slot_count,
                map_id: record.map_id,
                position: record.position,
                max_health: record.max_health,
                health: record.health,
                powers: record.powers,
                display_power: record.display_power,
                faction_template: record.faction_template,
                display_id: record.display_id,
                player_flags: record.player_flags,
                player_flags_ex: record.player_flags_ex,
                extra_flags: record.extra_flags,
                metadata,
            },
            resolver,
        );
    }

    pub fn apply_db_load_lifecycle(
        &mut self,
        record: PlayerDbLoadLifecycleRecord,
        resolver: &impl PlayerPowerIndexResolver,
    ) {
        let metadata = PlayerLifecycleMetadata {
            account_id: Some(record.account_id),
            create_time: record.create_time,
            create_mode: record.create_mode,
            played_time_total: record.played_time_total,
            played_time_level: record.played_time_level,
            active_talent_group: record.active_talent_group,
            zone_id: record.zone_id,
        };

        self.apply_lifecycle_base(
            PlayerLifecycleBase {
                guid: record.guid,
                name: record.name,
                race: record.race,
                class_id: record.class_id,
                gender: record.gender,
                level: record.level,
                xp: record.xp,
                money: record.money,
                inventory_slot_count: record.inventory_slot_count,
                bank_bag_slot_count: record.bank_bag_slot_count,
                map_id: record.map_id,
                position: record.position,
                max_health: record.max_health,
                health: record.health,
                powers: record.powers,
                display_power: record.display_power,
                faction_template: record.faction_template,
                display_id: record.display_id,
                player_flags: record.player_flags,
                player_flags_ex: record.player_flags_ex,
                extra_flags: record.extra_flags,
                metadata,
            },
            resolver,
        );
    }

    fn apply_lifecycle_base(
        &mut self,
        record: PlayerLifecycleBase,
        resolver: &impl PlayerPowerIndexResolver,
    ) {
        self.unit.world_mut().object_mut().create(record.guid);
        self.unit.world_mut().object_mut().set_scale(1.0);
        self.unit.world_mut().set_name(record.name);
        self.unit
            .world_mut()
            .world_relocate(record.map_id, record.position);

        self.set_race_class_gender(record.race, record.class_id, record.gender);
        self.unit.set_level(record.level);
        self.set_inventory_slot_count(record.inventory_slot_count);
        self.set_bank_bag_slot_count(record.bank_bag_slot_count);
        self.set_xp(record.xp);
        self.set_money(record.money);
        self.replace_all_player_flags(record.player_flags);
        self.replace_all_player_flags_ex(record.player_flags_ex);
        self.extra_flags = record.extra_flags;
        self.lifecycle_metadata = record.metadata;

        self.unit.set_display_power(record.display_power);
        if let Some(faction_template) = record.faction_template {
            self.unit.set_faction(faction_template);
        }
        if let Some(display_id) = record.display_id {
            self.unit.set_display_id(display_id, true);
        }

        self.configure_power_indices_for_class(resolver);
        self.unit.set_max_health(record.max_health);
        self.unit.set_health(record.health);
        for power in record.powers {
            self.unit.set_max_power(power.power, power.max);
            self.unit.set_power(power.power, power.current);
        }

        self.clear_data_changes();
    }

    pub const fn unit(&self) -> &Unit {
        &self.unit
    }

    pub fn unit_mut(&mut self) -> &mut Unit {
        &mut self.unit
    }

    pub const fn session_id(&self) -> Option<u64> {
        self.session_id
    }

    pub const fn data(&self) -> &PlayerDataValues {
        &self.data
    }

    pub const fn active_data(&self) -> &ActivePlayerDataValues {
        &self.active_data
    }

    /// Gameplay bridge state is not update-mask tracked yet; this is a documented no-op baseline
    /// hook for future DB/session integration.
    pub fn clear_gameplay_changes(&mut self) {}

    pub const fn duel_info_like_cpp(&self) -> Option<PlayerDuelInfoLikeCpp> {
        self.duel
    }

    pub fn set_duel_info_like_cpp(&mut self, duel: Option<PlayerDuelInfoLikeCpp>) {
        self.duel = duel;
    }

    pub fn set_duel_opponent_in_progress_like_cpp(&mut self, opponent: ObjectGuid) {
        self.duel = Some(PlayerDuelInfoLikeCpp {
            opponent,
            state: PlayerDuelStateLikeCpp::InProgress,
        });
    }

    pub fn clear_duel_like_cpp(&mut self) {
        self.duel = None;
    }

    pub fn is_dueling_opponent_in_progress_like_cpp(&self, opponent: ObjectGuid) -> bool {
        self.duel.is_some_and(|duel| {
            duel.opponent == opponent && duel.state == PlayerDuelStateLikeCpp::InProgress
        })
    }

    pub const fn hit_chances(&self) -> (f32, f32, f32) {
        (
            self.mod_melee_hit_chance,
            self.mod_ranged_hit_chance,
            self.mod_spell_hit_chance,
        )
    }

    pub const fn team(&self) -> u8 {
        self.team
    }

    pub const fn is_active(&self) -> bool {
        self.is_active
    }

    pub const fn controlled_by_player(&self) -> bool {
        self.controlled_by_player
    }

    pub const fn accept_whispers(&self) -> bool {
        self.accept_whispers
    }

    pub const fn ingame_time(&self) -> u32 {
        self.ingame_time
    }

    pub const fn extra_flags(&self) -> u32 {
        self.extra_flags
    }

    pub const fn is_game_master_like_cpp(&self) -> bool {
        (self.extra_flags & PLAYER_EXTRA_GM_ON) != 0
    }

    pub fn set_game_master_like_cpp(&mut self, on: bool) {
        if on {
            self.extra_flags |= PLAYER_EXTRA_GM_ON;
        } else {
            self.extra_flags &= !PLAYER_EXTRA_GM_ON;
        }
    }

    pub const fn lifecycle_metadata(&self) -> PlayerLifecycleMetadata {
        self.lifecycle_metadata
    }

    pub fn player_data_changes_mask(&self) -> &UpdateMask {
        &self.player_data_changes
    }

    pub fn active_player_data_changes_mask(&self) -> &UpdateMask {
        &self.active_player_data_changes
    }

    pub fn clear_player_data_changes(&mut self) {
        self.player_data_changes.reset_all();
    }

    pub fn clear_active_player_data_changes(&mut self) {
        self.active_player_data_changes.reset_all();
        self.rest_info_change_masks = [0; 2];
    }

    pub fn clear_data_changes(&mut self) {
        self.clear_player_data_changes();
        self.clear_active_player_data_changes();
        self.unit.clear_unit_data_changes();
        self.unit.world_mut().object_mut().clear_update_mask(false);
    }

    pub fn set_selection(&mut self, guid: ObjectGuid) {
        self.unit.set_target(guid);
    }

    pub const fn inebriation_like_cpp(&self) -> u8 {
        self.data.inebriation
    }

    pub fn set_inebriation_like_cpp(&mut self, value: u8) {
        self.set_player_u8(PLAYER_DATA_INEBRIATION_BIT, value.min(100), |data| {
            &mut data.inebriation
        });
    }

    pub fn set_player_flag(&mut self, flag: u32) {
        self.replace_all_player_flags(self.data.player_flags | flag);
    }

    pub fn remove_player_flag(&mut self, flag: u32) {
        self.replace_all_player_flags(self.data.player_flags & !flag);
    }

    pub fn has_player_flag(&self, flag: u32) -> bool {
        (self.data.player_flags & flag) != 0
    }

    pub fn set_player_flag_ex(&mut self, flag: u32) {
        self.replace_all_player_flags_ex(self.data.player_flags_ex | flag);
    }

    pub fn remove_player_flag_ex(&mut self, flag: u32) {
        self.replace_all_player_flags_ex(self.data.player_flags_ex & !flag);
    }

    pub fn has_player_flag_ex(&self, flag: u32) -> bool {
        (self.data.player_flags_ex & flag) != 0
    }

    pub fn set_primary_specialization(&mut self, spec: u32) {
        self.set_player_u32(PLAYER_DATA_CURRENT_SPEC_ID_BIT, spec, |data| {
            &mut data.current_spec_id
        });
    }

    pub fn set_farsight_object_like_cpp(&mut self, guid: ObjectGuid) {
        self.set_active_guid(ACTIVE_PLAYER_DATA_FARSIGHT_OBJECT_BIT, guid, |data| {
            &mut data.farsight_object
        });
        self.unit_mut()
            .set_seer_can_always_see_target_guid_like_cpp(guid);
    }

    pub fn set_honor_like_cpp(&mut self, honor: i32) {
        self.set_active_i32_in_section(
            ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT,
            ACTIVE_PLAYER_DATA_HONOR_BIT,
            honor,
            |data| &mut data.honor,
        );
    }

    pub fn set_free_primary_professions(&mut self, points: u16) {
        self.set_active_i32(
            ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT,
            i32::from(points),
            |data| &mut data.character_points,
        );
    }

    /// Set C++ `UF::ActivePlayerData::CharacterPoints` without narrowing the
    /// signed update-field value used by talent initialization.
    pub fn set_character_points_like_cpp(&mut self, points: i32) {
        self.set_active_i32(ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT, points, |data| {
            &mut data.character_points
        });
    }

    pub fn is_valid_pos(&self, bag: u8, slot: u8, explicit_pos: bool) -> bool {
        if bag == NULL_BAG && !explicit_pos {
            return true;
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            if slot == NULL_SLOT && !explicit_pos {
                return true;
            }
            if slot < EQUIPMENT_SLOT_END {
                return true;
            }
            if (PROFESSION_SLOT_START..PROFESSION_SLOT_END).contains(&slot) {
                return true;
            }
            if (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot) {
                return true;
            }
            if (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot) {
                return true;
            }
            let backpack_end = INVENTORY_SLOT_ITEM_START
                .saturating_add(self.active_data.num_backpack_slots)
                .min(INVENTORY_SLOT_ITEM_END);
            if (INVENTORY_SLOT_ITEM_START..backpack_end).contains(&slot) {
                return true;
            }
            if (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END).contains(&slot) {
                return true;
            }
            if (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot) {
                return true;
            }
            if (KEYRING_SLOT_START..KEYRING_SLOT_END).contains(&slot) {
                return true;
            }
            return false;
        }

        let Some(bag_storage) = self
            .inventory
            .bags
            .get(bag as usize)
            .and_then(Option::as_ref)
        else {
            return false;
        };

        if slot == NULL_SLOT && !explicit_pos {
            return true;
        }

        slot < bag_storage.bag_size
    }

    pub fn is_valid_packed_pos(&self, pos: u16, explicit_pos: bool) -> bool {
        let [bag, slot] = pos.to_be_bytes();
        self.is_valid_pos(bag, slot, explicit_pos)
    }

    pub fn set_buyback_price(&mut self, slot: usize, price: u32) {
        if slot >= BUYBACK_SLOT_COUNT || self.active_data.buyback_price[slot] == price {
            return;
        }

        self.active_data.buyback_price[slot] = price;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT,
            slot,
        );
    }

    pub fn mark_buyback_price_changed(&mut self, slot: usize) {
        if slot >= BUYBACK_SLOT_COUNT {
            return;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT,
            slot,
        );
    }

    pub fn set_buyback_timestamp(&mut self, slot: usize, timestamp: i64) {
        if slot >= BUYBACK_SLOT_COUNT || self.active_data.buyback_timestamp[slot] == timestamp {
            return;
        }

        self.active_data.buyback_timestamp[slot] = timestamp;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
            slot,
        );
    }

    pub fn mark_buyback_timestamp_changed(&mut self, slot: usize) {
        if slot >= BUYBACK_SLOT_COUNT {
            return;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
            slot,
        );
    }

    pub const fn can_titan_grip(&self) -> bool {
        self.can_titan_grip
    }

    pub fn set_can_titan_grip(&mut self, value: bool, penalty_spell_id: u32) {
        if value == self.can_titan_grip {
            return;
        }

        self.can_titan_grip = value;
        self.titan_grip_penalty_spell_id = penalty_spell_id;
    }

    pub fn is_two_hand_used_template(&self, main_template: Option<&ItemStorageTemplate>) -> bool {
        let Some(template) = main_template else {
            return false;
        };

        (template.inventory_type == InventoryType::Weapon2Hand && !self.can_titan_grip)
            || template.inventory_type == InventoryType::Ranged
            || (template.inventory_type == InventoryType::RangedRight
                && template.class_id == ItemClass::Weapon
                && template.subclass_id != ItemSubClassWeapon::Wand as u32)
    }

    pub fn is_using_two_handed_weapon_in_one_hand_template(
        main_template: Option<&ItemStorageTemplate>,
        off_template: Option<&ItemStorageTemplate>,
    ) -> bool {
        if off_template
            .is_some_and(|template| template.inventory_type == InventoryType::Weapon2Hand)
        {
            return true;
        }

        main_template.is_some_and(|template| template.inventory_type == InventoryType::Weapon2Hand)
            && off_template.is_some()
    }

    pub fn check_titan_grip_penalty_action(
        &self,
        using_two_handed_weapon_in_one_hand: bool,
        has_penalty_aura: bool,
    ) -> TitanGripPenaltyAction {
        if !self.can_titan_grip {
            return TitanGripPenaltyAction::None;
        }

        if using_two_handed_weapon_in_one_hand {
            if has_penalty_aura {
                TitanGripPenaltyAction::None
            } else {
                TitanGripPenaltyAction::Cast(self.titan_grip_penalty_spell_id)
            }
        } else {
            TitanGripPenaltyAction::Remove(self.titan_grip_penalty_spell_id)
        }
    }

    pub fn changed_object_type_mask(&self, include_active_player: bool) -> u32 {
        self.unit.changed_object_type_mask()
            | if self.player_data_changes.is_any_set() {
                1 << TYPEID_PLAYER
            } else {
                0
            }
            | if include_active_player && self.active_player_data_changes.is_any_set() {
                1 << TYPEID_ACTIVE_PLAYER
            } else {
                0
            }
    }

    fn set_player_u32(
        &mut self,
        bit: usize,
        value: u32,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut u32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    fn set_player_i32(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    fn set_player_u8(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut PlayerDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.data);
        if *target != value {
            *target = value;
            self.mark_player_data(bit);
        }
    }

    fn set_active_u64(
        &mut self,
        bit: usize,
        value: u64,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut u64,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }

    fn set_active_i32(
        &mut self,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }

    fn set_active_i32_in_section(
        &mut self,
        parent_bit: usize,
        bit: usize,
        value: i32,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut i32,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data_section(parent_bit, bit);
        }
    }

    fn set_active_u8(
        &mut self,
        bit: usize,
        value: u8,
        field: impl FnOnce(&mut ActivePlayerDataValues) -> &mut u8,
    ) {
        let target = field(&mut self.active_data);
        if *target != value {
            *target = value;
            self.mark_active_player_data(bit);
        }
    }

    fn mark_player_data(&mut self, bit: usize) {
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(bit);
    }

    fn mark_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.player_data_changes.set(parent_bit);
        self.player_data_changes.set(first_element_bit + index);
    }

    fn mark_active_player_data(&mut self, bit: usize) {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes.set(bit);
    }

    fn mark_active_player_data_section(&mut self, parent_bit: usize, bit: usize) {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes.set(parent_bit);
        self.active_player_data_changes.set(bit);
    }

    fn mark_active_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.active_player_data_changes.set(parent_bit);
        self.active_player_data_changes
            .set(first_element_bit + index);
    }

    fn set_dynamic_update_mask_index(mask: &mut Option<Vec<u32>>, index: usize) {
        let block = index / 32;
        let bit = index % 32;
        let blocks = mask.get_or_insert_with(Vec::new);
        if blocks.len() <= block {
            blocks.resize(block + 1, 0);
        }
        blocks[block] |= 1 << bit;
    }
}

fn equip_slot_candidates(args: FindEquipSlotArgs<'_>) -> [u8; 4] {
    let mut slots = [NULL_SLOT; 4];
    match args.proto.inventory_type {
        InventoryType::Head => slots[0] = EQUIPMENT_SLOT_HEAD,
        InventoryType::Neck => slots[0] = EQUIPMENT_SLOT_NECK,
        InventoryType::Shoulders => slots[0] = EQUIPMENT_SLOT_SHOULDERS,
        InventoryType::Body => slots[0] = EQUIPMENT_SLOT_BODY,
        InventoryType::Chest | InventoryType::Robe => slots[0] = EQUIPMENT_SLOT_CHEST,
        InventoryType::Waist => slots[0] = EQUIPMENT_SLOT_WAIST,
        InventoryType::Legs => slots[0] = EQUIPMENT_SLOT_LEGS,
        InventoryType::Feet => slots[0] = EQUIPMENT_SLOT_FEET,
        InventoryType::Wrists => slots[0] = EQUIPMENT_SLOT_WRISTS,
        InventoryType::Hands => slots[0] = EQUIPMENT_SLOT_HANDS,
        InventoryType::Finger => {
            slots[0] = EQUIPMENT_SLOT_FINGER1;
            slots[1] = EQUIPMENT_SLOT_FINGER2;
        }
        InventoryType::Trinket => {
            slots[0] = EQUIPMENT_SLOT_TRINKET1;
            slots[1] = EQUIPMENT_SLOT_TRINKET2;
        }
        InventoryType::Cloak => slots[0] = EQUIPMENT_SLOT_BACK,
        InventoryType::Weapon => {
            slots[0] = EQUIPMENT_SLOT_MAINHAND;
            if args.can_dual_wield {
                slots[1] = EQUIPMENT_SLOT_OFFHAND;
            }
        }
        InventoryType::Shield | InventoryType::WeaponOffhand | InventoryType::Holdable => {
            slots[0] = EQUIPMENT_SLOT_OFFHAND;
        }
        InventoryType::Ranged | InventoryType::WeaponMainhand | InventoryType::RangedRight => {
            slots[0] = EQUIPMENT_SLOT_MAINHAND;
        }
        InventoryType::Weapon2Hand => {
            slots[0] = EQUIPMENT_SLOT_MAINHAND;
            if args.can_dual_wield && args.can_titan_grip {
                slots[1] = EQUIPMENT_SLOT_OFFHAND;
            }
        }
        InventoryType::Tabard => slots[0] = EQUIPMENT_SLOT_TABARD,
        InventoryType::Bag => {
            slots[0] = INVENTORY_SLOT_BAG_START;
            slots[1] = INVENTORY_SLOT_BAG_START + 1;
            slots[2] = INVENTORY_SLOT_BAG_START + 2;
            slots[3] = INVENTORY_SLOT_BAG_START + 3;
        }
        InventoryType::ProfessionTool | InventoryType::ProfessionGear => {
            if args.proto.class_id != ItemClass::Profession || !args.has_required_profession_skill {
                return slots;
            }

            let is_tool = args.proto.inventory_type == InventoryType::ProfessionTool;
            match args.proto.subclass_id {
                value if value == ItemSubclassProfession::Cooking as u32 => {
                    slots[0] = if is_tool {
                        PROFESSION_SLOT_COOKING_TOOL
                    } else {
                        PROFESSION_SLOT_COOKING_GEAR1
                    };
                }
                value if value == ItemSubclassProfession::Fishing as u32 => {
                    if !is_tool {
                        return [NULL_SLOT; 4];
                    }
                    slots[0] = PROFESSION_SLOT_FISHING_TOOL;
                }
                value
                    if value == ItemSubclassProfession::Blacksmithing as u32
                        || value == ItemSubclassProfession::Leatherworking as u32
                        || value == ItemSubclassProfession::Alchemy as u32
                        || value == ItemSubclassProfession::Herbalism as u32
                        || value == ItemSubclassProfession::Mining as u32
                        || value == ItemSubclassProfession::Tailoring as u32
                        || value == ItemSubclassProfession::Engineering as u32
                        || value == ItemSubclassProfession::Enchanting as u32
                        || value == ItemSubclassProfession::Skinning as u32
                        || value == ItemSubclassProfession::Jewelcrafting as u32
                        || value == ItemSubclassProfession::Inscription as u32 =>
                {
                    let Some(profession_slot) = args.profession_slot else {
                        return [NULL_SLOT; 4];
                    };

                    if is_tool {
                        slots[0] = PROFESSION_SLOT_PROFESSION1_TOOL
                            + profession_slot * PROFESSION_SLOT_MAX_COUNT;
                    } else {
                        // C++ writes slots[0] twice here, so primary profession gear1 is unreachable.
                        slots[0] = PROFESSION_SLOT_PROFESSION1_GEAR1
                            + profession_slot * PROFESSION_SLOT_MAX_COUNT;
                        slots[0] = PROFESSION_SLOT_PROFESSION1_GEAR2
                            + profession_slot * PROFESSION_SLOT_MAX_COUNT;
                    }
                }
                _ => return [NULL_SLOT; 4],
            }
        }
        _ => return slots,
    }
    slots
}

fn paired_unique_ignore_slot(slot: u8) -> Option<u8> {
    match slot {
        EQUIPMENT_SLOT_MAINHAND => Some(EQUIPMENT_SLOT_OFFHAND),
        EQUIPMENT_SLOT_OFFHAND => Some(EQUIPMENT_SLOT_MAINHAND),
        EQUIPMENT_SLOT_FINGER1 => Some(EQUIPMENT_SLOT_FINGER2),
        EQUIPMENT_SLOT_FINGER2 => Some(EQUIPMENT_SLOT_FINGER1),
        EQUIPMENT_SLOT_TRINKET1 => Some(EQUIPMENT_SLOT_TRINKET2),
        EQUIPMENT_SLOT_TRINKET2 => Some(EQUIPMENT_SLOT_TRINKET1),
        PROFESSION_SLOT_PROFESSION1_GEAR1 => Some(PROFESSION_SLOT_PROFESSION1_GEAR2),
        PROFESSION_SLOT_PROFESSION1_GEAR2 => Some(PROFESSION_SLOT_PROFESSION1_GEAR1),
        PROFESSION_SLOT_PROFESSION2_GEAR1 => Some(PROFESSION_SLOT_PROFESSION2_GEAR2),
        PROFESSION_SLOT_PROFESSION2_GEAR2 => Some(PROFESSION_SLOT_PROFESSION2_GEAR1),
        _ => None,
    }
}

fn has_equipped_item_entry(
    equipped_items: &[ItemStorageRef<'_>],
    entry: u32,
    except_slot: u8,
) -> bool {
    equipped_items.iter().any(|stored| {
        stored.bag == INVENTORY_SLOT_BAG_0
            && stored.slot != except_slot
            && stored.item.object().entry() == entry
    })
}

fn has_equipped_gem_entry(equipped_gems: &[EquippedGemRef], entry: u32, except_slot: u8) -> bool {
    equipped_gems
        .iter()
        .any(|gem| gem.slot != except_slot && gem.entry == entry)
}

fn equipped_item_limit_category_count(
    equipped_items: &[ItemStorageRef<'_>],
    limit_category: u32,
    except_slot: u8,
) -> u32 {
    equipped_items
        .iter()
        .filter(|stored| {
            stored.bag == INVENTORY_SLOT_BAG_0
                && stored.slot != except_slot
                && stored
                    .template
                    .is_some_and(|template| template.item_limit_category == limit_category)
        })
        .map(|stored| stored.item.count())
        .sum()
}

fn equipped_gem_limit_category_count(
    equipped_gems: &[EquippedGemRef],
    limit_category: u32,
    except_slot: u8,
) -> u32 {
    equipped_gems
        .iter()
        .filter(|gem| gem.slot != except_slot && gem.limit_category == limit_category)
        .count() as u32
}

fn destroy_item_count_item_by_pos<'a>(
    items: &[DestroyItemCountItemRef<'a>],
    bag: u8,
    slot: u8,
) -> Option<DestroyItemCountItemRef<'a>> {
    items
        .iter()
        .find(|item_ref| item_ref.bag == bag && item_ref.slot == slot)
        .copied()
}

fn destroy_item_count_consider_item(
    plan: &mut DestroyItemCountPlan,
    item_ref: DestroyItemCountItemRef<'_>,
    item_entry: u32,
    requested_count: u32,
    require_unequip_for_full_stack: bool,
    unequip_check: bool,
) {
    if plan.removed_count >= requested_count
        || item_ref.item.object().entry() != item_entry
        || item_ref.item.is_in_trade()
    {
        return;
    }

    let needed = requested_count - plan.removed_count;
    let item_count = item_ref.item.count();
    if item_count <= needed {
        if require_unequip_for_full_stack
            && unequip_check
            && item_ref.can_unequip_result != InventoryResult::Ok
        {
            return;
        }

        plan.actions.push(DestroyItemCountAction {
            bag: item_ref.bag,
            slot: item_ref.slot,
            removed_count: item_count,
            remaining_count: 0,
            destroy_stack: true,
        });
        plan.removed_count += item_count;
    } else {
        plan.actions.push(DestroyItemCountAction {
            bag: item_ref.bag,
            slot: item_ref.slot,
            removed_count: needed,
            remaining_count: item_count - needed,
            destroy_stack: false,
        });
        plan.removed_count = requested_count;
    }
}

fn destroy_item_count_scan_top_level_range(
    plan: &mut DestroyItemCountPlan,
    items: &[DestroyItemCountItemRef<'_>],
    item_entry: u32,
    requested_count: u32,
    start: u8,
    end: u8,
    require_unequip_for_full_stack: bool,
    unequip_check: bool,
) {
    for slot in start..end {
        if let Some(item_ref) = destroy_item_count_item_by_pos(items, INVENTORY_SLOT_BAG_0, slot) {
            destroy_item_count_consider_item(
                plan,
                item_ref,
                item_entry,
                requested_count,
                require_unequip_for_full_stack,
                unequip_check,
            );
            if plan.removed_count >= requested_count {
                return;
            }
        }
    }
}

fn destroy_item_count_scan_bag_ranges(
    plan: &mut DestroyItemCountPlan,
    items: &[DestroyItemCountItemRef<'_>],
    item_entry: u32,
    requested_count: u32,
    start_bag: u8,
    end_bag: u8,
) {
    for bag in start_bag..end_bag {
        for slot in 0..MAX_BAG_SIZE as u8 {
            if let Some(item_ref) = destroy_item_count_item_by_pos(items, bag, slot) {
                destroy_item_count_consider_item(
                    plan,
                    item_ref,
                    item_entry,
                    requested_count,
                    false,
                    false,
                );
                if plan.removed_count >= requested_count {
                    return;
                }
            }
        }
    }
}

fn destroy_filtered_item_by_pos(
    items: &[DestroyFilteredItemRef],
    bag: u8,
    slot: u8,
) -> Option<DestroyFilteredItemRef> {
    items
        .iter()
        .find(|item_ref| item_ref.bag == bag && item_ref.slot == slot)
        .copied()
}

fn destroy_filtered_consider_item(
    actions: &mut Vec<DestroyFilteredItemAction>,
    item_ref: DestroyFilteredItemRef,
) {
    if item_ref.should_destroy {
        actions.push(DestroyFilteredItemAction {
            bag: item_ref.bag,
            slot: item_ref.slot,
        });
    }
}

fn destroy_filtered_scan_top_level_range(
    actions: &mut Vec<DestroyFilteredItemAction>,
    items: &[DestroyFilteredItemRef],
    start: u8,
    end: u8,
) {
    for slot in start..end {
        if let Some(item_ref) = destroy_filtered_item_by_pos(items, INVENTORY_SLOT_BAG_0, slot) {
            destroy_filtered_consider_item(actions, item_ref);
        }
    }
}

fn destroy_filtered_scan_bag_ranges(
    actions: &mut Vec<DestroyFilteredItemAction>,
    items: &[DestroyFilteredItemRef],
    start_bag: u8,
    end_bag: u8,
) {
    for bag in start_bag..end_bag {
        for slot in 0..MAX_BAG_SIZE as u8 {
            if let Some(item_ref) = destroy_filtered_item_by_pos(items, bag, slot) {
                destroy_filtered_consider_item(actions, item_ref);
            }
        }
    }
}

fn swap_item_real_swap_target_for_destination(
    destination: u16,
    can_store_result: InventoryResult,
    can_bank_result: InventoryResult,
    can_equip_result: InventoryResult,
    equip_dest: u16,
    equip_dest_can_unequip_result: InventoryResult,
) -> (InventoryResult, SwapItemRealSwapTarget) {
    if is_inventory_packed_pos(destination) {
        return (can_store_result, SwapItemRealSwapTarget::Inventory);
    }

    if is_bank_packed_pos(destination) {
        return (can_bank_result, SwapItemRealSwapTarget::Bank);
    }

    if is_equipment_packed_pos(destination) {
        if can_equip_result == InventoryResult::Ok {
            return (
                equip_dest_can_unequip_result,
                SwapItemRealSwapTarget::Equip { dest: equip_dest },
            );
        }

        return (
            can_equip_result,
            SwapItemRealSwapTarget::Equip { dest: equip_dest },
        );
    }

    (InventoryResult::Ok, SwapItemRealSwapTarget::None)
}

fn is_bag_storage_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

fn is_buyback_slot(slot: u8) -> bool {
    (BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot)
}

fn validate_split_source(source: &Item, count: u32) -> Result<(), PlayerStorageError> {
    if source.loot_generated() {
        return Err(PlayerStorageError::SplitItemLootGenerated);
    }

    let available = source.count();
    if count == 0 || available == count {
        return Err(PlayerStorageError::InvalidSplitCount {
            available,
            requested: count,
        });
    }

    if available < count {
        return Err(PlayerStorageError::TooFewItemsToSplit {
            available,
            requested: count,
        });
    }

    if source.is_in_trade() {
        return Err(PlayerStorageError::SplitItemInTrade);
    }

    Ok(())
}

fn can_store_item_error(
    result: InventoryResult,
    count: u32,
    no_similar_count: u32,
) -> CanStoreItemOutcome {
    CanStoreItemOutcome {
        result,
        no_space_count: Some(count + no_similar_count),
    }
}

fn can_store_item_count_zero(count: u32, no_similar_count: u32) -> Option<CanStoreItemOutcome> {
    (count == 0).then(|| {
        if no_similar_count == 0 {
            CanStoreItemOutcome {
                result: InventoryResult::Ok,
                no_space_count: None,
            }
        } else {
            can_store_item_error(InventoryResult::ItemMaxCount, count, no_similar_count)
        }
    })
}

fn can_equip_item_outcome(result: InventoryResult) -> CanEquipItemOutcome {
    CanEquipItemOutcome {
        result,
        dest: 0,
        unique_ignore_slot: None,
    }
}

fn can_take_more_similar_ok() -> CanTakeMoreSimilarItemsOutcome {
    CanTakeMoreSimilarItemsOutcome {
        result: InventoryResult::Ok,
        no_space_count: None,
        offending_item_id: None,
    }
}

#[cfg(test)]
#[path = "../player_tests.rs"]
mod tests;
