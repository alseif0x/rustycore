// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player entity.
//!
//! Issue #226 split the former 9,268-line `player.rs` into private state-family
//! modules. `Player` remains one type with one semantic owner: no storage
//! location, writer, mirror or runtime clock changed.

mod auras;
pub use auras::AuraDurationUpdatePolicy;
mod away_status;
mod collections;
mod damage_control;
mod death_operation;
mod deferred_save;
mod identity;
mod inebriation;
mod inventory_runtime;
mod inventory_values;
pub use inventory_runtime::{ItemObjectUpdateLikeCpp, PlayerInventoryRuntime};
mod battleground;
mod cinematic;
mod collection_state;
mod combat;
mod difficulty;
mod effective_stats;
mod enchantments;
mod equipment_sets;
pub use enchantments::{
    EnchantmentConditionFacts, EnchantmentConditionSource, GemConditionEvaluationError,
    GemConditionOperandSide, SkillEnchantmentApplyArgs, SkillEnchantmentAttackPowerIndex,
    SkillEnchantmentAttackPowerModifier, SkillEnchantmentEffectOperation, SkillEnchantmentEffects,
    SkillEnchantmentError, SkillEnchantmentItemFacts, SkillEnchantmentItemTemplate,
    SkillEnchantmentRating, SkillEnchantmentRecord, SkillEnchantmentSource,
    SkillEnchantmentUnitMod, SkillEnchantmentUnitModifier, apply_all_enchantments_native_like_cpp,
    apply_enchantment_native_like_cpp, enchantment_condition_fits_native_like_cpp,
    update_skill_enchantments_native_like_cpp,
};
mod item_modifiers;
mod items;
mod lifecycle;
mod location;
mod menu;
mod movement_control;
mod pending_spell_cast;
mod persistent_capabilities;
mod pet_lifecycle;
mod progression;
mod records;
mod skill_sources;
mod spell_book;
pub use spell_book::{
    AddPlayerSpell, CastOverrideAura, CastSpellAuras, CastSpellError, CastSpellResult,
    LearnPlayerSpell, PlayerSpellBook, PlayerSpellEntry, PlayerSpellState, PlayerSpellTrait,
    RemovePlayerSpell, SpellBookMessage, SpellBookMutation, SpellBookOrderError,
    SpellLearnCriterion, SpellLearningEffects, SpellLoadRow, SpellSaveAction, SpellSaveBatch,
    SpellSaveFinishError, SpellSavePrepareError,
};
mod spell_sources;
pub use spell_sources::{
    SpellBookDefinition, SpellBookEffect, SpellDefinitionSource, SpellLearningAbility,
    SpellLearningError, SpellLearningNode, SpellLearningRank, SpellLearningSkill,
    SpellLearningSource, SpellLearningSourceError,
};
mod skills;
pub use skill_sources::{
    SkillAbilityMetadata, SkillCatalog, SkillLineMetadata, SkillLoadSource, SkillRaceClassMetadata,
    SkillRewardSpellMetadata, SkillSetError, SkillSetSource, SkillUpdate,
};
pub use skills::{
    PLAYER_MAX_SKILLS, PlayerSkills, ProfessionItemMove, SkillBonusError, SkillCriteria,
    SkillFields, SkillLoadError, SkillLoadOutcome, SkillLoadRow, SkillSaveBatch, SkillSaveRow,
    SkillSetEffects, SkillSetOutcome, SkillStatusEntry, SkillStatusOrderError, SkillUpdateState,
    SkillValuesSnapshot,
};
mod pvp;
mod quest_log;
mod quest_state;
mod reputation;
mod rest;
mod spell_runtime;
mod talent_runtime;
mod taxi_state;
mod trade;
mod trade_transfer;
pub use trade_transfer::TradeTakenItemLikeCpp;
mod vehicle;
mod world_local;
pub use battleground::PlayerBattlegroundState;
pub use cinematic::PlayerCinematicStateLikeCpp;
pub use collection_state::PlayerCollectionStateLikeCpp;
pub use death_operation::{
    PlayerDeathOperationContext, PlayerDeathOperationId, PlayerDeathOperationPhase,
    PlayerDeathOperationSnapshot,
};
pub use effective_stats::{PlayerDerivedStatValuesUpdate, PlayerEffectiveCombatStatsLikeCpp};
pub use equipment_sets::PlayerEquipmentSetsLikeCpp;
pub use item_modifiers::{
    PlayerItemBonusStateLikeCpp, PlayerItemLevelCapsLikeCpp, PlayerItemModifierRuntimeStateLikeCpp,
    PlayerItemSetEffectLikeCpp, loaded_enchantment_effect_action_is_unrepresented_like_cpp,
    represented_item_bonus_action_updates_stats_like_cpp,
};
pub use items::InventoryPlayerSnapshotLikeCpp;
pub use items::ItemStorageView;
pub const APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS: usize = 5;
pub use items::bonus_actions::{
    item_resistance_bonus_actions_like_cpp, item_scaling_stat_bonus_actions_like_cpp,
    item_shield_block_bonus_action_like_cpp, item_stat_bonus_actions_like_cpp,
    item_weapon_damage_actions_like_cpp,
};
pub use items::enchantment_records::{
    ApplyEnchantmentArgs, ApplyEnchantmentBaseMod, ApplyEnchantmentCombatRating,
    ApplyEnchantmentDurationAction, ApplyEnchantmentEffectAction, ApplyEnchantmentEffectKind,
    ApplyEnchantmentEffectRef, ApplyEnchantmentGemRequirementRef, ApplyEnchantmentPlan,
    ApplyEnchantmentRandomSuffixRef, ApplyEnchantmentResult, ApplyEnchantmentSkipReason,
    ApplyEnchantmentSocketContext, ApplyEnchantmentTemplateRef, ApplyEnchantmentUnitMod,
    ApplyEnchantmentUnitModifier, ArenaEnchantmentItemRef, PlayerEnchantDuration,
    PlayerEnchantDurationItemRef, PlayerEnchantTimeUpdate, RemoveArenaEnchantmentAction,
    SkillEnchantmentItemRef, SkillEnchantmentTemplateRef, UpdateEnchantTimeAction,
    UpdateSkillEnchantmentAction, UpdateSkillEnchantmentReason, WeaponDamageBoundLikeCpp,
};
pub use items::plans::{
    BagTemplateRef, CanBankItemArgs, CanEquipItemArgs, CanEquipItemOutcome, CanEquipUniqueItemArgs,
    CanEquipUniqueItemTemplateArgs, CanStoreItemArgs, CanStoreItemOutcome,
    CanTakeMoreSimilarItemsArgs, CanTakeMoreSimilarItemsOutcome, CanUnequipItemArgs,
    CanUseItemArgs, CanUseItemTemplateArgs, DestroyFilteredItemAction, DestroyFilteredItemRef,
    DestroyItemCountAction, DestroyItemCountItemRef, DestroyItemCountPlan, EquipItemObjectOutcome,
    EquippedGemRef, FindEquipSlotArgs, ItemLimitCategoryTemplate, ItemPosCount, ItemSlotRef,
    ItemStorageRef, SocketedGemUniqueRef, SwapBagItemMove, SwapBagItemRef, SwapBagRef,
    SwapItemBagExchangePlan, SwapItemBagExchangeResult, SwapItemEmptyDestinationPlan,
    SwapItemEmptyDestinationResult, SwapItemErrorItemOrder, SwapItemMergeFillPlan,
    SwapItemMergeFillResult, SwapItemMissingPhase, SwapItemOrchestrationPlan,
    SwapItemOrchestrationResult, SwapItemPreflightItem, SwapItemPreflightPlan,
    SwapItemPreflightResult, SwapItemRealSwapExecutionPlan, SwapItemRealSwapTarget,
    SwapItemRealSwapValidationPlan, SwapItemRealSwapValidationResult,
    SwapItemRealSwapValidationSubject,
};
pub use items::{
    ExistingStorageStackUpdateLikeCpp, InventoryStorageMovePlanLikeCpp,
    plan_inventory_storage_move_like_cpp,
};
pub use items::{
    PlayerSpellCastCurrencyDebitActionLikeCpp, PlayerSpellCastItemDebitActionLikeCpp,
    PlayerSpellCastItemUseOutcomeLikeCpp, PlayerSpellCastItemUsePlanLikeCpp,
    PlayerSpellCastResourceDebitOutcomeLikeCpp, PlayerSpellCastResourceDebitPlanLikeCpp,
};
pub use items::{
    represented_avg_total_item_level_maybe_replace_slot_like_cpp,
    represented_total_avg_equipment_slot_candidates_like_cpp,
};
pub use progression::PreparedPlayerSpellAcquisitionLikeCpp;
pub use quest_log::{QuestLogRowUpdateLikeCpp, QuestLogValuesLikeCpp};
pub use quest_state::{
    PlayerQuestGameplayState, PlayerQuestStatusSaveIntentLikeCpp, PlayerQuestStatusSaveTypeLikeCpp,
    QuestBoundItemObjectiveProgressLikeCpp, QuestItemObjectiveProgressLikeCpp,
};
pub use records::{
    PlayerCreateLifecycleRecord, PlayerDbLoadLifecycleRecord, PlayerGameplayLoadPlan,
    PlayerGameplayLoadRecord, PlayerGameplayLoadStep, PlayerLifecycleMetadata,
    PlayerLifecyclePower, PlayerLoginLifecyclePlan, PlayerLoginLifecycleStep,
    PlayerWorldInsertionState,
};
pub use reputation::{
    PlayerFactionStateLikeCpp, PlayerReputationStateLikeCpp, ReputationRankCounterLikeCpp,
    ReputationRankCountersLikeCpp,
};
pub use rest::PlayerRestState;
pub use spell_runtime::{
    ForgottenKnownSpellLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp, PlayerSpellRuntimeState,
};
pub use talent_runtime::PlayerTalentRuntimeState;
pub use taxi_state::{PlayerTaxiFlightNodeLikeCpp, PlayerTaxiFlightStateLikeCpp, PlayerTaxiState};
pub use world_local::PlayerWorldLocalState;
mod collections_hydration;
mod composite_state;
mod group_membership;
mod load_hydration;
mod recent_instances;
mod resurrection;
mod save_ack;
mod scalar_transitions;
pub use save_ack::{
    PlayerQuestNativeSaveProjectionLikeCpp, PlayerQuestObjectiveSaveValueLikeCpp,
    PlayerQuestStatusSaveProjectionLikeCpp, PlayerSaveAcknowledgementLikeCpp,
    PlayerSavedGroupsLikeCpp,
};
mod social;
mod spellbook;
mod trait_config;
mod transport_and_faction;
pub use trait_config::{PlayerTraitConfigDetails, PlayerTraitConfigState, PlayerTraitEntry};
mod visibility;
mod vitals;
mod void_storage;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use crate::PlayerGameplayState;
use bitflags::bitflags;
use items::stack_plans::{
    destroy_filtered_scan_bag_ranges, destroy_filtered_scan_top_level_range,
    destroy_item_count_scan_bag_ranges, destroy_item_count_scan_top_level_range,
    validate_split_source,
};
use wow_constants::{
    EnchantmentSlot, Gender, InventoryResult, InventoryType, ItemBondingType, ItemEnchantmentType,
    ItemFieldFlags, ItemFieldFlags2, ItemModType, ItemModifier, ItemSubclassProfession,
    ItemUpdateState, PowerType, Stats, TypeId, TypeMask, WeaponAttackType, spell::SpellSchools,
};
#[cfg(test)]
use wow_constants::{ItemClass, ItemSubClassQuiver, ItemSubClassWeapon};
use wow_core::{ObjectGuid, Position};

