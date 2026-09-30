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
mod away_status;
mod collections;
mod damage_control;
mod deferred_save;
mod identity;
mod inventory_runtime;
pub use inventory_runtime::{ItemObjectUpdateLikeCpp, PlayerInventoryRuntime};
pub use wow_data_model::power::PlayerPowerIndexResolver;
mod battleground;
mod cinematic;
mod collection_state;
mod combat;
mod construction;
mod difficulty;
mod effective_stats;
mod equipment_sets;
mod inventory_positions;
mod inventory_storage;
mod item_effect_actions;
mod item_modifiers;
mod items;
mod location;
mod menu;
mod movement_control;
mod pending_spell_cast;
mod persistent_capabilities;
mod pet_lifecycle;
mod progression;
mod pvp;
mod quest_state;
mod reputation;
mod rest;
mod spell_runtime;
mod storage_helpers;
mod talent_runtime;
mod taxi_state;
mod trade;
mod update_values;
pub use update_values::{
    ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT,
    ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT, ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT,
    ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT, ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT,
    ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT, ACTIVE_PLAYER_DATA_COINAGE_BIT,
    ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT, ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT,
    ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT, ACTIVE_PLAYER_DATA_FARSIGHT_OBJECT_BIT,
    ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT, ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT,
    ACTIVE_PLAYER_DATA_HONOR_BIT, ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT,
    ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT, ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT,
    ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT, ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT,
    ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT, ACTIVE_PLAYER_DATA_PARENT_BIT,
    ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT, ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT,
    ACTIVE_PLAYER_DATA_REST_INFO_FIRST_BIT, ACTIVE_PLAYER_DATA_REST_INFO_PARENT_BIT,
    ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT,
    ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT,
    ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT, ACTIVE_PLAYER_DATA_TOYS_BIT,
    ACTIVE_PLAYER_DATA_TRANSMOG_BIT, ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT,
    ACTIVE_PLAYER_DATA_XP_BIT, ActivePlayerDataUpdate, ActivePlayerDataValues,
    PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT, PLAYER_DATA_CURRENT_SPEC_ID_BIT,
    PLAYER_DATA_FLAGS_BIT, PLAYER_DATA_FLAGS_EX_BIT, PLAYER_DATA_HONOR_LEVEL_BIT,
    PLAYER_DATA_INEBRIATION_BIT, PLAYER_DATA_LOOT_TARGET_GUID_BIT, PLAYER_DATA_NATIVE_SEX_BIT,
    PLAYER_DATA_NUM_BANK_SLOTS_BIT, PLAYER_DATA_PARENT_BIT, PLAYER_DATA_PARTY_TYPE_FIRST_BIT,
    PLAYER_DATA_PARTY_TYPE_PARENT_BIT, PLAYER_DATA_PLAYER_TITLE_BIT,
    PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT, PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT,
    PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, PlayerDataUpdate, PlayerDataValues,
    PlayerRestInfoValueLikeCpp, PlayerValuesUpdate, QUESTS_COMPLETED_BITS_PER_BLOCK,
    QUESTS_COMPLETED_BITS_SIZE, VisibleItemValues,
};
mod vehicle;
mod world_local;
pub use battleground::PlayerBattlegroundState;
pub use cinematic::PlayerCinematicStateLikeCpp;
pub use collection_state::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
    AppearanceAcquisitionSource, AppearanceAdmissionSource, AppearanceModifiedFacts,
    AppearanceSearchFacts, AppearanceSparseFacts, AppearanceStorageFacts,
    DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP, PlayerCollectionStateLikeCpp, RuntimeAppearanceRoute,
};
pub use effective_stats::PlayerEffectiveCombatStatsLikeCpp;
pub use equipment_sets::PlayerEquipmentSetsLikeCpp;
pub use inventory_positions::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BANK_SLOT_ITEM_END, BANK_SLOT_ITEM_START,
    BUYBACK_SLOT_COUNT, BUYBACK_SLOT_END, BUYBACK_SLOT_START, CHILD_EQUIPMENT_SLOT_END,
    CHILD_EQUIPMENT_SLOT_START, INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_END,
    INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START,
    ITEM_LIMIT_CATEGORY_MODE_EQUIP, ITEM_LIMIT_CATEGORY_MODE_HAVE, KEYRING_SLOT_END,
    KEYRING_SLOT_START, PLAYER_SLOT_END, REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, is_bag_pos,
    is_bank_packed_pos, is_bank_pos, is_child_equipment_packed_pos, is_child_equipment_pos,
    is_equipment_packed_pos, is_equipment_pos, is_inventory_packed_pos, is_inventory_pos,
    make_item_pos,
};
pub use inventory_storage::{PlayerBagStorage, PlayerInventoryItem, PlayerInventoryStorage};
pub use item_effect_actions::{
    item_resistance_bonus_actions_like_cpp, item_scaling_stat_bonus_actions_like_cpp,
    item_shield_block_bonus_action_like_cpp, item_stat_bonus_actions_like_cpp,
    item_weapon_damage_actions_like_cpp,
};
pub use item_modifiers::{
    PlayerItemBonusStateLikeCpp, PlayerItemLevelCapsLikeCpp, PlayerItemModifierRuntimeStateLikeCpp,
    PlayerItemSetEffectLikeCpp, loaded_enchantment_effect_action_is_unrepresented_like_cpp,
    represented_item_bonus_action_updates_stats_like_cpp,
};
pub use items::{
    ExistingStorageStackUpdateLikeCpp, InventoryStorageMovePlanLikeCpp,
    plan_inventory_storage_move_like_cpp,
};
pub use items::{
    represented_avg_total_item_level_maybe_replace_slot_like_cpp,
    represented_total_avg_equipment_slot_candidates_like_cpp,
};
pub use progression::PreparedPlayerSpellAcquisitionLikeCpp;
pub use quest_state::{
    PlayerQuestGameplayState, QuestBoundItemObjectiveProgressLikeCpp,
    QuestItemObjectiveProgressLikeCpp, SeasonalQuestBitReset, SeasonalQuestResetOutcome,
    SeasonalQuestResetPlan, SeasonalQuestResetReason,
};
pub use reputation::{
    PlayerFactionStateLikeCpp, PlayerReputationStateLikeCpp, ReputationRankCounterLikeCpp,
    ReputationRankCountersLikeCpp,
};
pub use rest::PlayerRestState;
pub use spell_runtime::{
    ForgottenKnownSpellLikeCpp, LearnedSkillInput, LearnedSkillLookup, LearnedSkillNode,
    LearnedSkillOperation, LearnedSkillRange, LearnedSkillStep, LearnedSkillWrite,
    LoadedSpellDependency, LoadedSpellInput, LoadedSpellReconstruction, LoadedSpellStep,
    PlayerSpellAcquisitionSnapshotLikeCpp, PlayerSpellRuntimeState, SpellUnlearnEdge,
    SpellUnlearnInput, SpellUnlearnOperation, SpellUnlearnOwnerOutcome, SpellUnlearnOwnerStep,
    SpellUnlearnStep,
};
pub use storage_helpers::is_buyback_slot;
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
pub use save_ack::{PlayerSaveAcknowledgementLikeCpp, PlayerSavedGroupsLikeCpp};
mod social;
mod spellbook;
mod trait_config;
mod transport_and_faction;
pub use trait_config::{PlayerTraitConfigDetails, PlayerTraitConfigState, PlayerTraitEntry};
mod lifecycle_models;
mod visibility;
mod vitals;
mod void_storage;
pub use lifecycle_models::{
    PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP, PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP,
    PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP, PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP,
    PlayerAchievementCriteriaRecord, PlayerAchievementRecord, PlayerActionButtonRecord,
    PlayerBattlegroundQueueRecord, PlayerBattlegroundQueueSlotLikeCpp,
    PlayerBattlegroundQueueTypeIdLikeCpp, PlayerCreateLifecycleRecord, PlayerCufProfile,
    PlayerCurrencyRecord, PlayerCustomizationChoice, PlayerDbLoadLifecycleRecord,
    PlayerDuelInfoLikeCpp, PlayerDuelStateLikeCpp, PlayerEquipmentSetLikeCpp,
    PlayerEquipmentSetTypeLikeCpp, PlayerEquipmentSetUpdateStateLikeCpp, PlayerGameplayLoadPlan,
    PlayerGameplayLoadRecord, PlayerGameplayLoadStep, PlayerGroupState,
    PlayerGroupUpdateSequenceLikeCpp, PlayerGuildState, PlayerKnownSpellRecord,
    PlayerLifecycleMetadata, PlayerLifecyclePower, PlayerLoginLifecyclePlan,
    PlayerLoginLifecycleStep, PlayerMailRecord, PlayerPersistentCapabilityStateLikeCpp,
    PlayerQuestStatusRecord, PlayerRandomBattlegroundState, PlayerSkillLoadState,
    PlayerSkillRecord, PlayerSocialState, PlayerSpellChargeRecord, PlayerSpellCooldownRecord,
    PlayerSpellLoadState, PlayerTradeStateLikeCpp, PlayerTransportState,
    PlayerVoidStorageItemLikeCpp, PlayerWorldInsertionState,
};

mod inventory_models;
pub use inventory_models::{
    BagTemplateRef, CanBankItemArgs, CanStoreItemArgs, CanStoreItemOutcome,
    CanTakeMoreSimilarItemsArgs, CanTakeMoreSimilarItemsOutcome, ItemLimitCategoryTemplate,
    ItemPosCount, ItemSearchCallbackResult, ItemSearchLocation, ItemSlotRef, ItemStorageRef,
    PlayerStorageError,
};
mod equipment_models;
pub use equipment_models::{
    CanEquipItemArgs, CanEquipItemOutcome, CanEquipUniqueItemArgs, CanEquipUniqueItemTemplateArgs,
    CanUnequipItemArgs, CanUseItemArgs, CanUseItemTemplateArgs, EquipItemObjectOutcome,
    EquippedGemRef, FindEquipSlotArgs, SocketedGemUniqueRef, TitanGripPenaltyAction,
};
mod destruction_models;
pub use destruction_models::{
    DestroyFilteredItemAction, DestroyFilteredItemRef, DestroyItemCountAction,
    DestroyItemCountItemRef, DestroyItemCountPlan,
};
mod swap_models;
pub use swap_models::{
    SwapBagItemMove, SwapBagItemRef, SwapBagRef, SwapItemBagExchangePlan,
    SwapItemBagExchangeResult, SwapItemEmptyDestinationPlan, SwapItemEmptyDestinationResult,
    SwapItemErrorItemOrder, SwapItemMergeFillPlan, SwapItemMergeFillResult, SwapItemMissingPhase,
    SwapItemOrchestrationPlan, SwapItemOrchestrationResult, SwapItemPreflightItem,
    SwapItemPreflightPlan, SwapItemPreflightResult, SwapItemRealSwapExecutionPlan,
    SwapItemRealSwapTarget, SwapItemRealSwapValidationPlan, SwapItemRealSwapValidationResult,
    SwapItemRealSwapValidationSubject,
};
mod duration_models;
pub use duration_models::{
    ItemDurationRef, PlayerEnchantDuration, PlayerEnchantDurationItemRef, PlayerEnchantTimeUpdate,
    PlayerItemTimeUpdate, SoulboundTradeableItemRef, UpdateEnchantTimeAction,
    UpdateItemDurationAction,
};
mod enchantment_models;
pub use enchantment_models::{
    APPLY_ENCHANTMENT_RANDOM_SUFFIX_EFFECTS, ApplyEnchantmentArgs, ApplyEnchantmentBaseMod,
    ApplyEnchantmentCombatRating, ApplyEnchantmentDurationAction, ApplyEnchantmentEffectAction,
    ApplyEnchantmentEffectKind, ApplyEnchantmentEffectRef, ApplyEnchantmentGemRequirementRef,
    ApplyEnchantmentPlan, ApplyEnchantmentRandomSuffixRef, ApplyEnchantmentResult,
    ApplyEnchantmentSkipReason, ApplyEnchantmentSocketContext, ApplyEnchantmentTemplateRef,
    ApplyEnchantmentUnitMod, ApplyEnchantmentUnitModifier, ArenaEnchantmentItemRef,
    RemoveArenaEnchantmentAction, SkillEnchantmentItemRef, SkillEnchantmentTemplateRef,
    UpdateSkillEnchantmentAction, UpdateSkillEnchantmentReason, WeaponDamageBoundLikeCpp,
};
mod publication_models;
pub use publication_models::{
    SendNewItemArgs, SendNewItemDelivery, SendNewItemDisplayText, SendNewItemInstancePlan,
    SendNewItemModifier, SendNewItemPlan, SendNewItemTemplateRef,
};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use crate::PlayerGameplayState;
use bitflags::bitflags;
use item_effect_actions::{
    apply_enchantment_effect_action, arena_enchantment_ref_by_guid, is_socket_enchantment_slot,
    push_arena_inventory_enchantment_action, push_update_skill_enchantment_action,
    skill_enchantment_transition,
};
use lifecycle_models::PlayerLifecycleBase;
use storage_helpers::{
    can_equip_item_outcome, can_store_item_count_zero, can_store_item_error,
    can_take_more_similar_ok, destroy_filtered_scan_bag_ranges,
    destroy_filtered_scan_top_level_range, destroy_item_count_scan_bag_ranges,
    destroy_item_count_scan_top_level_range, equip_slot_candidates,
    equipped_gem_limit_category_count, equipped_item_limit_category_count, has_equipped_gem_entry,
    has_equipped_item_entry, is_bag_storage_slot, paired_unique_ignore_slot,
    swap_item_real_swap_target_for_destination, validate_split_source,
};
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
/// C++ `SKILL_ENGINEERING` (`SharedDefines.h:5388`), read by
/// `Spell::EffectEnergize`'s Runic Mana Injector bonus.
pub const SKILL_ENGINEERING_LIKE_CPP: u16 = 202;
pub const NULL_BAG: u8 = 0;
/// C++ `TRADE_SLOT_COUNT`; kept with the Player-owned `TradeData` projection so
/// the entity crate does not depend on packet serialization.
pub const PLAYER_TRADE_SLOT_COUNT_LIKE_CPP: usize = 7;

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

fn item_ref_by_pos<'a>(items: &'a [ItemSlotRef<'a>], bag: u8, slot: u8) -> Option<&'a Item> {
    items
        .iter()
        .find(|slot_item| slot_item.bag == bag && slot_item.slot == slot)
        .map(|slot_item| slot_item.item)
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

#[derive(Debug, Clone, PartialEq)]
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
    deferred_save: deferred_save::DeferredPlayerSave,
    player_xp_table_like_cpp: Option<Arc<Vec<u32>>>,
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

    pub fn set_selection(&mut self, guid: ObjectGuid) {
        self.unit.set_target(guid);
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
}

#[cfg(test)]
#[path = "../player_tests.rs"]
mod tests;