use crate::{
    BASE_MAXDAMAGE, BASE_MINDAMAGE, Bag, EQUIPMENT_SLOT_BACK, EQUIPMENT_SLOT_BODY,
    EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_END, EQUIPMENT_SLOT_FEET, EQUIPMENT_SLOT_FINGER1,
    EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD, EQUIPMENT_SLOT_LEGS,
    EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_NECK, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_SHOULDERS,
    EQUIPMENT_SLOT_TABARD, EQUIPMENT_SLOT_TRINKET1, EQUIPMENT_SLOT_TRINKET2, EQUIPMENT_SLOT_WAIST,
    EQUIPMENT_SLOT_WRISTS, INVENTORY_SLOT_BAG_0, Item, ItemReadView, ItemStorageTemplate,
    MAX_BAG_SIZE, MAX_ENCHANTMENT_SLOT, MAX_POWERS, MAX_POWERS_PER_CLASS, NULL_SLOT,
    ObjectDataUpdate, PROFESSION_SLOT_COOKING_GEAR1, PROFESSION_SLOT_COOKING_TOOL,
    PROFESSION_SLOT_END, PROFESSION_SLOT_FISHING_TOOL, PROFESSION_SLOT_MAX_COUNT,
    PROFESSION_SLOT_PROFESSION1_GEAR1, PROFESSION_SLOT_PROFESSION1_GEAR2,
    PROFESSION_SLOT_PROFESSION1_TOOL, PROFESSION_SLOT_PROFESSION2_GEAR1,
    PROFESSION_SLOT_PROFESSION2_GEAR2, PROFESSION_SLOT_START, Unit, UnitDataUpdate, UpdateMask,
    item_can_go_into_bag,
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
/// C++ `SKILL_ENGINEERING` (`SharedDefines.h:5388`), read by
/// `Spell::EffectEnergize`'s Runic Mana Injector bonus.
pub const SKILL_ENGINEERING_LIKE_CPP: u16 = 202;
pub const NULL_BAG: u8 = 0;
/// C++ `TRADE_SLOT_COUNT`; kept with the Player-owned `TradeData` projection so
/// the entity crate does not depend on packet serialization.
pub const PLAYER_TRADE_SLOT_COUNT_LIKE_CPP: usize = 7;

pub trait PlayerPowerIndexResolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestStatusRecord {
    pub quest_id: u32,
    pub status: u8,
    pub explored: bool,
    pub accept_time_secs: i64,
    pub end_time_secs: i64,
    pub objective_counts: Vec<i32>,
    pub slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSkillRecord {
    pub skill_line_id: u32,
    pub current_value: u16,
    pub max_value: u16,
    pub step: u16,
    pub profession_slot: i8,
    pub state: PlayerSkillLoadState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerSkillLoadState {
    #[default]
    Unchanged,
    Changed,
    New,
    Deleted,
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
    pub spell_id: i32,
    pub state: PlayerSpellLoadState,
    pub active: bool,
    pub disabled: bool,
    pub favorite: bool,
    pub dependent: bool,
}

pub const PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP: usize = 4;
pub const PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerActionButtonRecord {
    pub button: u8,
    pub action_id: u32,
    pub action_type: u8,
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
    /// C++ `Mail::messageType`; non-normal mail uses the raw sender as the
    /// alternate sender identifier rather than a Player GUID.
    pub message_type: u8,
    pub sender: u64,
    pub receiver: u64,
    pub template_id: Option<u32>,
    pub deliver_time: u64,
    pub expire_time: u64,
    pub checked_flags: u32,
    pub stationery_id: i32,
}

/// C++ `CUFProfile`, owned by `Player::_CUFProfiles` rather than the packet
/// session. Wire conversion remains in `wow-world`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfile {
    pub profile_name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
    pub bool_options: u32,
}

pub const PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP: usize = 19;
pub const PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetTypeLikeCpp {
    Equipment = 0,
    Transmog = 1,
}

impl PlayerEquipmentSetTypeLikeCpp {
    pub fn handler_branch_from_i32_like_cpp(value: i32) -> Option<Self> {
        if value > Self::Transmog.as_i32_like_cpp() {
            return None;
        }
        if value == Self::Equipment.as_i32_like_cpp() {
            Some(Self::Equipment)
        } else {
            Some(Self::Transmog)
        }
    }

    pub const fn as_i32_like_cpp(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetUpdateStateLikeCpp {
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Deleted = 3,
}

/// C++ `EquipmentSetInfo`, owned by `Player::_equipmentSets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEquipmentSetLikeCpp {
    pub raw_set_type: i32,
    pub set_type: PlayerEquipmentSetTypeLikeCpp,
    pub guid: u64,
    pub set_id: u32,
    pub ignore_mask: u32,
    pub pieces: [ObjectGuid; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub appearances: [i32; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub enchants: [i32; 2],
    pub secondary_shoulder_appearance_id: i32,
    pub secondary_shoulder_slot: i32,
    pub secondary_weapon_appearance_id: i32,
    pub secondary_weapon_slot: i32,
    pub assigned_spec_index: i32,
    pub set_name: String,
    pub set_icon: String,
    pub state: PlayerEquipmentSetUpdateStateLikeCpp,
}

impl PlayerEquipmentSetLikeCpp {
    pub fn equipment(
        set_id: u32,
        assigned_spec_index: i32,
        state: PlayerEquipmentSetUpdateStateLikeCpp,
    ) -> Self {
        Self {
            raw_set_type: 0,
            set_type: PlayerEquipmentSetTypeLikeCpp::Equipment,
            guid: 0,
            set_id,
            ignore_mask: 0,
            pieces: [ObjectGuid::EMPTY; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
            appearances: [0; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_shoulder_appearance_id: 0,
            secondary_shoulder_slot: 0,
            secondary_weapon_appearance_id: 0,
            secondary_weapon_slot: 0,
            assigned_spec_index,
            set_name: String::new(),
            set_icon: String::new(),
            state,
        }
    }

    pub fn transmog(
        set_id: u32,
        assigned_spec_index: i32,
        state: PlayerEquipmentSetUpdateStateLikeCpp,
    ) -> Self {
        let mut set = Self::equipment(set_id, assigned_spec_index, state);
        set.raw_set_type = 1;
        set.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
        set
    }
}

/// C++ `VoidStorageItem`, owned by `Player::_voidStorageItems`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVoidStorageItemLikeCpp {
    pub item_id: u64,
    pub item_entry: u32,
    pub creator_guid: ObjectGuid,
    pub fixed_scaling_level: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerGroupState {
    pub group_guid: ObjectGuid,
    pub leader_guid: ObjectGuid,
    pub role_mask: u8,
    pub subgroup: u8,
}

/// C++ `Player::GroupUpdateSequence`, owned per player and group category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerGroupUpdateSequenceLikeCpp {
    pub group_guid: Option<u64>,
    pub update_sequence_number: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerGuildState {
    pub guild_id: Option<u64>,
    pub invited_guild_id: Option<u64>,
    pub rank_id: Option<u32>,
    /// True after C++ `_LoadGuild` resolved membership, including no guild.
    pub authority_complete: bool,
}

/// C++ `TradeData`, uniquely owned by `Player::m_trade` while a trade is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTradeStateLikeCpp {
    pub partner_guid: ObjectGuid,
    pub accepted: bool,
    pub in_accept_process: bool,
    pub client_state_index: u32,
    pub server_state_index: u32,
    pub items: [Option<ObjectGuid>; PLAYER_TRADE_SLOT_COUNT_LIKE_CPP],
    pub money: u64,
    pub spell_id: u32,
    pub spell_cast_item_guid: Option<ObjectGuid>,
}

/// Persistent C++ `Player` capability fields loaded with the character row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerPersistentCapabilityStateLikeCpp {
    pub at_login_flags: u16,
    pub weapon_proficiency: u32,
    pub armor_proficiency: u32,
}

impl PlayerTradeStateLikeCpp {
    pub const fn new(partner_guid: ObjectGuid) -> Self {
        Self {
            partner_guid,
            accepted: false,
            in_accept_process: false,
            client_state_index: 1,
            server_state_index: 1,
            items: [None; PLAYER_TRADE_SLOT_COUNT_LIKE_CPP],
            money: 0,
            spell_id: 0,
            spell_cast_item_guid: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBattlegroundQueueTypeIdLikeCpp {
    pub battlemaster_list_id: u16,
    pub queue_type: u8,
    pub rated: bool,
    pub team_size: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBattlegroundQueueSlotLikeCpp {
    pub slot: u32,
    pub queue_type_id: PlayerBattlegroundQueueTypeIdLikeCpp,
    pub invited_instance_guid: u32,
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

fn representable_power_types() -> [PowerType; MAX_POWERS] {
    [
        PowerType::Mana,
        PowerType::Rage,
        PowerType::Focus,
        PowerType::Energy,
        PowerType::ComboPoints,
        PowerType::Runes,
        PowerType::RunicPower,
        PowerType::SoulShards,
        PowerType::LunarPower,
        PowerType::HolyPower,
        PowerType::AlternatePower,
        PowerType::Maelstrom,
        PowerType::Chi,
        PowerType::Insanity,
        PowerType::BurningEmbers,
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
pub const ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT: usize = 12;
pub const ACTIVE_PLAYER_DATA_NO_REAGENT_COST_MASK_PARENT_BIT: usize = 615;
pub const ACTIVE_PLAYER_DATA_NO_REAGENT_COST_MASK_FIRST_BIT: usize = 616;
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

fn item_ref_by_pos<'a>(items: &[ItemSlotRef<'a>], bag: u8, slot: u8) -> Option<ItemReadView<'a>> {
    items
        .iter()
        .find(|slot_item| slot_item.bag == bag && slot_item.slot == slot)
        .map(|slot_item| slot_item.item)
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

    /// Bag records are held behind a pointer in `PlayerInventoryStorage::bags`; this keeps
    /// the call sites one line, matching the physical-source ceiling on their file.
    pub fn boxed(bag_guid: ObjectGuid, bag_size: u8) -> Box<Self> {
        Box::new(Self::new(bag_guid, bag_size))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInventoryStorage {
    pub items: [Option<ObjectGuid>; PLAYER_SLOT_END],
    /// Only the slots `is_bag_storage_slot` accepts can ever hold a bag, so the record is
    /// reached through a pointer the way C++ reaches it through `Bag*` (`Player.h:1314`,
    /// backed by `Item* m_items[PLAYER_SLOTS_COUNT]` at `Player.h:2950`) rather than being
    /// inlined into all `PLAYER_SLOT_END` slots.
    pub bags: [Option<Box<PlayerBagStorage>>; PLAYER_SLOT_END],
    pub current_buyback_slot: u8,
}

impl PlayerInventoryStorage {
    /// Borrow the bag record at an absolute player slot, hiding the pointer indirection
    /// from callers so they read the same as they did when the record was inlined.
    pub fn bag_at(&self, slot: u8) -> Option<&PlayerBagStorage> {
        self.bags[slot as usize].as_deref()
    }

    pub fn get_item_by_guid_everywhere(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        self.items
            .iter()
            .enumerate()
            .filter(|(slot, _)| !is_buyback_slot(*slot as u8))
            .find_map(|(_, item_guid)| (*item_guid == Some(guid)).then_some(guid))
            .or_else(|| {
                self.bags
                    .iter()
                    .filter_map(Option::as_ref)
                    .flat_map(|bag| bag.slots.into_iter().take(bag.bag_size as usize))
                    .find_map(|item_guid| (item_guid == Some(guid)).then_some(guid))
            })
    }
}

impl Default for PlayerInventoryStorage {
    fn default() -> Self {
        Self {
            items: [None; PLAYER_SLOT_END],
            bags: [const { None }; PLAYER_SLOT_END],
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
    quest_log: [QuestLogValuesLikeCpp; quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP],
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
            quest_log: [QuestLogValuesLikeCpp::default(); quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP],
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
    pub self_res_spells: Vec<i32>,
    pub self_res_spells_update_mask: Option<Vec<u32>>,
    pub no_reagent_cost_mask: [u32; 4],
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
            self_res_spells: Vec::new(),
            self_res_spells_update_mask: None,
            no_reagent_cost_mask: [0; 4],
            quest_completed: [0; QUESTS_COMPLETED_BITS_SIZE],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDataUpdate {
    pub mask: UpdateMask,
    pub values: PlayerDataValues,
    pub quest_log: [QuestLogRowUpdateLikeCpp; quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivePlayerDataUpdate {
    pub mask: UpdateMask,
    pub values: ActivePlayerDataValues,
    /// Immutable projection of the canonical gameplay field for this delta.
    pub local_flags: u32,
    pub rest_info_change_masks: [u8; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
    pub unit_data: Option<UnitDataUpdate>,
    pub player_data: Option<PlayerDataUpdate>,
    pub active_player_data: Option<ActivePlayerDataUpdate>,
    pub derived_stats: Option<PlayerDerivedStatValuesUpdate>,
}

impl PlayerValuesUpdate {
    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug)]
pub struct Player {
    unit: Unit,
    session_id: Option<u64>,
    data: PlayerDataValues,
    active_data: ActivePlayerDataValues,
    inventory: Box<PlayerInventoryStorage>,
    inventory_runtime: Box<PlayerInventoryRuntime>,
    gameplay_state: PlayerGameplayState,
    /// C++ `Player::UpdateAllStats` derived snapshot shared by combat and
    /// update-field publication. It is rebuilt from current inputs and is not
    /// persisted as gameplay state.
    effective_combat_stats: PlayerEffectiveCombatStatsLikeCpp,
    /// C++ `Player::m_swingErrorMsg`, updated by `Player::SetAttackSwingError`.
    swing_error_msg_like_cpp: Option<u8>,
    /// C++ `Player::m_drunkTimer`, reset by `SetDrunkValue` while not sobering.
    drunk_timer: u32,
    deferred_save: deferred_save::DeferredPlayerSave,
    death_operation: death_operation::PlayerDeathOperationState,
    player_xp_table_like_cpp: Option<Arc<Vec<u32>>>,
    player_data_changes: UpdateMask,
    quest_log_change_masks: [u32; quest_log::MAX_QUEST_LOG_SIZE_LIKE_CPP],
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
    duel_arbiter: Option<ObjectGuid>,
}

impl Player {
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

    pub const fn duel_arbiter_like_cpp(&self) -> Option<ObjectGuid> {
        self.duel_arbiter
    }

    pub fn set_duel_arbiter_like_cpp(&mut self, arbiter: Option<ObjectGuid>) {
        self.duel_arbiter = arbiter;
    }

    pub fn set_duel_opponent_in_progress_like_cpp(&mut self, opponent: ObjectGuid) {
        self.duel = Some(PlayerDuelInfoLikeCpp {
            opponent,
            state: PlayerDuelStateLikeCpp::InProgress,
        });
    }

    pub fn clear_duel_like_cpp(&mut self) {
        self.duel = None;
        self.duel_arbiter = None;
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
        inventory_values::clear_player_data_changes_like_cpp(
            &mut self.player_data_changes,
            &mut self.quest_log_change_masks,
        );
    }

    pub fn clear_active_player_data_changes(&mut self) {
        inventory_values::clear_active_player_data_changes_like_cpp(
            &mut self.active_player_data_changes,
            &mut self.active_data,
            &mut self.rest_info_change_masks,
        );
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

    pub fn set_buyback_price(&mut self, slot: usize, price: u32) {
        let update_requested = self.inventory_values_mut().set_buyback_price(slot, price);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn mark_buyback_price_changed(&mut self, slot: usize) {
        let update_requested = self.inventory_values_mut().mark_buyback_price_changed(slot);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn set_buyback_timestamp(&mut self, slot: usize, timestamp: i64) {
        let update_requested = self
            .inventory_values_mut()
            .set_buyback_timestamp(slot, timestamp);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn mark_buyback_timestamp_changed(&mut self, slot: usize) {
        let update_requested = self
            .inventory_values_mut()
            .mark_buyback_timestamp_changed(slot);
        self.apply_inventory_value_update_request(update_requested);
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

    fn mark_player_data(&mut self, bit: usize) {
        self.player_data_changes.set(PLAYER_DATA_PARENT_BIT);
        self.player_data_changes.set(bit);
        self.unit
            .world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
    }

    fn mark_player_data_array(
        &mut self,
        parent_bit: usize,
        first_element_bit: usize,
        index: usize,
    ) {
        self.player_data_changes.set(parent_bit);
        self.player_data_changes.set(first_element_bit + index);
        self.unit
            .world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
    }

    fn mark_active_player_data(&mut self, bit: usize) {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes.set(bit);
        self.unit
            .world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
    }

    fn mark_active_player_data_section(&mut self, parent_bit: usize, bit: usize) {
        self.active_player_data_changes
            .set(ACTIVE_PLAYER_DATA_PARENT_BIT);
        self.active_player_data_changes.set(parent_bit);
        self.active_player_data_changes.set(bit);
        self.unit
            .world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
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
        self.unit
            .world_mut()
            .object_mut()
            .add_to_object_update_if_needed();
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

pub fn is_buyback_slot(slot: u8) -> bool {
    (BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot)
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
