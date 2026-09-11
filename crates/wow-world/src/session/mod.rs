// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession` — per-player session that receives packets from the
//! [`WorldSocket`](wow_network::WorldSocket) and dispatches them to handlers.

mod admission;
mod appearance;
mod connection;
mod deferred_visibility;
pub use crate::player_directory as directory;
mod dispatch;
mod driver;
mod lifecycle;
pub use lifecycle::PlayerSaveOutcomeLikeCpp;
mod combat;
mod effect_learning;
mod instances;
mod legacy_runtime;
use legacy_runtime::*;
// The legacy tick entry points are called from world-server as
// `wow_world::session::run_legacy_*`. `legacy_runtime` is private, so the
// original external path is preserved by re-exporting them here.
pub use legacy_runtime::{
    run_legacy_creature_aggro_tick_once_like_cpp,
    run_legacy_creature_aggro_tick_once_with_config_like_cpp,
    run_legacy_creature_lifecycle_tick_once_like_cpp, run_legacy_creature_melee_tick_once_like_cpp,
    run_legacy_creature_movement_tick_once_like_cpp, run_legacy_creature_spell_tick_once_like_cpp,
    run_legacy_player_melee_tick_once_like_cpp,
};
mod canonical_access;
mod catalogs;
mod chat;
mod collections;
mod lifecycle_ops;
mod loot;
pub mod mailbox;
mod money;
mod movement;
mod persistence;
mod pets;
mod player_cast;
mod player_items;
mod progression;
mod publication;
mod quest;
pub mod registry;
mod social;
mod spell_effects;
mod spell_state;
mod taxi;
mod test_support;
mod trainer_acquisition;
mod trait_configs;
mod visibility;
mod world_entities;
mod world_state;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::{
    Arc, Mutex, OnceLock, Weak,
    atomic::{AtomicBool, AtomicI64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::{Rng, RngCore, SeedableRng, rngs::StdRng, seq::SliceRandom};
use tracing::{debug, info, trace, warn};

use crate::battle_pet_account::{
    BattlePetAccountAttachmentLikeCpp, BattlePetAddFailureLikeCpp, BattlePetAddOutcomeLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetAddRequestLikeCpp, BattlePetMutationFailureLikeCpp,
};
use crate::entity_update_bridge::{
    bag_values_update_to_update_object, dynamic_object_values_update_to_update_object,
    game_object_values_update_to_update_object, item_values_update_to_update_object,
    player_values_update_to_update_object, unit_values_update_to_packet,
    unit_values_update_to_update_object,
};
use crate::loot_persistence::{
    DurableLootMoneyCompletionLikeCpp, DurableLootMoneyPersistenceGuardLikeCpp,
    DurableLootMoneyPersistenceTrackerLikeCpp, DurableLootMoneySaveFenceLikeCpp,
};
use crate::map_manager::{
    PendingRespawn, RecipientRule, RuntimeEvent, RuntimeOutput, RuntimePlan, RuntimeTickOwner,
    WorldMMapPathfinderWorkerLikeCpp,
};
use crate::phasing::{init_db_phase_shift_like_cpp, init_db_visible_map_id_like_cpp};
use crate::reputation::{ReputationMgrLikeCpp, reputation_to_rank_like_cpp};
use crate::session::directory::{
    PlayerRegistry, PlayerSessionRegistrationLikeCpp, PlayerVisibilityCreateSnapshot,
};
use crate::session::mailbox::{
    CreatureAttackStartLikeCppCommand, GameEventQuestCompleteClientOutcomeLikeCpp,
    GameEventQuestCompleteCommandLikeCpp, KickLikeCppCommand, LootRollCommandIdentityLikeCpp,
    NotifyLootMoneyRemovedLikeCppCommand, SendIfVisibleLikeCppCommand, SessionCommand,
    SharedClientVisibleGuidsLikeCpp,
};
use crate::session_policy::{
    ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp,
    LootDropRatesLikeCpp, PacketSpoofConfigLikeCpp, ReputationRatesLikeCpp,
};
use wow_ai::{
    CURRENT_EXPANSION_LIKE_CPP, CreatureAiCanAttackInputLikeCpp, CreatureAiKindLikeCpp,
    CreatureAiSelectionInputLikeCpp, CreatureAttackDistanceInputLikeCpp,
    creature_ai_can_attack_like_cpp, creature_ai_uses_base_move_in_line_of_sight_like_cpp,
    creature_attack_distance_like_cpp, max_level_for_expansion_like_cpp,
    select_creature_ai_like_cpp,
};
use wow_constants::creature::{CreatureFlagsExtra, CreatureType, CreatureTypeFlags};
use wow_constants::item::{
    CurrencyTypes, CurrencyTypesFlags, EnchantmentSlot, ItemBonusType, ItemFieldFlags,
    ItemFieldFlags2,
};
use wow_constants::movement::MovementFlag;
use wow_constants::shared::DifficultyFlags;
use wow_constants::unit::{
    Gender, NPCFlags1, PowerType, SheathState, Team, UnitFlags, UnitFlags2, UnitPvpFlags,
    UnitStandStateType, WeaponAttackType,
};
use wow_constants::{
    BagFamilyMask, BuyResult, ClientOpcodes, InventoryResult, InventoryType, ItemBondingType,
    ItemClass, ItemContext, ItemEnchantmentType, ItemFlags, ItemFlags2, ItemFlags3, ItemModifier,
    ItemQuality, ItemSpelltriggerType, ItemSubClassArmor, ItemSubClassWeapon, SellResult,
    ServerOpcodes, SpellCastResult, SpellItemEnchantmentFlags, Stats, TypeId, UnitState,
};
use wow_core::{
    EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position,
    VoidStorageItemIdGeneratorLikeCpp, guid::HighGuid,
};
use wow_data::character_progression::{ChrClassesStore, ChrRacesStore, PowerTypeStore};
use wow_data::trait_tree::{TraitDefinitionStore, TraitNodeEntryStore};
use wow_data::{
    AccessRequirementStoreLikeCpp, AdventureMapPoiStore, AreaTableStore, AreaTriggerDb2Store,
    AreaTriggerScriptStoreLikeCpp, AreaTriggerStore, BankBagSlotPricesStore, BattlemasterListStore,
    ChrSpecializationStore, CinematicSequencesStore, CombatRatingsGameTableLikeCpp,
    ConditionEntriesByTypeStore, CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp,
    CreatureClassificationHealthRatesLikeCpp, CreatureDifficultyStoreLikeCpp,
    CreatureDisplayInfoExtraStore, CreatureDisplayInfoStore, CreatureEquipmentStoreLikeCpp,
    CreatureModelDataStore, CreatureSpellDisableDecisionLikeCpp,
    CreatureTemplateLifecycleStoreLikeCpp, CreatureTemplateMountStoreLikeCpp, CurrencyTypesEntry,
    CurrencyTypesStore, DISABLE_TYPE_BATTLEGROUND, DISABLE_TYPE_MAP, DifficultyStore,
    DisableMgrLikeCpp, DisableWorldObjectRefLikeCpp, DurabilityCostsStore, DurabilityQualityStore,
    EmotesStore, EmotesTextStore, ExplorationBaseXpStoreLikeCpp, FishingBaseSkillStoreLikeCpp,
    GameObjectDisplayInfoStore, GameObjectTemplateLifecycleStoreLikeCpp, GemPropertiesStore,
    GlyphPropertiesStore, GraveyardStore, HeirloomEntry, HeirloomStore, HotfixBlobCache,
    ImportPriceStores, ItemAppearanceStore, ItemBonusDb2Store, ItemChildEquipmentEntry,
    ItemChildEquipmentStore, ItemClassStore, ItemCurrencyCostStore, ItemDisenchantLootStore,
    ItemEffectStore, ItemExtendedCostStore, ItemLimitCategoryConditionStore,
    ItemLimitCategoryStore, ItemModifiedAppearanceStore, ItemPriceBaseStore,
    ItemRandomEnchantmentTemplateStore, ItemRandomPropertiesStore, ItemRandomPropertyTemplateEntry,
    ItemRandomSuffixStore, ItemSearchNameStore, ItemSetSpellStore, ItemSetStore,
    ItemSpecOverrideStore, ItemStatsStore, ItemStore, LfgDungeonStoreLikeCpp, LfgDungeonsStore,
    LockStore, MapDifficultyStore, MapDifficultyXConditionStore, MapStore, MountCapabilityStore,
    MountDefinitionStoreLikeCpp, MountStore, MountTypeXCapabilityStore, MountXDisplayStore,
    MovieStore, NpcSpellClickStoreLikeCpp, PhaseGroupStore, PhaseStore, PlayerConditionAuraLikeCpp,
    PlayerConditionContextLikeCpp, PlayerConditionCountLikeCpp, PlayerConditionPartyStatusLikeCpp,
    PlayerConditionQuestKillLikeCpp, PlayerConditionReputationLikeCpp, PlayerConditionSkillLikeCpp,
    PlayerConditionStore, PlayerCreateInfoCastSpellStoreLikeCpp,
    PlayerCreateInfoCustomSpellStoreLikeCpp, PlayerCreateInfoStoreLikeCpp, PlayerStatsStore,
    PvpItemStore, RandPropPointsStore, SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, ScriptIdLikeCpp,
    ScriptNameInternerLikeCpp, ShieldBlockRegularGameTableLikeCpp, SkillLineStore,
    SkillRangeTypeLikeCpp, SkillStore, SkillTiersStoreLikeCpp, SpellAcquisitionCatalogLikeCpp,
    SpellAreaLikeCpp, SpellAreaStoreLikeCpp, SpellAuraOptionsStore, SpellAuraRestrictionsStore,
    SpellCategoryStore, SpellChainStoreLikeCpp, SpellCustomAttributeStoreLikeCpp,
    SpellDurationStore, SpellEquippedItemsEntry, SpellEquippedItemsStore,
    SpellGroupStackRuleLikeCpp, SpellGroupStackRuleStoreLikeCpp, SpellGroupStoreLikeCpp,
    SpellItemEnchantmentConditionStore, SpellItemEnchantmentStore, SpellLearnSkillLookupLikeCpp,
    SpellLearnSkillNodeLikeCpp, SpellLearnSkillStoreLikeCpp, SpellLearnSpellNodeLikeCpp,
    SpellLearnSpellStoreLikeCpp, SpellLevelsStore, SpellLinkedStoreLikeCpp, SpellLinkedTypeLikeCpp,
    SpellMiscStore, SpellPetAuraStoreLikeCpp, SpellProcEntryLikeCpp, SpellProcStoreLikeCpp,
    SpellRadiusStore, SpellRangeStore, SpellRequiredStoreLikeCpp, SpellShapeshiftFormStore,
    SpellStore, SpellTargetPositionStoreLikeCpp, SpellTargetRestrictionsStore,
    SpellThreatEntryLikeCpp, SpellThreatStoreLikeCpp, SummonPropertiesEntry, TactKeyStore,
    TalentStore, TalentTabStore, TavernAreaTriggerStoreLikeCpp, ToyStore, TrainerStoreLikeCpp,
    TransmogSetEntry, TransmogSetItemStore, TrinityStringStoreLikeCpp,
    VEHICLE_SEAT_FLAG_CAN_ATTACK, VehicleAccessoryStoreLikeCpp, VehicleSeatStore, VehicleStore,
    WorldSafeLocStore, is_player_meeting_condition_like_cpp,
    progression_rewards::{
        ContentTuningStore, CurvePointStore, CurveStore, FactionEntry, FactionStore,
        FactionTemplateStore, FriendshipRepReactionStore, NumTalentsAtLevelStore,
        ParagonReputationStore, QuestFactionRewardStore, QuestInfoStore, QuestMoneyRewardStore,
        QuestPackageItemStore, QuestV2Store, ScalingStatDistributionEntry,
        ScalingStatDistributionStore, ScalingStatValuesStore,
    },
    reputation::{
        CreatureOnKillReputationStoreLikeCpp, RepSpilloverTemplateStoreLikeCpp,
        ReputationRewardRateStoreLikeCpp,
    },
    spell_click::{
        SPELL_CLICK_USER_FRIEND_LIKE_CPP, SPELL_CLICK_USER_PARTY_LIKE_CPP,
        SPELL_CLICK_USER_RAID_LIKE_CPP, UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
    },
    spell_duration_ms_like_cpp, spell_effect_radius_like_cpp,
};
#[cfg(test)]
use wow_data::{
    BattlePetBreedQualityStore, BattlePetBreedStateStore, BattlePetSpeciesStateStore,
    BattlePetSpeciesStore, BattlePetXpGameTableLikeCpp, calculate_battle_pet_stats_like_cpp,
};
#[cfg(test)]
use wow_data::{
    PetDefaultSpellStoreLikeCpp, PetDefaultSpellsEntryLikeCpp, PetFamilySpellStoreLikeCpp,
    PetLevelupSpellSetLikeCpp, PetLevelupSpellStoreLikeCpp, ServersideSpellInfoLikeCpp,
    ServersideSpellStoreLikeCpp, SpellEnchantProcEntryLikeCpp, SpellEnchantProcStoreLikeCpp,
    SpellTotemModelStoreLikeCpp, VehicleTemplateStoreLikeCpp,
};
#[cfg(test)]
use wow_entities::TitanGripPenaltyAction;
use wow_entities::player_rules::is_using_two_handed_weapon_in_one_hand_template as two_handed_in_one_hand_like_cpp;
use wow_entities::{
    AccessorObjectKind, ActiveState, ApplyEnchantmentArgs, ApplyEnchantmentDurationAction,
    ApplyEnchantmentEffectAction, ApplyEnchantmentEffectRef, ApplyEnchantmentGemRequirementRef,
    ApplyEnchantmentPlan, ApplyEnchantmentRandomSuffixRef, ApplyEnchantmentResult,
    ApplyEnchantmentSocketContext, ApplyEnchantmentTemplateRef, BANK_SLOT_BAG_END,
    BANK_SLOT_BAG_START, BUYBACK_SLOT_COUNT, BUYBACK_SLOT_END, BUYBACK_SLOT_START, BagTemplateRef,
    CanBankItemArgs, CanEquipItemArgs, CanEquipItemOutcome, CanEquipUniqueItemArgs,
    CanStoreItemArgs, CanUnequipItemArgs, CanUseItemArgs, CanUseItemTemplateArgs,
    CreatureAddonLifecycleRecordLikeCpp, EQUIPMENT_SLOT_BACK, EQUIPMENT_SLOT_BODY,
    EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_END, EQUIPMENT_SLOT_FEET, EQUIPMENT_SLOT_FINGER1,
    EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD, EQUIPMENT_SLOT_LEGS,
    EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_NECK, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_RANGED,
    EQUIPMENT_SLOT_SHOULDERS, EQUIPMENT_SLOT_TABARD, EQUIPMENT_SLOT_TRINKET1,
    EQUIPMENT_SLOT_TRINKET2, EQUIPMENT_SLOT_WAIST, EQUIPMENT_SLOT_WRISTS, EquippedGemRef,
    GAMEOBJECT_TYPE_GUILD_BANK, GameObject, INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END,
    INVENTORY_SLOT_ITEM_START, ITEM_DATA_BITS, ITEM_DATA_CONTAINED_IN_BIT, ITEM_DATA_CREATOR_BIT,
    ITEM_DATA_DURABILITY_BIT, ITEM_DATA_DYNAMIC_FLAGS_BIT, ITEM_DATA_DYNAMIC_FLAGS2_BIT,
    ITEM_DATA_ENCHANTMENT_FIRST_BIT, ITEM_DATA_ENCHANTMENT_PARENT_BIT, ITEM_DATA_PARENT_BIT,
    ITEM_DATA_PROPERTY_SEED_BIT, ITEM_DATA_RANDOM_PROPERTIES_ID_BIT, Item, ItemCreateInfo,
    ItemDataUpdate, ItemLimitCategoryTemplate, ItemPosCount, ItemSlotRef, ItemStorageRef,
    ItemStorageTemplate, ItemValuesUpdate, MAX_BAG_SIZE, MAX_ITEM_SPELLS, MAX_MONEY_AMOUNT,
    MAX_POWERS, MAX_POWERS_PER_CLASS, MovementGeneratorKind, MovementSlot, NULL_BAG, NULL_SLOT,
    PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, PLAYER_SLOT_END, PROFESSION_SLOT_END, Pet, PetAuraLikeCpp,
    PetDeclinedNamesLikeCpp, PetSaveMode, PetSpellState, PetSpellType, PetStable, PetStableInfo,
    PetType, PhaseShift, Player, PlayerEnchantTimeUpdate, PlayerInteractionDataLikeCpp,
    PlayerInventoryRuntime, PlayerItemTimeUpdate, PlayerPetLifecycleStateLikeCpp,
    PlayerQuestGameplayState, PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp,
    PlayerTeleportStateLikeCpp, QUESTS_COMPLETED_BITS_PER_BLOCK, QUESTS_COMPLETED_BITS_SIZE,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, ReactState, SendNewItemDelivery,
    SendNewItemDisplayText, SendNewItemPlan, SocketedGemUniqueRef, SwapItemPreflightItem,
    SwapItemPreflightPlan, TYPEID_CONTAINER, TYPEID_ITEM, UNIT_DATA_BITS,
    UNIT_DATA_EMOTE_STATE_BIT, UNIT_DATA_HEALTH_BIT, UNIT_DATA_MODS_PARENT_BIT, Unit,
    UnitDataUpdate, UnitDataValues, UnitVisibilityDetectionStateLikeCpp, UpdateMask, Vehicle,
    VehicleAccessory, VisibleItemValues, WorldObject,
    explored_zones_db_string_from_blocks_like_cpp, is_bag_pos, is_equipment_packed_pos,
    is_inventory_pos, item_resistance_bonus_actions_like_cpp,
    item_scaling_stat_bonus_actions_like_cpp, item_shield_block_bonus_action_like_cpp,
    item_stat_bonus_actions_like_cpp, item_weapon_damage_actions_like_cpp, make_item_pos,
    parse_explored_zones_db_string_like_cpp,
};
use wow_entities::{
    BagValuesUpdate, CONTAINER_DATA_BITS, CONTAINER_DATA_SLOTS_FIRST_BIT,
    CONTAINER_DATA_SLOTS_PARENT_BIT, ContainerDataUpdate, ContainerDataValues,
};
pub(crate) use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_handler::{PacketProcessing, SessionStatus};

// Only the test modules mounted into this file name these directly; the
// production surface reaches them through `wow-session` since #297.
#[cfg(test)]
use wow_network::{SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp};

use registry::{PacketHandlerEntry, build_dispatch_table};
use wow_loot::{
    LootClaimLease, LootStoreKind, LootStores, OwnedLootAuthority, OwnedLootAuthorityLifecycle,
    OwnedLootAuthorityStamp, OwnedLootScope, OwnedLootSnapshot,
};
use wow_map::coords::SIZE_OF_GRID_CELL;
use wow_network::SocketTimeoutsLikeCpp;
use wow_network::session_mgr::SessionManager;
use wow_packet::packets::chat::{ChatMsg, ChatPkt, PrintNotification};
use wow_packet::packets::gossip::ClientGossipText;
use wow_packet::packets::item::{
    InventoryChangeFailure, ItemEnchantTimeUpdate, ItemInstance, ItemMod, ItemModList,
    ItemPushResult, ItemPushResultDisplayType, ItemTimeUpdate,
};
use wow_packet::packets::misc::{
    AccountHeirloom, AccountHeirloomUpdate, AccountMount, AccountMountUpdate, AccountToy,
    AccountToyUpdate, BuyFailed, DungeonDifficultySet, EQUIP_ERR_NOT_ENOUGH_MONEY_LIKE_CPP,
    FeatureSystemConfigLikeCpp, FeatureSystemStatus, FeatureSystemStatusGlueScreen,
    MOUNT_RESULT_SHAPESHIFTED_LIKE_CPP, MountResult, NUM_ACCOUNT_DATA_TYPES, RaidDifficultySet,
    SellResponse, SetProficiency, SetupCurrency, SetupCurrencyRecord, SpellChargeEntry,
    SpellHistoryEntry, TRADE_SLOT_COUNT_LIKE_CPP, TRADE_STATUS_ACCEPTED_LIKE_CPP,
    TRADE_STATUS_CANCELLED_LIKE_CPP, TRADE_STATUS_STATE_CHANGED_LIKE_CPP,
    TRADE_STATUS_UNACCEPTED_LIKE_CPP, TradeStatus,
};
use wow_packet::packets::quest::{
    QuestGiverOfferReward, QuestGiverQuestDetails, QuestGiverQuestList, QuestGiverRequestItems,
    QuestGiverRequestItemsCollect, QuestGiverRequestItemsCurrency, QuestListEntry,
    QuestObjectiveSimple, QuestRewardsBlock,
};
use wow_packet::packets::spell::SpellTargetData;
use wow_social::group::{
    GroupInfo, GroupInstanceResetMethodLikeCpp, GroupInstanceResetResultLikeCpp, GroupRegistry,
    PendingInvites, group_guid_by_db_store_id_like_cpp,
};

const QUEST_OBJECTIVE_ITEM_LIKE_CPP: u8 = 1;
const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP: u8 = 4;
const QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP: u8 = 6;
const QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP: u8 = 7;
const QUEST_OBJECTIVE_MONEY_LIKE_CPP: u8 = 8;
const DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP: u32 = 15 * 60 * 1000;
const QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP: u8 = 9;
const QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP: u8 = 16;
const QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP: u8 = 17;
const QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP: u8 = 18;
#[cfg(test)]
const DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP: u32 = 100;
const QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP: u32 = 0x0080;
const QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP: u32 = 0x1;
const QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP: u32 = 0x0010_0000;
const QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP: u32 = 0x0000_1000;
const MAX_GAMEOBJECT_SLOT_LIKE_CPP: usize = 4;
pub(crate) const MAX_SPECIALIZATIONS_LIKE_CPP: usize = 4;
const NEEDED_TALENT_POINT_PER_TIER_LIKE_CPP: u32 = 5;
const PLAYER_FLAGS_UBER_LIKE_CPP: u32 = 0x0008_0000;
const PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP: u32 = 0x0000_0001;
pub(crate) const PLAYER_FLAGS_AFK_LIKE_CPP: u32 = 0x0000_0002;
pub(crate) const PLAYER_FLAGS_DND_LIKE_CPP: u32 = 0x0000_0004;
pub(crate) const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = 0x0000_0010;
const PLAYER_FLAGS_RESTING_LIKE_CPP: u32 = 0x0000_0020;
const PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP: u32 = 0x0000_0800;
const PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP: u32 = 0x0200_0000;
pub(crate) const PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP: u32 = 0x2000_0000;
pub(crate) const REST_STATE_RESTED_LIKE_CPP: u8 = 1;
pub(crate) const REST_STATE_NORMAL_LIKE_CPP: u8 = 2;
pub(crate) const REST_STATE_RAF_LINKED_LIKE_CPP: u8 = 6;

/// Capability-specific immutable query owner consumed by query/gameobject
/// handlers. It mirrors C++ ObjectMgr startup stores and contains no database
/// handle or mutable gameplay state.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Default))]
pub struct ObjectMgrCatalogsLikeCpp {
    pub creature: Arc<wow_data::CreatureQueryCatalogLikeCpp>,
    pub gameobject: Arc<wow_data::GameObjectQueryCatalogLikeCpp>,
    pub gameobject_quest_items: Arc<wow_data::GameObjectQuestItemStoreLikeCpp>,
    pub page_text: Arc<wow_data::PageTextCatalogLikeCpp>,
}

/// Process-owned DB2 catalogs used by C++'s static item valuation helpers.
///
/// C++ reads these globals directly in `Item::GetBuyPrice`,
/// `Item::GetSellPrice`, and `Item::GetDisenchantLoot`
/// (`Entities/Item/Item.cpp:1732-1969`); `WorldSession` owns none of them.
pub struct ItemValuationCatalogsLikeCpp {
    pub import_prices: Arc<ImportPriceStores>,
    pub price_base: Arc<ItemPriceBaseStore>,
    pub item_classes: Arc<ItemClassStore>,
    pub currency_costs: Arc<ItemCurrencyCostStore>,
    pub disenchant_loot: Arc<ItemDisenchantLootStore>,
}

/// Process-owned C++ Player creation data and glyph catalog borrowed during login.
///
/// C++ stores the base row plus `customSpells` and per-create-mode
/// `castSpells` under `ObjectMgr` (`Globals/ObjectMgr.h:649-663`) and resolves
/// it through `sObjectMgr->GetPlayerInfo`; it is never session-owned.
pub struct PlayerBootstrapCatalogsLikeCpp {
    pub create_info: Arc<PlayerCreateInfoStoreLikeCpp>,
    /// C++ process-wide sGlyphPropertiesStore, borrowed during Player::_LoadGlyphs.
    pub glyph_properties: Arc<GlyphPropertiesStore>,
    pub talent_tabs: Arc<TalentTabStore>,
    /// C++ process-owned sTraitNodeEntryStore, borrowed while loading traits.
    pub trait_node_entries: Arc<TraitNodeEntryStore>,
    pub cast_spells: Arc<PlayerCreateInfoCastSpellStoreLikeCpp>,
    pub custom_spells: Arc<PlayerCreateInfoCustomSpellStoreLikeCpp>,
    /// C++ `World` policy consumed by `Player::LearnCustomSpells`.
    pub start_all_spells: bool,
    /// C++ `World` policy consumed by the first-login `Player` path.
    pub start_all_explored: bool,
    /// C++ `World` policy consumed by the first-login `Player` path.
    pub start_all_reputation: bool,
}

/// Process-owned C++ `World` rates borrowed by `Player`/`RestMgr` transitions.
///
/// C++ reads these from `sWorld` at `Player.cpp:17898-17899` and
/// `RestMgr.cpp:139-151`; a session does not own private copies.
#[derive(Debug, Clone, Copy)]
pub struct PlayerRestRatePolicyLikeCpp {
    pub offline_wilderness: f32,
    pub offline_tavern_or_city: f32,
    pub ingame: f32,
}

/// Process-owned C++ `ObjectMgr` creature materialization catalogs and
/// `World` creature-health policy.
///
/// C++ resolves these through `sObjectMgr`/`sWorld` while `Creature::InitEntry`,
/// `UpdateLevelDependantStats`, `LoadEquipment`, and `GetCreatureAddon` build a
/// creature (`Creature.cpp:491-615,1550-1615,1931-1965,2722-2755`). A
/// `WorldSession` borrows them only while adapting visibility; it owns none of
/// the stores or rates.
pub struct CreatureSpawnCatalogsLikeCpp {
    pub difficulty: Arc<CreatureDifficultyStoreLikeCpp>,
    pub base_stats: Arc<CreatureBaseStatsStoreLikeCpp>,
    pub health_rates: CreatureClassificationHealthRatesLikeCpp,
    pub addons: Arc<CreatureAddonStoreLikeCpp>,
    pub equipment: Arc<CreatureEquipmentStoreLikeCpp>,
    pub power_types: Arc<PowerTypeStore>,
}

/// Process-owned player-level and exploration XP catalogs plus immutable World policy.
///
/// C++ resolves these through ObjectMgr world tables and `sWorld` while
/// adapting Player progression operations. A `WorldSession` borrows this
/// capability for the operation; it does not own a copy of any catalog or
/// configuration value.
#[derive(Clone)]
#[cfg_attr(test, derive(Default))]
pub struct ProgressionCatalogsLikeCpp {
    pub player_xp: Arc<Vec<u32>>,
    pub exploration_base_xp: Arc<ExplorationBaseXpStoreLikeCpp>,
    pub exploration_xp_rate: f32,
    pub min_discovered_scaled_xp_ratio: u32,
    /// C++ World policy read by Player::ResetTalents, never cached on Session.
    pub no_reset_talent_cost: bool,
}

#[cfg(test)]
impl Default for CreatureSpawnCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            difficulty: Arc::new(CreatureDifficultyStoreLikeCpp::default()),
            base_stats: Arc::new(CreatureBaseStatsStoreLikeCpp::default()),
            health_rates: CreatureClassificationHealthRatesLikeCpp::default(),
            addons: Arc::new(CreatureAddonStoreLikeCpp::default()),
            equipment: Arc::new(CreatureEquipmentStoreLikeCpp::default()),
            power_types: Arc::new(PowerTypeStore::from_entries([])),
        }
    }
}

impl Default for PlayerRestRatePolicyLikeCpp {
    fn default() -> Self {
        Self {
            offline_wilderness: 1.0,
            offline_tavern_or_city: 1.0,
            ingame: 1.0,
        }
    }
}

/// Process-owned C++ `World` chat policy borrowed by chat handlers.
///
/// C++ loads these values once into `World::{m_bool,m_int,m_float}_configs`
/// (`World.cpp:769,785-789,1241-1245,1294-1296,1323-1325`) and handlers read
/// them through `sWorld`; a `WorldSession` never owns a private policy copy.
#[derive(Debug, Clone, Copy)]
pub struct ChatPolicyCatalogsLikeCpp {
    pub addon_channel: bool,
    pub fake_message_preventing: bool,
    pub strict_link_checking_kick: bool,
    pub level_requirements: ChatLevelRequirementsLikeCpp,
    pub listen_ranges: ChatListenRangesLikeCpp,
    pub flood: ChatFloodConfigLikeCpp,
    pub party_raid_warnings: bool,
}

impl Default for ChatPolicyCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            addon_channel: true,
            fake_message_preventing: false,
            strict_link_checking_kick: false,
            level_requirements: ChatLevelRequirementsLikeCpp::default(),
            listen_ranges: ChatListenRangesLikeCpp::default(),
            flood: ChatFloodConfigLikeCpp::default(),
            party_raid_warnings: false,
        }
    }
}

/// Process-owned C++ `World` policy for party invitation admission.
///
/// C++ loads these at `World.cpp:790,913,1163`; `GroupHandler.cpp:78-108`
/// borrows them through `sWorld` while validating an invitation.
#[derive(Debug, Clone, Copy)]
pub struct GroupInvitePolicyLikeCpp {
    pub allow_gm_group: bool,
    pub allow_two_side_interaction: bool,
    pub minimum_level: u32,
}

/// Process-owned support and feature-system policy borrowed by Session edges.
///
/// C++ initializes the support switches in `World.cpp:584-595` and the feature
/// switches in `World.cpp:1597-1599`. `SupportMgr` and the feature-status send
/// helpers read that process state; it is not copied into each `WorldSession`.
#[derive(Debug, Clone, Copy)]
pub struct SupportFeaturePolicyLikeCpp {
    pub support_enabled: bool,
    pub tickets_enabled: bool,
    pub bugs_enabled: bool,
    pub complaints_enabled: bool,
    pub suggestions_enabled: bool,
    pub character_undelete_enabled: bool,
    pub bpay_store_enabled: bool,
    pub max_characters_per_realm: u32,
    pub declined_names_used: bool,
}

impl Default for SupportFeaturePolicyLikeCpp {
    fn default() -> Self {
        Self {
            support_enabled: true,
            tickets_enabled: false,
            bugs_enabled: false,
            complaints_enabled: false,
            suggestions_enabled: false,
            character_undelete_enabled: false,
            bpay_store_enabled: false,
            max_characters_per_realm: 60,
            declined_names_used: false,
        }
    }
}

impl SupportFeaturePolicyLikeCpp {
    pub(crate) fn bug_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.bugs_enabled
    }

    pub(crate) fn complaint_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.complaints_enabled
    }

    pub(crate) fn suggestion_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.suggestions_enabled
    }

    fn feature_system_config_like_cpp(self) -> FeatureSystemConfigLikeCpp {
        FeatureSystemConfigLikeCpp {
            support_tickets_enabled: self.tickets_enabled,
            support_bugs_enabled: self.bugs_enabled,
            support_complaints_enabled: self.complaints_enabled,
            support_suggestions_enabled: self.suggestions_enabled,
            char_undelete_enabled: self.character_undelete_enabled,
            bpay_store_enabled: self.bpay_store_enabled,
        }
    }
}

impl Default for GroupInvitePolicyLikeCpp {
    fn default() -> Self {
        Self {
            allow_gm_group: false,
            allow_two_side_interaction: false,
            minimum_level: 1,
        }
    }
}

#[cfg(test)]
impl Default for ItemValuationCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            import_prices: Arc::new(ImportPriceStores {
                armor: wow_data::ImportPriceArmorStore::from_entries([]),
                quality: wow_data::ImportPriceQualityStore::from_entries([]),
                shield: wow_data::ImportPriceShieldStore::from_entries([]),
                weapon: wow_data::ImportPriceWeaponStore::from_entries([]),
            }),
            price_base: Arc::new(ItemPriceBaseStore::from_entries([])),
            item_classes: Arc::new(ItemClassStore::from_entries([])),
            currency_costs: Arc::new(ItemCurrencyCostStore::from_entries([])),
            disenchant_loot: Arc::new(ItemDisenchantLootStore::from_entries([])),
        }
    }
}

#[cfg(test)]
impl Default for PlayerBootstrapCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            create_info: Arc::new(PlayerCreateInfoStoreLikeCpp::default()),
            glyph_properties: Arc::new(GlyphPropertiesStore::from_entries([])),
            talent_tabs: Arc::new(TalentTabStore::from_entries([])),
            trait_node_entries: Arc::new(TraitNodeEntryStore::from_entries([])),
            cast_spells: Arc::new(PlayerCreateInfoCastSpellStoreLikeCpp::default()),
            custom_spells: Arc::new(PlayerCreateInfoCustomSpellStoreLikeCpp::default()),
            start_all_spells: false,
            start_all_explored: false,
            start_all_reputation: false,
        }
    }
}

/// Process-owned area-trigger lookup and extension capability.
///
/// C++ reads the DB2/ObjectMgr stores and the ScriptMgr hook while handling
/// movement/area-trigger transitions; none of those owners belong to a
/// `WorldSession`. The optional dispatcher represents the legitimate absence
/// of a linked content script, not a missing production catalog.
#[derive(Clone)]
pub struct AreaTriggerCatalogsLikeCpp {
    pub db2: Arc<AreaTriggerDb2Store>,
    pub destinations: Arc<AreaTriggerStore>,
    pub scripts: Arc<AreaTriggerScriptStoreLikeCpp>,
    pub taverns: Arc<TavernAreaTriggerStoreLikeCpp>,
    pub script_dispatcher: Option<AreaTriggerScriptDispatcherLikeCpp>,
}

/// Process-wide ObjectMgr identifier allocators.
///
/// C++ initializes these once from database maxima. Sessions consume values
/// but never own an allocator or return a value after a later failure.
pub struct SessionIdGeneratorsLikeCpp {
    pub player: Arc<ObjectGuidGenerator>,
    pub item: Arc<ObjectGuidGenerator>,
    pub equipment_set: Arc<EquipmentSetGuidGeneratorLikeCpp>,
    pub void_storage_item: Arc<VoidStorageItemIdGeneratorLikeCpp>,
}

#[cfg(test)]
impl Default for SessionIdGeneratorsLikeCpp {
    fn default() -> Self {
        Self {
            player: Arc::new(ObjectGuidGenerator::new(HighGuid::Player, 1)),
            item: Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)),
            equipment_set: Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
            void_storage_item: Arc::new(VoidStorageItemIdGeneratorLikeCpp::new(1)),
        }
    }
}

#[cfg(test)]
impl Default for AreaTriggerCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            db2: Arc::new(AreaTriggerDb2Store::from_entries([])),
            destinations: Arc::new(AreaTriggerStore::default()),
            scripts: Arc::new(AreaTriggerScriptStoreLikeCpp::default()),
            taverns: Arc::new(TavernAreaTriggerStoreLikeCpp::default()),
            script_dispatcher: None,
        }
    }
}

/// Immutable process-owned capabilities borrowed by the outer driver for one
/// session pass. Each driver phase and packet registration narrows this
/// aggregate to the exact capability it consumes; production `WorldSession`
/// never stores the aggregate.
#[derive(Clone)]
pub struct SessionHandlerCatalogsLikeCpp {
    pub object_mgr: Arc<ObjectMgrCatalogsLikeCpp>,
    pub area_triggers: Arc<AreaTriggerCatalogsLikeCpp>,
    /// Map/runtime capability borrowed only by movement and login operations.
    /// C++ `Map::EnsureGridLoadedForActiveObject` belongs to the Map, not to
    /// `WorldSession`; production installs one process-owned adapter here.
    pub player_grid_loader: PlayerGridLoadResolverLikeCpp,
    pub item_valuation: Arc<ItemValuationCatalogsLikeCpp>,
    pub player_bootstrap: Arc<PlayerBootstrapCatalogsLikeCpp>,
    pub player_rest_rates: Arc<PlayerRestRatePolicyLikeCpp>,
    pub creature_spawns: Arc<CreatureSpawnCatalogsLikeCpp>,
    pub progression: Arc<ProgressionCatalogsLikeCpp>,
    pub battle_pet_trainer_selection:
        Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    pub chat_policy: Arc<ChatPolicyCatalogsLikeCpp>,
    pub group_invite_policy: Arc<GroupInvitePolicyLikeCpp>,
    pub support_feature_policy: Arc<SupportFeaturePolicyLikeCpp>,
    pub bank_bag_slot_prices: Arc<BankBagSlotPricesStore>,
    pub adventure_map_pois: Arc<AdventureMapPoiStore>,
    /// C++ sQuestInfoStore: borrowed by questgiver queries, never installed by dispatch.
    pub quest_info: Arc<QuestInfoStore>,
    pub battlemaster_lists: Arc<BattlemasterListStore>,
    pub emotes: Arc<EmotesStore>,
    pub emotes_text: Arc<EmotesTextStore>,
    pub graveyards: Arc<GraveyardStore>,
    pub lfg_dungeons: Arc<LfgDungeonStoreLikeCpp>,
    pub tact_keys: Arc<TactKeyStore>,
    /// C++ `sDB2Manager` hotfix delivery data, borrowed by auth and requests.
    pub hotfixes: Arc<HotfixBlobCache>,
    pub modules: Arc<wow_module_api::ModuleRegistry>,
    pub id_generators: Arc<SessionIdGeneratorsLikeCpp>,
}

#[cfg(test)]
impl Default for SessionHandlerCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            object_mgr: Arc::new(ObjectMgrCatalogsLikeCpp::default()),
            area_triggers: Arc::new(AreaTriggerCatalogsLikeCpp::default()),
            player_grid_loader: Arc::new(|_, _, _| PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: true,
                ..Default::default()
            }),
            item_valuation: Arc::new(ItemValuationCatalogsLikeCpp::default()),
            player_bootstrap: Arc::new(PlayerBootstrapCatalogsLikeCpp::default()),
            player_rest_rates: Arc::new(PlayerRestRatePolicyLikeCpp::default()),
            creature_spawns: Arc::new(CreatureSpawnCatalogsLikeCpp::default()),
            progression: Arc::new(ProgressionCatalogsLikeCpp::default()),
            battle_pet_trainer_selection: Arc::new(
                wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::default(),
            ),
            chat_policy: Arc::new(ChatPolicyCatalogsLikeCpp::default()),
            group_invite_policy: Arc::new(GroupInvitePolicyLikeCpp::default()),
            support_feature_policy: Arc::new(SupportFeaturePolicyLikeCpp::default()),
            bank_bag_slot_prices: Arc::new(BankBagSlotPricesStore::from_entries([])),
            adventure_map_pois: Arc::new(AdventureMapPoiStore::from_entries([])),
            quest_info: Arc::new(QuestInfoStore::from_entries([])),
            battlemaster_lists: Arc::new(BattlemasterListStore::from_entries([])),
            emotes: Arc::new(EmotesStore::from_entries([])),
            emotes_text: Arc::new(EmotesTextStore::from_entries([])),
            graveyards: Arc::new(GraveyardStore::default()),
            lfg_dungeons: Arc::new(LfgDungeonStoreLikeCpp::default()),
            tact_keys: Arc::new(TactKeyStore::from_entries([])),
            hotfixes: Arc::new(HotfixBlobCache::new()),
            modules: Arc::new(wow_module_api::ModuleRegistry::new()),
            id_generators: Arc::new(SessionIdGeneratorsLikeCpp::default()),
        }
    }
}

#[derive(Debug)]
pub(crate) enum LootMoneyPersistenceErrorLikeCpp {
    MissingPlayer,
    MissingCharacterDatabase,
    WorkerTerminated,
    Claim(wow_loot::LootClaimCommitError),
    Persistence(String),
    CommitOutcomeUnknownPersistence(String),
}

/// Owned exclusion held while an absolute character-money mutation derives
/// and persists its new value. Admission stays closed and the same
/// per-character serial lock used by group/stored loot remains held through
/// the caller's COMMIT.
#[must_use]
pub(crate) struct ExclusivePlayerMoneyPersistenceLikeCpp {
    _save_fence: DurableLootMoneySaveFenceLikeCpp,
    _mutation_lock: tokio::sync::OwnedMutexGuard<()>,
}

/// Process-wide ownership of a character's live `Player` runtime.
///
/// C++ has one `Player*` per GUID in `ObjectAccessor`; accepting a second
/// session would create two independent save authorities for the same rows.
/// Reserve the GUID before the asynchronous login pipeline starts. Weak
/// values make an abandoned session claim recoverable without a global sweep.
static ACTIVE_CHARACTER_LOGIN_CLAIMS_LIKE_CPP: OnceLock<dashmap::DashMap<ObjectGuid, Weak<()>>> =
    OnceLock::new();

/// Pure post-reset snapshot used to make the durable talent reset and its
/// runtime publication describe the same state.
///
/// C++ `Player::ResetTalents` calls `RemoveTalent` for the active group, then
/// `_SaveTalents` rewrites every group. `RemoveTalent` calls
/// `RemoveSpell(..., disabled=true)`, and the complete C++ `_SaveSpells` path
/// rewrites those rows with their exact active/disabled/favorite state. Rust
/// does not retain the full `PlayerSpellMap` active/disabled/temporary state,
/// so this deliberately leaves `character_spell` and
/// `character_spell_favorite` untouched. A known non-dependent spell proves a
/// normal persisted row exists; it does not prove that the talent is its only
/// source. Deleting that row would lose normal ownership where C++ instead
/// preserves a disabled row. Recursive `RemoveSpell` persistence and exact
/// disabled/favorite preservation therefore remain a represented boundary,
/// while active `character_talent` rows can no longer survive a committed fee.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RepresentedTalentResetStatePlanLikeCpp {
    active_group: u8,
    active_talents: BTreeMap<u32, u8>,
    post_talents: [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
}

/// A successfully committed reset whose covered runtime state still has to be
/// published synchronously before the money exclusion is released.
#[must_use]
pub(crate) struct CommittedRepresentedTalentResetLikeCpp {
    money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
    old_money: u64,
    new_money: u64,
    cost: u32,
    reset_time_secs: u64,
    state_plan: RepresentedTalentResetStatePlanLikeCpp,
}

/// Once a SQL transaction containing an absolute money write starts awaiting
/// COMMIT, cancellation is itself an unknown outcome. This synchronous drop
/// fence prevents a cancelled packet/shutdown future from reopening payout
/// admission and then letting disconnect-save overwrite a transaction whose
/// COMMIT reply was never observed.
pub(crate) struct PlayerMoneyCommitCancellationFenceLikeCpp {
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    armed: bool,
}

impl PlayerMoneyCommitCancellationFenceLikeCpp {
    fn new(tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>) -> Self {
        Self {
            tracker,
            armed: true,
        }
    }

    /// Build a fence which is inert until the database layer is immediately
    /// about to await a COMMIT.  Multi-step sagas use this form so cancelling
    /// during pre-commit validation does not quarantine a session whose
    /// transaction was never submitted.
    pub(crate) fn new_disarmed_like_cpp(
        tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    ) -> Self {
        Self {
            tracker,
            armed: false,
        }
    }

    pub(crate) fn arm_like_cpp(&mut self) {
        self.armed = true;
    }

    pub(crate) fn disarm_like_cpp(&mut self) {
        self.armed = false;
    }
}

impl wow_persistence::BattlePetPurchaseCommitFenceLikeCpp
    for PlayerMoneyCommitCancellationFenceLikeCpp
{
    fn arm_like_cpp(&mut self) {
        PlayerMoneyCommitCancellationFenceLikeCpp::arm_like_cpp(self);
    }

    fn disarm_like_cpp(&mut self) {
        PlayerMoneyCommitCancellationFenceLikeCpp::disarm_like_cpp(self);
    }
}

impl Drop for PlayerMoneyCommitCancellationFenceLikeCpp {
    fn drop(&mut self) {
        if self.armed {
            self.tracker.mark_indeterminate_like_cpp();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbsolutePlayerMoneyCommitReconciliationLikeCpp {
    Committed,
    RolledBack,
    Indeterminate,
}

/// Reconcile an ambiguous COMMIT using a money row whose value changed in the
/// transaction. Equal before/after values are deliberately not evidence: a
/// caller may have bundled other durable mutations whose outcome cannot be
/// inferred from an unchanged money column.
fn reconcile_absolute_player_money_commit_like_cpp(
    money_before: u64,
    money_after: u64,
    observed_money: Option<u64>,
) -> AbsolutePlayerMoneyCommitReconciliationLikeCpp {
    if money_before == money_after {
        return AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate;
    }

    match observed_money {
        Some(observed) if observed == money_after => {
            AbsolutePlayerMoneyCommitReconciliationLikeCpp::Committed
        }
        Some(observed) if observed == money_before => {
            AbsolutePlayerMoneyCommitReconciliationLikeCpp::RolledBack
        }
        Some(_) | None => AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate,
    }
}

/// Routing data retained by the detached durable worker so viewers that open
/// the same C++ `Loot` while SQL is in flight are not missed. The worker
/// samples the authority only after the money claim commits; an opener before
/// that point saw the non-zero pool and receives `CoinRemoved`, while a later
/// opener observes zero directly in its `LootResponse`.
pub(crate) struct LootMoneyViewerFanoutLikeCpp {
    pub scope_player: ObjectGuid,
    pub source_player: ObjectGuid,
    pub source_command_tx: flume::Sender<SessionCommand>,
    pub player_registry: Option<Arc<PlayerRegistry>>,
    pub map_id: u16,
    pub instance_id: u32,
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub payout_recipients: HashSet<ObjectGuid>,
}

/// Opaque destination for one already-durable loot-money publication.
#[derive(Clone)]
pub(crate) enum LootMoneyDeliveryAddressLikeCpp {
    /// The source session owns this sender directly.
    Source(flume::Sender<SessionCommand>),
    /// A remote session is addressed by its directory incarnation.
    Directory {
        registry: Arc<PlayerRegistry>,
        registration: crate::session::directory::PlayerRegistration,
    },
}

impl LootMoneyDeliveryAddressLikeCpp {
    fn queue_reliably_like_cpp(self, command: SessionCommand) {
        match self {
            Self::Source(command_tx) => {
                if let Err(error) = command_tx.try_send(command) {
                    let command = error.into_inner();
                    tokio::spawn(async move {
                        let _ = command_tx.send_async(command).await;
                    });
                }
            }
            Self::Directory {
                registry,
                registration,
            } => {
                let _ = registry.queue_current_command_reliably(registration, command);
            }
        }
    }
}

/// Routing state for one durable item claim. The completion owns this
/// independently of the packet waiter, so a timeout, cancellation, or later
/// `CMSG_LOOT_RELEASE` cannot suppress the result of an already ordered claim.
/// C++ serializes these handlers and notifies synchronously; Rust's SQL wait is
/// an implementation detail, so the pre-COMMIT cohort is retained and the
/// exact COMMIT snapshot adds viewers that opened during that wait. `published`
/// is shared with the normal handler path and provides the single publication
/// CAS.
#[derive(Clone)]
pub(crate) struct DurableLootItemFanoutLikeCpp {
    pub owner_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub player_guid: ObjectGuid,
    pub free_for_all: bool,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub precommit_snapshot: OwnedLootSnapshot,
    /// Exact post-mutation pool captured by the claim commit while the
    /// authority mutex is still held. A viewer opening after that point has
    /// already observed the removed item and must not receive a stale
    /// `LootRemoved` fanout.
    pub committed_snapshot: Arc<std::sync::OnceLock<OwnedLootSnapshot>>,
    pub source_send_tx: flume::Sender<Vec<u8>>,
    pub player_registry: Option<Arc<PlayerRegistry>>,
    pub map_id: u16,
    pub instance_id: u32,
    pub published: Arc<AtomicBool>,
}

impl std::fmt::Debug for DurableLootItemFanoutLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableLootItemFanoutLikeCpp")
            .field("owner_guid", &self.owner_guid)
            .field("loot_obj", &self.loot_obj)
            .field("loot_list_id", &self.loot_list_id)
            .field("player_guid", &self.player_guid)
            .field("free_for_all", &self.free_for_all)
            .field("authority_generation", &self.authority_generation)
            .field("map_id", &self.map_id)
            .field("instance_id", &self.instance_id)
            .field("published", &self.published.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

/// Runtime publication retained by a detached durable-loot worker after its
/// SQL transaction commits. Every detached item grant is tracked until the
/// live Player inventory is synchronized; Item owners additionally use the
/// completion to publish stored-container removal/release state. The same
/// tracker also prevents a committed stored-container money payout from being
/// overwritten by a stale disconnect save.
#[derive(Debug, Clone)]
pub(crate) struct DurableItemLootCompletionLikeCpp {
    pub owner_guid: ObjectGuid,
    pub loot_list_id: u8,
    pub player_guid: ObjectGuid,
    pub item_owner_auto_release: bool,
    /// Delta accepted from the character row locked in the same transaction
    /// that deletes an Item owner's stored-money row. Keeping a delta preserves
    /// intervening local/group changes; `None` identifies an item grant.
    pub durable_item_money_applied_amount: Option<u64>,
    /// Original C++ loot-money notification amount. This is retained instead
    /// of reconstructing it from runtime balances, and is emitted even when it
    /// is zero.
    pub durable_item_money_notified_amount: Option<u64>,
    /// Exact-once gate shared with the per-character durable money tracker.
    /// A save fence may apply the balance delta before this completion gets a
    /// chance to publish source removal and client notification.
    pub durable_item_money_balance_applied: Option<Arc<AtomicBool>>,
    /// Retained only for object-owned item claims. Stored-container money has
    /// its own durable fanout route.
    pub item_fanout: Option<DurableLootItemFanoutLikeCpp>,
    /// Exact-once gate for item/source publication and lifecycle. This is
    /// intentionally separate from `durable_item_money_balance_applied`.
    /// Item-grant completions observed false require a relog; stored-money
    /// completions can publish source removal after a save applied the balance.
    pub runtime_inventory_applied: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct DurableItemLootPersistenceStateLikeCpp {
    in_flight: usize,
    completions: Vec<DurableItemLootCompletionLikeCpp>,
}

/// Session-local counterpart to an authority persistence guard for durable
/// loot grants. It lets logout/disconnect wait for detached item/money
/// transactions, publish their committed runtime state, and only then run C++
/// `DoLootReleaseAll` and save the Player.
#[derive(Debug, Clone)]
pub(crate) struct DurableItemLootPersistenceTrackerLikeCpp {
    state: Arc<Mutex<DurableItemLootPersistenceStateLikeCpp>>,
    changed: tokio::sync::watch::Sender<u64>,
}

impl Default for DurableItemLootPersistenceTrackerLikeCpp {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            state: Arc::new(Mutex::new(DurableItemLootPersistenceStateLikeCpp::default())),
            changed,
        }
    }
}

impl DurableItemLootPersistenceTrackerLikeCpp {
    pub(crate) fn begin_like_cpp(&self) -> DurableItemLootPersistenceGuardLikeCpp {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .in_flight += 1;
        DurableItemLootPersistenceGuardLikeCpp {
            tracker: self.clone(),
            completion: None,
        }
    }

    pub(crate) async fn wait_until_idle_like_cpp(&self) {
        let mut changed = self.changed.subscribe();
        loop {
            if self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .in_flight
                == 0
            {
                return;
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }

    pub(crate) fn take_completions_like_cpp(&self) -> Vec<DurableItemLootCompletionLikeCpp> {
        std::mem::take(
            &mut self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .completions,
        )
    }
}

#[derive(Debug)]
pub(crate) struct DurableItemLootPersistenceGuardLikeCpp {
    tracker: DurableItemLootPersistenceTrackerLikeCpp,
    completion: Option<DurableItemLootCompletionLikeCpp>,
}

impl DurableItemLootPersistenceGuardLikeCpp {
    pub(crate) fn mark_committed_like_cpp(&mut self, completion: DurableItemLootCompletionLikeCpp) {
        self.completion = Some(completion);
    }
}

impl Drop for DurableItemLootPersistenceGuardLikeCpp {
    fn drop(&mut self) {
        let mut state = self
            .tracker
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(completion) = self.completion.take() {
            state.completions.push(completion);
        }
        state.in_flight = state.in_flight.saturating_sub(1);
        drop(state);
        self.tracker
            .changed
            .send_modify(|version| *version = version.wrapping_add(1));
    }
}

#[cfg(test)]
mod durable_item_loot_persistence_tracker_tests {
    use std::time::Duration;

    use super::DurableItemLootPersistenceTrackerLikeCpp;

    #[tokio::test]
    async fn completion_between_idle_check_and_wait_poll_is_not_lost_like_cpp() {
        let tracker = DurableItemLootPersistenceTrackerLikeCpp::default();
        let guard = tracker.begin_like_cpp();
        let mut changed = tracker.changed.subscribe();
        assert_eq!(
            tracker
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .in_flight,
            1
        );

        // Deliberately publish after the locked busy observation but before
        // `changed()` is first polled. watch retains the version transition.
        drop(guard);
        tokio::time::timeout(Duration::from_secs(1), changed.changed())
            .await
            .expect("durable item persistence wake must not be lost")
            .unwrap();
        tracker.wait_until_idle_like_cpp().await;
    }
}

impl std::fmt::Display for LootMoneyPersistenceErrorLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPlayer => formatter.write_str("loot-money player is missing"),
            Self::MissingCharacterDatabase => {
                formatter.write_str("loot-money character database is missing")
            }
            Self::WorkerTerminated => formatter.write_str("loot-money persistence worker stopped"),
            Self::Claim(error) => write!(formatter, "loot-money claim failure: {error:?}"),
            Self::Persistence(reason) => {
                write!(formatter, "loot-money persistence failure: {reason}")
            }
            Self::CommitOutcomeUnknownPersistence(reason) => {
                write!(formatter, "loot-money COMMIT outcome is unknown: {reason}")
            }
        }
    }
}

impl std::error::Error for LootMoneyPersistenceErrorLikeCpp {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingPlayer
            | Self::MissingCharacterDatabase
            | Self::WorkerTerminated
            | Self::Claim(_)
            | Self::Persistence(_)
            | Self::CommitOutcomeUnknownPersistence(_) => None,
        }
    }
}

/// C++ `Player::ModifyMoney` accepts the whole positive delta or leaves the
/// balance unchanged when it would cross `MAX_MONEY_AMOUNT`.
pub(crate) fn loot_money_durable_outcome_like_cpp(
    current_money: u64,
    requested_delta: u64,
) -> (u64, u64) {
    current_money
        .checked_add(requested_delta)
        .filter(|new_money| *new_money <= MAX_MONEY_AMOUNT)
        .map_or((current_money, 0), |new_money| (new_money, requested_delta))
}

/// Live seam for C++ `ScriptMgr::OnAreaTrigger`.
///
/// A ported content script receives the mutable session and returns the same
/// consumed/not-consumed boolean that controls C++ handler continuation.
pub type AreaTriggerScriptDispatcherLikeCpp =
    Arc<dyn Fn(&mut WorldSession, ScriptIdLikeCpp, u32, bool) -> bool + Send + Sync>;

#[cfg(test)]
type GivePlayerXpScriptDispatcherLikeCpp =
    Arc<dyn Fn(wow_script::player::GivePlayerXpContextLikeCpp, &mut u32) + Send + Sync>;

const REST_FLAG_IN_TAVERN_LIKE_CPP: u32 = 0x1;
const REST_FLAG_IN_CITY_LIKE_CPP: u32 = 0x2;
const REST_FLAG_IN_FACTION_AREA_LIKE_CPP: u32 = 0x4;
// C++ `RestMgr::SetRestBonus`: `float(next_level_xp) * 1.5f / 2`.
#[cfg(test)]
const REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP: f32 = 1.5 / 2.0;
const REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP: f32 = 0.031;
const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP: f32 = 0.125;
const REST_ONLINE_INGAME_BUBBLE_LIKE_CPP: f32 = 0.125;
const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;
const MAP_INSTANCE_LIKE_CPP: u8 = 1;
const MAP_RAID_LIKE_CPP: u8 = 2;
pub(crate) const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0x0000_0100;
const PLAYER_FLAGS_IN_PVP_LIKE_CPP: u32 = 0x0000_0200;
const PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP: u32 = 0x0002_0000;
const PLAYER_FLAGS_PVP_TIMER_LIKE_CPP: u32 = 0x0004_0000;
const PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP: u32 = 0x0800_0000;
const SPELL_PVP_RULES_ENABLED_LIKE_CPP: i32 = 134_735;
const LANG_RESET_SPELLS_LIKE_CPP: u32 = 215;
const LANG_RESET_TALENTS_LIKE_CPP: u32 = 216;
const LANG_RESET_SPELLS_TEXT_LIKE_CPP: &str = "Your spells have been reset.";
const LANG_RESET_TALENTS_TEXT_LIKE_CPP: &str = "Your talents have been reset.";
pub(crate) const TRADE_STATUS_PLAYER_BUSY_LIKE_CPP: u8 = 0;
const PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP: u32 = 0x0000_0800;
const AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP: u32 = 0x0000_0010;
const AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP: u32 = 0x0000_0080;
const AREA_FLAG_CONTESTED_LIKE_CPP: u32 = 0x0004_0000;
const AREA_FLAG_COMBAT_ZONE_LIKE_CPP: u32 = 0x0100_0000;
const CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP: u8 = 0x13;
pub(crate) type TeleportToOptionsLikeCpp = u32;
pub(crate) const TELE_TO_NONE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x00;
#[allow(dead_code)]
pub(crate) const TELE_TO_GM_MODE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x01;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_TRANSPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x02;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x04;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP: TeleportToOptionsLikeCpp = 0x08;
pub(crate) const TELE_TO_SPELL_LIKE_CPP: TeleportToOptionsLikeCpp = 0x10;
#[allow(dead_code)]
pub(crate) const TELE_TO_TRANSPORT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x20;
#[allow(dead_code)]
pub(crate) const TELE_REVIVE_AT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x40;
pub(crate) const TELE_TO_SEAMLESS_LIKE_CPP: TeleportToOptionsLikeCpp = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAwayModeLikeCpp {
    Afk,
    Dnd,
}

fn party_member_power_kind_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

fn party_member_power_to_u16_like_cpp(value: i32) -> u16 {
    u16::try_from(value.max(0)).unwrap_or(u16::MAX)
}

fn spell_effect_is_represented_summon_object_slot_like_cpp(effect: u32) -> bool {
    let slot_base = wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_SLOT1;
    let slot_end = slot_base + u32::try_from(MAX_GAMEOBJECT_SLOT_LIKE_CPP).unwrap_or(0);
    (slot_base..slot_end).contains(&effect)
}

fn spell_effect_has_non_or_db_nearby_entry_destination_like_cpp(
    effect: &wow_data::SpellEffectInfo,
) -> bool {
    matches!(
        effect.implicit_target_1,
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
            | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
    ) || matches!(
        effect.implicit_target_2,
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
            | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
    )
}

fn quest_has_represented_item_objective_like_cpp(quest: &wow_data::quest::QuestTemplate) -> bool {
    quest
        .objectives
        .iter()
        .any(|objective| objective.obj_type == QUEST_OBJECTIVE_ITEM_LIKE_CPP)
}

fn quest_rewards_block_like_cpp(quest: &wow_data::quest::QuestTemplate) -> QuestRewardsBlock {
    let mut rewards = QuestRewardsBlock {
        money: quest.reward_money_difficulty as i32,
        completion_spell: quest.reward_spell as i32,
        ..QuestRewardsBlock::default()
    };
    for (idx, reward_item) in quest.reward_items.iter().enumerate() {
        if let Some(reward_slot) = rewards.items.get_mut(idx) {
            let amount = quest.reward_amounts.get(idx).copied().unwrap_or(0);
            *reward_slot = (*reward_item, amount);
        }
    }
    for (idx, display_spell) in quest.reward_display_spell.iter().enumerate() {
        if let Some(slot) = rewards.display_spells.get_mut(idx) {
            *slot = *display_spell;
        }
    }
    for (idx, choice_item) in quest.reward_choice_items.iter().enumerate() {
        if let Some(slot) = rewards.choice_items.get_mut(idx) {
            *slot = *choice_item;
        }
    }
    rewards.choice_item_types = quest.reward_choice_item_types;
    rewards
}

fn quest_giver_creature_id_from_source_like_cpp(source_guid: ObjectGuid) -> i32 {
    if source_guid.is_any_type_creature() {
        i32::try_from(source_guid.entry()).unwrap_or(0)
    } else {
        0
    }
}
const ATTACK_DISPLAY_DELAY_LIKE_CPP_MS: u32 = 200;
const DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP: f32 = 1.5;
const MIN_MELEE_REACH_LIKE_CPP: f32 = 2.0;
const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;
const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP: u32 = 0x0000_0010;
const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP: u32 = 0x0001_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAttackStartLikeCppResult {
    Rejected,
    Accepted { send_attack_start: bool },
}

const QUEST_MENU_ICON_TURN_IN_LIKE_CPP: u8 = 0;
const QUEST_MENU_ICON_AVAILABLE_LIKE_CPP: u8 = 2;
const QUEST_MENU_ICON_COMPLETE_LIKE_CPP: u8 = 4;

const fn react_state_from_db_like_cpp(value: u8) -> ReactState {
    match value {
        0 => ReactState::Passive,
        1 => ReactState::Defensive,
        2 => ReactState::Aggressive,
        _ => ReactState::Passive,
    }
}

const fn pet_type_from_db_like_cpp(value: u8) -> PetType {
    match value {
        0 => PetType::Summon,
        1 => PetType::Hunter,
        _ => PetType::Max,
    }
}

const fn active_state_from_db_like_cpp(value: u8) -> ActiveState {
    match value {
        0x00 => ActiveState::Decide,
        0x01 => ActiveState::Passive,
        0x81 => ActiveState::Disabled,
        0xC1 => ActiveState::Enabled,
        _ => ActiveState::Disabled,
    }
}

const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

#[cfg(test)]
const fn primary_power_type_for_player_class_like_cpp(class_id: u8) -> PowerType {
    match class_id {
        1 => PowerType::Rage,
        4 => PowerType::Energy,
        6 => PowerType::RunicPower,
        _ => PowerType::Mana,
    }
}

const fn unit_stand_state_from_u8_like_cpp(value: u8) -> UnitStandStateType {
    match value {
        1 => UnitStandStateType::Sit,
        2 => UnitStandStateType::SitChair,
        3 => UnitStandStateType::Sleep,
        4 => UnitStandStateType::SitLowChair,
        5 => UnitStandStateType::SitMediumChair,
        6 => UnitStandStateType::SitHighChair,
        7 => UnitStandStateType::Dead,
        8 => UnitStandStateType::Kneel,
        9 => UnitStandStateType::Submerged,
        10 => UnitStandStateType::Max,
        _ => UnitStandStateType::Stand,
    }
}

const fn sheath_state_from_u8_like_cpp(value: u8) -> SheathState {
    match value {
        1 => SheathState::Melee,
        2 => SheathState::Ranged,
        _ => SheathState::Unarmed,
    }
}

#[derive(Debug, Clone)]
struct RepresentedPreparedQuestMenuItemLikeCpp {
    quest: wow_data::quest::QuestTemplate,
    quest_icon: u8,
    has_starter_relation: bool,
    has_involved_relation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharacterPetStableRowLikeCpp {
    pub pet_number: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub level: u8,
    pub experience: u32,
    pub react_state: u8,
    pub slot: i16,
    pub name: String,
    pub was_renamed: bool,
    pub health: u32,
    pub mana: u32,
    pub action_bar: String,
    pub last_save_time: u32,
    pub created_by_spell_id: u32,
    pub pet_type: u8,
    pub specialization_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterPetSpellRowLikeCpp {
    pub spell_id: u32,
    pub active: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterPetSpellCooldownRowLikeCpp {
    pub spell_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterPetSpellChargeRowLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterPetAuraRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterPetAuraEffectRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterAuraRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterAuraEffectRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

pub(crate) fn adjusted_represented_pet_aura_remain_time_like_cpp(
    remain_time_ms: i32,
    timediff_secs: u32,
    is_positive: bool,
    aura_expires_offline: bool,
) -> Option<i32> {
    if remain_time_ms != -1 && (!is_positive || aura_expires_offline) {
        let timediff_secs = i32::try_from(timediff_secs).unwrap_or(i32::MAX);
        if remain_time_ms / 1_000 <= timediff_secs {
            return None;
        }

        return Some(remain_time_ms.saturating_sub(timediff_secs.saturating_mul(1_000)));
    }

    Some(remain_time_ms)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharacterPetDeclinedNamesRowLikeCpp {
    pub names: [String; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResetSeasonalQuestStatusReasonLikeCpp {
    MissingEvent,
    EmptyEvent,
    RemovedOlderCompletions,
    NoOlderCompletions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetSeasonalQuestStatusOutcomeLikeCpp {
    pub event_id: u16,
    pub event_start_time: u64,
    pub reason: ResetSeasonalQuestStatusReasonLikeCpp,
    pub removed_quest_ids: Vec<u32>,
    pub completed_bit_cleared: usize,
    pub completed_bit_skipped_no_quest_v2_store: usize,
    pub completed_bit_skipped_zero_unique_bit: usize,
    pub completed_bit_no_change_or_noop: usize,
    pub completed_bit_clear_unrepresented: usize,
    pub event_bucket_erased: bool,
    pub seasonal_quest_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SeasonalQuestStatusDbRowLikeCpp {
    pub quest_id: u32,
    pub event_id: u32,
    pub completed_time: i64,
}

/// Session-local representation of C++ `Player::GetPlayerSharingQuest()` state
/// until full party/ObjectAccessor quest sharing runtime owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPendingQuestSharingLikeCpp {
    pub sender_guid: ObjectGuid,
    pub quest_id: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PacketCounterLikeCpp {
    last_receive_time_secs: u64,
    amount_counter: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CreateMapSideEffectApplySummaryLikeCpp {
    pub player_recent_instance_sets: u32,
    pub group_recent_instance_sets: u32,
    pub skipped_group_recent_instance_sets: u32,
    pub instance_lock_creates: u32,
    pub skipped_instance_lock_creates: u32,
    pub instance_lock_instance_id_updates: u32,
    pub skipped_instance_lock_instance_id_updates: u32,
    pub pending_battleground_entry_teleports: u32,
}

const PACKET_SPOOF_BAN_REASON_LIKE_CPP: &str = "DOS (Packet Flooding/Spoofing";
const PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP: &str = "Server: AutoDOS";

#[derive(Debug, Clone, PartialEq, Eq)]
enum PacketSpoofPendingBanTargetLikeCpp {
    Account { account_id: u32 },
    Ip { address: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PacketSpoofPendingBanLikeCpp {
    target: PacketSpoofPendingBanTargetLikeCpp,
    duration_secs: u32,
}

/// Evidence for the bounded `HandleQuestPushResult` sender-match seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestPushResultResponseLikeCpp {
    pub receiver_guid: ObjectGuid,
    pub sender_guid: ObjectGuid,
    pub parsed_quest_id: u32,
    pub pending_quest_id: u32,
    pub result: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedPushQuestToPartyOutcomeReasonLikeCpp {
    NotAllowed,
    NotDaily,
    QuestPoolActiveCheckUnrepresented,
    NotInParty,
    GroupRuntimeUnrepresented,
    ReceiverBusy,
    ReceiverDead,
    ReceiverAlreadyDone,
    ReceiverOnQuest,
    ReceiverLogFull,
    ReceiverSatisfyQuestDayAlreadyDone,
    ReceiverSatisfyQuestMinLevelLowLevel,
    ReceiverSatisfyQuestMaxLevelHighLevel,
    ReceiverSatisfyQuestClassWrongClass,
    ReceiverSatisfyQuestRaceWrongRace,
    ReceiverSatisfyQuestReputationLowFaction,
    ReceiverSatisfyQuestReputationHighFaction,
    ReceiverSatisfyQuestPreviousQuestPrerequisite,
    ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite,
    ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite,
    ReceiverSatisfyQuestExpansionRequiredExpansion,
    ReceiverCanTakeQuestInvalid,
    #[allow(dead_code)]
    ReceiverRepeatableTurnInRequestItemsUnrepresented,
    ReceiverRepeatableTurnInRequestItemsPrompted,
    ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
    ReceiverSuccessQuestDetailsPrompted,
    ReceiverQuestDetailsPromptCommandFailed,
    ReceiverEligibilityUnrepresented,
}

/// Session-local evidence for the bounded sender-side `HandlePushQuestToParty` preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPushQuestToPartyOutcomeLikeCpp {
    pub sender_guid: Option<ObjectGuid>,
    pub quest_id: u32,
    pub target_guid: Option<ObjectGuid>,
    pub reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    pub quest_pool_active_check_unrepresented: bool,
    pub group_runtime_unrepresented: bool,
    pub receiver_fanout_unrepresented: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedWargameInviteAcceptanceLikeCpp {
    pub inviter_name: String,
    pub inviter_guid: ObjectGuid,
    pub player_group_guid: u64,
    pub inviter_group_guid: u64,
    pub group_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedSignPetitionLikeCpp {
    pub petition_guid: ObjectGuid,
    pub choice: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDeclinePetitionLikeCpp {
    pub petition_guid: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQueryPetitionLikeCpp {
    pub petition_id: u32,
    pub item_guid: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedActivateTaxiLikeCpp {
    pub vendor: ObjectGuid,
    pub node: u32,
    pub ground_mount_id: u32,
    pub flying_mount_id: u32,
    pub preferred_mount_display: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedAlterAppearanceLikeCpp {
    pub new_sex: u8,
    pub customizations: Vec<wow_packet::packets::character::ChrCustomizationChoice>,
    pub customized_race: i32,
    pub customized_chr_model_id: i32,
    pub cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedConfirmBarbersChoiceLikeCpp {
    pub customizations: Vec<wow_packet::packets::character::ChrCustomizationChoice>,
    pub cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedConfirmRespecWipeLikeCpp {
    pub respec_master: ObjectGuid,
    pub respec_type: u8,
}

/// Evidence for C++ `sScriptMgr->OnPlayerTalentsReset(this, noCost)`.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedTalentResetScriptHookLikeCpp {
    pub no_cost: bool,
}

/// Evidence for C++ `RemoveAtLoginFlag(flags, persist=true)`.
///
/// Non-persistent at-login removals intentionally mutate only the represented
/// in-memory flag field and do not push this boundary record.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAtLoginFlagRemovalLikeCpp {
    pub flags: u16,
    pub persist: bool,
    pub db_statement_unrepresented: bool,
}

/// Evidence for `unit->CastSpell(_player, 14867, true)` after talent reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedTalentRespecVisualSpellCastLikeCpp {
    pub caster_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub spell_id: u32,
    pub triggered: bool,
    pub spell_runtime_unrepresented: bool,
}

/// Evidence for the two C++ `Player::ResetTalents` achievement criteria updates.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedTalentRespecCriteriaEventLikeCpp {
    MoneySpentOnRespecs { amount: u32 },
    TotalRespecs { quantity: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAdventureMapStartQuestLikeCpp {
    pub quest_id: u32,
    pub adventure_map_poi_id: u32,
    pub player_condition_id: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedSilencePartyTalkerLikeCpp {
    pub target: ObjectGuid,
    pub silent: bool,
}

/// Represented outcome for the bounded post-template `HandleQuestConfirmAccept` gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp {
    OriginalPlayerMissing,
    NotInSameRaid,
    OriginalPlayerNotActiveQuest,
    ReceiverCanTakeQuestFailed,
    ReceiverCanAddQuestLogFull,
    ReceiverCanAddQuestSourceItemFailed,
    ReceiverGiveQuestSourceItemStartQuestNoGrant,
    ReceiverGiveQuestSourceItemMaxCountNoGrant,
    ReceiverGiveQuestSourceItemStoredNewItem,
    ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
    GiveQuestSourceItemStoreNewItemUnrepresented,
    ReceiverAddQuestLocalStateRepresented,
    #[allow(dead_code)]
    AddQuestRuntimeUnrepresented,
}

/// Evidence that `HandleQuestConfirmAccept` reached the post-clear/template-present seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestConfirmAcceptLikeCpp {
    pub receiver_guid: Option<ObjectGuid>,
    pub sender_guid_before_clear: ObjectGuid,
    pub quest_id: u32,
    pub raw_quest_id: i32,
    pub reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    pub object_accessor_unrepresented: bool,
    pub party_runtime_unrepresented: bool,
    pub can_add_source_item_unrepresented: bool,
    pub can_add_source_item_result: Option<InventoryResult>,
    pub add_quest_runtime_unrepresented: bool,
    pub source_spell_unrepresented: bool,
    /// Source spell id whose C++ triggered self-casts are represented as evidence only.
    pub represented_source_spell_id: Option<u32>,
    /// Count of represented triggered self-casts C++ would perform in this shared-confirm path.
    pub represented_source_spell_self_casts: u8,
}

/// Evidence for represented `Player::CompleteQuest` status-update side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestCompleteStatusUpdateLikeCpp {
    pub quest_id: u32,
    pub old_status: u8,
    pub new_status: u8,
    pub send_quest_update_called: bool,
    pub quest_slot_state_complete_represented: bool,
    pub quest_slot_state_live_update_unrepresented: bool,
    pub visible_gameobjects_or_spellclicks_refresh_unrepresented: bool,
    pub spell_area_runtime_unrepresented: bool,
    pub tracking_event_auto_reward_unrepresented: bool,
    pub quest_tracker_complete_time_unrepresented: bool,
    pub script_status_change_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestRewardSpellKindLikeCpp {
    RewardSpell,
    RewardDisplaySpell { index: u8 },
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardSpellCastLikeCpp {
    pub quest_id: u32,
    pub spell_id: u32,
    pub kind: RepresentedQuestRewardSpellKindLikeCpp,
    pub can_delay_teleport_like_cpp: bool,
    pub spell_info_lookup_unrepresented: bool,
    pub caster_selection_unrepresented: bool,
    pub cast_spell_runtime_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedForceDeselectLikeCpp {
    pub caster_guid: ObjectGuid,
    pub visibility_range_yards: u32,
    pub break_target_packet_bytes: Vec<u8>,
    pub clear_target_packet_bytes: Vec<u8>,
    pub hostile_visible_fanout_unrepresented: bool,
    pub attacker_pet_attack_stop_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardTitleLikeCpp {
    pub quest_id: u32,
    pub title_id: u32,
    pub char_title_lookup_unrepresented: bool,
    pub set_title_runtime_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardTalentPointsLikeCpp {
    pub quest_id: u32,
    pub points: u32,
    pub init_talent_for_level_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardMailLikeCpp {
    pub quest_id: u32,
    pub mail_template_id: u32,
    pub delay_secs: u32,
    pub sender_entry: Option<u32>,
    pub quest_giver_guid: Option<ObjectGuid>,
    pub mail_template_lookup_unrepresented: bool,
    pub mail_draft_runtime_unrepresented: bool,
    pub character_db_transaction_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestRewardReputationSourceLikeCpp {
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardReputationLikeCpp {
    pub quest_id: u32,
    pub slot: u8,
    pub faction_id: u32,
    pub reward_faction_value: i32,
    pub reward_faction_override: i32,
    pub reward_faction_cap_in: i32,
    pub base_reputation_before_gain: i32,
    pub reputation_after_low_level_rate_like_cpp: i32,
    pub reputation_after_reward_rate_like_cpp: i32,
    pub no_quest_bonus: bool,
    pub no_spillover: bool,
    pub source: RepresentedQuestRewardReputationSourceLikeCpp,
    pub faction_store_lookup_unrepresented: bool,
    pub quest_faction_reward_store_lookup_unrepresented: bool,
    pub reputation_reward_rate_lookup_unrepresented: bool,
    pub gray_level_script_hook_unrepresented: bool,
    pub reputation_rank_cap_check_unrepresented: bool,
    pub calculate_reputation_gain_unrepresented: bool,
    pub modify_reputation_runtime_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedFactionReactionInputLikeCpp {
    pub source_faction_template_id: u32,
    pub target_faction_template_id: u32,
    pub target_has_player_owner: bool,
    pub target_player_owner_is_current_session: bool,
    pub target_player_contested_pvp: bool,
    pub target_is_unit: bool,
    pub target_ignores_reputation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGetReactionInputLikeCpp {
    pub self_faction_template_id: u32,
    pub target_faction_template_id: u32,
    pub same_object: bool,
    pub attackable_by_summoner: bool,
    pub same_charmer_or_owner_or_self: bool,
    pub self_has_player_owner: bool,
    pub target_has_player_owner: bool,
    pub target_player_owner_is_current_session: bool,
    pub target_owner_forced_rank_for_self: Option<wow_data::reputation::ReputationRankLikeCpp>,
    pub same_player_owner: bool,
    pub duel_in_progress: bool,
    pub same_raid: bool,
    pub self_unit_player_controlled: bool,
    pub target_unit_player_controlled: bool,
    pub self_ffa_pvp: bool,
    pub target_ffa_pvp: bool,
    pub self_ignores_reputation: bool,
    pub target_ignores_reputation: bool,
    pub target_is_unit: bool,
    pub target_player_contested_pvp: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LoadSeasonalQuestStatusOutcomeLikeCpp {
    pub rows_seen: usize,
    pub inserted: usize,
    pub replaced: usize,
    pub skipped_no_quest_store: usize,
    pub skipped_missing_quest: usize,
    pub skipped_event_out_of_range: usize,
    pub skipped_negative_completed_time: usize,
    pub completed_bit_set: usize,
    pub completed_bit_skipped_no_quest_v2_store: usize,
    pub completed_bit_skipped_zero_unique_bit: usize,
    pub completed_bit_no_change_or_noop: usize,
    pub seasonal_quest_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AttackReputationFactionSnapshotLikeCpp {
    faction_id: u32,
    contested_guard: bool,
    can_have_reputation: Option<bool>,
}

fn rounded_median_u32(sorted_values: &[u32]) -> u32 {
    debug_assert!(!sorted_values.is_empty());
    let mid = sorted_values.len() / 2;
    if sorted_values.len() % 2 == 1 {
        sorted_values[mid]
    } else {
        ((f64::from(sorted_values[mid - 1]) + f64::from(sorted_values[mid])) / 2.0).round() as u32
    }
}

fn set_active_player_update_bit_like_cpp(mask: &mut [u32; 48], bit: usize) {
    mask[bit / 32] |= 1 << (bit % 32);
}

fn action_button_action_like_cpp(packed: u32) -> u32 {
    packed & 0x00FF_FFFF
}

fn action_button_type_like_cpp(packed: u32) -> u8 {
    ((packed & 0xFF00_0000) >> 24) as u8
}

fn make_action_button_like_cpp(action: u32, action_type: u8) -> u32 {
    action_button_action_like_cpp(action) | ((action_type as u32) << 24)
}

use wow_packet::WorldPacket;

const TRANSFER_ABORT_DIFFICULTY_LIKE_CPP: u32 = 8;
const TRANSFER_ABORT_ERROR_LIKE_CPP: u32 = 1;
const TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP: u32 = 2;
const TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP: u32 = 4;
const TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP: u32 = 6;
const TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP: u32 = 7;
const TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP: u32 = 9;
const TRANSFER_ABORT_NEED_GROUP_LIKE_CPP: u32 = 11;
const TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP: u32 = 16;
const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
const DEATH_KNIGHT_START_MAP_LIKE_CPP: u16 = 609;
const DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP: i32 = 50977;
const HOUR_SECS_LIKE_CPP: u64 = 60 * 60;
const MAP_BATTLEGROUND_LIKE_CPP: i8 = 3;
const MAP_ARENA_LIKE_CPP: i8 = 4;
const GROUP_XP_DISTANCE_LIKE_CPP: f32 = 74.0;
const BATTLEGROUND_WS_LIKE_CPP: u32 = 2;
// C++ `SpellCastSource::Normal` is encoded in the six-bit Cast GUID subtype.
// The capture contract validates this field rather than treating it as a
// runtime counter, so keep the canonical numeric value here.
const SPELL_CAST_SOURCE_NORMAL_LIKE_CPP: u8 = 3;
pub(crate) const CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP: u32 = 0x08000;

/// C++ `CAST_FLAG_PENDING` (`Spells/Spell.h:78`). `SendSpellStart` and
/// `SendSpellGo` set it for a triggered cast that is not `m_fromClient`.
pub(crate) use crate::session_rules::represented_gameobject_dynamic_flags_update_like_cpp;

pub(crate) const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;
#[cfg(test)]
pub(crate) static NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP: AtomicI64 = AtomicI64::new(1);
const BATTLEGROUND_EY_LIKE_CPP: u32 = 7;

// C++ `ObjectGuid::Create<HighGuid::Cast>` passes realm id 0 to
// `CreateWorldObject`, then `ObjectGuidFactory.cpp:GetRealmIdForObjectGuid(0)`
// substitutes `realm.Id.Realm`. Rust passes that already-resolved local realm
// explicitly; using the caster realm preserves the same visible GUID bits.
fn represented_spell_cast_guid_for_map_like_cpp(
    realm_id: u16,
    map_id: u16,
    spell_id: i32,
    counter: i64,
) -> ObjectGuid {
    ObjectGuid::create_world_object(
        HighGuid::Cast,
        SPELL_CAST_SOURCE_NORMAL_LIKE_CPP,
        realm_id,
        map_id,
        0,
        u32::try_from(spell_id).unwrap_or_default(),
        counter,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MMapRuntimeConfigLikeCpp {
    pub data_dir: String,
    pub enabled: bool,
    pub disabled_map_ids: HashSet<u32>,
}

pub type WaypointPathResolverLikeCpp =
    Arc<dyn Fn(u32) -> Option<wow_movement::WaypointPath> + Send + Sync>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerGridLoadOutcomeLikeCpp {
    pub map_unavailable: bool,
    pub map_created: bool,
    pub grid_loaded_now: bool,
    pub metadata_entries: usize,
    pub skipped_already_loaded: usize,
    pub skipped_should_not_spawn: usize,
    pub skipped_difficulty_mismatch: usize,
    pub stale_index_entries: usize,
    pub creature_records_added: usize,
    pub gameobject_records_added: usize,
    pub area_trigger_records_added: usize,
    pub pre_add_records_added: usize,
    pub add_to_map_errors: usize,
    pub load_record_missing: usize,
    pub creature_load_record_missing: usize,
    pub gameobject_load_record_missing: usize,
    pub area_trigger_load_record_missing: usize,
    pub legacy_creature_mirrors: usize,
}

pub type PlayerGridLoadResolverLikeCpp =
    Arc<dyn Fn(u16, Option<u32>, Position) -> PlayerGridLoadOutcomeLikeCpp + Send + Sync>;

impl Default for MMapRuntimeConfigLikeCpp {
    fn default() -> Self {
        Self {
            data_dir: "./Data".to_string(),
            enabled: true,
            disabled_map_ids: HashSet::new(),
        }
    }
}

impl MMapRuntimeConfigLikeCpp {
    pub fn pathfinding_enabled_for_map_like_cpp(&self, map_id: u32) -> bool {
        self.enabled && !self.disabled_map_ids.contains(&map_id)
    }

    pub fn should_try_pathfinding_like_cpp(
        &self,
        map_id: u32,
        owner_ignores_pathfinding: bool,
    ) -> bool {
        self.pathfinding_enabled_for_map_like_cpp(map_id) && !owner_ignores_pathfinding
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedLootRollVote {
    pub vote: u8,
    pub roll_number: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct RepresentedLootRollState {
    pub owner_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub authority_scope: OwnedLootScope,
    /// Exact C++ `LootRoll*` lifetime surrogate published to remote sessions.
    /// This must change even if a replacement reuses the same packet key,
    /// authority allocation, and authority generation.
    pub command_identity: LootRollCommandIdentityLikeCpp,
    pub end_time: Instant,
    pub voters: HashMap<ObjectGuid, RepresentedLootRollVote>,
}

pub(crate) type CharacterPowerSnapshotLikeCpp = [Option<i32>; MAX_POWERS_PER_CLASS];

#[cfg(test)]
fn empty_character_power_snapshot_like_cpp() -> CharacterPowerSnapshotLikeCpp {
    [None; MAX_POWERS_PER_CLASS]
}

fn loaded_character_power_snapshot_like_cpp(
    powers: [i32; MAX_POWERS_PER_CLASS],
) -> CharacterPowerSnapshotLikeCpp {
    powers.map(|power| Some(power.max(0)))
}

fn character_power_snapshot_values_like_cpp(
    powers: &CharacterPowerSnapshotLikeCpp,
) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
    let mut values = [0; MAX_POWERS_PER_CLASS];
    for (index, power) in powers.iter().copied().enumerate() {
        values[index] = power?;
    }
    Some(values)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlayerSaveToDbSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub level: u8,
    pub xp: u32,
    pub money: u64,
    pub health: u32,
    pub max_health: u32,
    pub powers: CharacterPowerSnapshotLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RepresentedGameObjectUseEffect {
    UseRejectedNoDamageImmune {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    BattlegroundObjectUseRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        reason: RepresentedBattlegroundObjectUseRejection,
    },
    RemoveMountedAuras {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    RemoveStealthOrInvisibilityAuras {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    ClearPlayerTalkMenus {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    GossipHelloAi {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        handled: bool,
    },
    CooldownStarted {
        gameobject_guid: ObjectGuid,
        cooldown_secs: u32,
    },
    CooldownRejected {
        gameobject_guid: ObjectGuid,
    },
    TrapBombSpellCast {
        gameobject_guid: ObjectGuid,
        spell_id: u32,
    },
    TrapTargetActivated {
        gameobject_guid: ObjectGuid,
        target_guid: ObjectGuid,
    },
    TrapTargetSpellCast {
        gameobject_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
        original_caster_guid: ObjectGuid,
    },
    DoorOrButtonUsed {
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        restore_time_ms: u32,
        go_state: wow_entities::GoState,
    },
    DoorOrButtonRejectedNotReady {
        gameobject_guid: ObjectGuid,
    },
    #[allow(dead_code)]
    DoorOrButtonReset {
        gameobject_guid: ObjectGuid,
        go_state: wow_entities::GoState,
    },
    TriggerCinematic {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        cinematic_id: u32,
    },
    ShowPageText {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        page_id: u32,
    },
    SendGossip {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gossip_id: u32,
    },
    ReportUseAi {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        handled: bool,
    },
    TriggerGameEvent {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        event_id: u32,
    },
    TriggerLinkedTrap {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        trap_entry: u32,
    },
    KillCreditGo {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        entry: u32,
    },
    GooberQuestGateRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        quest_id: u32,
    },
    GooberSetGoStateForPlayer {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        go_state: wow_entities::GoState,
    },
    GooberDespawnForPlayer {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        despawn_secs: u32,
    },
    GameObjectPerPlayerStateExpired {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        despawned: bool,
        needs_state_update: bool,
    },
    GooberUsed {
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        custom_anim: u32,
        auto_close_ms: u32,
        go_state: Option<wow_entities::GoState>,
    },
    #[allow(dead_code)]
    GooberLinkedTrapDespawn {
        gameobject_guid: ObjectGuid,
        trap_entry: u32,
    },
    #[allow(dead_code)]
    GameObjectLinkedTrapDespawn {
        gameobject_guid: ObjectGuid,
        trap_entry: u32,
    },
    #[allow(dead_code)]
    GooberUniqueUserSpell {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_id: u32,
    },
    #[allow(dead_code)]
    GooberCleared {
        gameobject_guid: ObjectGuid,
        loot_state: wow_entities::LootState,
        go_state: Option<wow_entities::GoState>,
    },
    ChairUsed {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        slot: u32,
        teleport_position: Position,
        stand_state: u32,
    },
    ChairNoFreeSlot {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    BarberChairUsed {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        customization_scope: u32,
        teleport_position: Position,
        stand_state: u32,
        sit_anim_kit: u32,
    },
    UiLinkOpened {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ui_link_type: u32,
        interaction_type: i32,
    },
    ItemForgeUsed {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        condition_id: u32,
        forge_type: u32,
    },
    CapturePointAssaultRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        capture_time_ms: u32,
        world_state_id: u32,
        contested_event_horde: u32,
        contested_event_alliance: u32,
    },
    CapturePointAssaultAi {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        handled: bool,
    },
    CapturePointUpdated {
        gameobject_guid: ObjectGuid,
        state: RepresentedCapturePointStateLikeCpp,
        broadcast_text_id: u32,
        event_id: u32,
        world_state_id: u32,
        spell_visual_id: u32,
        custom_anim: u32,
        assault_timer_ms: u32,
    },
    BattlegroundFlagStandClicked {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        pickup_spell_id: u32,
        return_aura_id: u32,
        return_spell_id: u32,
    },
    BattlegroundFlagDropClicked {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        click_target: BattlegroundFlagDropClickTarget,
        event_id: u32,
        pickup_spell_id: u32,
        expire_duration_ms: u32,
    },
    GameObjectDeleted {
        gameobject_guid: ObjectGuid,
    },
    NewFlagPickupRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        pickup_spell_id: u32,
        expire_duration_ms: u32,
        respawn_time_ms: u32,
        flag_drop_entry: u32,
        exclusive_category: i32,
        world_state_id: u32,
        return_on_defender_interact: bool,
    },
    NewFlagDropInteracted {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spawn_vignette_id: u32,
    },
    NewFlagOwnerStateRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        state: RepresentedNewFlagStateRequest,
    },
    RitualWaitingForParticipants {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        unique_user_count: u32,
        casters_required: u32,
    },
    RitualCasterTargetSpellRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_id: u32,
        target_count: u32,
    },
    RitualCasterTargetSpellCast {
        gameobject_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_id: u32,
        triggered: bool,
    },
    RitualCompleted {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        final_spell_id: u32,
        triggered: bool,
        persistent: bool,
        unique_user_count: u32,
    },
    MeetingStoneSummonRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        gameobject_entry: u32,
        spell_id: u32,
        area_id: u32,
        prevent_unfriendly_outside_instances: bool,
    },
    MeetingStoneTargetRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        target_guid: Option<ObjectGuid>,
    },
    MeetingStoneLevelRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        player_level: u8,
        target_level: u8,
        required_level: i32,
    },
    GameObjectPostUseSpellMissing {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        spell_id: u32,
        go_type: u32,
        spell_lookup_difficulty_id: u8,
    },
    OutdoorPvpCustomSpellRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        spell_id: u32,
        go_type: u32,
        spell_lookup_difficulty_id: u8,
        spell_info_missing: bool,
    },
    GameObjectPostUseSpellCast {
        gameobject_guid: ObjectGuid,
        target_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        spell_id: u32,
        triggered: bool,
        caster: RepresentedGameObjectSpellCaster,
        spell_lookup_difficulty_id: u8,
    },
    FishingNodeOwnerRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    },
    FishingNodeActivated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    FishingBobberReady {
        gameobject_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    },
    FishingSkillUpdated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    FishingLootRoll {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        player_fishing_level: i32,
        area_fishing_level: i32,
        chance: i32,
        roll: i32,
    },
    FishingHoleDelegated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fishing_hole_guid: ObjectGuid,
    },
    FishingLootRequested {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot_type: u8,
    },
    FishNotHooked {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    FishingHoleCatchCriteriaUpdated {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
    },
    FinishChanneledSpell {
        player_guid: ObjectGuid,
    },
    SpellcasterPartyOnlyRejected {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    },
    GameObjectUseCountIncremented {
        gameobject_guid: ObjectGuid,
        use_count: u32,
    },
    GameObjectChargesDepleted {
        gameobject_guid: ObjectGuid,
        max_charges: u32,
        loot_state: wow_entities::LootState,
    },
    GameObjectJustDeactivatedCleared {
        gameobject_guid: ObjectGuid,
        deleted: bool,
    },
    CastSpell {
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        spell_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlegroundFlagDropClickTarget {
    None,
    WarsongGulch,
    EyeOfTheStorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlegroundObjectUseRejection {
    UnfriendlyFaction,
    RecentlyDroppedFlag,
    DamageImmune,
    Dead,
    NotInBattleground,
    Vehicle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RepresentedNewFlagStateRequest {
    InBase,
    Taken,
    Dropped,
    Respawning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RepresentedCapturePointStateLikeCpp {
    Neutral,
    ContestedHorde,
    ContestedAlliance,
    HordeCaptured,
    AllianceCaptured,
}

impl RepresentedCapturePointStateLikeCpp {
    fn custom_anim_and_spell_visual_like_cpp(
        self,
        source: wow_entities::CapturePointUseSource,
    ) -> (u32, u32) {
        match self {
            Self::Neutral => (0, source.spell_visual_ids[0]),
            Self::ContestedHorde => (1, source.spell_visual_ids[1]),
            Self::ContestedAlliance => (2, source.spell_visual_ids[2]),
            Self::HordeCaptured => (3, source.spell_visual_ids[3]),
            Self::AllianceCaptured => (4, source.spell_visual_ids[4]),
        }
    }

    fn packet_state_like_cpp(self) -> u8 {
        match self {
            Self::Neutral => 1,
            Self::ContestedHorde => 2,
            Self::ContestedAlliance => 3,
            Self::HordeCaptured => 4,
            Self::AllianceCaptured => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedGameObjectSpellCaster {
    User,
    GameObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPendingBind {
    pub map_id: u32,
    pub instance_id: u32,
    pub completed_mask: u32,
    pub time_until_lock_ms: u32,
}

pub(crate) type RepresentedHomebindLikeCpp = wow_entities::PlayerHomebindLikeCpp;

struct HomebindPersistenceJobLikeCpp {
    port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    request: wow_persistence::PlayerHomebindPersistenceRequestLikeCpp,
    guid_counter: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RepresentedGameObjectUseState {
    pub loot_state: Option<wow_entities::LootState>,
    pub loot_state_unit_guid: wow_core::ObjectGuid,
    pub owner_guid: Option<wow_core::ObjectGuid>,
    pub ritual_owner_guid: Option<wow_core::ObjectGuid>,
    pub owner_in_combat: Option<bool>,
    pub owner_current_channeled_spell_active: Option<bool>,
    pub go_state: Option<wow_entities::GoState>,
    pub prev_go_state: Option<wow_entities::GoState>,
    pub gameobject_flags: u32,
    pub gameobject_override_flags: Option<u32>,
    pub dynamic_flags: u32,
    pub despawn_delay_secs: Option<u32>,
    pub despawn_delay_until: Option<Instant>,
    pub respawn_delay_secs: Option<u32>,
    pub respawn_until: Option<Instant>,
    pub spawned_by_default: Option<bool>,
    pub per_player_despawn_secs: Option<u32>,
    pub per_player_despawn_until: Option<Instant>,
    pub per_player_state_player_guid: Option<wow_core::ObjectGuid>,
    pub per_player_go_state: Option<wow_entities::GoState>,
    pub per_player_go_state_until: Option<Instant>,
    pub personal_loot_uses: u32,
    pub use_count: u32,
    pub max_charges: Option<u32>,
    pub unique_users: Vec<wow_core::ObjectGuid>,
    pub chair_slots: Vec<Option<wow_core::ObjectGuid>>,
    pub chest_restock_time_secs: Option<u32>,
    pub chest_restock_until: Option<Instant>,
    pub chest_consumable: Option<bool>,
    pub chest_loot_source: Option<wow_entities::GameObjectLootSource>,
    pub chest_personal_loot_id: Option<u32>,
    pub gathering_node_loot_id: Option<u32>,
    pub map_id: Option<u16>,
    pub zone_id: Option<u32>,
    pub area_id: Option<u32>,
    pub position: Option<wow_core::Position>,
    pub go_anim_progress: u8,
    pub despawn_at_action: bool,
    pub display_id: Option<u32>,
    pub scale: f32,
    pub rotation: [f32; 4],
    pub go_type: Option<u8>,
    pub faction_template: Option<u32>,
    pub interact_radius_override: Option<u32>,
    /// Immutable template evidence for C++
    /// `GameObjectTemplate::IconName != "Point"`.
    pub icon_name_allows_interaction_like_cpp: Option<bool>,
    pub condition_id1: Option<u32>,
    pub lock_id: Option<u32>,
    pub fishing_hole_max_opens: Option<u32>,
    pub fishing_hole_radius: Option<f32>,
    pub fishing_area_level: Option<i32>,
    pub player_fishing_level: Option<i32>,
    pub fishing_roll: Option<i32>,
    pub nearby_fishing_hole_guid: Option<wow_core::ObjectGuid>,
    pub fishing_bobber_ready_at: Option<Instant>,
    pub linked_trap_entry: Option<u32>,
    pub linked_trap_guid: Option<wow_core::ObjectGuid>,
    pub trap_use_source: Option<wow_entities::TrapUseSource>,
    pub trap_target_guid: Option<wow_core::ObjectGuid>,
    pub goober_use_source: Option<wow_entities::GooberUseSource>,
    pub capture_point_state: Option<RepresentedCapturePointStateLikeCpp>,
    pub capture_point_source: Option<wow_entities::CapturePointUseSource>,
    pub capture_point_last_team_capture: Team,
    pub capture_point_assault_until: Option<Instant>,
    pub new_flag_state: Option<RepresentedNewFlagStateRequest>,
    pub new_flag_carrier_guid: Option<wow_core::ObjectGuid>,
    pub new_flag_taken_from_base_game_time_ms: Option<u32>,
    pub new_flag_respawn_until: Option<Instant>,
    pub new_flag_return_on_defender_interact: Option<bool>,
    pub new_flag_pickup_spell_id: Option<u32>,
    pub new_flag_entry: Option<u32>,
    pub report_use_ai_returns_true: bool,
    pub gossip_hello_ai_returns_true: bool,
    pub capture_point_assault_ai_returns_true: bool,
    pub cooldown_until: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedGameObjectAccessLikeCpp {
    pub entry: u32,
    pub position: wow_core::Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedCreatureAccessLikeCpp {
    pub entry: u32,
    pub position: wow_core::Position,
    pub npc_flags: u32,
    pub npc_flags2: u32,
    pub trainer_class: u8,
    pub faction_template_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedCanSeeSpellClickOutcomeLikeCpp {
    Visible,
    Hidden,
    ExactContextUnrepresented,
}

const NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP: u8 = 0x01;
const NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP: u8 = 0x02;
const NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedSpellClickUnitRefLikeCpp {
    Clicker,
    Clickee,
    Owner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedSpellClickCastLikeCpp {
    pub spell_id: u32,
    pub caster: RepresentedSpellClickUnitRefLikeCpp,
    pub target: RepresentedSpellClickUnitRefLikeCpp,
    pub original_caster: RepresentedSpellClickUnitRefLikeCpp,
    pub cast_flags: u8,
    pub vehicle_seat_id: Option<i8>,
    pub vehicle_control_effect_index: Option<u32>,
    pub vehicle_spellmod_basepoint_value: Option<i32>,
    pub vehicle_aura_fallback_basepoint_value: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RepresentedSpellClickPlanLikeCpp {
    pub casts: Vec<RepresentedSpellClickCastLikeCpp>,
    pub exact_context_unrepresented: bool,
    pub ai_on_spell_click_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RepresentedSpellClickExecutionOutcomeLikeCpp {
    pub planned_casts: usize,
    pub executed_casts: usize,
    pub ai_on_spell_click_represented: bool,
    pub ai_on_spell_click_unrepresented: bool,
    pub skipped_unrepresented_caster: usize,
    pub skipped_unrepresented_target: usize,
    pub skipped_unrepresented_original_caster: usize,
    pub failed_casts: usize,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedVehicleSeatChangeRequestLikeCpp {
    pub seat_id: i8,
    pub next: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedVehicleSeatSpellClickRequestLikeCpp {
    pub vehicle_guid: ObjectGuid,
    pub seat_id: i8,
    pub planned_casts: usize,
    pub exact_context_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedVehicleEnterRequestLikeCpp {
    pub vehicle_guid: ObjectGuid,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedVehicleDismissMovementLikeCpp {
    pub vehicle_guid: ObjectGuid,
    pub sanitized_flags: MovementFlag,
    pub position: Position,
    pub time: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedVehicleBaseMovementLikeCpp {
    pub vehicle_guid: ObjectGuid,
    pub sanitized_flags: MovementFlag,
    pub position: Position,
    pub time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepresentedSpellClickClickeeCasterOutcomeLikeCpp {
    Executed,
    UnsupportedCaster,
    UnsupportedTarget,
    UnsupportedOriginalCaster,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
struct RepresentedSpellClickCreatureSnapshotLikeCpp {
    guid: ObjectGuid,
    entry: u32,
    map_id: u32,
    instance_id: u32,
    position: Position,
    phase_shift: PhaseShift,
    npc_flags: u32,
    faction_template_id: u32,
    level: u32,
    health: u64,
    max_health: u64,
    is_alive: bool,
    is_in_world: bool,
    is_summon: bool,
    owner_guid: Option<ObjectGuid>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedItemModsReapplyEventLikeCpp {
    pub item_guid: ObjectGuid,
    pub slot: u8,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedItemBonusActionLikeCpp {
    pub item_guid: ObjectGuid,
    pub slot: u8,
    pub action: ApplyEnchantmentEffectAction,
}

const ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP: u32 = 0x01;

pub(crate) type RepresentedItemSetEffectLikeCpp = wow_entities::PlayerItemSetEffectLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedItemSetSpellEventLikeCpp {
    pub item_set_id: u32,
    pub spell_entry_id: u32,
    pub spell_id: u32,
    pub threshold: u8,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedItemSetAuraRefreshEventLikeCpp {
    pub item_set_id: u32,
    pub spell_entry_id: u32,
    pub spell_id: u32,
    pub apply: bool,
    pub form_change: bool,
}

pub(crate) type RepresentedItemBonusStateLikeCpp = wow_entities::PlayerItemBonusStateLikeCpp;

fn represented_player_stat_changes_like_cpp(
    state: &RepresentedItemBonusStateLikeCpp,
) -> wow_packet::packets::update::PlayerStatChanges {
    let mut changes = wow_packet::packets::update::PlayerStatChanges {
        base_mana: state.mana_base,
        base_health: state.health_base,
        attack_power: state.attack_power_total,
        ranged_attack_power: state.ranged_attack_power_total,
        stats: state.stats_base,
        stat_pos_buff: state.stats_base,
        armor: state.armor_base + state.armor_total + state.resistances_base[0],
        combat_ratings: state.combat_ratings,
        spell_power: state.spell_power_bonus,
        shield_block: i32::try_from(state.shield_block_value).unwrap_or(i32::MAX),
        ..Default::default()
    };

    changes.min_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][0];
    changes.max_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][1];
    changes.min_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][0];
    changes.max_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][1];
    changes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepresentedScalingStatContextLikeCpp {
    stat_id: [i32; 10],
    bonus: [i32; 10],
    ssd_multiplier: i32,
    spell_bonus: i32,
    armor_mod: i32,
    dps_mod: i32,
    is_two_hand: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedCombatStatRecalculationLikeCpp {
    Expertise { attack: WeaponAttackType },
    Rating { combat_rating: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildRepairBankStateLikeCpp {
    pub available_repair_money: u64,
    pub withdraw_repair_money_allowed: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildRepairBankWithdrawLikeCpp {
    pub amount: u64,
    pub repair: bool,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBankItemMoveLikeCpp {
    pub to_bank: bool,
    pub inv_update_items: Vec<(u8, u8)>,
    pub bag: u8,
    pub slot: u8,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankInventoryMoveLikeCpp {
    pub banker: ObjectGuid,
    pub guild_id: u64,
    pub to_char: bool,
    pub bank_tab: u8,
    pub bank_slot: u8,
    pub player_bag: u8,
    pub player_slot: u8,
    pub stack_count: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankListRequestLikeCpp {
    pub banker: ObjectGuid,
    pub guild_id: u64,
    pub tab: u8,
    pub full_update: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankMoneyMoveLikeCpp {
    pub banker: ObjectGuid,
    pub guild_id: u64,
    pub deposit: bool,
    pub money: u64,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedGuildBankTabActionLikeCpp {
    pub banker: Option<ObjectGuid>,
    pub guild_id: u64,
    pub tab: i32,
    pub action: RepresentedGuildBankTabActionKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepresentedGuildBankTabActionKindLikeCpp {
    Buy,
    Update { name: String, icon: String },
    LogQuery,
    TextQuery,
    SetText { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionReplicateRequestLikeCpp {
    pub auctioneer: ObjectGuid,
    pub change_number_global: u32,
    pub change_number_cursor: u32,
    pub change_number_tombstone: u32,
    pub count: u32,
    pub tainted_by_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionPlaceBidLikeCpp {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub bid_amount: u64,
    pub tainted_by_present: bool,
    pub copper_rejected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionRemoveItemLikeCpp {
    pub auctioneer: ObjectGuid,
    pub auction_id: i32,
    pub item_id: i32,
    pub tainted_by_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAuctionSellItemLikeCpp {
    pub auctioneer: ObjectGuid,
    pub item_guid: Option<ObjectGuid>,
    pub item_use_count: Option<u32>,
    pub min_bid: u64,
    pub buyout_price: u64,
    pub runtime_minutes: u32,
    pub tainted_by_present: bool,
    pub item_list_rejected: bool,
    pub use_count_rejected: bool,
    pub no_price_rejected: bool,
    pub max_money_rejected: bool,
    pub copper_rejected: bool,
    pub auctioneer_accepted: bool,
    pub runtime_rejected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedCalendarCommunityInviteLikeCpp {
    pub guild_id: u64,
    pub min_level: u8,
    pub max_level: u8,
    pub max_rank_order: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedCalendarAddEventLikeCpp {
    pub guild_id: Option<u64>,
    pub club_id: u64,
    pub event_type: u8,
    pub texture_id: i32,
    pub time_packed: u32,
    pub flags: u32,
    pub invite_count: usize,
    pub title: String,
    pub description: String,
    pub max_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCalendarRemoveEventLikeCpp {
    pub event_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterHelloLikeCpp {
    pub unit: ObjectGuid,
    pub entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlefieldListLikeCpp {
    pub list_id: u32,
}

pub(crate) type RepresentedBattlegroundQueueTypeIdLikeCpp =
    wow_entities::PlayerBattlegroundQueueTypeIdLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterJoinLikeCpp {
    pub packed_queue_id: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub roles: u8,
    pub blacklist_map: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterJoinArenaLikeCpp {
    pub team_size_index: u8,
    pub roles: u8,
    pub arena_type: u8,
    pub group_guid: u64,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlemasterJoinSkirmishLikeCpp {
    pub bg_type_id: u32,
    pub bracket_id: u32,
    pub as_group: bool,
    pub is_rated_packet_value: u8,
    pub arena_type: u8,
    pub group_guid: Option<u64>,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
}

#[cfg(test)]
pub(crate) type RepresentedBattlegroundQueueSlotLikeCpp =
    wow_entities::PlayerBattlegroundQueueSlotLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlefieldPortLikeCpp {
    pub ticket: wow_packet::packets::misc::LfgRideTicket,
    pub accepted_invite: bool,
    pub queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    pub invited_instance_guid: u32,
}

pub(crate) fn battleground_queue_type_id_from_packed_like_cpp(
    packed_queue_id: u64,
) -> RepresentedBattlegroundQueueTypeIdLikeCpp {
    RepresentedBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: (packed_queue_id & 0xFFFF) as u16,
        queue_type: ((packed_queue_id >> 16) & 0xF) as u8,
        rated: ((packed_queue_id >> 20) & 1) != 0,
        team_size: ((packed_queue_id >> 24) & 0x3F) as u8,
    }
}

fn arena_team_type_by_slot_like_cpp(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(2),
        1 => Some(3),
        2 => Some(5),
        _ => None,
    }
}

fn arena_skirmish_type_like_cpp(bg_type_id: u32, bracket_id: u32) -> u8 {
    if bg_type_id == 3 || bg_type_id == 5 {
        bg_type_id as u8
    } else if bracket_id == 3 || bracket_id == 5 {
        bracket_id as u8
    } else {
        2
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCanDuelSpellCastLikeCpp {
    pub target_guid: ObjectGuid,
    pub spell_id: u32,
    pub to_the_death: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDuelRequestedLikeCpp {
    pub target_guid: ObjectGuid,
    pub arbiter_guid: ObjectGuid,
    pub gameobject_entry: u32,
    pub to_the_death: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDuelAcceptedLikeCpp {
    pub opponent_guid: ObjectGuid,
    pub arbiter_guid: ObjectGuid,
    pub countdown_ms: u32,
}

/// Validated session-owned intent whose side effects cross the represented to
/// live boundary. Variants deliberately own their payload so future intents
/// may contain non-`Copy` data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepresentedLiveIntentLikeCpp {
    StandStateChanged(RepresentedStandStateChangedLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedStandStateChangedLikeCpp {
    pub state: UnitStandStateType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedStandChannelCancellationBoundary {
    Interrupted {
        spell_id: u32,
        canonical_spells_interrupted: usize,
        session_cast_interrupted: bool,
    },
    UnknownInterruptMetadata {
        spell_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedLiveIntentAppliedLikeCpp {
    StandStateChanged {
        canonical_field_changed: bool,
        canonical_auras_removed: usize,
        represented_auras_removed: usize,
        channel_cancellation_boundary: Option<RepresentedStandChannelCancellationBoundary>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedLiveIntentApplyOutcomeLikeCpp {
    Applied(RepresentedLiveIntentAppliedLikeCpp),
    RejectedMissingPlayer,
    RejectedMissingCanonicalPlayer,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedLiveApplicationLikeCpp {
    pub intent: RepresentedLiveIntentLikeCpp,
    pub outcome: RepresentedLiveIntentApplyOutcomeLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum RepresentedDuelCancelOutcomeLikeCpp {
    Interrupted,
    Surrendered,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDuelCancelledLikeCpp {
    pub opponent_guid: ObjectGuid,
    pub outcome: RepresentedDuelCancelOutcomeLikeCpp,
    pub beg_spell_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MovementAckEventLikeCpp {
    pub opcode: ClientOpcodes,
    pub mover_guid: ObjectGuid,
    pub ack_index: Option<i32>,
    pub movement_force_id: Option<ObjectGuid>,
    pub movement_force_type: Option<u8>,
    pub adjusted_time: Option<u32>,
    pub speed: Option<f32>,
    pub time_skipped: Option<u32>,
    pub spline_id: Option<i32>,
    pub accepted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveSplineDoneTaxiActionLikeCpp {
    InvalidMovement,
    InProgressNoFlightGenerator,
    InProgressNoTeleport,
    TeleportRequested,
    FinalCleanup,
    IgnoredUnexpectedFinalPath,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedTaxiFlightNodeLikeCpp {
    pub map_id: u16,
    pub position: wow_core::Position,
    pub teleport_flag: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MoveSplineDoneTaxiEventLikeCpp {
    pub spline_id: i32,
    pub action: MoveSplineDoneTaxiActionLikeCpp,
    pub destination_node_id: Option<u32>,
    pub teleport_map_id: Option<u16>,
    pub teleport_position: Option<wow_core::Position>,
    pub honorless_target_cast: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveTeleportAckActionLikeCpp {
    NotBeingTeleportedNear,
    WrongMover,
    MissingDestination,
    MissingPlayerOwner,
    Accepted,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MoveTeleportAckEventLikeCpp {
    pub mover_guid: ObjectGuid,
    pub ack_index: i32,
    pub move_time: i32,
    pub action: MoveTeleportAckActionLikeCpp,
    pub destination_map_id: Option<u16>,
    pub destination_position: Option<wow_core::Position>,
    pub old_zone_id: Option<u32>,
    pub new_zone_id: Option<u32>,
    pub new_area_id: Option<u32>,
    pub honorless_target_cast: bool,
    pub pvp_disabled: bool,
    pub pet_resummon_requested: bool,
    pub delayed_operations_processed: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedAreaZoneCriteriaLikeCpp {
    EnterArea(u32),
    LeaveArea(u32),
    EnterTopLevelArea(u32),
    LeaveTopLevelArea(u32),
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RepresentedTaxiFlightStateLikeCpp {
    current_node: RepresentedTaxiFlightNodeLikeCpp,
    node_after_teleport: Option<RepresentedTaxiFlightNodeLikeCpp>,
}

#[cfg(test)]
fn canonical_taxi_flight_node_like_cpp(
    node: RepresentedTaxiFlightNodeLikeCpp,
) -> wow_entities::PlayerTaxiFlightNodeLikeCpp {
    wow_entities::PlayerTaxiFlightNodeLikeCpp {
        map_id: node.map_id,
        position: node.position,
        teleport_flag: node.teleport_flag,
    }
}

#[cfg(test)]
fn represented_taxi_flight_node_like_cpp(
    node: wow_entities::PlayerTaxiFlightNodeLikeCpp,
) -> RepresentedTaxiFlightNodeLikeCpp {
    RepresentedTaxiFlightNodeLikeCpp {
        map_id: node.map_id,
        position: node.position,
        teleport_flag: node.teleport_flag,
    }
}

#[cfg(test)]
fn canonical_taxi_flight_state_like_cpp(
    flight: RepresentedTaxiFlightStateLikeCpp,
) -> wow_entities::PlayerTaxiFlightStateLikeCpp {
    wow_entities::PlayerTaxiFlightStateLikeCpp {
        current_node: canonical_taxi_flight_node_like_cpp(flight.current_node),
        node_after_teleport: flight
            .node_after_teleport
            .map(canonical_taxi_flight_node_like_cpp),
    }
}

#[cfg(test)]
fn represented_taxi_flight_state_like_cpp(
    flight: wow_entities::PlayerTaxiFlightStateLikeCpp,
) -> RepresentedTaxiFlightStateLikeCpp {
    RepresentedTaxiFlightStateLikeCpp {
        current_node: represented_taxi_flight_node_like_cpp(flight.current_node),
        node_after_teleport: flight
            .node_after_teleport
            .map(represented_taxi_flight_node_like_cpp),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MovementSpeedAckActionLikeCpp {
    Accepted,
    SkippedPending,
    Corrected,
    Kicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitMoveTypeLikeCpp {
    Walk = 0,
    Run = 1,
    RunBack = 2,
    Swim = 3,
    SwimBack = 4,
    TurnRate = 5,
    Flight = 6,
    FlightBack = 7,
    PitchRate = 8,
}

impl UnitMoveTypeLikeCpp {
    pub(crate) const COUNT: usize = 9;

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MovementSpeedAckEventLikeCpp {
    pub opcode: ClientOpcodes,
    pub move_type: Option<UnitMoveTypeLikeCpp>,
    pub ack_speed: f32,
    pub expected_speed: Option<f32>,
    pub remaining_forced_changes: Option<u8>,
    pub action: MovementSpeedAckActionLikeCpp,
}

const PLAYER_BASE_MOVE_SPEED_LIKE_CPP: [f32; UnitMoveTypeLikeCpp::COUNT] = [
    2.5,      // MOVE_WALK
    7.0,      // MOVE_RUN
    4.5,      // MOVE_RUN_BACK
    4.722222, // MOVE_SWIM
    2.5,      // MOVE_SWIM_BACK
    3.141594, // MOVE_TURN_RATE
    7.0,      // MOVE_FLIGHT
    4.5,      // MOVE_FLIGHT_BACK
    3.14,     // MOVE_PITCH_RATE
];

impl Default for RepresentedGameObjectUseState {
    fn default() -> Self {
        Self {
            loot_state: None,
            loot_state_unit_guid: wow_core::ObjectGuid::EMPTY,
            owner_guid: None,
            ritual_owner_guid: None,
            owner_in_combat: None,
            owner_current_channeled_spell_active: None,
            go_state: None,
            prev_go_state: None,
            gameobject_flags: 0,
            gameobject_override_flags: None,
            dynamic_flags: 0,
            despawn_delay_secs: None,
            despawn_delay_until: None,
            respawn_delay_secs: None,
            respawn_until: None,
            spawned_by_default: None,
            per_player_despawn_secs: None,
            per_player_despawn_until: None,
            per_player_state_player_guid: None,
            per_player_go_state: None,
            per_player_go_state_until: None,
            personal_loot_uses: 0,
            use_count: 0,
            max_charges: None,
            unique_users: Vec::new(),
            chair_slots: Vec::new(),
            chest_restock_time_secs: None,
            chest_restock_until: None,
            chest_consumable: None,
            chest_loot_source: None,
            chest_personal_loot_id: None,
            gathering_node_loot_id: None,
            map_id: None,
            zone_id: None,
            area_id: None,
            position: None,
            go_anim_progress: 255,
            despawn_at_action: false,
            display_id: None,
            scale: 1.0,
            rotation: [0.0, 0.0, 0.0, 1.0],
            go_type: None,
            faction_template: None,
            interact_radius_override: None,
            icon_name_allows_interaction_like_cpp: None,
            condition_id1: None,
            lock_id: None,
            fishing_hole_max_opens: None,
            fishing_hole_radius: None,
            fishing_area_level: None,
            player_fishing_level: None,
            fishing_roll: None,
            nearby_fishing_hole_guid: None,
            fishing_bobber_ready_at: None,
            linked_trap_entry: None,
            linked_trap_guid: None,
            trap_use_source: None,
            trap_target_guid: None,
            goober_use_source: None,
            capture_point_state: None,
            capture_point_source: None,
            capture_point_last_team_capture: Team::Other,
            capture_point_assault_until: None,
            new_flag_state: None,
            new_flag_carrier_guid: None,
            new_flag_taken_from_base_game_time_ms: None,
            new_flag_respawn_until: None,
            new_flag_return_on_defender_interact: None,
            new_flag_pickup_spell_id: None,
            new_flag_entry: None,
            report_use_ai_returns_true: false,
            gossip_hello_ai_returns_true: false,
            capture_point_assault_ai_returns_true: false,
            cooldown_until: None,
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedLootRollCriteriaEvent {
    RollAnyNeed {
        player_guid: ObjectGuid,
        quantity: u32,
    },
    RollAnyGreed {
        player_guid: ObjectGuid,
        quantity: u32,
    },
    RollNeed {
        player_guid: ObjectGuid,
        item_id: u32,
        roll_number: u8,
    },
    RollGreed {
        player_guid: ObjectGuid,
        item_id: u32,
        roll_number: u8,
    },
    Disenchant {
        player_guid: ObjectGuid,
        spell_id: u32,
    },
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedGameObjectCriteriaEvent {
    UseGameobject {
        player_guid: ObjectGuid,
        gameobject_entry: u32,
    },
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedTransmogCriteriaEvent {
    LearnAnyTransmogInSlot {
        equipment_slot: u32,
        item_modified_appearance_id: u32,
    },
    CollectTransmogSetFromGroup {
        transmog_set_group_id: u32,
    },
}

pub(crate) use wow_entities::PlayerAccountHeirloomDataLikeCpp as AccountHeirloomDataLikeCpp;
pub(crate) use wow_entities::PlayerFavoriteAppearanceStateLikeCpp as FavoriteAppearanceStateLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AccountItemAppearanceSavePlanLikeCpp {
    pub(crate) appearance_blocks: Vec<(u32, u32)>,
    pub(crate) favorite_inserts: Vec<u32>,
    pub(crate) favorite_deletes: Vec<u32>,
}

impl AccountItemAppearanceSavePlanLikeCpp {
    pub(crate) fn is_empty(&self) -> bool {
        self.appearance_blocks.is_empty()
            && self.favorite_inserts.is_empty()
            && self.favorite_deletes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AccountTransmogIllusionSavePlanLikeCpp {
    pub(crate) illusion_blocks: Vec<(u32, u32)>,
}

impl AccountTransmogIllusionSavePlanLikeCpp {
    pub(crate) fn is_empty(&self) -> bool {
        self.illusion_blocks.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountMountSaveRowLikeCpp {
    pub(crate) bnet_account_id: u32,
    pub(crate) mount_spell_id: u32,
    pub(crate) flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountToySaveRowLikeCpp {
    pub(crate) bnet_account_id: u32,
    pub(crate) item_id: u32,
    pub(crate) is_favorite: bool,
    pub(crate) has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountHeirloomSaveRowLikeCpp {
    pub(crate) bnet_account_id: u32,
    pub(crate) item_id: u32,
    pub(crate) flags: u32,
}

const DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP: [u32; 7] = [
    3,  // Lifestealing
    13, // Crusader
    22, // Striking
    23, // Agility
    34, // Hide Weapon Enchant
    43, // Beastslayer
    44, // Titanguard
];

pub(crate) const BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP: u16 = 0x01;
pub(crate) const BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP: u8 = 1;
pub(crate) const BATTLE_PET_SLOT_COUNT_LIKE_CPP: usize = 3;
#[cfg(test)]
pub(crate) const BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP: u32 = 82_800;
#[allow(dead_code)]
pub(crate) const BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP: u8 = 3;
pub(crate) const BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP: u32 = 222;
pub(crate) const DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP: u8 = 3;
pub(crate) const MAX_BATTLE_PET_LEVEL_LIKE_CPP: u16 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetXpSourceLikeCpp {
    PetBattle,
    SpellEffect,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetSaveInfoLikeCpp {
    New,
    Changed,
    #[allow(dead_code)]
    Unchanged,
    Removed,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlePetCageItemLikeCpp {
    pub(crate) item_id: u32,
    pub(crate) species_id: u32,
    pub(crate) breed_data: u32,
    pub(crate) level: u16,
    pub(crate) display_id: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetCageOutcomeLikeCpp {
    Caged(RepresentedBattlePetCageItemLikeCpp),
    NoJournalLock,
    UnknownPet,
    NotTradable,
    InBattleSlot,
    Damaged,
    InventoryUnavailable,
    StoreFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlePetCalculatedStatsLikeCpp {
    pub(crate) max_health: u32,
    pub(crate) power: u32,
    pub(crate) speed: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetQualityOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    QualityAboveRare,
    CantBattle,
    NotUpgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlePetLevelCriteriaLikeCpp {
    pub(crate) species: u32,
    pub(crate) level: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetGrantLevelOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    CantBattle,
    AlreadyMaxLevel,
    NoGrantedLevels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    InvalidXpOrSource,
    CantBattle,
    AlreadyMaxLevel,
    MissingXpRow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedBattlePetDataLikeCpp {
    pub(crate) species: u32,
    pub(crate) creature_id: u32,
    pub(crate) display_id: u32,
    pub(crate) breed: u16,
    pub(crate) level: u16,
    pub(crate) exp: u16,
    pub(crate) flags: u16,
    pub(crate) power: u32,
    pub(crate) health: u32,
    pub(crate) max_health: u32,
    pub(crate) speed: u32,
    pub(crate) quality: u8,
    pub(crate) owner_info: Option<wow_packet::packets::misc::BattlePetJournalPetOwnerInfo>,
    pub(crate) name: String,
    pub(crate) name_timestamp: i64,
    pub(crate) declined_names: Option<wow_packet::packets::misc::DeclinedNamesLikeCpp>,
    pub(crate) save_info: RepresentedBattlePetSaveInfoLikeCpp,
}

/// Represented ObjectAccessor/TempSummon facts needed by
/// `WorldSession::HandleQueryBattlePetName`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlePetQueryCompanionLikeCpp {
    pub(crate) creature_id: i32,
    pub(crate) name_timestamp: i64,
    pub(crate) is_summon: bool,
    pub(crate) owner_is_player: bool,
    pub(crate) battle_pet_companion_guid: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedBattlePetSlotLikeCpp {
    pub(crate) pet_guid: Option<ObjectGuid>,
    pub(crate) collar_id: u32,
    pub(crate) index: u8,
    pub(crate) locked: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct ChatFloodThrottleDataLikeCpp {
    time: i64,
    count: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ChatFloodThrottleIndexLikeCpp {
    Regular = 0,
    Addon = 1,
}

impl RepresentedBattlePetSlotLikeCpp {
    pub(crate) fn locked_empty(index: u8) -> Self {
        Self {
            pet_guid: None,
            collar_id: 0,
            index,
            locked: true,
        }
    }

    pub(crate) fn packet_slot_like_cpp(&self) -> wow_packet::packets::misc::BattlePetJournalSlot {
        wow_packet::packets::misc::BattlePetJournalSlot {
            pet_guid: self
                .pet_guid
                .unwrap_or_else(wow_packet::packets::misc::empty_battle_pet_guid_like_cpp),
            collar_id: self.collar_id,
            index: self.index,
            locked: self.locked,
        }
    }
}

impl RepresentedBattlePetDataLikeCpp {
    pub(crate) fn minimal_like_cpp(
        flags: u16,
        save_info: RepresentedBattlePetSaveInfoLikeCpp,
    ) -> Self {
        Self {
            species: 0,
            creature_id: 0,
            display_id: 0,
            breed: 0,
            level: 0,
            exp: 0,
            flags,
            power: 0,
            health: 0,
            max_health: 0,
            speed: 0,
            quality: 0,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info,
        }
    }

    pub(crate) fn packet_info_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> wow_packet::packets::misc::BattlePetJournalPet {
        wow_packet::packets::misc::BattlePetJournalPet {
            guid,
            species: self.species,
            creature_id: self.creature_id,
            display_id: self.display_id,
            breed: self.breed,
            level: self.level,
            exp: self.exp,
            flags: self.flags,
            power: self.power,
            health: self.health,
            max_health: self.max_health,
            speed: self.speed,
            quality: self.quality,
            owner_info: self.owner_info,
            name: self.name.clone(),
        }
    }
}

fn heirloom_bonus_for_flags_like_cpp(heirloom: &HeirloomEntry, flags: u32) -> u32 {
    for upgrade_level in (0..heirloom.upgrade_item_id.len()).rev() {
        if flags & (1_u32 << upgrade_level) != 0 {
            return u32::from(heirloom.upgrade_item_bonus_list_id[upgrade_level]);
        }
    }

    0
}

pub(crate) const SKILL_FISHING_LIKE_CPP: u16 = 356;
pub(crate) const SKILL_RIDING_LIKE_CPP: u16 = 762;
pub(crate) const SKILL_ENCHANTING_LIKE_CPP: u16 = 333;
pub const LIQUID_MAP_IN_WATER_LIKE_CPP: u32 = 0x0000_0004;
pub const LIQUID_MAP_UNDER_WATER_LIKE_CPP: u32 = 0x0000_0008;
const TOY_FLAG_FAVORITE_LIKE_CPP: u32 = 0x01;
const TOY_FLAG_HAS_FANFARE_LIKE_CPP: u32 = 0x02;
const DAMAGE_FALL_LIKE_CPP: u8 = 2;
const DAMAGE_FIRE_LIKE_CPP: u8 = 5;
const DAMAGE_FALL_TO_VOID_LIKE_CPP: u8 = 6;
const SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP: i32 = 0x0000_0001;
const CREATURE_MODEL_DATA_FLAG_CAN_MOUNT_WHILE_TRANSFORMED_AS_THIS_LIKE_CPP: u32 = 0x0000_0080;
const CHR_RACES_FLAG_CAN_MOUNT_LIKE_CPP: i32 = 0x0000_0004;
pub(crate) const SPELL_DUEL_LIKE_CPP: u32 = 7266;
pub(crate) const SPELL_MOUNTED_DUEL_LIKE_CPP: u32 = 62875;
#[cfg(test)]
pub(crate) const SPELL_DUEL_BEG_LIKE_CPP: u32 = 7267;
pub(crate) const DUEL_COUNTDOWN_MS_LIKE_CPP: u32 = 3000;
pub type SharedCanonicalMapManager = Arc<Mutex<wow_map::MapManager>>;

pub(crate) fn relocate_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    position: Position,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().relocate_map_object_like_cpp(guid, position);
}

pub(crate) fn sync_canonical_creature_entity_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    mut creature: wow_entities::Creature,
) -> Option<OwnedLootAuthority> {
    let guid = creature.unit().world().object().guid();
    let Ok(mut manager) = manager.lock() else {
        return None;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return None;
    };
    if map
        .map()
        .creature_transform_vitals_snapshot_like_cpp(guid)
        .is_none()
    {
        return None;
    }
    let accept_incoming_entity_state = map.map().with_creature_like_cpp(guid, |current| {
        let incoming_unit = creature.unit();
        let current_unit = current.unit();
        let shares_health_timeline = incoming_unit.shares_health_state_revision_authority_like_cpp(
            &current_unit.health_state_revision_authority_like_cpp(),
        );
        let incoming_revision = incoming_unit.health_state_revision_like_cpp();
        let current_revision = current_unit.health_state_revision_like_cpp();
        let health_tuple_matches = incoming_unit.data().health == current_unit.data().health
            && incoming_unit.data().max_health == current_unit.data().max_health
            && incoming_unit.death_state() == current_unit.death_state();

        // Whole-entity replacement is safe only inside the same incarnation
        // timeline. A lower revision is a stale snapshot even when health has
        // completed an ABA cycle; an equal revision is valid only when its full
        // represented health tuple agrees. Reject the entire snapshot instead
        // of copying only health, because death/respawn hooks also mutate AI,
        // combat, loot, aura, timer, flag, and runtime-plan state.
        shares_health_timeline
            && (incoming_revision > current_revision
                || (incoming_revision == current_revision && health_tuple_matches))
    })?;

    let current_authority = map
        .map()
        .with_creature_like_cpp(guid, |current| current.loot_authority_like_cpp().clone())?;
    let incoming_authority = creature.loot_authority_like_cpp().clone();
    let current_stamp = current_authority.stamp_like_cpp();
    let incoming_stamp = incoming_authority.stamp_like_cpp();
    let authority = reconcile_creature_loot_authority_mirrors_like_cpp(
        &current_authority,
        current_stamp,
        &incoming_authority,
        incoming_stamp,
    );
    map.map_mut().with_creature_mut_like_cpp(guid, |current| {
        current.rebind_loot_authority_if_current_like_cpp(
            &current_authority,
            current_stamp,
            authority.clone(),
        )
    })??;
    if !accept_incoming_entity_state {
        // The actual legacy owner performs its own expected-stamp CAS with the
        // returned authority. Its rejected transport clone must not replace any
        // canonical lifecycle fields.
        return Some(authority);
    }
    // `creature` is a cloned transport snapshot whose old authority is still
    // owned by the live legacy entity. Do not detach it here; the caller
    // performs the expected-stamp CAS on that actual entity.
    creature.adopt_loot_authority_for_snapshot_like_cpp(authority);
    let old_threat_guids = map
        .map()
        .with_creature_like_cpp(guid, |current| {
            current.unit().subsystems().combat.sorted_threat_guids()
        })
        .unwrap_or_default();
    let incoming_threat_guids: HashSet<_> = creature
        .unit()
        .subsystems()
        .combat
        .sorted_threat_guids()
        .into_iter()
        .collect();
    let removed_threat_guids: Vec<_> = old_threat_guids
        .into_iter()
        .filter(|threat_guid| !incoming_threat_guids.contains(threat_guid))
        .collect();
    let mirrored_threat_guids: Vec<_> = incoming_threat_guids.iter().copied().collect();

    creature.unit_mut().world_mut().object_mut().add_to_world();
    let Ok(record) = wow_entities::MapObjectRecord::new_creature(creature) else {
        return None;
    };
    let authority = record
        .creature()
        .map(|creature| creature.loot_authority_like_cpp().clone())?;
    map.map_mut().insert_map_object_record(record).ok()?;
    for added_guid in mirrored_threat_guids {
        let threat_ref = map
            .map()
            .with_creature_like_cpp(guid, |creature| {
                creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_ref(added_guid)
                    .copied()
            })
            .flatten();
        let Some(threat_ref) = threat_ref else {
            continue;
        };
        if let Some(player) = map.map_mut().get_typed_player_mut(added_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        } else if let Some(creature) = map.map_mut().get_typed_creature_mut(added_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        }
    }
    for removed_guid in removed_threat_guids {
        if let Some(player) = map.map_mut().get_typed_player_mut(removed_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        } else if let Some(creature) = map.map_mut().get_typed_creature_mut(removed_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        }
    }
    Some(authority)
}

/// Selects one backing authority for two mirrors without ever merging two
/// independently claimable active states. Distinct non-pristine authorities
/// are quarantined as one retired canonical tombstone.
pub(crate) fn reconcile_creature_loot_authority_mirrors_like_cpp(
    canonical: &OwnedLootAuthority,
    canonical_stamp: OwnedLootAuthorityStamp,
    incoming: &OwnedLootAuthority,
    incoming_stamp: OwnedLootAuthorityStamp,
) -> OwnedLootAuthority {
    if canonical.shares_storage_like_cpp(incoming) {
        return canonical.clone();
    }

    use OwnedLootAuthorityLifecycle::{Active, Detached, Pristine, Quarantined, Retired};

    match (canonical_stamp.lifecycle, incoming_stamp.lifecycle) {
        // Once divergent live pools were observed, keep the attached terminal
        // tombstone until object destruction. It must not be reopened merely
        // because another stale mirror still looks active.
        (Quarantined, _) => canonical.clone(),
        (_, Quarantined) => incoming.clone(),
        // Two independently claimable live pools, or a live pool conflicting
        // with an attached destruction tombstone, are ambiguous without a
        // shared incarnation id. Converge on a terminal fail-closed authority.
        (Active, Active) | (Active, Retired) | (Retired, Active) => {
            return OwnedLootAuthority::new_retired_tombstone_like_cpp();
        }
        // A live authority can safely fill a never-used placeholder. A
        // detached allocation has already lost entity ownership.
        (Active, Pristine | Detached) => canonical.clone(),
        (Pristine | Detached, Active) => incoming.clone(),
        // A still-attached retired authority is the lifetime tombstone shared
        // across respawn/restock. A displaced authority is classified as
        // `Detached`, so it cannot win this branch or be resurrected.
        (Retired, Pristine) => canonical.clone(),
        (Pristine, Retired) => incoming.clone(),
        (Pristine, Pristine) | (Retired, Retired) => canonical.clone(),
        (Detached, Detached) => OwnedLootAuthority::new_retired_tombstone_like_cpp(),
        (Detached, _) => incoming.clone(),
        (_, Detached) => canonical.clone(),
    }
}

pub(crate) fn insert_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    mut creature: wow_entities::Creature,
) -> Option<OwnedLootAuthority> {
    let guid = creature.unit().world().object().guid();
    let Ok(mut manager) = manager.lock() else {
        return None;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return None;
    };
    if map.map().get_creature(guid).is_some() {
        let current = map.map_mut().get_typed_creature_mut(guid)?;
        let current_authority = current.loot_authority_like_cpp().clone();
        let incoming_authority = creature.loot_authority_like_cpp().clone();
        let current_stamp = current_authority.stamp_like_cpp();
        let incoming_stamp = incoming_authority.stamp_like_cpp();
        let authority = reconcile_creature_loot_authority_mirrors_like_cpp(
            &current_authority,
            current_stamp,
            &incoming_authority,
            incoming_stamp,
        );
        current.rebind_loot_authority_if_current_like_cpp(
            &current_authority,
            current_stamp,
            authority.clone(),
        )?;
        creature.adopt_loot_authority_for_snapshot_like_cpp(authority.clone());
        return Some(authority);
    }

    if creature.loot_authority_like_cpp().lifecycle_like_cpp()
        == OwnedLootAuthorityLifecycle::Detached
    {
        return None;
    }

    let object = creature.unit().world().clone();
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::Creature, object);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    let authority = creature.loot_authority_like_cpp().clone();
    let Ok(record) = wow_entities::MapObjectRecord::new_creature(creature) else {
        return None;
    };
    map.map_mut().insert_map_object_record(record).ok()?;
    Some(authority)
}

pub(crate) fn remove_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().remove_from_map_like_cpp(guid, true);
}

pub(crate) fn add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    info: wow_map::RespawnInfoLikeCpp,
) -> (bool, bool) {
    let Ok(mut manager) = manager.lock() else {
        return (false, false);
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return (false, false);
    };

    let respawn_added = matches!(
        map.map_mut().add_respawn_info_like_cpp(info),
        wow_map::AddRespawnInfoOutcomeLikeCpp::Inserted
            | wow_map::AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    let object_removed = map.map_mut().remove_from_map_like_cpp(guid, true).is_ok();
    (respawn_added, object_removed)
}

pub(crate) fn remove_canonical_respawn_time_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return false;
    };
    map.map_mut()
        .remove_respawn_time_like_cpp(object_type, spawn_id)
        .is_some()
}

#[derive(Debug, Clone)]
pub struct LegacyCreatureMovementTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub movement_packets: usize,
    pub canonical_syncs: usize,
    pub plan: crate::map_manager::RuntimePlan,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureLifecycleTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub corpses_despawned: usize,
    pub respawns_processed: usize,
    pub respawn_db_mutations: Vec<wow_persistence::RespawnPersistenceMutationLikeCpp>,
    pub canonical_removes: usize,
    pub canonical_inserts: usize,
    pub canonical_respawn_adds: usize,
    pub canonical_respawn_removes: usize,
    /// Map instances whose sessions must recompute creature visibility.
    pub refresh_map_keys: Vec<(u16, u32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCreatureAggroCandidateLikeCpp {
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub map_difficulty_id: u8,
    pub position: Position,
    pub player_visibility_represented: bool,
    pub player_phase_shift: PhaseShift,
    pub player_visibility_detection: UnitVisibilityDetectionStateLikeCpp,
    pub player_combat_reach: f32,
    pub player_detected_range_aura_mod: f32,
    pub player_liquid_status_like_cpp: u32,
    pub player_level: u8,
    pub player_gray_level: u8,
    pub player_unit_flags: u32,
    pub player_unit_flags2: u32,
    pub player_unit_state: u32,
    pub player_is_game_master: bool,
    pub player_is_contested_pvp: bool,
    pub player_faction_template_id: u32,
    pub player_reputation_standings: Vec<(u32, i32)>,
    pub player_reputation_state_flags: Vec<(u32, u32)>,
    pub player_forced_reputation_ranks: Vec<(u32, wow_data::reputation::ReputationRankLikeCpp)>,
    pub player_forced_reputation_faction_ids: Vec<u32>,
    pub player_school_immunity_mask: u32,
    pub player_damage_immunity_mask: u32,
    pub player_has_confuse_aura: bool,
    pub player_has_breakable_stun_aura: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct LegacyCreatureAggroOwnerSnapshotLikeCpp {
    map_id: u16,
    instance_id: u32,
    position: Position,
    phase_shift: PhaseShift,
    combat_reach: f32,
    alive: bool,
    in_water: bool,
    in_evade_mode: bool,
    unit_flags: UnitFlags,
    faction_template_id: Option<u32>,
    school_immunity_mask: u32,
    damage_immunity_mask: u32,
    has_confuse_aura: bool,
    has_breakable_stun_aura: bool,
}

const DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP: f32 = 533.0;

/// Prove that one spell cannot enter any process-wide C++ runtime hook that
/// this bounded Rust spell path does not execute.
///
/// Every source is optional because both sessions and the global Creature
/// runtime can be constructed before startup authority is installed. Missing
/// or indeterminate authority must therefore reject the spell rather than
/// treating an empty runtime registry as proof that no DB hook exists.
#[allow(clippy::too_many_arguments)]
fn spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp(
    spell_id: u32,
    exact_spell_ids: Option<&BTreeSet<u32>>,
    all_rank_root_spell_ids: Option<&BTreeSet<u32>>,
    legacy_spell_ids: Option<&BTreeSet<u32>>,
    rejected_linked_trigger_spell_ids: Option<&BTreeSet<u32>>,
    chains: Option<&SpellChainStoreLikeCpp>,
    linked: Option<&SpellLinkedStoreLikeCpp>,
) -> bool {
    let (
        Some(exact_spell_ids),
        Some(all_rank_root_spell_ids),
        Some(legacy_spell_ids),
        Some(rejected_linked_trigger_spell_ids),
        Some(chains),
        Some(linked),
    ) = (
        exact_spell_ids,
        all_rank_root_spell_ids,
        legacy_spell_ids,
        rejected_linked_trigger_spell_ids,
        chains,
        linked,
    )
    else {
        return false;
    };
    if chains
        .indeterminate_diagnostics_for_spell_like_cpp(spell_id)
        .is_some()
    {
        return false;
    }
    let first_rank = chains.first_spell_in_chain_like_cpp(spell_id);
    if exact_spell_ids.contains(&spell_id)
        || all_rank_root_spell_ids.contains(&first_rank)
        || legacy_spell_ids.contains(&spell_id)
        || rejected_linked_trigger_spell_ids.contains(&spell_id)
    {
        return false;
    }

    [
        SpellLinkedTypeLikeCpp::Cast,
        SpellLinkedTypeLikeCpp::Hit,
        SpellLinkedTypeLikeCpp::Aura,
        SpellLinkedTypeLikeCpp::Remove,
    ]
    .into_iter()
    .all(|kind| linked.get_spell_linked_like_cpp(kind, spell_id).is_none())
}

/// Map-owned creature aggro fidelity switches derived from C++ world configs.
///
/// C++ anchor: `Creature::CheckNoGrayAggroConfig` reads
/// `CONFIG_NO_GRAY_AGGRO_ABOVE` and `CONFIG_NO_GRAY_AGGRO_BELOW` after
/// `Trinity::XP::GetColorCode(playerLevel, creatureLevel) == XP_GRAY`.
#[derive(Clone)]
pub struct LegacyCreatureAggroConfigLikeCpp {
    pub no_gray_aggro_above: u32,
    pub no_gray_aggro_below: u32,
    pub creature_aggro_rate: f32,
    pub max_player_level_config: u32,
    pub faction_template_store: Option<Arc<FactionTemplateStore>>,
    pub faction_store: Option<Arc<FactionStore>>,
    pub map_store: Option<Arc<MapStore>>,
    pub disable_mgr: Option<Arc<DisableMgrLikeCpp>>,
    pub spell_misc_store: Option<Arc<SpellMiscStore>>,
    pub spell_range_store: Option<Arc<SpellRangeStore>>,
    pub spell_duration_store: Option<Arc<SpellDurationStore>>,
    pub spell_cooldowns_store: Option<Arc<wow_data::SpellCooldownsStore>>,
    pub spell_category_store: Option<Arc<SpellCategoryStore>>,
    pub spell_x_spell_visual_store: Option<Arc<wow_data::SpellXSpellVisualStore>>,
    pub spell_target_restrictions_store: Option<Arc<SpellTargetRestrictionsStore>>,
    pub spell_casting_requirements_store: Option<Arc<wow_data::SpellCastingRequirementsStore>>,
    pub spell_aura_restrictions_store: Option<Arc<SpellAuraRestrictionsStore>>,
    pub spell_store: Option<Arc<SpellStore>>,
    pub spell_chain_store: Option<Arc<SpellChainStoreLikeCpp>>,
    pub spell_linked_store: Option<Arc<SpellLinkedStoreLikeCpp>>,
    pub spell_condition_store: Option<Arc<ConditionEntriesByTypeStore>>,
    pub spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub spell_script_all_rank_root_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub spell_linked_rejected_trigger_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub spell_custom_attribute_store: Option<Arc<SpellCustomAttributeStoreLikeCpp>>,
    pub difficulty_store: Option<Arc<DifficultyStore>>,
    pub visibility_distance_continents: f32,
    pub visibility_distance_instances: f32,
    pub visibility_distance_battlegrounds: f32,
    pub visibility_distance_arenas: f32,
    pub family_assistance_radius: f32,
    pub family_assistance_delay_ms: u32,
}

impl Default for LegacyCreatureAggroConfigLikeCpp {
    fn default() -> Self {
        Self {
            no_gray_aggro_above: 0,
            no_gray_aggro_below: 0,
            creature_aggro_rate: 1.0,
            max_player_level_config: 80,
            faction_template_store: None,
            faction_store: None,
            map_store: None,
            disable_mgr: Some(Arc::new(DisableMgrLikeCpp::default())),
            spell_misc_store: None,
            spell_range_store: None,
            spell_duration_store: None,
            spell_cooldowns_store: None,
            spell_category_store: None,
            spell_x_spell_visual_store: None,
            spell_target_restrictions_store: None,
            spell_casting_requirements_store: None,
            spell_aura_restrictions_store: None,
            spell_store: None,
            spell_chain_store: None,
            spell_linked_store: None,
            spell_condition_store: None,
            spell_script_exact_spell_ids_like_cpp: None,
            spell_script_all_rank_root_spell_ids_like_cpp: None,
            legacy_spell_script_spell_ids_like_cpp: None,
            spell_linked_rejected_trigger_spell_ids_like_cpp: None,
            spell_custom_attribute_store: None,
            difficulty_store: None,
            visibility_distance_continents: wow_entities::DEFAULT_VISIBILITY_DISTANCE,
            visibility_distance_instances: wow_entities::DEFAULT_VISIBILITY_INSTANCE,
            visibility_distance_battlegrounds: DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP,
            visibility_distance_arenas: DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP,
            family_assistance_radius: 10.0,
            family_assistance_delay_ms: 1_500,
        }
    }
}

impl LegacyCreatureAggroConfigLikeCpp {
    pub(in crate::session) fn spell_has_no_unrepresented_runtime_hooks_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Some(conditions) = self.spell_condition_store.as_deref() else {
            return false;
        };
        let Ok(signed_spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        if conditions
            .conditions_for_like_cpp(
                wow_constants::ConditionSourceType::Spell,
                wow_data::conditions::ConditionId::new(0, signed_spell_id, 0),
            )
            .is_some()
        {
            // C++ Spell::CheckCast evaluates SourceType 17 before explicit
            // target validation. M2.6 cannot evaluate those predicates.
            return false;
        }
        spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp(
            spell_id,
            self.spell_script_exact_spell_ids_like_cpp.as_deref(),
            self.spell_script_all_rank_root_spell_ids_like_cpp
                .as_deref(),
            self.legacy_spell_script_spell_ids_like_cpp.as_deref(),
            self.spell_linked_rejected_trigger_spell_ids_like_cpp
                .as_deref(),
            self.spell_chain_store.as_deref(),
            self.spell_linked_store.as_deref(),
        )
    }

    /// Prove that C++ `Spell::CheckCast` has no caster-facing requirement
    /// that this bounded creature publication path would otherwise skip.
    ///
    /// Startup installs the effective DB2 + SQL + hotfix authority. Missing
    /// authority and spell IDs outside DB2's signed key domain fail closed;
    /// an absent effective row means the spell has no such requirement.
    fn spell_has_no_unrepresented_casting_requirements_like_cpp(&self, spell_id: u32) -> bool {
        let Some(store) = self.spell_casting_requirements_store.as_deref() else {
            return false;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        match store.entry_for_spell_id_like_cpp(spell_id) {
            Some(requirement) => {
                requirement.facing_caster_flags == 0
                    && requirement.required_areas_id == 0
                    && requirement.requires_spell_focus == 0
            }
            None => true,
        }
    }

    /// This bounded creature-cast slice does not yet own the complete
    /// shapeshift-form authority needed by `SpellInfo::CheckShapeshift`.
    /// A neutral mask is provably safe; every non-neutral mask fails closed.
    fn spell_has_no_unrepresented_shapeshift_requirements_like_cpp(&self, spell_id: u32) -> bool {
        let Some(store) = self.spell_store.as_deref() else {
            return false;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        let (stances, stances_not) = store.shapeshift_masks_like_cpp(spell_id);
        stances == 0 && stances_not == 0
    }

    /// C++ applies the effective `SpellAuraRestrictions` row during
    /// `Spell::CheckCast`/`SpellInfo::CheckExplicitTarget`. This bounded path
    /// does not yet evaluate aura states or required/excluded aura spells, so
    /// only an absent or entirely neutral effective row is safe to publish.
    fn spell_has_no_unrepresented_aura_restrictions_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> bool {
        let Some(store) = self.spell_aura_restrictions_store.as_deref() else {
            return false;
        };
        let Some(restriction) = store.resolved_for_difficulty_chain_like_cpp(
            spell_id,
            creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, self)
                .into_iter()
                .map(u32::from),
        ) else {
            return true;
        };

        restriction.caster_aura_state == 0
            && restriction.target_aura_state == 0
            && restriction.exclude_caster_aura_state == 0
            && restriction.exclude_target_aura_state == 0
            && restriction.caster_aura_spell == 0
            && restriction.target_aura_spell == 0
            && restriction.exclude_caster_aura_spell == 0
            && restriction.exclude_target_aura_spell == 0
    }

    fn creature_faction_template_is_neutral_to_all_like_cpp(
        &self,
        faction_template_id: u32,
    ) -> bool {
        let Some(faction_template_store) = self.faction_template_store.as_ref() else {
            return faction_template_id == 35;
        };
        let Some(faction_template) = faction_template_store.get(faction_template_id) else {
            return false;
        };

        if faction_template.faction == 0 {
            return true;
        }

        if let Some(faction_store) = self.faction_store.as_ref()
            && let Some(raw_faction) = faction_store.get(u32::from(faction_template.faction))
            && raw_faction.can_have_reputation_like_cpp()
        {
            return false;
        }

        faction_template.is_neutral_to_all_like_cpp()
    }

    fn map_is_dungeon_like_cpp(&self, map_id: u16) -> bool {
        self.map_store
            .as_ref()
            .and_then(|store| store.get(u32::from(map_id)))
            .is_some_and(|entry| entry.is_dungeon())
    }

    fn map_visibility_range_like_cpp(&self, map_id: u16) -> f32 {
        let Some(entry) = self
            .map_store
            .as_ref()
            .and_then(|store| store.get(u32::from(map_id)))
        else {
            return self.visibility_distance_continents;
        };

        match entry.instance_type {
            wow_data::map::MAP_ARENA => self.visibility_distance_arenas,
            wow_data::map::MAP_BATTLEGROUND => self.visibility_distance_battlegrounds,
            wow_data::map::MAP_INSTANCE | wow_data::map::MAP_RAID | wow_data::map::MAP_SCENARIO
                if !entry.is_garrison() =>
            {
                self.visibility_distance_instances
            }
            _ => self.visibility_distance_continents,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureAggroTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub sightless_creatures_skipped: usize,
    pub candidates_seen: usize,
    pub targetability_rejections: usize,
    pub visibility_unrepresented: usize,
    pub visibility_rejections: usize,
    pub hostility_rejections: usize,
    pub hostility_unrepresented: usize,
    pub accessibility_rejections: usize,
    pub owner_position_unrepresented: usize,
    pub attacker_evade_rejections: usize,
    pub home_range_rejections: usize,
    pub gray_aggro_rejections: usize,
    pub ai_selection_unrepresented: usize,
    pub ai_los_suppressed: usize,
    pub ai_can_attack_unrepresented: usize,
    pub ai_can_attack_rejections: usize,
    pub alert_triggers: usize,
    pub alert_rejections: usize,
    pub movement_interrupts: usize,
    pub victim_switches: usize,
    pub evades_started: usize,
    pub assistance_scheduled: usize,
    pub assistance_starts: usize,
    pub plan: crate::map_manager::RuntimePlan,
    pub aggro_starts: usize,
    pub commands: Vec<crate::session::mailbox::CreatureAttackStartLikeCppCommand>,
    pub stop_commands: Vec<crate::session::mailbox::CreatureAttackStopLikeCppCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LegacyCreatureThreatUpdateLikeCpp {
    Unchanged,
    Switched {
        previous_victim: ObjectGuid,
    },
    Evade {
        previous_victim: Option<ObjectGuid>,
        participant_guids: Vec<ObjectGuid>,
        removed_taunt_slots: Vec<u8>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureMeleeTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub swings_ready: usize,
    pub runtime_rng_authority_rejections: usize,
    pub melee_outcomes_unrepresented: usize,
    pub melee_precondition_rejections: usize,
    pub melee_range_rejections: usize,
    pub melee_facing_rejections: usize,
    pub attacker_state_rejections: usize,
    pub attacker_incarnation_rejections: usize,
    pub melee_los_rejections: usize,
    pub attacking_interrupt_auras_removed: usize,
    pub canonical_hits: usize,
    pub canonical_creature_hits: usize,
    pub legacy_creature_victim_syncs: usize,
    pub legacy_creature_victim_sync_cas_rejections: usize,
    pub commands: Vec<crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand>,
    pub plan: RuntimePlan,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyCreatureSpellTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub maps_seen: usize,
    pub creatures_seen: usize,
    pub ai_selection_unrepresented: usize,
    pub missing_spell_metadata: usize,
    pub schedules_initialized: usize,
    pub casts_ready: usize,
    pub noninstant_casts_unrepresented: usize,
    pub spell_runtime_hooks_unrepresented: usize,
    pub spell_casting_requirements_unrepresented: usize,
    pub spell_disable_context_unrepresented: usize,
    pub spells_disabled: usize,
    /// TurretAI attempts whose rejected `CastSpell` still consumed BASE_ATTACK
    /// because the raw combat-range gate had admitted them.
    pub turret_rejected_attempt_swings: usize,
    /// Casts dropped because the live creature was no longer the incarnation the
    /// plan had been captured from.
    pub caster_incarnation_rejections: usize,
    pub spell_effects_unrepresented: usize,
    pub spell_projectiles_unrepresented: usize,
    pub spell_visuals_unrepresented: usize,
    pub unit_state_casting_skips: usize,
    pub spell_range_rejections: usize,
    pub spell_los_rejections: usize,
    pub spell_hit_results_unrepresented: usize,
    pub runtime_rng_authority_rejections: usize,
    pub spell_hits: usize,
    pub spell_misses: usize,
    pub canonical_cast_preconditions_passed: usize,
    pub canonical_cast_missing_target: usize,
    pub canonical_cast_target_rejections: usize,
    pub canonical_cast_cooldown_rejections: usize,
    pub plan: RuntimePlan,
}

fn creature_ai_spell_disable_decision_like_cpp(
    spell_id: u32,
    map_id: u16,
    creature: &crate::map_manager::WorldCreature,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> CreatureSpellDisableDecisionLikeCpp {
    let Some(disable_mgr) = config.disable_mgr.as_deref() else {
        return CreatureSpellDisableDecisionLikeCpp::ContextUnrepresented;
    };
    let instance_type = config
        .map_store
        .as_deref()
        .and_then(|store| store.get(u32::from(map_id)))
        .map(|entry| entry.instance_type);
    let area_id = creature.creature.unit().world().area_id();
    disable_mgr.creature_spell_disable_decision_like_cpp(
        spell_id,
        u32::from(map_id),
        (area_id != 0).then_some(area_id),
        instance_type.map(|kind| kind == wow_data::map::MAP_ARENA),
        instance_type.map(|kind| kind == wow_data::map::MAP_BATTLEGROUND),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureSpellTargetHitResultLikeCpp {
    Hit,
    Miss,
}

fn gender_from_u8(value: u8) -> Gender {
    match value {
        1 => Gender::Female,
        2 => Gender::None,
        _ => Gender::Male,
    }
}

fn unix_secs_to_ms_like_cpp(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0).saturating_mul(1_000)
}

fn represented_pet_aura_slot_like_cpp(index: usize) -> Option<u8> {
    u8::try_from(index).ok().filter(|slot| *slot < u8::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedPlayerSkillStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPlayerSkillLikeCpp {
    pub skill_id: u16,
    pub step: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
    pub state: RepresentedPlayerSkillStateLikeCpp,
}

fn canonical_player_skill_record_like_cpp(
    skill: RepresentedPlayerSkillLikeCpp,
) -> wow_entities::PlayerSkillRecord {
    wow_entities::PlayerSkillRecord {
        skill_line_id: u32::from(skill.skill_id),
        current_value: skill.value,
        max_value: skill.max,
        step: skill.step,
        profession_slot: skill.profession_slot,
        state: match skill.state {
            RepresentedPlayerSkillStateLikeCpp::Unchanged => {
                wow_entities::PlayerSkillLoadState::Unchanged
            }
            RepresentedPlayerSkillStateLikeCpp::Changed => {
                wow_entities::PlayerSkillLoadState::Changed
            }
            RepresentedPlayerSkillStateLikeCpp::New => wow_entities::PlayerSkillLoadState::New,
            RepresentedPlayerSkillStateLikeCpp::Deleted => {
                wow_entities::PlayerSkillLoadState::Deleted
            }
        },
    }
}

fn represented_player_skill_record_like_cpp(
    skill: &wow_entities::PlayerSkillRecord,
) -> Option<RepresentedPlayerSkillLikeCpp> {
    Some(RepresentedPlayerSkillLikeCpp {
        skill_id: u16::try_from(skill.skill_line_id).ok()?,
        step: skill.step,
        value: skill.current_value,
        max: skill.max_value,
        profession_slot: skill.profession_slot,
        state: match skill.state {
            wow_entities::PlayerSkillLoadState::Unchanged => {
                RepresentedPlayerSkillStateLikeCpp::Unchanged
            }
            wow_entities::PlayerSkillLoadState::Changed => {
                RepresentedPlayerSkillStateLikeCpp::Changed
            }
            wow_entities::PlayerSkillLoadState::New => RepresentedPlayerSkillStateLikeCpp::New,
            wow_entities::PlayerSkillLoadState::Deleted => {
                RepresentedPlayerSkillStateLikeCpp::Deleted
            }
        },
    })
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedPlayerSpellStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Removed,
    Temporary,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPlayerSpellLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
    pub dependent: bool,
    pub favorite: bool,
    pub state: RepresentedPlayerSpellStateLikeCpp,
}

#[derive(Debug, Clone, Default)]
struct RepresentedPlayerSpellRuntimeLikeCpp {
    known_spells: Vec<i32>,
    rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    #[cfg(test)]
    rows_loaded: bool,
    rows_complete: bool,
    fallback_rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    dependent_known_spells: HashSet<i32>,
    removed_known_spells: HashSet<i32>,
    favorite_known_spells: HashSet<i32>,
    trait_definition_ids: HashMap<i32, i32>,
    trait_definition_ids_complete: bool,
    trait_config_rows: BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
    trait_config_rows_complete: bool,
    trait_entry_rows_complete: bool,
    trait_entry_rows_empty: bool,
    #[cfg(test)]
    override_spells: HashMap<i32, BTreeSet<i32>>,
    override_spells_complete: bool,
}

fn canonical_player_spell_record_like_cpp(
    row: RepresentedPlayerSpellLikeCpp,
) -> wow_entities::PlayerKnownSpellRecord {
    wow_entities::PlayerKnownSpellRecord {
        spell_id: row.spell_id,
        state: match row.state {
            RepresentedPlayerSpellStateLikeCpp::Unchanged => {
                wow_entities::PlayerSpellLoadState::Unchanged
            }
            RepresentedPlayerSpellStateLikeCpp::Changed => {
                wow_entities::PlayerSpellLoadState::Changed
            }
            RepresentedPlayerSpellStateLikeCpp::New => wow_entities::PlayerSpellLoadState::New,
            RepresentedPlayerSpellStateLikeCpp::Removed => {
                wow_entities::PlayerSpellLoadState::Removed
            }
            RepresentedPlayerSpellStateLikeCpp::Temporary => {
                wow_entities::PlayerSpellLoadState::Temporary
            }
        },
        active: row.active,
        disabled: row.disabled,
        favorite: row.favorite,
        dependent: row.dependent,
    }
}

fn represented_player_spell_record_like_cpp(
    row: &wow_entities::PlayerKnownSpellRecord,
) -> RepresentedPlayerSpellLikeCpp {
    RepresentedPlayerSpellLikeCpp {
        spell_id: row.spell_id,
        active: row.active,
        disabled: row.disabled,
        dependent: row.dependent,
        favorite: row.favorite,
        state: match row.state {
            wow_entities::PlayerSpellLoadState::Unchanged => {
                RepresentedPlayerSpellStateLikeCpp::Unchanged
            }
            wow_entities::PlayerSpellLoadState::Changed => {
                RepresentedPlayerSpellStateLikeCpp::Changed
            }
            wow_entities::PlayerSpellLoadState::New => RepresentedPlayerSpellStateLikeCpp::New,
            wow_entities::PlayerSpellLoadState::Removed => {
                RepresentedPlayerSpellStateLikeCpp::Removed
            }
            wow_entities::PlayerSpellLoadState::Temporary => {
                RepresentedPlayerSpellStateLikeCpp::Temporary
            }
        },
    }
}

#[cfg(test)]
fn canonical_player_spell_runtime_like_cpp(
    runtime: RepresentedPlayerSpellRuntimeLikeCpp,
) -> wow_entities::PlayerSpellRuntimeState {
    wow_entities::PlayerSpellRuntimeState {
        known_spells: runtime.known_spells,
        rows: runtime
            .rows
            .into_iter()
            .map(|(spell_id, row)| (spell_id, canonical_player_spell_record_like_cpp(row)))
            .collect(),
        rows_loaded: runtime.rows_loaded,
        rows_complete: runtime.rows_complete,
        fallback_rows: runtime
            .fallback_rows
            .into_iter()
            .map(|(spell_id, row)| (spell_id, canonical_player_spell_record_like_cpp(row)))
            .collect(),
        dependent_known_spells: runtime.dependent_known_spells.into_iter().collect(),
        removed_known_spells: runtime.removed_known_spells.into_iter().collect(),
        favorite_known_spells: runtime.favorite_known_spells.into_iter().collect(),
        trait_definition_ids: runtime.trait_definition_ids.into_iter().collect(),
        trait_definition_ids_complete: runtime.trait_definition_ids_complete,
        trait_config_rows: runtime.trait_config_rows,
        trait_config_rows_complete: runtime.trait_config_rows_complete,
        trait_entry_rows_complete: runtime.trait_entry_rows_complete,
        trait_entry_rows_empty: runtime.trait_entry_rows_empty,
        override_spells: runtime.override_spells.into_iter().collect(),
        override_spells_complete: runtime.override_spells_complete,
    }
}

fn represented_player_spell_runtime_like_cpp(
    runtime: &wow_entities::PlayerSpellRuntimeState,
) -> RepresentedPlayerSpellRuntimeLikeCpp {
    RepresentedPlayerSpellRuntimeLikeCpp {
        known_spells: runtime.known_spells.clone(),
        rows: runtime
            .rows
            .iter()
            .map(|(&spell_id, row)| (spell_id, represented_player_spell_record_like_cpp(row)))
            .collect(),
        #[cfg(test)]
        rows_loaded: runtime.rows_loaded,
        rows_complete: runtime.rows_complete,
        fallback_rows: runtime
            .fallback_rows
            .iter()
            .map(|(&spell_id, row)| (spell_id, represented_player_spell_record_like_cpp(row)))
            .collect(),
        dependent_known_spells: runtime.dependent_known_spells.iter().copied().collect(),
        removed_known_spells: runtime.removed_known_spells.iter().copied().collect(),
        favorite_known_spells: runtime.favorite_known_spells.iter().copied().collect(),
        trait_definition_ids: runtime
            .trait_definition_ids
            .iter()
            .map(|(&spell_id, &trait_definition_id)| (spell_id, trait_definition_id))
            .collect(),
        trait_definition_ids_complete: runtime.trait_definition_ids_complete,
        trait_config_rows: runtime.trait_config_rows.clone(),
        trait_config_rows_complete: runtime.trait_config_rows_complete,
        trait_entry_rows_complete: runtime.trait_entry_rows_complete,
        trait_entry_rows_empty: runtime.trait_entry_rows_empty,
        #[cfg(test)]
        override_spells: runtime
            .override_spells
            .iter()
            .map(|(&spell_id, overrides)| (spell_id, overrides.clone()))
            .collect(),
        override_spells_complete: runtime.override_spells_complete,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCharacterSpellCooldownLikeCpp {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCharacterSpellChargeLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    pub plans: Vec<ApplyEnchantmentPlan>,
    pub duration_updates: Vec<PlayerEnchantTimeUpdate>,
    pub send_stat_update: bool,
    pub visible_item_changes: Vec<(u8, i32, u16, u16)>,
    pub effect_actions: Vec<RepresentedItemBonusActionLikeCpp>,
    pub unrepresented_effect_actions: Vec<RepresentedItemBonusActionLikeCpp>,
}

impl LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    pub(crate) fn append(&mut self, mut other: Self) {
        self.plans.append(&mut other.plans);
        self.duration_updates.append(&mut other.duration_updates);
        self.send_stat_update |= other.send_stat_update;
        self.visible_item_changes
            .append(&mut other.visible_item_changes);
        self.effect_actions.append(&mut other.effect_actions);
        self.unrepresented_effect_actions
            .append(&mut other.unrepresented_effect_actions);
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct InitialLoadedItemModsOutcomeLikeCpp {
    pub item_set_auras: usize,
    pub item_equip_auras: usize,
    pub enchantments: LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
}

#[allow(dead_code)]
fn represented_skill_records_from_values_like_cpp(
    skill_values: &HashMap<u16, u16>,
) -> HashMap<u16, RepresentedPlayerSkillLikeCpp> {
    skill_values
        .iter()
        .map(|(&skill_id, &value)| {
            (
                skill_id,
                RepresentedPlayerSkillLikeCpp {
                    skill_id,
                    step: 0,
                    value,
                    max: value,
                    profession_slot: -1,
                    state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
                },
            )
        })
        .collect()
}

fn represented_skill_values_from_records_like_cpp(
    skill_records: &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
) -> HashMap<u16, u16> {
    skill_records
        .iter()
        .map(|(&skill_id, record)| (skill_id, record.value))
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct SessionPlayerController {
    guid: ObjectGuid,
    name: String,
    position: wow_core::Position,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
}

impl SessionPlayerController {
    pub(crate) fn new(
        guid: ObjectGuid,
        name: String,
        position: wow_core::Position,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) -> Self {
        Self {
            guid,
            name,
            position,
            map_id,
            race,
            class,
            level,
            gender,
        }
    }

    pub(crate) fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn position(&self) -> wow_core::Position {
        self.position
    }

    pub(crate) fn map_id(&self) -> u16 {
        self.map_id
    }

    pub(crate) fn race(&self) -> u8 {
        self.race
    }

    pub(crate) fn class(&self) -> u8 {
        self.class
    }

    pub(crate) fn level(&self) -> u8 {
        self.level
    }

    pub(crate) fn gender(&self) -> u8 {
        self.gender
    }
}

/// Current state of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Authenticated but no character selected.
    Authed,
    /// Character is logged into the world.
    LoggedIn,
    /// Character is transferring between maps.
    Transfer,
    /// Session is being disconnected.
    Disconnecting,
}

// Compatibility paths while #578 moves cast consumers out of the Session adapter.
pub(crate) use wow_entities::PendingSpellCastRequestLikeCpp as RepresentedPendingSpellCastRequestLikeCpp;
pub use wow_entities::{SpellCastBattlePetItemModifiersLikeCpp, SpellCastMetadata, SpellCastState};

const SPELL_FAILED_DONT_REPORT_LIKE_CPP: i32 = 32;

/// C++ `WorldSession::_accountData[NUM_ACCOUNT_DATA_TYPES]` entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AccountDataLikeCpp {
    pub time: i64,
    pub data: String,
}

pub(crate) const ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP: u32 = 0x7FFF;
pub(crate) const GLOBAL_CACHE_MASK_LIKE_CPP: u32 = 0x2515;
pub(crate) const PER_CHARACTER_CACHE_MASK_LIKE_CPP: u32 = 0x5AEA;

fn default_account_data_like_cpp() -> [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES] {
    std::array::from_fn(|_| AccountDataLikeCpp::default())
}

fn trinity_sprintf_like_cpp(format: &str, args: &[&str]) -> String {
    let mut output = String::with_capacity(format.len());
    let mut chars = format.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some('%') => output.push('%'),
            Some('u' | 'd' | 's') => {
                if let Some(arg) = args.get(arg_idx) {
                    output.push_str(arg);
                    arg_idx += 1;
                }
            }
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }

    output
}

fn player_cuf_profile_from_packet_like_cpp(
    profile: wow_packet::packets::misc::CufProfile,
) -> wow_entities::PlayerCufProfile {
    wow_entities::PlayerCufProfile {
        profile_name: profile.profile_name,
        frame_height: profile.frame_height,
        frame_width: profile.frame_width,
        sort_by: profile.sort_by,
        health_text: profile.health_text,
        top_point: profile.top_point,
        bottom_point: profile.bottom_point,
        left_point: profile.left_point,
        top_offset: profile.top_offset,
        bottom_offset: profile.bottom_offset,
        left_offset: profile.left_offset,
        bool_options: profile.bool_options,
    }
}

fn player_cuf_profile_to_packet_like_cpp(
    profile: &wow_entities::PlayerCufProfile,
) -> wow_packet::packets::misc::CufProfile {
    wow_packet::packets::misc::CufProfile {
        profile_name: profile.profile_name.clone(),
        frame_height: profile.frame_height,
        frame_width: profile.frame_width,
        sort_by: profile.sort_by,
        health_text: profile.health_text,
        top_point: profile.top_point,
        bottom_point: profile.bottom_point,
        left_point: profile.left_point,
        top_offset: profile.top_offset,
        bottom_offset: profile.bottom_offset,
        left_offset: profile.left_offset,
        bool_options: profile.bool_options,
    }
}

#[cfg(test)]
struct PlayerTransportLoginStateLikeCpp {
    info: wow_packet::packets::movement::TransportInfo,
}

/// Per-player session on the world server.
///
/// Receives deserialized packets from the socket layer via a channel,
/// dispatches them to registered handlers, and sends responses back.
#[derive(Clone, Default)]
pub struct SessionAdmissionPersistenceLikeCpp {
    pub(crate) character_administration:
        Option<Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>>,
    pub(crate) character_enumeration:
        Option<Arc<dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp>>,
    pub(crate) session_account_state:
        Option<Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>>,
    pub(crate) packet_spoof_ban:
        Option<Arc<dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp>>,
    pub(crate) player_name_query:
        Option<Arc<dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp>>,
    pub(crate) support_bug_report:
        Option<Arc<dyn wow_persistence::SupportBugReportPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct PlayerPersistenceCapabilitiesLikeCpp {
    pub(crate) player_lifecycle: Option<Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>>,
    pub(crate) void_storage: Option<Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp>>,
    pub(crate) social: Option<Arc<dyn wow_persistence::SocialPersistencePortLikeCpp>>,
    pub(crate) stored_item_money:
        Option<Arc<dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp>>,
    pub(crate) stored_item: Option<Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp>>,
    pub(crate) player_inventory:
        Option<Arc<dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp>>,
    pub(crate) player_quest: Option<Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp>>,
    /// Commits one complete quest-reward operation as a single character
    /// transaction, the way C++ closes `Player::RewardQuest` with
    /// `SaveToDB(false)` (Player.cpp:14867).
    pub(crate) player_quest_reward:
        Option<Arc<dyn wow_persistence::PlayerQuestRewardPersistencePortLikeCpp>>,
    pub(crate) vendor_trade: Option<Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp>>,
    pub(crate) player_spell_acquisition:
        Option<Arc<dyn wow_persistence::PlayerSpellAcquisitionPersistencePortLikeCpp>>,
    pub(crate) instance_lock: Option<Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>>,
    pub(crate) battle_pet_purchase:
        Option<Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct WorldPersistenceCapabilitiesLikeCpp {
    pub(crate) map_corpse: Option<Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>>,
    pub(crate) group_loot_money:
        Option<Arc<dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp>>,
    pub(crate) represented_group:
        Option<Arc<dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct CatalogPersistenceCapabilitiesLikeCpp {
    pub(crate) quest_poi: Option<Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp>>,
    pub(crate) item_template_addon_catalog:
        Option<Arc<dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp>>,
    pub(crate) loot_template_catalog:
        Option<Arc<dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp>>,
    pub(crate) vendor_catalog:
        Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>>,
    pub(crate) visibility_spawn_catalog:
        Option<Arc<dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp>>,
    pub(crate) gossip_catalog:
        Option<Arc<dyn wow_persistence::GossipCatalogPersistencePortLikeCpp>>,
}

#[derive(Clone, Default)]
pub struct SessionPersistencePortsLikeCpp {
    pub(crate) admission: SessionAdmissionPersistenceLikeCpp,
    pub(crate) player: PlayerPersistenceCapabilitiesLikeCpp,
    pub(crate) world: WorldPersistenceCapabilitiesLikeCpp,
    pub(crate) catalogs: CatalogPersistenceCapabilitiesLikeCpp,
}

pub struct WorldSession {
    /// The realm/instance transport, owned by `wow-session` (#297).
    ///
    /// The first piece of this type to earn its own crate: it compiles without
    /// gameplay, databases or catalogs, so the compiler now prevents transport
    /// decisions from reaching a `Player`, a `Map` or a query.
    connection: wow_session::SessionConnection,
    // Account info
    pub account_id: u32,
    battlenet_account_id: u32,
    realm_list_secret_like_cpp: [u8; 32],
    recruiter_id_like_cpp: u32,
    is_a_recruiter_like_cpp: bool,
    pub account_name: String,
    pub security: u8,
    pub expansion: u8,
    pub account_expansion: u8,
    server_expansion_like_cpp: u8,
    #[cfg(test)]
    characters_per_realm_like_cpp: u32,
    #[cfg(test)]
    declined_names_used_like_cpp: bool,
    #[cfg(test)]
    feature_system_bpay_store_enabled_like_cpp: bool,
    #[cfg(test)]
    feature_system_character_undelete_enabled_like_cpp: bool,
    instance_ignore_raid_like_cpp: bool,
    instance_ignore_level_like_cpp: bool,
    max_instances_per_hour_like_cpp: u32,
    #[cfg(test)]
    start_all_explored_like_cpp: bool,
    #[cfg(test)]
    start_all_reputation_like_cpp: bool,
    #[cfg(test)]
    start_all_spells_like_cpp: bool,
    #[cfg(test)]
    player_create_info_store_like_cpp: Option<Arc<PlayerCreateInfoStoreLikeCpp>>,
    #[cfg(test)]
    player_create_cast_spell_store_like_cpp: Option<Arc<PlayerCreateInfoCastSpellStoreLikeCpp>>,
    #[cfg(test)]
    player_create_custom_spell_store_like_cpp: Option<Arc<PlayerCreateInfoCustomSpellStoreLikeCpp>>,
    pub build: u32,
    pub session_key: Vec<u8>,
    pub locale: String,
    mute_time_like_cpp: i64,

    // Inbound packet queue (from WorldSocket)

    // Outbound channel (serialized bytes back to WorldSocket)
    // FIFO completion fence paired with the current physical send channel.

    // Cross-session commands executed by this session's own update loop.
    session_command_tx: flume::Sender<SessionCommand>,
    session_command_rx: flume::Receiver<SessionCommand>,
    durable_creature_runtime_commands_like_cpp:
        Arc<std::sync::Mutex<crate::session::mailbox::DurableCreatureRuntimeCommandsLikeCpp>>,
    visibility_refresh_pending_like_cpp: Arc<AtomicBool>,

    // State
    state: SessionState,
    last_packet_time: Instant,
    socket_timeouts_like_cpp: SocketTimeoutsLikeCpp,
    socket_timeout_deadline_like_cpp: Instant,
    packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp,
    packet_throttling_like_cpp: HashMap<u16, PacketCounterLikeCpp>,
    remote_address_like_cpp: Option<String>,
    pending_packet_spoof_ban_like_cpp: Option<PacketSpoofPendingBanLikeCpp>,
    legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp,
    /// Session-owned RNG for represented gameplay choices that C++ resolves through
    /// `urand`/`SelectRandomContainerElement` while the owning Player/Map runtime is
    /// still being split out of `WorldSession`.
    represented_runtime_rng_like_cpp: StdRng,

    // Dispatch table (built once, shared ref)
    dispatch_table: HashMap<ClientOpcodes, &'static PacketHandlerEntry>,

    // FIFO sender for C++ CharacterDatabase.Execute-style detached homebind
    // writes. Its single worker drains queued jobs after session teardown and
    // preserves call order.
    homebind_persistence_tx_like_cpp:
        Option<tokio::sync::mpsc::UnboundedSender<HomebindPersistenceJobLikeCpp>>,

    /// Typed database capabilities live behind one indirection so adding a
    /// persistence workflow does not keep growing this already-large session;
    /// `wow-database` supplies the concrete adapters.
    pub(crate) persistence_ports_like_cpp: Box<SessionPersistencePortsLikeCpp>,

    // C++ ObjectMgr trainer definitions and creature bindings.
    trainer_store_like_cpp: Option<Arc<TrainerStoreLikeCpp>>,

    // BankBagSlotPrices.db2 store used by C++ HandleBuyBankSlotOpcode.
    #[cfg(test)]
    bank_bag_slot_prices_store: Option<Arc<BankBagSlotPricesStore>>,

    // Currency types store (CurrencyTypes.db2 data)
    currency_types_store: Option<Arc<CurrencyTypesStore>>,

    // Import price stores (ImportPrice*.db2 data)
    #[cfg(test)]
    import_price_stores: Option<Arc<ImportPriceStores>>,

    // Emotes.db2 / EmotesText.db2 stores used by C++ chat text-emote handling.
    #[cfg(test)]
    emotes_store: Option<Arc<EmotesStore>>,
    #[cfg(test)]
    emotes_text_store: Option<Arc<EmotesTextStore>>,

    // Item class store (ItemClass.db2 data)
    #[cfg(test)]
    item_class_store: Option<Arc<ItemClassStore>>,

    // Item currency cost store (ItemCurrencyCost.db2 data)
    #[cfg(test)]
    item_currency_cost_store: Option<Arc<ItemCurrencyCostStore>>,

    /// Item template and item-data catalogs a session reads. Owned by one type (#670).
    pub(crate) items: crate::catalogs::item::ItemCatalogsLikeCpp,

    // Trinity strings loaded from world DB `trinity_string`.
    trinity_string_store: Option<Arc<TrinityStringStoreLikeCpp>>,

    // Heirloom store (Heirloom.db2 data)
    heirloom_store: Option<Arc<HeirloomStore>>,

    // Toy store (Toy.db2 data)
    toy_store: Option<Arc<ToyStore>>,

    // Battle-pet stat stores used by C++ `BattlePet::CalculateStats`.
    #[cfg(test)]
    battle_pet_breed_quality_store: Option<Arc<BattlePetBreedQualityStore>>,
    #[cfg(test)]
    battle_pet_breed_state_store: Option<Arc<BattlePetBreedStateStore>>,
    #[cfg(test)]
    battle_pet_species_store: Option<Arc<BattlePetSpeciesStore>>,
    #[cfg(test)]
    battle_pet_species_state_store: Option<Arc<BattlePetSpeciesStateStore>>,
    #[cfg(test)]
    battle_pet_xp_game_table: Option<Arc<BattlePetXpGameTableLikeCpp>>,

    // C++ `sCombatRatingsGameTable` used by `Player::GetRatingMultiplier`.
    combat_ratings_game_table: Option<Arc<CombatRatingsGameTableLikeCpp>>,

    // C++ `sShieldBlockRegularGameTable` used by `ItemTemplate::GetShieldBlockValue`.
    shield_block_regular_game_table: Option<Arc<ShieldBlockRegularGameTableLikeCpp>>,

    // Transmog set item store (TransmogSetItem.db2 data)
    transmog_set_item_store: Option<Arc<TransmogSetItemStore>>,

    // Item price base store (ItemPriceBase.db2 data)
    #[cfg(test)]
    item_price_base_store: Option<Arc<ItemPriceBaseStore>>,

    // Player level stats store (race/class/level → base stats)
    player_stats: Option<Arc<PlayerStatsStore>>,
    #[cfg(test)]
    represented_item_level_caps_like_cpp: RepresentedItemLevelCapsLikeCpp,
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::_usePvpItemLevels` lives on the canonical Player.
    #[cfg(test)]
    represented_using_pvp_item_levels_like_cpp: bool,

    pvp_item_store: Option<Arc<PvpItemStore>>,
    /// Every spell and aura catalog slot, owned by one type (#668).
    pub(crate) spell_catalogs: crate::catalogs::spell::SpellCatalogsLikeCpp,
    durability_costs_store: Option<Arc<DurabilityCostsStore>>,
    durability_quality_store: Option<Arc<DurabilityQualityStore>>,
    item_template_addon_quest_log_item_ids_like_cpp: HashMap<u32, u32>,

    // RandPropPoints store (RandPropPoints.db2 data)
    rand_prop_points_store: Option<Arc<RandPropPointsStore>>,

    // ItemDisenchantLoot store (ItemDisenchantLoot.db2 data)
    #[cfg(test)]
    item_disenchant_loot_store: Option<Arc<ItemDisenchantLootStore>>,

    // C++ LootTemplates_* store foundation.
    loot_stores: Option<Arc<LootStores>>,

    // C++ ConditionMgr condition store loaded from world.conditions.
    condition_store: Option<Arc<ConditionEntriesByTypeStore>>,

    // C++ PlayerCondition.db2 store used by ConditionMgr player-condition checks.
    player_condition_store: Option<Arc<PlayerConditionStore>>,

    // C++ AdventureMapPOI.db2 store used by Adventure Map quest starts.
    #[cfg(test)]
    adventure_map_poi_store: Option<Arc<AdventureMapPoiStore>>,

    // C++ ContentTuning.db2 store used by level gates such as Meeting Stone.
    content_tuning_store: Option<Arc<ContentTuningStore>>,
    curve_store: Option<Arc<CurveStore>>,
    curve_point_store: Option<Arc<CurvePointStore>>,
    scaling_stat_distribution_store: Option<Arc<ScalingStatDistributionStore>>,
    scaling_stat_values_store: Option<Arc<ScalingStatValuesStore>>,

    // C++ DisableMgr store loaded from world.disables.
    disable_mgr: Option<Arc<DisableMgrLikeCpp>>,

    // C++ Difficulty.db2 store used by sDifficultyStore difficulty changes.
    difficulty_store: Option<Arc<DifficultyStore>>,

    // Lock store (Lock.db2 data)
    lock_store: Option<Arc<LockStore>>,

    gem_properties_store: Option<Arc<GemPropertiesStore>>,

    #[cfg(test)]
    tact_key_store: Option<Arc<TactKeyStore>>,

    // Skill store (auto-learned spells from SkillLineAbility.db2 + SkillRaceClassInfo.db2)
    skill_store: Option<Arc<SkillStore>>,

    // TraitDefinition.db2 store used by represented PlayerSpell::TraitDefinitionId cleanup.
    trait_definition_store: Option<Arc<TraitDefinitionStore>>,

    // SkillLine.db2 store for C++ parent/expansion skill resolution.
    skill_line_store: Option<Arc<SkillLineStore>>,

    // C++ ObjectMgr::_skillTiers loaded from world.skill_tiers.
    skill_tiers_store: Option<Arc<SkillTiersStoreLikeCpp>>,

    // Area table store (area hierarchy + mount flags)
    area_table_store: Option<Arc<AreaTableStore>>,

    // C++ ObjectMgr fishing base skill levels loaded from skill_fishing_base_level.
    fishing_base_skill_store: Option<Arc<FishingBaseSkillStoreLikeCpp>>,

    // Area-trigger catalogs are process-owned and borrowed for each
    // production session pass. These retained fields are test fixtures only.
    #[cfg(test)]
    area_trigger_db2_store: Option<Arc<AreaTriggerDb2Store>>,
    #[cfg(test)]
    area_trigger_store: Option<Arc<AreaTriggerStore>>,
    #[cfg(test)]
    area_trigger_script_store: Option<Arc<AreaTriggerScriptStoreLikeCpp>>,
    #[cfg(test)]
    area_trigger_script_dispatcher_like_cpp: Option<AreaTriggerScriptDispatcherLikeCpp>,
    #[cfg(test)]
    give_player_xp_script_dispatcher_like_cpp: Option<GivePlayerXpScriptDispatcherLikeCpp>,
    /// Ordered record of the driver phases this session has run. Test-only:
    /// it exists so tests assert on the production sequence in
    /// `session::driver` instead of reimplementing it.
    #[cfg(test)]
    driver_phase_trace_like_cpp: Vec<crate::session::driver::phases::SessionDriverPhaseLikeCpp>,
    #[cfg(test)]
    tavern_area_trigger_store: Option<Arc<TavernAreaTriggerStoreLikeCpp>>,

    // C++ ObjectMgr::GraveyardStore loaded from graveyard_zone plus attached conditions.
    #[cfg(test)]
    graveyard_store: Option<Arc<GraveyardStore>>,

    /// Character race/class catalogs a session reads. Owned by one type (#670).
    pub(crate) chr: crate::catalogs::chr::ChrCatalogsLikeCpp,

    /// Map and map-difficulty catalogs a session reads. Owned by one type (#670).
    pub(crate) maps: crate::catalogs::map::MapCatalogsLikeCpp,
    world_safe_loc_store_like_cpp: Option<Arc<WorldSafeLocStore>>,
    access_requirement_store: Option<Arc<AccessRequirementStoreLikeCpp>>,
    lfg_dungeons_store: Option<Arc<LfgDungeonsStore>>,
    #[cfg(test)]
    lfg_dungeon_store_like_cpp: Option<Arc<LfgDungeonStoreLikeCpp>>,
    #[cfg(test)]
    battlemaster_list_store: Option<Arc<BattlemasterListStore>>,
    #[cfg(test)]
    pub(crate) represented_dungeon_difficulty_id_like_cpp: u32,
    #[cfg(test)]
    represented_raid_difficulty_id_like_cpp: u32,
    #[cfg(test)]
    represented_legacy_raid_difficulty_id_like_cpp: u32,
    #[cfg(test)]
    represented_player_recent_instances_like_cpp: HashMap<u32, u32>,
    /// Faction and reputation catalogs a session reads. Owned by one type (#670).
    pub(crate) factions: crate::catalogs::faction::FactionCatalogsLikeCpp,
    friendship_rep_reaction_store: Option<Arc<FriendshipRepReactionStore>>,
    paragon_reputation_store: Option<Arc<ParagonReputationStore>>,
    reputation_reward_rate_store: Option<Arc<ReputationRewardRateStoreLikeCpp>>,
    /// Creature template and creature-data catalogs a session reads. Owned by one type (#670).
    pub(crate) creatures: crate::catalogs::creature::CreatureCatalogsLikeCpp,
    reputation_spillover_template_store: Option<Arc<RepSpilloverTemplateStoreLikeCpp>>,
    #[cfg(test)]
    championing_faction_like_cpp: u32,
    #[cfg(test)]
    creature_equipment_store_like_cpp: Option<Arc<CreatureEquipmentStoreLikeCpp>>,
    /// GameObject template catalogs a session reads. Owned by one type (#670).
    pub(crate) gameobjects: crate::catalogs::gameobject::GameObjectCatalogsLikeCpp,
    #[cfg(test)]
    creature_addon_store_like_cpp: Option<Arc<CreatureAddonStoreLikeCpp>>,
    #[cfg(test)]
    creature_difficulty_store_like_cpp: Option<Arc<CreatureDifficultyStoreLikeCpp>>,
    #[cfg(test)]
    creature_base_stats_store_like_cpp: Option<Arc<CreatureBaseStatsStoreLikeCpp>>,
    #[cfg(test)]
    creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp,
    mount_store: Option<Arc<MountStore>>,
    mount_definition_store_like_cpp: Option<Arc<MountDefinitionStoreLikeCpp>>,
    mount_capability_store: Option<Arc<MountCapabilityStore>>,
    mount_type_x_capability_store: Option<Arc<MountTypeXCapabilityStore>>,
    mount_x_display_store: Option<Arc<MountXDisplayStore>>,
    vehicle_store: Option<Arc<VehicleStore>>,
    vehicle_seat_store: Option<Arc<VehicleSeatStore>>,
    #[cfg(test)]
    vehicle_template_store: Option<Arc<VehicleTemplateStoreLikeCpp>>,
    vehicle_accessory_store: Option<Arc<VehicleAccessoryStoreLikeCpp>>,
    terrain_swap_store: Option<Arc<wow_data::TerrainSwapStore>>,
    phase_store: Option<Arc<PhaseStore>>,
    phase_group_store: Option<Arc<PhaseGroupStore>>,

    // Shared player registry for broadcasting to nearby sessions
    player_registry: Option<Arc<PlayerRegistry>>,

    // Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    game_event_quest_complete_tx: Option<flume::Sender<GameEventQuestCompleteCommandLikeCpp>>,

    // Shared group registry for party management
    group_registry: Option<Arc<GroupRegistry>>,

    // Pending party invites: invited_guid → inviter_guid
    pending_invites: Option<Arc<PendingInvites>>,

    // Test-only compatibility for pre-#578 fixtures. Production group
    // membership and Player-owned update sequences live on canonical Player.
    #[cfg(test)]
    pub(crate) group_guid: Option<u64>,
    #[cfg(test)]
    represented_subgroup_like_cpp: Option<u8>,
    #[cfg(test)]
    represented_group_update_sequences_like_cpp: [wow_entities::PlayerGroupUpdateSequenceLikeCpp;
        wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP as usize],
    #[cfg(test)]
    pub(crate) pass_on_group_loot: bool,
    #[cfg(test)]
    pub(crate) represented_enchanting_skill: u16,
    #[cfg(test)]
    player_skill_values_like_cpp: HashMap<u16, u16>,
    #[cfg(test)]
    player_skill_records_like_cpp: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    // C++ `_SaveSkills` keeps deleted update-field slots as UNCHANGED
    // tombstones after deleting their Character DB rows. Rust's represented
    // full-save path rewrites the table, so retain their non-durable identity
    // explicitly and never manufacture zero-valued DB rows on a later save.
    #[cfg(test)]
    player_skill_non_durable_tombstones_like_cpp: BTreeSet<u16>,
    #[cfg(test)]
    player_skill_records_loaded_like_cpp: bool,
    #[cfg(test)]
    player_skill_records_complete_like_cpp: bool,
    #[cfg(test)]
    player_skill_occupied_slots_like_cpp: Option<u16>,
    #[cfg(test)]
    represented_gray_level_script_overrides_like_cpp: HashMap<u8, u8>,

    // Realm ID for GUID creation
    realm_id: u16,
    realm_region: u8,
    realm_battlegroup: u8,
    realm_names_like_cpp: BTreeMap<u32, (String, String)>,

    // Process-owned GUID generators retained only as test fixtures.
    #[cfg(test)]
    guid_generator: Option<Arc<ObjectGuidGenerator>>,
    // Process-wide C++ ObjectMgr generator retained only as a test fixture.
    #[cfg(test)]
    item_guid_generator_like_cpp: Option<Arc<ObjectGuidGenerator>>,
    // Process-wide C++ ObjectMgr generator shared by equipment and transmog sets.
    #[cfg(test)]
    equipment_set_guid_generator_like_cpp: Option<Arc<EquipmentSetGuidGeneratorLikeCpp>>,
    // Process-wide C++ ObjectMgr generator for character_void_storage.itemId.
    #[cfg(test)]
    void_storage_item_id_generator_like_cpp: Option<Arc<VoidStorageItemIdGeneratorLikeCpp>>,

    // Characters confirmed for this account
    legit_characters: Vec<ObjectGuid>,

    // Pending async packets to process
    pending_packets: VecDeque<WorldPacket>,
    character_rename_callbacks: driver::RenameCallbacks,

    // ── ConnectTo flow ──────────────────────────────────────────
    /// GUID of the character being logged in (set during PlayerLogin).
    player_loading: Option<ObjectGuid>,
    /// Strong identity for this session's process-wide live-character claim.
    player_login_claim_like_cpp: Option<(ObjectGuid, Arc<()>)>,
    /// C++ `WorldSession::m_playerLogout`: true only while the logout routine is executing.
    player_logout_like_cpp: bool,
    finalization: Option<crate::finalization::SessionFinalization>,

    /// Session manager for ConnectTo flow (shared with instance listener).
    session_mgr: Option<Arc<SessionManager>>,

    // ── Time sync ─────────────────────────────────────────────────
    /// Next sequence index for TimeSyncRequest.
    pub(crate) time_sync_next_counter: u32,

    /// Time remaining until next TimeSyncRequest (in ms).
    pub(crate) time_sync_timer_ms: u32,
    /// Time sync requests sent to the client: sequence index -> server send time.
    pub(crate) time_sync_pending_requests: HashMap<u32, u32>,
    /// Last Trinity-style clock delta samples, `(clock_delta, round_trip_duration)`.
    pub(crate) time_sync_clock_delta_queue: VecDeque<(i64, u32)>,
    /// Server-client clock delta used to translate movement times.
    pub(crate) time_sync_clock_delta: i64,

    // ── Logout ──────────────────────────────────────────────────────
    /// When set, the session is counting down to logout (20s timer).
    /// `None` means no logout is pending.
    pub(crate) logout_time: Option<Instant>,
    /// Timestamp set when the player enters the world (PlayerLogin).
    pub(crate) login_time: Option<Instant>,
    /// C++ `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` in milliseconds.
    player_save_interval_ms_like_cpp: u32,
    /// C++ `Player::m_nextSave` countdown in milliseconds; 0 disables autosave.
    next_player_save_ms_like_cpp: u32,
    /// Set by the sync update loop when the autosave countdown expires.
    pending_periodic_player_save_like_cpp: bool,
    /// Total played time loaded from DB (seconds).
    pub(crate) total_played_time: u32,
    /// Time played at current level loaded from DB (seconds).
    pub(crate) level_played_time: u32,
    /// C++ `CONFIG_MAX_PLAYER_LEVEL`. `RestMgr::SetRestBonus` reads this value
    /// directly; `Player::IsMaxLevel` reads the expansion-bounded active field.
    max_player_level_config_like_cpp: u32,
    /// C++ `CONFIG_MAX_PRIMARY_TRADE_SKILL`, kept independent from talent
    /// `CharacterPoints` and from the two physical profession associations.
    max_primary_trade_skills_like_cpp: u8,
    /// C++ `World::IsPvPRealm()` classification.
    is_pvp_realm_like_cpp: bool,
    /// C++ `World::IsFFAPvPRealm()` classification.
    is_ffa_pvp_realm_like_cpp: bool,
    /// C++ Recruit-A-Friend XP level gates used by `Player::GetsRecruitAFriendBonus(true)`.
    max_recruit_a_friend_bonus_player_level_like_cpp: u32,
    max_recruit_a_friend_bonus_player_level_difference_like_cpp: u32,
    /// C++ `RestMgr::_restBonus[REST_TYPE_XP]`, loaded from `characters.rest_bonus`.
    #[cfg(test)]
    represented_rest_bonus_xp_like_cpp: f32,
    /// C++ `UF::ActivePlayerData::RestInfo[REST_TYPE_XP].StateID`.
    #[cfg(test)]
    represented_rest_state_xp_like_cpp: u8,
    /// C++ `RestMgr::_restFlagMask`.
    #[cfg(test)]
    represented_rest_flag_mask_like_cpp: u32,
    /// Whether UpdateArea/UpdateZone has rebuilt the session-local RestMgr flags.
    #[cfg(test)]
    represented_rest_location_initialized_like_cpp: bool,
    /// Coalesce UpdateArea + UpdateZone rest mutations into one visible field transition.
    #[cfg(test)]
    represented_defer_rest_flag_sync_like_cpp: bool,
    /// A deferred RestMgr zero-boundary crossing marked PLAYER_FLAGS_RESTING dirty.
    #[cfg(test)]
    represented_deferred_rest_flag_update_dirty_like_cpp: bool,
    /// C++ `RestMgr::_innAreaTriggerId`.
    #[cfg(test)]
    represented_inn_area_trigger_id_like_cpp: u32,
    /// C++ `RestMgr::_restTime`, represented as Unix seconds.
    #[cfg(test)]
    represented_rest_time_secs_like_cpp: u64,
    /// C++ `characters.playerFlags` as loaded by `Player::LoadFromDB`.
    #[cfg(test)]
    represented_loaded_player_flags_like_cpp: Option<u32>,
    /// C++ `characters.playerFlagsEx` as loaded by `Player::LoadFromDB`.
    #[cfg(test)]
    represented_loaded_player_flags_ex_like_cpp: Option<u32>,
    #[cfg(test)]
    represented_loaded_player_flags_applied_like_cpp: bool,
    /// C++ `RATE_REST_OFFLINE_IN_WILDERNESS`.
    #[cfg(test)]
    rest_offline_wilderness_rate_like_cpp: f32,
    /// C++ `RATE_REST_OFFLINE_IN_TAVERN_OR_CITY`.
    #[cfg(test)]
    rest_offline_tavern_or_city_rate_like_cpp: f32,
    /// C++ `RATE_REST_INGAME`.
    #[cfg(test)]
    rest_ingame_rate_like_cpp: f32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    /// Production money lives exclusively in `Player::ActivePlayerData::Coinage`.
    #[cfg(test)]
    player_gold: u64,
    /// Handle-less test fallback for C++ `Player::_specializationInfo.ResetTalentsCost`.
    #[cfg(test)]
    represented_talent_reset_cost_like_cpp: u32,
    /// Handle-less test fallback for C++ `Player::_specializationInfo.ResetTalentsTime`.
    #[cfg(test)]
    represented_talent_reset_time_secs_like_cpp: u64,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_bank_bag_slot_count_like_cpp: u8,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_inventory_slot_count_like_cpp: u8,
    /// C++ `UF::ActivePlayerData::CharacterPoints`, recalculated by InitTalentForLevel/LearnTalent.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_character_points_like_cpp: i32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    represented_player_powers_like_cpp: CharacterPowerSnapshotLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    represented_player_max_powers_like_cpp: CharacterPowerSnapshotLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    represented_player_base_mana_like_cpp: i32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    represented_bank_bag_slot_flags_like_cpp: [u32; 7],
    #[cfg(test)]
    represented_bank_item_moves_like_cpp: Vec<RepresentedBankItemMoveLikeCpp>,
    #[cfg(test)]
    represented_guild_bank_inventory_moves_like_cpp: Vec<RepresentedGuildBankInventoryMoveLikeCpp>,
    #[cfg(test)]
    represented_guild_bank_list_requests_like_cpp: Vec<RepresentedGuildBankListRequestLikeCpp>,
    #[cfg(test)]
    represented_guild_bank_money_moves_like_cpp: Vec<RepresentedGuildBankMoneyMoveLikeCpp>,
    #[cfg(test)]
    represented_guild_bank_tab_actions_like_cpp: Vec<RepresentedGuildBankTabActionLikeCpp>,
    #[cfg(test)]
    represented_auction_replicate_requests_like_cpp: Vec<RepresentedAuctionReplicateRequestLikeCpp>,
    #[cfg(test)]
    represented_auction_place_bids_like_cpp: Vec<RepresentedAuctionPlaceBidLikeCpp>,
    #[cfg(test)]
    represented_auction_remove_items_like_cpp: Vec<RepresentedAuctionRemoveItemLikeCpp>,
    #[cfg(test)]
    represented_auction_sell_items_like_cpp: Vec<RepresentedAuctionSellItemLikeCpp>,
    #[cfg(test)]
    represented_auto_unequip_offhand_requests_like_cpp: Vec<RepresentedAutoUnequipOffhandLikeCpp>,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_xp: u32,
    /// XP required to reach next level, cached from player_xp_for_level.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_next_level_xp: u32,
    /// Currently selected target GUID (SetSelection).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    selection_guid: Option<wow_core::ObjectGuid>,

    /// GUID of the character currently logged in (set after login completes).
    player_guid: Option<ObjectGuid>,
    /// C++ `WorldSession::m_GUIDLow`: last logged-in character low GUID kept after logout.
    recent_player_guid_low_like_cpp: u64,
    /// Test fixtures may attach a Player bootstrap before injecting the
    /// production MapManager. Production attachment is represented solely by
    /// the generation-checked PlayerHandle.
    #[cfg(test)]
    player_bootstrap_attached_like_cpp: bool,
    /// C++ `WorldSession::_accountData`, represented in-memory until DB load/save is wired.
    account_data_like_cpp: [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES],
    /// C++ `WorldSession::_tutorials`, account-scoped tutorial completion flags.
    tutorials_like_cpp: [u32; 8],
    tutorials_loaded_from_db_like_cpp: bool,
    tutorials_loaded_coherently_like_cpp: bool,
    tutorials_changed_like_cpp: bool,

    /// Pending creature spawn request (set during login, processed async).
    pub(crate) pending_creature_spawn: Option<PendingCreatureSpawn>,
    /// Creature kills observed from synchronous melee ticks and completed in `process_pending`.
    pending_creature_kill_loot_like_cpp: Vec<ObjectGuid>,
    pending_creature_kill_rewards_like_cpp: Vec<PendingCreatureKillRewardLikeCpp>,
    #[cfg(test)]
    represented_creature_kill_events_like_cpp: Vec<RepresentedCreatureKillEventLikeCpp>,

    /// In-memory inventory: slot → (item ObjectGuid, entry_id, db_guid).
    #[cfg(test)]
    inventory_items: HashMap<u8, InventoryItem>,

    /// In-memory buyback slots, kept separate from normal inventory like C++ `GetItemByGuid`.
    #[cfg(test)]
    buyback_items: HashMap<u8, InventoryItem>,
    #[cfg(test)]
    buyback_price: [u32; BUYBACK_SLOT_COUNT],
    #[cfg(test)]
    buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    #[cfg(test)]
    current_buyback_slot: u8,
    #[cfg(test)]
    represented_item_mod_reapply_events_like_cpp: Vec<RepresentedItemModsReapplyEventLikeCpp>,
    #[cfg(test)]
    represented_item_bonus_actions_like_cpp: Vec<RepresentedItemBonusActionLikeCpp>,
    #[cfg(test)]
    represented_item_bonus_state_like_cpp: RepresentedItemBonusStateLikeCpp,
    #[cfg(test)]
    represented_item_set_effects_like_cpp: HashMap<u32, RepresentedItemSetEffectLikeCpp>,
    #[cfg(test)]
    represented_item_set_spell_events_like_cpp: Vec<RepresentedItemSetSpellEventLikeCpp>,
    #[cfg(test)]
    represented_item_set_aura_refresh_events_like_cpp:
        Vec<RepresentedItemSetAuraRefreshEventLikeCpp>,
    #[cfg(test)]
    represented_combat_stat_recalculations_like_cpp: Vec<RepresentedCombatStatRecalculationLikeCpp>,
    #[cfg(test)]
    represented_titan_grip_penalty_actions_like_cpp: Vec<TitanGripPenaltyAction>,
    #[cfg(test)]
    represented_avg_equipped_item_level_updates_like_cpp: Vec<f32>,
    #[cfg(test)]
    represented_guild_id_like_cpp: u64,
    /// True once the active Player's character/guild membership source was
    /// loaded or explicitly replaced. A numeric zero alone is only the
    /// session default and cannot prove that Guild::SendLoginInfo is skipped.
    #[cfg(test)]
    represented_guild_id_authority_complete_like_cpp: bool,
    #[cfg(test)]
    represented_guild_id_invited_like_cpp: u64,
    #[cfg(test)]
    represented_guild_accept_invites_like_cpp: Vec<u64>,
    #[cfg(test)]
    represented_calendar_community_invites_like_cpp: Vec<RepresentedCalendarCommunityInviteLikeCpp>,
    #[cfg(test)]
    represented_calendar_add_events_like_cpp: Vec<RepresentedCalendarAddEventLikeCpp>,
    #[cfg(test)]
    represented_calendar_remove_events_like_cpp: Vec<RepresentedCalendarRemoveEventLikeCpp>,
    #[cfg(test)]
    represented_arena_team_id_invited_like_cpp: u32,
    #[cfg(test)]
    represented_wargame_invite_acceptances_like_cpp: Vec<RepresentedWargameInviteAcceptanceLikeCpp>,
    #[cfg(test)]
    represented_active_trade_partner_like_cpp: Option<ObjectGuid>,
    #[cfg(test)]
    represented_trade_accepted_like_cpp: bool,
    #[cfg(test)]
    represented_partner_trade_server_state_index_like_cpp: u32,
    #[cfg(test)]
    represented_trade_client_state_index_like_cpp: u32,
    #[cfg(test)]
    represented_trade_server_state_index_like_cpp: u32,
    #[cfg(test)]
    represented_trade_items_like_cpp: [Option<ObjectGuid>; TRADE_SLOT_COUNT_LIKE_CPP as usize],
    #[cfg(test)]
    represented_trade_money_like_cpp: u64,
    #[cfg(test)]
    represented_trade_spell_like_cpp: u32,
    #[cfg(test)]
    represented_trade_spell_cast_item_like_cpp: Option<ObjectGuid>,
    #[cfg(test)]
    represented_trade_cancel_statuses_like_cpp: Vec<u8>,
    #[cfg(test)]
    represented_sign_petitions_like_cpp: Vec<RepresentedSignPetitionLikeCpp>,
    #[cfg(test)]
    represented_decline_petitions_like_cpp: Vec<RepresentedDeclinePetitionLikeCpp>,
    #[cfg(test)]
    represented_query_petitions_like_cpp: Vec<RepresentedQueryPetitionLikeCpp>,
    #[cfg(test)]
    represented_silence_party_talker_like_cpp: Vec<RepresentedSilencePartyTalkerLikeCpp>,
    #[cfg(test)]
    represented_can_duel_spell_casts_like_cpp: Vec<RepresentedCanDuelSpellCastLikeCpp>,
    /// Handle-less compatibility for older tests. Production C++
    /// `PlayerData::DuelArbiter` lives on the canonical Player.
    #[cfg(test)]
    represented_duel_arbiter_guid_like_cpp: Option<ObjectGuid>,
    #[cfg(test)]
    represented_duel_requests_like_cpp: Vec<RepresentedDuelRequestedLikeCpp>,
    #[cfg(test)]
    represented_force_deselects_like_cpp: Vec<RepresentedForceDeselectLikeCpp>,
    #[cfg(test)]
    represented_duel_accepts_like_cpp: Vec<RepresentedDuelAcceptedLikeCpp>,
    #[cfg(test)]
    represented_duel_cancels_like_cpp: Vec<RepresentedDuelCancelledLikeCpp>,
    represented_guild_repair_bank_state_like_cpp: Option<RepresentedGuildRepairBankStateLikeCpp>,
    #[cfg(test)]
    represented_guild_repair_bank_withdraws_like_cpp:
        Vec<RepresentedGuildRepairBankWithdrawLikeCpp>,

    /// Legacy handle-less test fixture for C++ `Player::_currencyStorage`.
    #[cfg(test)]
    player_currencies: HashMap<u32, PlayerCurrency>,
    represented_quest_objective_progress_events_like_cpp:
        VecDeque<RepresentedQuestObjectiveProgressEventLikeCpp>,
    represented_quest_objective_progress_draining_like_cpp: bool,

    /// In-memory item objects keyed by item GUID, mirroring C++ `Player::m_items` ownership.
    #[cfg(test)]
    inventory_item_objects: HashMap<ObjectGuid, Item>,

    /// Current map ID for VALUES update packets.
    current_map_id: u16,

    /// Race of the currently logged-in character (set at login).
    player_race: u8,
    /// Class of the currently logged-in character (set at login).
    player_class: u8,
    /// Level of the currently logged-in character (set at login).
    player_level: u8,
    /// Gender of the currently logged-in character (set at login).
    player_gender: u8,
    /// C++ `Player::m_createMode`, loaded from `characters.createMode`.
    #[cfg(test)]
    player_create_mode_like_cpp: u8,
    /// Represented C++ `Player::GetShapeshiftForm()` until shapeshift aura state owns it.
    #[cfg(test)]
    represented_shapeshift_form_like_cpp: u32,
    /// C++ ActivePlayerData::LootSpecID represented session state.
    #[cfg(test)]
    loot_specialization_id: u32,
    /// Represented C++ ActivePlayerData::CurrentSpecID / GetPrimarySpecialization.
    #[cfg(test)]
    pub(crate) represented_primary_specialization_id_like_cpp: u32,
    /// All known spell IDs for the logged-in character (DB + DBC merged).
    #[cfg(test)]
    known_spells: Vec<i32>,
    /// Complete represented C++ `PlayerSpellMap`, retained independently from
    /// the `known_spells` `Player::HasSpell` mirror, including active/persistence flags.
    #[cfg(test)]
    represented_player_spell_rows_like_cpp: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    /// True when the backing spell queries or another explicit row source
    /// were retained losslessly.
    #[cfg(test)]
    represented_player_spell_rows_loaded_like_cpp: bool,
    /// True only when the retained rows represent the final post-`AddSpell`
    /// logical map, not merely the raw DB rows.
    #[cfg(test)]
    represented_player_spell_rows_complete_like_cpp: bool,
    /// Dirty `EffectLearnSpell` base rows retained when ancillary projection
    /// authority is incomplete. Unlike the complete map replacement, these
    /// rows can be persisted independently without claiming that untouched
    /// spell rows are authoritative.
    #[cfg(test)]
    represented_fallback_player_spell_rows_like_cpp: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    /// Represented C++ `PlayerSpell::dependent` for known spells that must not
    /// be persisted by `_SaveSpells`.
    #[cfg(test)]
    represented_dependent_known_spells_like_cpp: HashSet<i32>,
    /// Represented C++ `PLAYERSPELL_REMOVED` rows for known spells removed
    /// after a coherent character spell load.
    #[cfg(test)]
    represented_removed_known_spells_like_cpp: HashSet<i32>,
    /// Represented C++ `PlayerSpell::favorite` rows loaded from
    /// `character_spell_favorite`.
    #[cfg(test)]
    represented_favorite_known_spells_like_cpp: HashSet<i32>,
    /// Represented C++ `PlayerSpell::TraitDefinitionId`, keyed by learned spell id.
    #[cfg(test)]
    represented_spell_trait_definition_ids_like_cpp: HashMap<i32, i32>,
    /// True only when trait-definition IDs were replaced from a complete source.
    /// An empty represented map is otherwise indistinguishable from an unloaded one.
    #[cfg(test)]
    represented_spell_trait_definition_ids_complete_like_cpp: bool,
    /// Exact persisted C++ `TraitConfig` headers, keyed by config ID. The
    /// narrow aura proof needs type/spec/flags independently from the derived
    /// trait-spell map because an empty entry query can still make C++ create
    /// or replace a config with condition-granted entries.
    #[cfg(test)]
    represented_trait_config_rows_like_cpp: BTreeMap<i32, wow_entities::PlayerTraitConfigState>,
    #[cfg(test)]
    represented_trait_config_rows_complete_like_cpp: bool,
    #[cfg(test)]
    represented_trait_entry_rows_complete_like_cpp: bool,
    #[cfg(test)]
    represented_trait_entry_rows_empty_like_cpp: bool,
    /// Test-only causal trace. Production applies every represented
    /// post-commit action immediately; retaining a second action history on
    /// the Session would be audit state, not C++ runtime authority.
    #[cfg(test)]
    represented_spell_acquisition_post_commit_actions_like_cpp:
        Vec<crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp>,
    /// C++ `Player::m_weaponProficiency`; `Spell::EffectProficiency` ORs into it.
    #[cfg(test)]
    represented_weapon_proficiency_like_cpp: u32,
    /// C++ `Player::m_armorProficiency`; `Spell::EffectProficiency` ORs into it.
    #[cfg(test)]
    represented_armor_proficiency_like_cpp: u32,
    /// C++ `CollectionMgr::_mounts` represented account mount collection.
    #[cfg(test)]
    account_mounts_like_cpp: HashMap<i32, u8>,
    /// Login snapshot of the player's spell history + charge packets. C++ reads these
    /// live from `Player::GetSpellHistory()` in `SendInitialPacketsBeforeAddToMap`; Rust
    /// persists the login snapshot so the before-add helper can re-send it on far teleport
    /// without a DB round trip. #NEXT.R8.ENTITIES.1229.
    #[cfg(test)]
    represented_spell_history_packets_like_cpp: (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>),
    /// Handle-less unit-test fallback; production C++ `Player::_CUFProfiles` lives on canonical
    /// `wow_entities::Player`.
    #[cfg(test)]
    cuf_profiles_like_cpp: Vec<Option<wow_packet::packets::misc::CufProfile>>,
    #[cfg(test)]
    cuf_profiles_loaded_like_cpp: bool,

    // ── Dual-connection (realm + instance) ───────────────────────
    // After ConnectTo completes, the session uses the instance socket for
    // game packets but MUST keep the realm socket alive — the WoW client
    // disconnects if either connection drops.

    // ── Movement & World position ─────────────────────────────────
    /// Server-side position of the player (updated from CMSG_MOVE_*).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_position: Option<wow_core::Position>,
    /// Last accepted player movement flags, mirroring C++ `Unit::m_movementInfo`.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_movement_flags_like_cpp: MovementFlag,
    /// Represented C++ `MOVEMENTFLAG2_CAN_SWIM_TO_FLY_TRANS` server-controlled state.
    #[cfg(test)]
    represented_can_swim_to_fly_transition_like_cpp: bool,
    /// Represented `m_unitMovedByMe->GetVehicle()->GetVehicleInfo()->Flags & VEHICLE_FLAG_FIXED_POSITION`.
    #[cfg(test)]
    represented_mover_fixed_position_vehicle_like_cpp: bool,

    /// Cached character name for chat messages.
    player_name: Option<String>,

    // Addon chat filtering state. Mirrors C++ WorldSession::_registeredAddonPrefixes
    // and _filterAddonMessages.
    pub(crate) registered_addon_prefixes: Vec<String>,
    pub(crate) filter_addon_messages: bool,

    // ── Creature AI tracking ──────────────────────────────────────
    /// Tick counter for creature movement (throttle to every N ticks).
    pub(crate) creature_tick: u32,
    /// Per-session finite vendor stock state, mirroring Creature::m_vendorItemCounts
    /// until vendor ownership moves into the shared creature model.
    pub(crate) vendor_item_counts: HashMap<(wow_core::ObjectGuid, u32), VendorItemCount>,
    /// Test-only replacement for one resolved `VendorItem` row. Production
    /// always resolves the row through CharacterHandler's WorldDB query.
    #[cfg(test)]
    vendor_buy_item_test_override_like_cpp: Option<VendorBuyItemTestOverrideLikeCpp>,

    /// Shared, server-wide map state. When `Some`, creature reads/writes can
    /// route through here so all sessions on the same map see the same world.
    /// `None` until the world server injects the manager (see `set_map_manager`).
    pub(crate) map_manager: Option<crate::map_manager::SharedMapManager>,
    /// Canonical C++-style `wow-map` manager. This is injected separately from
    /// the legacy `wow-world` manager while handlers migrate to `wow-map`.
    pub(crate) canonical_map_manager: Option<SharedCanonicalMapManager>,
    /// Generation-checked identity of the one canonical Player value owned by
    /// MapManager. It remains resolvable while detached for a far teleport.
    player_handle_like_cpp: Option<wow_map::PlayerHandle>,
    /// Dedicated Detour owner handle. The underlying `MMapManager` remains on
    /// its worker thread because Detour state is not `Send + Sync`.
    mmap_pathfinder_like_cpp: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    /// Shared C++ `InstanceLockMgr` analogue used by raid-info and instance entry paths.
    pub(crate) instance_lock_mgr: Option<Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>>,

    // ── Combat state ─────────────────────────────────────────────
    /// Current auto-attack target (None if not in combat).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(crate) combat_target: Option<wow_core::ObjectGuid>,
    /// Last represented player melee tick used to decrement C++ `m_attackTimer`.
    combat_tick_last_at_like_cpp: Instant,
    /// Represented result of C++ `IsWithinLOSInMap(victim)` for melee swings until LOS runtime is canonical.
    /// C++ `Player::m_swingErrorMsg`; suppresses duplicate `SMSG_ATTACK_SWING_ERROR` packets.
    player_swing_error_msg_like_cpp: Option<u8>,

    /// True when the player is engaged in combat.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(crate) in_combat: bool,
    /// Test-only legacy fixture for sessions without an installed Player owner.
    #[cfg(test)]
    player_alive_like_cpp: bool,
    /// Handle-less fixture for C++ `Player::IsGameMaster()`.
    #[cfg(test)]
    player_game_master_like_cpp: bool,
    /// Represented `CHEAT_GOD` movement/fall guard.
    #[cfg(test)]
    player_cheat_god_like_cpp: bool,
    /// Represented `IsImmunedToDamage(SPELL_SCHOOL_MASK_NORMAL)` fall guard.
    #[cfg(test)]
    player_normal_damage_immune_like_cpp: bool,
    /// Represented `IsImmuneToEnvironmentalDamage()` guard inside EnvironmentalDamage.
    #[cfg(test)]
    player_environmental_damage_immune_like_cpp: bool,
    /// Test-only legacy health fixture for sessions without a Player handle.
    #[cfg(test)]
    player_health_like_cpp: u32,
    /// Test-only legacy max-health fixture for sessions without a Player handle.
    #[cfg(test)]
    player_max_health_like_cpp: u32,
    /// High-water mark for map-owned creature-melee presentation commands.
    /// Canonical health/death authority lives on `wow-map`; this suppresses
    /// durable FIFO replay without writing delayed values back to that owner.
    last_presented_creature_melee_health_state_revision_like_cpp: u64,
    /// Represented `Unit::m_movementInfo.time` for client movement ACK side effects.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_movement_time_like_cpp: u32,
    /// Represented `Unit::m_movementInfo.jump`, reset by `Player::TeleportTo`.
    /// Test-only evidence: production consumes the typed movement status and
    /// does not retain a second packet-shaped jump mirror.
    #[cfg(test)]
    player_movement_jump_like_cpp: wow_packet::packets::movement::JumpInfo,
    /// C++ `Player::m_lastFallTime`.
    #[cfg(test)]
    last_fall_time_like_cpp: u32,
    /// C++ `Player::m_lastFallZ`.
    #[cfg(test)]
    last_fall_z_like_cpp: f32,
    /// Recorded fall damage events until combat log/update packet runtime is complete.
    #[cfg(test)]
    fall_damage_events_like_cpp: Vec<MovementFallDamageEvent>,
    /// C++ `PLAYER_FLAGS_IS_OUT_OF_BOUNDS` represented state.
    #[cfg(test)]
    player_out_of_bounds_like_cpp: bool,
    /// Recorded `DAMAGE_FALL_TO_VOID` events until environmental damage packets are complete.
    #[cfg(test)]
    under_map_damage_events_like_cpp: Vec<MovementUnderMapDamageEvent>,
    /// Represented stand state used by movement side effects until UnitData owns it.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_stand_state_like_cpp: UnitStandStateType,
    /// Test-only successful represented->live evidence. Production emits
    /// bounded structured telemetry instead of retaining client-controlled
    /// history for the lifetime of the session.
    #[cfg(test)]
    represented_live_applications_like_cpp: Vec<RepresentedLiveApplicationLikeCpp>,
    /// Represented `UnitData::EmoteState`, used to clear stateful emotes on movement like C++.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_emote_state_like_cpp: u32,
    /// Count of C++ temporary pet unsummon side effects requested by movement.
    #[cfg(test)]
    temporary_pet_unsummon_requests_like_cpp: u32,
    /// Count of C++ jump proc side effects requested by movement.
    #[cfg(test)]
    movement_jump_proc_requests_like_cpp: u32,
    /// Represented `ActivePlayerData::LocalFlags`.
    #[cfg(test)]
    active_player_local_flags_like_cpp: u32,
    /// Represented `ActivePlayerData::TransportServerTime`.
    #[cfg(test)]
    active_player_transport_server_time_like_cpp: i32,
    /// Represented `ActivePlayerData::MultiActionBars`.
    #[cfg(test)]
    active_player_multi_action_bars_like_cpp: u8,
    /// Test-only action-button owner for fixtures without a canonical Player.
    #[cfg(test)]
    represented_action_buttons_like_cpp: [u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS],
    #[cfg(test)]
    represented_action_buttons_loaded_like_cpp: bool,
    /// C++ `Player::_advancedCombatLoggingEnabled`; consumed when combat-log fanout selects full/basic payloads.
    /// C++ `WorldSession::_filterAddonMessages`' sibling for
    /// `SMSG_SPELL_GO`: shared so a producer can commit the combat-log packet
    /// variant per recipient while distributing a cast, the way C++ selects it
    /// synchronously inside `WorldObject::SendCombatLogMessage`.
    advanced_combat_logging_enabled_like_cpp: Arc<AtomicBool>,
    /// C++ `Player::GetUnitBeingMoved()` represented GUID.
    #[cfg(test)]
    player_moved_unit_guid_like_cpp: ObjectGuid,
    /// Count of visibility refreshes requested by movement initialization.
    movement_visibility_refresh_requests_like_cpp: u32,
    /// ACKs accepted by represented movement handling until full Unit movement runtime/broadcasts exist.
    #[cfg(test)]
    movement_ack_events_like_cpp: Vec<MovementAckEventLikeCpp>,
    /// Represented `PlayerTaxi::m_TaxiDestinations` until PlayerTaxi/MotionMaster runtime is canonical.
    #[cfg(test)]
    taxi_destinations_like_cpp: Vec<u32>,
    /// Represented accepted `CMSG_ACTIVATE_TAXI` requests until TaxiPathGraph/MotionMaster are canonical.
    #[cfg(test)]
    represented_activate_taxi_requests_like_cpp: Vec<RepresentedActivateTaxiLikeCpp>,
    /// Represented accepted barber-shop requests until ChrCustomization DB2/cost/update runtime is canonical.
    #[cfg(test)]
    represented_alter_appearance_requests_like_cpp: Vec<RepresentedAlterAppearanceLikeCpp>,
    /// Represented accepted barber confirmation requests until Player::SetCustomizations is canonical.
    #[cfg(test)]
    represented_confirm_barbers_choice_requests_like_cpp:
        Vec<RepresentedConfirmBarbersChoiceLikeCpp>,
    /// Represented accepted talent-respec wipe requests until Player::ResetTalents is canonical.
    #[cfg(test)]
    represented_confirm_respec_wipe_requests_like_cpp: Vec<RepresentedConfirmRespecWipeLikeCpp>,
    /// C++ `Player::m_atLoginFlags`, represented for reset-on-login side effects.
    #[cfg(test)]
    represented_at_login_flags_like_cpp: u16,
    /// Represented `sScriptMgr->OnPlayerTalentsReset` calls until ScriptMgr is live.
    #[cfg(test)]
    represented_talent_reset_script_hooks_like_cpp: Vec<RepresentedTalentResetScriptHookLikeCpp>,
    /// Represented persistent `RemoveAtLoginFlag` calls until direct character DB execution is live.
    #[cfg(test)]
    represented_at_login_flag_removals_like_cpp: Vec<RepresentedAtLoginFlagRemovalLikeCpp>,
    /// Represented `unit->CastSpell(_player, 14867, true)` after successful talent reset.
    #[cfg(test)]
    represented_talent_respec_visual_spell_casts_like_cpp:
        Vec<RepresentedTalentRespecVisualSpellCastLikeCpp>,
    /// Represented `CriteriaType::MoneySpentOnRespecs` / `TotalRespecs` events.
    #[cfg(test)]
    represented_talent_respec_criteria_events_like_cpp:
        Vec<RepresentedTalentRespecCriteriaEventLikeCpp>,
    /// Handle-less test fallback; production C++ `Player::_equipmentSets` lives on canonical Player.
    #[cfg(test)]
    represented_equipment_sets_like_cpp: BTreeMap<u64, RepresentedEquipmentSetLikeCpp>,
    #[cfg(test)]
    represented_equipment_sets_loaded_like_cpp: bool,
    /// Handle-less test fallback; production C++ `Player::_voidStorageItems` lives on canonical Player.
    #[cfg(test)]
    represented_void_storage_items_like_cpp: [Option<RepresentedVoidStorageItemLikeCpp>;
        wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP],
    #[cfg(test)]
    represented_void_storage_loaded_like_cpp: bool,
    /// Represented accepted Adventure Map quest starts until AddQuestAndCheckCompletion is canonical.
    #[cfg(test)]
    represented_adventure_map_start_quest_requests_like_cpp:
        Vec<RepresentedAdventureMapStartQuestLikeCpp>,
    /// Minimal TaxiNodes.db2 map lookup used by represented `MoveSplineDone` taxi transitions.
    taxi_node_map_ids_like_cpp: HashMap<u32, u16>,
    /// Represented active `FlightPathMovementGenerator`, if any.
    #[cfg(test)]
    taxi_flight_state_like_cpp: Option<RepresentedTaxiFlightStateLikeCpp>,
    /// Represented unit flags touched by `CleanupAfterTaxiFlight`.
    #[cfg(test)]
    taxi_unit_flags_like_cpp: UnitFlags,
    /// Represented mount state touched by `CleanupAfterTaxiFlight`.
    #[cfg(test)]
    taxi_mounted_like_cpp: bool,
    /// Handle-less fixture for C++ `Unit::GetMountDisplayId()`.
    #[cfg(test)]
    player_mount_display_id_like_cpp: i32,
    /// Represented vehicle id selected from mount creature template until VehicleKit exists.
    #[cfg(test)]
    player_mount_vehicle_id_like_cpp: u32,
    /// Legacy handle-less test fixture for C++ `Unit::m_vehicleKit`.
    #[cfg(test)]
    pub(crate) player_mount_vehicle_kit_like_cpp: Option<Vehicle>,
    /// Vehicle accessory rows selected by C++ `Vehicle::InstallAllAccessories(false)`.
    #[cfg(test)]
    player_mount_vehicle_accessories_like_cpp: Vec<VehicleAccessory>,
    /// Represented number of VehicleSeat rows installed by C++ `Vehicle` constructor.
    #[cfg(test)]
    player_mount_vehicle_seat_count_like_cpp: u8,
    /// Represented C++ `Vehicle::UsableSeatNum`.
    #[cfg(test)]
    player_mount_vehicle_usable_seat_count_like_cpp: u8,
    /// Legacy handle-less test fixture for current `VehicleSeatEntry::Flags`.
    #[cfg(test)]
    pub(crate) player_vehicle_seat_flags_like_cpp: Option<i32>,
    /// Legacy handle-less test fixture for current `VehicleSeatEntry::ID`.
    #[cfg(test)]
    pub(crate) player_vehicle_seat_id_like_cpp: Option<u32>,
    /// Represented `Player::ChangeSeat(seatId, next)` requests until live vehicle ownership exists.
    #[cfg(test)]
    represented_vehicle_seat_change_requests_like_cpp:
        Vec<RepresentedVehicleSeatChangeRequestLikeCpp>,
    /// Represented cross-vehicle `HandleSpellClick(player, seatId)` requests from vehicle switching.
    #[cfg(test)]
    represented_vehicle_seat_spell_click_requests_like_cpp:
        Vec<RepresentedVehicleSeatSpellClickRequestLikeCpp>,
    /// Represented `Player::EnterVehicle(targetPlayer)` requests from `CMSG_RIDE_VEHICLE_INTERACT`.
    #[cfg(test)]
    represented_vehicle_enter_requests_like_cpp: Vec<RepresentedVehicleEnterRequestLikeCpp>,
    /// Represented `m_movementInfo = MoveDismissVehicle.Status` before live `ExitVehicle`.
    #[cfg(test)]
    represented_vehicle_dismiss_movements_like_cpp: Vec<RepresentedVehicleDismissMovementLikeCpp>,
    /// Represented `vehicle_base->m_movementInfo = MoveChangeVehicleSeats.Status`.
    #[cfg(test)]
    represented_vehicle_base_movements_like_cpp: Vec<RepresentedVehicleBaseMovementLikeCpp>,
    /// Represented `Player::GetBattleground()->GetTypeID()` for C++ battleground object use.
    #[cfg(test)]
    player_battleground_type_id_like_cpp: Option<u32>,
    /// Represented `Player::GetBattleground()->GetMapId()` until live Battleground ownership exists.
    #[cfg(test)]
    player_battleground_map_id_like_cpp: Option<u32>,
    /// Represented `Battleground::GetStatus()` until live Battleground ownership exists.
    #[cfg(test)]
    represented_battleground_status_like_cpp: Option<u8>,
    /// Count of represented `Player::LeaveBattleground()` requests.
    #[cfg(test)]
    represented_battleground_leave_requests_like_cpp: u32,
    /// Represented `BattlegroundMgr::SendBattlegroundList` intents from battlemaster hello.
    #[cfg(test)]
    represented_battlemaster_hellos_like_cpp: Vec<RepresentedBattlemasterHelloLikeCpp>,
    /// Represented `BattlegroundMgr::SendBattlegroundList` intents from CMSG_BATTLEFIELD_LIST.
    #[cfg(test)]
    represented_battlefield_lists_like_cpp: Vec<RepresentedBattlefieldListLikeCpp>,
    /// Represented `BattlegroundQueue::AddGroup` intents from CMSG_BATTLEMASTER_JOIN.
    #[cfg(test)]
    represented_battlemaster_joins_like_cpp: Vec<RepresentedBattlemasterJoinLikeCpp>,
    /// Represented rated arena queue intents from CMSG_BATTLEMASTER_JOIN_ARENA.
    #[cfg(test)]
    represented_battlemaster_join_arenas_like_cpp: Vec<RepresentedBattlemasterJoinArenaLikeCpp>,
    /// Represented arena skirmish queue intents from CMSG_BATTLEMASTER_JOIN_SKIRMISH.
    #[cfg(test)]
    represented_battlemaster_join_skirmishes_like_cpp:
        Vec<RepresentedBattlemasterJoinSkirmishLikeCpp>,
    /// Represented `Player::m_bgBattlegroundQueueID[PLAYER_MAX_BATTLEGROUND_QUEUES]`.
    #[cfg(test)]
    represented_battleground_queue_slots_like_cpp: Vec<RepresentedBattlegroundQueueSlotLikeCpp>,
    /// Represented accepted/leave requests from CMSG_BATTLEFIELD_PORT before live BattlegroundMgr.
    #[cfg(test)]
    represented_battlefield_ports_like_cpp: Vec<RepresentedBattlefieldPortLikeCpp>,
    /// C++ `Player::_areaSpiritHealerGUID`, represented until battleground/player resurrection owns it.
    #[cfg(test)]
    area_spirit_healer_guid_like_cpp: ObjectGuid,
    /// Legacy handle-less test fixture for the current Player-owned pet GUID.
    #[cfg(test)]
    pub(crate) represented_pet_guid_like_cpp: Option<ObjectGuid>,
    /// C++ `Player::m_temporaryUnsummonedPetNumber`, represented until pet DB load/resummon is live.
    #[cfg(test)]
    represented_temporary_unsummoned_pet_number_like_cpp: u32,
    /// C++ `Player::m_oldpetspell`, used by `RemovePet(nullptr, ..., returnreagent=true)`.
    #[cfg(test)]
    represented_old_pet_spell_like_cpp: u32,
    /// Represented `character_pet`/stable rows until `Pet::LoadPetFromDB` is wired to DB.
    #[cfg(test)]
    represented_pet_stable_like_cpp: PetStable,
    /// True only after the current Player's complete `character_pet` query
    /// returned no rows. Any later pet load or lifetime mutation revokes this
    /// narrow proof instead of attempting to model pet-to-owner aura casts.
    #[cfg(test)]
    represented_character_pet_rows_empty_authority_complete_like_cpp: bool,
    /// Per-character asynchronous C++ `PetLoadQueryHolder` result lifetime.
    pet_load_query_holder_rows_like_cpp: lifecycle::PetLoadQueryHolderRowsLikeCpp,
    /// Represented `Pet::m_unitData->CreatedBySpell` for the active pet until UnitData owns it.
    #[cfg(test)]
    represented_pet_created_by_spell_like_cpp: u32,
    /// Represented current pet react state for C++ mount/dismount PetMode side effects.
    #[cfg(test)]
    represented_pet_react_state_like_cpp: u8,
    /// Represented current pet command state for C++ mount/dismount PetMode side effects.
    #[cfg(test)]
    represented_pet_command_state_like_cpp: u8,
    /// C++ `Player::m_temporaryPetReactState` saved by `DisablePetControlsOnMount`.
    #[cfg(test)]
    temporary_mount_pet_react_state_like_cpp: Option<u8>,
    /// Count of C++ `CreateVehicleKit` mount side effects represented until Vehicle runtime sends packets.
    #[cfg(test)]
    mount_vehicle_create_requests_like_cpp: u32,
    /// Count of C++ `RemoveVehicleKit` mount side effects represented until Vehicle runtime sends packets.
    #[cfg(test)]
    mount_vehicle_remove_requests_like_cpp: u32,
    /// Count of C++ `SendOnCancelExpectedVehicleRideAura` packets emitted after vehicle-kit creation.
    #[cfg(test)]
    mount_cancel_expected_vehicle_aura_packets_like_cpp: u32,
    /// Count of C++ `DisablePetControlsOnMount` side effects represented until pet runtime is canonical.
    #[cfg(test)]
    mount_pet_control_disable_requests_like_cpp: u32,
    /// Count of C++ `EnablePetControlsOnDismount` side effects represented until pet runtime is canonical.
    #[cfg(test)]
    mount_pet_control_enable_requests_like_cpp: u32,
    /// Count of C++ mount/dismount pet resummon calls represented until pet runtime is canonical.
    #[cfg(test)]
    mount_pet_resummon_requests_like_cpp: u32,
    /// Count of C++ mount collision-height updates represented until movement packets are canonical.
    #[cfg(test)]
    mount_collision_height_update_requests_like_cpp: u32,
    /// C++ `Unit::m_movementCounter`: one per-player counter shared by ALL movement-control
    /// packets (vehicle-rec, collision height, near-teleport, speed/flag changes) and read
    /// for `SMSG_RESUME_TOKEN` SequenceIndex on far teleport. Reset to 0 in
    /// `send_initial_packets_before_add_to_map` (non-seamless). #NEXT.R8.ENTITIES.1229.
    #[cfg(test)]
    movement_counter_like_cpp: u32,
    /// Represented `Unit::GetCollisionHeight()` until model-display collision data owns it.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_collision_height_like_cpp: f32,
    /// Handle-less fixture for C++ `Object::GetObjectScale()`.
    #[cfg(test)]
    player_object_scale_like_cpp: f32,
    /// Represented `UnitData::ScaleDuration` for movement collision-height packets.
    #[cfg(test)]
    player_scale_duration_like_cpp: i32,
    /// Handle-less fixture for C++ `UnitData::Flags`.
    #[cfg(test)]
    player_unit_flags_like_cpp: UnitFlags,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    player_faction_template_like_cpp: Option<u32>,
    /// Handle-less fixture for C++ `UNIT_FLAG_MOUNT`.
    #[cfg(test)]
    player_mounted_like_cpp: bool,
    /// Represented `pvpInfo.IsHostile` branch for Honorless Target after taxi landing.
    #[cfg(test)]
    player_pvp_hostile_like_cpp: bool,
    /// Represented `Player::IsPvP()` branch for friendly-area near teleport handling.
    #[cfg(test)]
    player_pvp_enabled_like_cpp: bool,
    /// Represented `PLAYER_FLAGS_IN_PVP` branch for friendly-area near teleport handling.
    #[cfg(test)]
    player_in_pvp_flag_like_cpp: bool,
    /// Represented `pvpInfo.EndTimer` consumed by `Player::UpdatePvPFlag`.
    #[cfg(test)]
    player_pvp_end_timer_like_cpp: Option<i64>,
    /// C++ `Player::m_contestedPvPTimer`, reset by `Player::ResetContestedPvP`.
    #[cfg(test)]
    player_contested_pvp_timer_like_cpp: u32,
    /// Current represented zone/area ids until Map/Terrain runtime can calculate them.
    #[cfg(test)]
    player_zone_id_like_cpp: u32,
    #[cfg(test)]
    player_area_id_like_cpp: u32,
    /// True only when the current zone/area came from an extracted C++ terrain
    /// tile, rather than the DB-seeded or map-wide fallback.
    #[cfg(test)]
    player_zone_area_authority_complete_like_cpp: bool,
    /// Permanent fail-closed marker for this C++ Player lifetime after a login
    /// cast closure that Rust did not retain losslessly (currently FIRST).
    #[cfg(test)]
    player_spell_hit_aura_authority_tombstoned_like_cpp: bool,
    /// `MoveSplineDone` taxi decisions recorded until full Taxi/MotionMaster runtime exists.
    #[cfg(test)]
    move_spline_done_taxi_events_like_cpp: Vec<MoveSplineDoneTaxiEventLikeCpp>,
    /// C++ `Player::m_bCanDelayTeleport`, represented around update-owned work.
    #[cfg(test)]
    represented_can_delay_teleport_like_cpp: bool,
    /// C++ `Player::m_bHasDelayedTeleport`, represented for same-map near teleports.
    #[cfg(test)]
    represented_has_delayed_teleport_like_cpp: bool,
    /// C++ `Player::mSemaphoreTeleport_Near` represented state.
    #[cfg(test)]
    near_teleport_pending_like_cpp: bool,
    /// C++ `Player::mSemaphoreTeleport_Far` represented state.
    #[cfg(test)]
    represented_far_teleport_pending_like_cpp: bool,
    /// C++ `Player::m_teleport_dest` represented state for near teleports.
    #[cfg(test)]
    near_teleport_destination_like_cpp: Option<(u16, wow_core::Position)>,
    /// Saved `TeleportTo` arguments while `m_bHasDelayedTeleport` is set.
    #[cfg(test)]
    represented_delayed_teleport_like_cpp:
        Option<(u32, wow_core::Position, TeleportToOptionsLikeCpp)>,
    /// Represented zone/area for the pending near-teleport destination.
    #[cfg(test)]
    near_teleport_destination_zone_area_like_cpp: Option<(u32, u32)>,
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::m_homebind` lives on the canonical Player.
    #[cfg(test)]
    represented_homebind_like_cpp: Option<RepresentedHomebindLikeCpp>,
    /// C++ `Player::_resurrectionData`, represented until real Player/Spell
    /// resurrection request ownership exists.
    #[cfg(test)]
    represented_resurrection_request_like_cpp: Option<PlayerResurrectionRequestLikeCpp>,
    /// C++ `DELAYED_RESURRECT_PLAYER`, represented for resurrection requests
    /// that initiate teleport and must apply after WorldPortResponse.
    #[cfg(test)]
    represented_delayed_resurrection_after_teleport_like_cpp:
        Option<PlayerResurrectionRequestLikeCpp>,
    /// C++ `ActivePlayerData::SelfResSpells`, represented until update-field
    /// ownership is canonical.
    #[cfg(test)]
    represented_self_res_spells_like_cpp: BTreeSet<i32>,
    /// C++ `Player::m_overrideSpells`, represented until active player spell
    /// cast resolution owns override lookup.
    #[cfg(test)]
    represented_override_spells_like_cpp: HashMap<i32, BTreeSet<i32>>,
    /// True only when all C++ `Player::m_overrideSpells` edges were replaced
    /// from a complete source rather than accumulated opportunistically.
    #[cfg(test)]
    represented_override_spells_complete_like_cpp: bool,
    /// C++ `CONFIG_CAST_UNSTUCK` represented until World config is injected into spell effects.
    represented_cast_unstuck_enabled_like_cpp: bool,
    /// C++ `Player::GetDeathTimer()` represented for `Spell::EffectStuck`.
    #[cfg(test)]
    represented_death_timer_active_like_cpp: bool,
    /// Near teleport ACK side-effect audit events.
    #[cfg(test)]
    move_teleport_ack_events_like_cpp: Vec<MoveTeleportAckEventLikeCpp>,
    /// Count of C++ `ResummonPetTemporaryUnSummonedIfAny` calls after near teleport ACK.
    #[cfg(test)]
    temporary_pet_resummon_requests_like_cpp: u32,
    /// Count of C++ `ProcessDelayedOperations` calls after successful near teleport ACK.
    #[cfg(test)]
    delayed_operations_processed_like_cpp: u32,
    /// C++ `Player::m_forced_speed_changes[MAX_MOVE_TYPE]` represented state.
    #[cfg(test)]
    forced_speed_changes_like_cpp: [u8; UnitMoveTypeLikeCpp::COUNT],
    /// C++ `Unit::m_speed_rate[MAX_MOVE_TYPE]` represented state for player-controlled movers.
    #[cfg(test)]
    movement_speed_rates_like_cpp: [f32; UnitMoveTypeLikeCpp::COUNT],
    /// C++ `Player::GetPet()->SetSpeedRate` propagation represented until pet Unit runtime owns it.
    #[cfg(test)]
    represented_pet_movement_speed_rates_like_cpp: [f32; UnitMoveTypeLikeCpp::COUNT],
    /// Count of represented player speed changes propagated to the active pet.
    #[cfg(test)]
    represented_pet_speed_propagations_like_cpp: u32,
    /// Represented transport guard for speed ACK anticheat; C++ skips speed mismatch while on transport.
    #[cfg(test)]
    player_on_transport_like_cpp: bool,
    /// C++ `Player::m_movementForceModMagnitudeChanges` represented state.
    #[cfg(test)]
    movement_force_mod_magnitude_changes_like_cpp: u8,
    /// C++ `MovementForces::GetModMagnitude()` represented value; default is 1.0 when no force container exists.
    #[cfg(test)]
    movement_force_mod_magnitude_like_cpp: f32,
    /// Speed ACK outcomes recorded until full Unit speed runtime owns this state.
    #[cfg(test)]
    movement_speed_ack_events_like_cpp: Vec<MovementSpeedAckEventLikeCpp>,

    // ── Aura system ───────────────────────────────────────────────
    /// Legacy fixture mirror for tests that construct a Session without a
    /// canonical Player owner.
    #[cfg(test)]
    pub(crate) visible_auras: HashMap<u8, AuraApplication>,
    /// True only after both persisted aura tables were read successfully for
    /// the active character. Absence is evidence only while this is complete.
    #[cfg(test)]
    player_aura_authority_complete_like_cpp: bool,
    /// True only when `SEL_CHAR_EQUIPMENT` succeeded and proved that the active
    /// character has no top-level persisted item rows. Empty runtime inventory
    /// alone is not source proof because malformed rows may be rejected.
    #[cfg(test)]
    player_equipment_inventory_authority_complete_like_cpp: bool,
    /// Difficulty-selected C++ `AuraEffect` identity captured when the aura is applied.
    #[cfg(test)]
    canonical_threat_aura_snapshots_like_cpp: HashMap<u8, CanonicalThreatAuraSnapshotLikeCpp>,

    pub(crate) spell_acquisition_cast_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp>>,
    pub(crate) spell_acquisition_craft_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp>>,
    /// Effective C++ spell-script hooks. These remain optional so a session
    /// constructed without the startup audit fails closed.
    spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    spell_script_all_rank_root_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    spell_linked_rejected_trigger_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    talent_store: Option<Arc<TalentStore>>,
    num_talents_at_level_store: Option<Arc<NumTalentsAtLevelStore>>,
    #[cfg(test)]
    power_type_store: Option<Arc<PowerTypeStore>>,
    cinematic_sequences_store: Option<Arc<CinematicSequencesStore>>,
    movie_store: Option<Arc<MovieStore>>,
    #[cfg(test)]
    represented_cinematic_like_cpp: Option<u32>,
    #[cfg(test)]
    represented_cinematic_camera_ids_like_cpp: Option<[u16; 8]>,
    #[cfg(test)]
    represented_cinematic_camera_index_like_cpp: i32,
    #[cfg(test)]
    represented_cinematic_next_camera_events_like_cpp: Vec<u16>,
    #[cfg(test)]
    represented_cinematic_end_events_like_cpp: Vec<u32>,
    #[cfg(test)]
    represented_movie_like_cpp: Option<u32>,
    #[cfg(test)]
    represented_movie_complete_events_like_cpp: Vec<u32>,
    #[cfg(test)]
    represented_support_enabled_like_cpp: bool,
    #[cfg(test)]
    represented_support_tickets_enabled_like_cpp: bool,
    #[cfg(test)]
    represented_support_bugs_enabled_like_cpp: bool,
    #[cfg(test)]
    represented_support_complaints_enabled_like_cpp: bool,
    #[cfg(test)]
    represented_support_suggestions_enabled_like_cpp: bool,
    script_name_interner: Option<Arc<ScriptNameInternerLikeCpp>>,
    #[cfg(test)]
    object_mgr_catalogs_like_cpp: Option<Arc<ObjectMgrCatalogsLikeCpp>>,
    gameobject_template_lifecycle_store_like_cpp:
        Option<Arc<GameObjectTemplateLifecycleStoreLikeCpp>>,
    /// Currently active spell cast (if any). Set when a cast starts, cleared when it completes.
    #[cfg(test)]
    pub(crate) active_spell_cast: Option<SpellCastState>,
    /// C++ `Player::_pendingSpellCastRequest`, represented separately from
    /// `active_spell_cast` so cancel queued spell does not interrupt a cast
    /// already in progress.
    #[cfg(test)]
    pub(crate) represented_pending_spell_cast_request_like_cpp:
        Option<RepresentedPendingSpellCastRequestLikeCpp>,
    /// Last time a spell was executed (used to enforce global cooldown timers).
    #[cfg(test)]
    pub(crate) last_spell_cast_time: Option<Instant>,
    /// Per-spell cooldown tracking: spell_id → last cast time.
    /// Used to enforce spell-specific cooldown timers.
    #[cfg(test)]
    pub(crate) last_spell_cast_time_per_spell: HashMap<i32, Instant>,
    #[cfg(test)]
    represented_character_spell_cooldowns_like_cpp:
        HashMap<u32, RepresentedCharacterSpellCooldownLikeCpp>,
    #[cfg(test)]
    represented_character_spell_cooldowns_loaded_like_cpp: bool,
    #[cfg(test)]
    represented_character_spell_charges_like_cpp:
        BTreeMap<u32, Vec<RepresentedCharacterSpellChargeLikeCpp>>,
    #[cfg(test)]
    represented_character_spell_charges_loaded_like_cpp: bool,
    #[cfg(test)]
    represented_active_talent_group_like_cpp: u8,
    #[cfg(test)]
    represented_bonus_talent_groups_like_cpp: u8,
    #[cfg(test)]
    represented_talents_like_cpp: [BTreeMap<u32, u8>; MAX_SPECIALIZATIONS_LIKE_CPP],
    #[cfg(test)]
    represented_talents_loaded_like_cpp: bool,
    #[cfg(test)]
    represented_glyphs_like_cpp: [[u16; wow_packet::packets::misc::MAX_GLYPH_SLOT_INDEX_LIKE_CPP];
        MAX_SPECIALIZATIONS_LIKE_CPP],
    #[cfg(test)]
    represented_glyphs_loaded_like_cpp: bool,

    /// Quest template and quest-rule catalogs, owned by one type (#674).
    pub(crate) quests: crate::catalogs::quest::QuestCatalogsLikeCpp,
    /// C++ `ObjectMgr::_questPOIStore`, loaded from `quest_poi` / `quest_poi_points`.
    pub(crate) quest_poi_store_like_cpp:
        Option<Arc<HashMap<i32, wow_packet::packets::query::QuestPoiData>>>,
    #[cfg(test)]
    pub(crate) player_xp_table: Option<Arc<Vec<u32>>>,
    #[cfg(test)]
    pub(crate) exploration_base_xp_store: Option<Arc<ExplorationBaseXpStoreLikeCpp>>,
    #[cfg(test)]
    pub(crate) exploration_xp_rate_like_cpp: f32,
    pub(crate) min_quest_scaled_xp_ratio_like_cpp: u32,
    #[cfg(test)]
    pub(crate) min_discovered_scaled_xp_ratio_like_cpp: u32,
    /// Active quests for this player: quest_id → status.
    #[cfg(test)]
    pub(crate) player_quests: HashMap<u32, crate::handlers::quest::PlayerQuestStatus>,
    /// Quests the player has already been rewarded for (non-repeatable quests cannot be re-taken).
    /// C++ `Player::m_RewardedQuests`.
    #[cfg(test)]
    pub(crate) rewarded_quests: std::collections::HashSet<u32>,
    /// Exact C++ GetQuestStatus source proof. Both active and rewarded status
    /// queries must succeed for the active Player before absence is evidence.
    #[cfg(test)]
    player_quest_status_authority_complete_like_cpp: bool,
    /// Every row read from `character_queststatus_rewarded`, including special
    /// quests that C++ deliberately omits from `m_RewardedQuests` after
    /// applying their login reward-spell side effects.
    #[cfg(test)]
    represented_rewarded_quest_rows_like_cpp: BTreeSet<u32>,
    /// C++ `Player::HasAchieved`, represented per-session until character achievements are fully loaded.
    #[cfg(test)]
    pub(crate) represented_completed_achievements_like_cpp: HashSet<u32>,
    /// C++ `Player::_instanceResetTimes`: instance id -> release time.
    pub(crate) represented_instance_reset_times_like_cpp: BTreeMap<u32, u64>,
    /// C++ `ActivePlayerData::DailyQuestsCompleted`, represented per-session until full Player runtime owns it.
    #[cfg(test)]
    pub(crate) daily_quests_completed_like_cpp: HashSet<u32>,
    /// C++ `Player::m_DFQuests`, represented per-session until full Player runtime owns it.
    #[cfg(test)]
    pub(crate) df_quests_like_cpp: HashSet<u32>,
    /// C++ `Player::m_weeklyquests`, represented per-session until full Player runtime owns it.
    #[cfg(test)]
    pub(crate) weekly_quests_completed_like_cpp: HashSet<u32>,
    /// C++ `Player::m_monthlyquests`, represented per-session until full Player runtime owns it.
    #[cfg(test)]
    pub(crate) monthly_quests_completed_like_cpp: HashSet<u32>,
    /// C++ `Player::m_lastDailyQuestTime`, represented for daily/DF quest status persistence.
    #[cfg(test)]
    pub(crate) last_daily_quest_time_like_cpp: i64,
    pub(crate) quest_low_level_hide_diff_like_cpp: u32,
    pub(crate) quest_high_level_hide_diff_like_cpp: u32,
    /// C++ `Player::m_seasonalquests`, represented per-session until full Player runtime owns it.
    #[cfg(test)]
    pub(crate) seasonal_quests_like_cpp: BTreeMap<u16, BTreeMap<u32, u64>>,
    /// C++ `Player::m_SeasonalQuestChanged` represented flag.
    #[cfg(test)]
    pub(crate) seasonal_quest_changed_like_cpp: bool,
    /// C++ `CollectionMgr::_heirlooms`, represented until account collection runtime is complete.
    #[cfg(test)]
    pub(crate) represented_account_heirlooms_like_cpp: BTreeMap<u32, AccountHeirloomDataLikeCpp>,
    /// C++ `CollectionMgr::_toys`, represented until account collection runtime is complete.
    #[cfg(test)]
    pub(crate) represented_account_toys_like_cpp: BTreeMap<u32, u32>,
    /// C++ `CollectionMgr::_appearances`, represented until account collection persistence is ported.
    #[cfg(test)]
    pub(crate) represented_item_appearances_like_cpp: HashSet<u32>,
    /// C++ `CollectionMgr::LoadAccountItemAppearances` block vector used by
    /// `ActivePlayerData::Transmog`. This preserves sparse `blobIndex` rows
    /// that cannot be recovered from `_appearances` alone.
    #[cfg(test)]
    pub(crate) represented_item_appearance_blocks_like_cpp: Vec<u32>,
    /// C++ `CollectionMgr::_temporaryAppearances`, represented until account collection persistence is ported.
    #[cfg(test)]
    pub(crate) represented_temporary_item_appearances_like_cpp: HashMap<u32, HashSet<ObjectGuid>>,
    /// C++ `CollectionMgr::_favoriteAppearances`, represented until account collection persistence is ported.
    #[cfg(test)]
    pub(crate) represented_favorite_item_appearances_like_cpp:
        HashMap<u32, FavoriteAppearanceStateLikeCpp>,
    /// C++ `CollectionMgr::_transmogIllusions`, represented until account collection runtime is complete.
    #[cfg(test)]
    pub(crate) represented_transmog_illusions_like_cpp: HashSet<u32>,
    /// C++ `BattlePetMgr::_pets`, represented minimally until full battle-pet runtime is ported.
    /// Production sessions use `battle_pet_account_attachment_like_cpp`; this
    /// map remains only as the isolated represented/test fallback.
    #[cfg(test)]
    pub(crate) represented_battle_pets_like_cpp:
        HashMap<ObjectGuid, RepresentedBattlePetDataLikeCpp>,
    battle_pet_account_attachment_like_cpp: Option<BattlePetAccountAttachmentLikeCpp>,
    /// World-DB breed/quality selection tables for battle-pet trainer
    /// purchases (issue #161), loaded once at bootstrap.
    #[cfg(test)]
    battle_pet_selection_store_like_cpp:
        Option<Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>>,
    /// Deterministic purchase selection override for saga tests (#161).
    #[cfg(test)]
    battle_pet_purchase_selection_override_like_cpp:
        Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp>,
    /// C++ `BattlePetMgr::_hasJournalLock`, represented until full battle-pet runtime is ported.
    #[cfg(test)]
    pub(crate) represented_battle_pet_journal_lock_like_cpp: bool,
    /// C++ `BattlePetMgr::_slots`, represented until full battle-pet slot
    /// persistence is ported.
    #[cfg(test)]
    pub(crate) represented_battle_pet_slots_like_cpp:
        [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
    /// True only when the isolated represented/test fallback above was
    /// replaced from one complete account-DB slot query. Production sessions
    /// instead prove this through `battle_pet_account_attachment_like_cpp`,
    /// which is published only after the canonical account load succeeds.
    #[cfg(test)]
    represented_battle_pet_slots_authority_complete_like_cpp: bool,
    /// Isolated-test fallback for canonical C++
    /// `ActivePlayerData::SummonedBattlePetGUID` ownership.
    #[cfg(test)]
    pub(crate) represented_summoned_battle_pet_guid_like_cpp: Option<ObjectGuid>,
    /// Isolated-test fallback for canonical C++ `UnitData::Critter` ownership.
    #[cfg(test)]
    pub(crate) represented_critter_guid_like_cpp: Option<ObjectGuid>,
    /// Evidence for represented `TempSummon::UnSummon` from `CMSG_DISMISS_CRITTER`.
    #[cfg(test)]
    pub(crate) represented_dismissed_critter_guids_like_cpp: Vec<ObjectGuid>,
    /// C++ ObjectAccessor/TempSummon query state for `CMSG_QUERY_BATTLE_PET_NAME`.
    #[cfg(test)]
    pub(crate) represented_battle_pet_query_companions_like_cpp:
        HashMap<ObjectGuid, RepresentedBattlePetQueryCompanionLikeCpp>,
    /// Represented caged-item creations from C++ `BattlePetMgr::CageBattlePet`.
    #[cfg(test)]
    pub(crate) represented_battle_pet_cage_items_like_cpp: Vec<RepresentedBattlePetCageItemLikeCpp>,
    /// C++ `sBattlePetXPGameTable` projected as level -> `uint16(Wins * Xp)`.
    #[cfg(test)]
    pub(crate) represented_battle_pet_xp_per_level_like_cpp: BTreeMap<u16, u16>,
    /// Represented `CriteriaType::BattlePetReachLevel` events from battle-pet level grants.
    #[cfg(test)]
    pub(crate) represented_battle_pet_level_criteria_like_cpp:
        Vec<RepresentedBattlePetLevelCriteriaLikeCpp>,
    /// Represented `CriteriaType::ActivelyEarnPetLevel` events from pet-battle XP grants.
    #[cfg(test)]
    pub(crate) represented_battle_pet_active_level_criteria_like_cpp:
        Vec<RepresentedBattlePetLevelCriteriaLikeCpp>,
    /// Represented `CriteriaType::UniquePetsOwned` updates from `BattlePetMgr::AddPet`.
    #[cfg(test)]
    pub(crate) represented_battle_pet_unique_owned_criteria_like_cpp: u32,
    /// Represented `CriteriaType::LearnedNewPet` updates from `BattlePetMgr::AddPet`.
    #[cfg(test)]
    pub(crate) represented_battle_pet_learned_new_pet_criteria_like_cpp: Vec<u32>,
    /// Evidence for represented `BattlePetMgr::UpdateBattlePetData` calls.
    #[cfg(test)]
    pub(crate) represented_battle_pet_data_updates_like_cpp: Vec<ObjectGuid>,
    /// Session-local evidence for represented `Player::RemoveTimedQuest` calls.
    #[cfg(test)]
    pub(crate) represented_timed_quest_removals_like_cpp: Vec<u32>,
    /// Session-local evidence for represented quest reward `Player::UpdateSkillPro` calls.
    #[cfg(test)]
    pub(crate) represented_quest_reward_skill_updates_like_cpp: Vec<(u32, u32)>,
    /// Session-local evidence for represented quest reward triggered spell casts.
    #[cfg(test)]
    pub(crate) represented_quest_reward_spell_casts_like_cpp:
        Vec<RepresentedQuestRewardSpellCastLikeCpp>,
    /// Session-local evidence for represented quest reward `SetTitle` calls.
    #[cfg(test)]
    pub(crate) represented_quest_reward_titles_like_cpp: Vec<RepresentedQuestRewardTitleLikeCpp>,
    /// C++ `ActivePlayerData::KnownTitles` represented as title bit indexes.
    #[cfg(test)]
    represented_known_titles_like_cpp: HashSet<u32>,
    /// C++ `PlayerData::PlayerTitle` represented chosen title id.
    #[cfg(test)]
    represented_chosen_title_like_cpp: i32,
    /// Session-local evidence for represented quest reward talent point grants.
    #[cfg(test)]
    pub(crate) represented_quest_reward_talent_points_like_cpp:
        Vec<RepresentedQuestRewardTalentPointsLikeCpp>,
    /// Session-local evidence for represented quest reward mail.
    #[cfg(test)]
    pub(crate) represented_quest_reward_mails_like_cpp: Vec<RepresentedQuestRewardMailLikeCpp>,
    /// Session-local evidence for represented quest reward reputation.
    #[cfg(test)]
    pub(crate) represented_quest_reward_reputations_like_cpp:
        Vec<RepresentedQuestRewardReputationLikeCpp>,
    /// Bridge for quest completed unique-bit state loaded before the canonical Player snapshot exists.
    #[cfg(test)]
    pub(crate) represented_quest_completed_bits_like_cpp: BTreeSet<u32>,
    /// C++ `ActivePlayerData::ExploredZones`, represented before the canonical Player owns persistence.
    #[cfg(test)]
    represented_explored_zones_like_cpp: [u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    /// Represented `CriteriaType::RevealWorldMapOverlay` events from area discovery.
    #[cfg(test)]
    represented_reveal_world_map_overlay_criteria_like_cpp: Vec<u32>,
    /// Represented `Player::UpdateArea` / `Player::UpdateZone` criteria side effects.
    #[cfg(test)]
    represented_area_zone_criteria_like_cpp: Vec<RepresentedAreaZoneCriteriaLikeCpp>,
    /// Session-local evidence for represented `ScriptMgr::OnQuestAcknowledgeAutoAccept` calls.
    #[cfg(test)]
    pub(crate) represented_auto_accept_acknowledged_quests_like_cpp: Vec<u32>,
    /// Session-local representation of C++ pending shared quest sender + quest id.
    #[cfg(test)]
    pub(crate) represented_pending_quest_sharing_like_cpp:
        Option<RepresentedPendingQuestSharingLikeCpp>,
    /// Evidence-only replacement for sender `SendPushToPartyResponse` until safe cross-session fanout exists.
    #[cfg(test)]
    pub(crate) represented_quest_push_result_responses_like_cpp:
        Vec<RepresentedQuestPushResultResponseLikeCpp>,
    /// Explicit mismatch counter: C++ still clears pending sharing when sender GUID differs.
    #[cfg(test)]
    pub(crate) represented_quest_push_result_sender_mismatch_count_like_cpp: u32,
    /// Evidence for represented `HandleQuestConfirmAccept` after clear + template lookup.
    #[cfg(test)]
    pub(crate) represented_quest_confirm_accepts_like_cpp:
        Vec<RepresentedQuestConfirmAcceptLikeCpp>,
    /// Evidence for represented `Player::CompleteQuest` status-update side effects.
    pub(crate) represented_quest_complete_status_updates_like_cpp:
        Vec<RepresentedQuestCompleteStatusUpdateLikeCpp>,
    /// Session-local evidence for represented sender-side `HandlePushQuestToParty` preflight.
    #[cfg(test)]
    pub(crate) represented_push_quest_to_party_outcomes_like_cpp:
        Vec<RepresentedPushQuestToPartyOutcomeLikeCpp>,

    // ── Loot ──────────────────────────────────────────────────────
    /// Active loot windows keyed by creature GUID.
    pub(crate) loot_table:
        std::collections::HashMap<wow_core::ObjectGuid, wow_packet::packets::loot::CreatureLoot>,
    /// Object-owned loot generation represented by each session-local packet cache.
    pub(crate) represented_loot_cache_generations_like_cpp:
        std::collections::HashMap<wow_core::ObjectGuid, u64>,
    /// Mirrors C++ PlayerData::LootTargetGUID for guards that compare active loot by GUID.
    pub(crate) active_loot_guid: wow_core::ObjectGuid,
    /// Represented owner GUIDs currently visible through C++ `Player::m_AELootView`.
    pub(crate) active_loot_view_owners: std::collections::HashSet<wow_core::ObjectGuid>,
    /// Object-owned generation that was actually opened for each active loot view.
    ///
    /// GUIDs are reused across creature respawns and gameobject restocks.  A delayed
    /// packet from an older window must therefore not be authorized merely because
    /// the replacement lifetime has the same owner/loot GUID and player eligibility.
    pub(crate) active_loot_view_generations_like_cpp:
        std::collections::HashMap<wow_core::ObjectGuid, u64>,
    /// Exact backing allocation opened for each view. Scope epochs restart at
    /// one in a newly allocated authority, so the generation map alone cannot
    /// prevent ABA when a creature GUID is recreated.
    pub(crate) active_loot_view_authorities_like_cpp:
        std::collections::HashMap<wow_core::ObjectGuid, OwnedLootAuthority>,
    /// Detached durable loot grants and their post-commit runtime
    /// publications. This covers claimed world-owner items plus Item-owner
    /// items/money; Item owners have no map-owned loot authority.
    durable_item_loot_persistence_like_cpp: DurableItemLootPersistenceTrackerLikeCpp,
    /// Per-character fence published to remote loot sources before they begin
    /// mutating this character's durable balance.
    durable_loot_money_persistence_like_cpp: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    /// Test fixture for the process-owned linked-module registry. Production
    /// borrows the required registry from the session driver.
    #[cfg(test)]
    module_registry_like_cpp: Option<Arc<wow_module_api::ModuleRegistry>>,
    /// Represented pending group/NBG loot rolls keyed by `(LootObj, LootListID)`.
    pub(crate) represented_loot_rolls:
        std::collections::HashMap<(wow_core::ObjectGuid, u8), RepresentedLootRollState>,
    /// Explicit test seam for persistence-sensitive loot-money paths. Production
    /// never bypasses the character database.
    #[cfg(test)]
    pub(crate) loot_money_persistence_test_result_like_cpp: Option<bool>,
    #[cfg(test)]
    pub(crate) loot_item_store_test_grants_like_cpp: Option<Arc<AtomicUsize>>,
    #[cfg(test)]
    pub(crate) loot_item_store_test_success_like_cpp: bool,
    /// Optional test-only COMMIT gate for exercising remote command timeout
    /// and release while the authority claim is already persistence-owned.
    #[cfg(test)]
    pub(crate) loot_item_store_test_commit_gate_like_cpp: Option<Arc<tokio::sync::Notify>>,
    #[cfg(test)]
    pub(crate) represented_loot_roll_criteria_events: Vec<RepresentedLootRollCriteriaEvent>,
    #[cfg(test)]
    pub(crate) represented_gameobject_criteria_events: Vec<RepresentedGameObjectCriteriaEvent>,
    #[cfg(test)]
    pub(crate) represented_transmog_criteria_events: Vec<RepresentedTransmogCriteriaEvent>,
    /// C++ `sWorld->getRate(...)` subset used by represented loot generation.
    loot_drop_rates: LootDropRatesLikeCpp,
    /// C++ `sWorld->getRate(...)` subset used by represented reputation gain.
    reputation_rates: ReputationRatesLikeCpp,
    /// C++ `sWorld->getRate(RATE_REPAIRCOST)` represented value.
    repair_cost_rate_like_cpp: f32,
    /// C++ `CONFIG_RESET_SCHEDULE_{HOUR,WEEK_DAY}` consumed by `InstanceLockMgr::GetNextResetTime`.
    reset_schedule_like_cpp: wow_instances::ResetSchedule,
    /// C++ `CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN` represented switch.
    represented_offhand_check_at_spell_unlearn_like_cpp: bool,
    /// C++ `CONFIG_VMAP_INDOOR_CHECK` represented switch.
    vmap_indoor_check_like_cpp: bool,
    /// Represented C++ `WorldObject::IsOutdoors()` result until VMAP owns it.
    #[cfg(test)]
    represented_is_outdoors_like_cpp: Option<bool>,
    /// Fixture-only fallback. Production C++ `ReputationMgr` state is owned by
    /// the generation-checked canonical `Player`.
    #[cfg(test)]
    reputation_mgr_like_cpp: ReputationMgrLikeCpp,
    /// C++ `ActivePlayerData::WatchedFactionIndex` represented state.
    #[cfg(test)]
    watched_faction_index_like_cpp: i32,
    /// C++ `CONFIG_ENABLE_AE_LOOT` represented switch.
    enable_ae_loot_like_cpp: bool,
    /// C++ `CONFIG_ADDON_CHANNEL` represented switch.
    #[cfg(test)]
    addon_channel_like_cpp: bool,
    /// C++ `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING` represented switch for chat validation.
    #[cfg(test)]
    chat_fake_message_preventing_like_cpp: bool,
    /// C++ `CONFIG_CHAT_PARTY_RAID_WARNINGS` represented switch.
    #[cfg(test)]
    party_raid_warnings_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_GM_GROUP` represented switch.
    #[cfg(test)]
    allow_gm_group_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP` represented switch.
    #[cfg(test)]
    allow_two_side_interaction_group_like_cpp: bool,
    /// C++ `CONFIG_PARTY_LEVEL_REQ` represented gate.
    #[cfg(test)]
    party_level_req_like_cpp: u32,
    /// C++ `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK` represented switch.
    #[cfg(test)]
    chat_strict_link_checking_kick_like_cpp: bool,
    /// C++ `CONFIG_CHAT_*_LEVEL_REQ` represented chat level gates.
    #[cfg(test)]
    chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp,
    /// C++ `CONFIG_LISTEN_RANGE_*` represented nearby-chat ranges.
    #[cfg(test)]
    chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp,
    /// C++ `CONFIG_CHATFLOOD_*` represented chat spam protection.
    #[cfg(test)]
    chat_flood_config_like_cpp: ChatFloodConfigLikeCpp,
    chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp; 2],
    /// C++ `CONFIG_ENABLE_MMAPS` + `DataDir` represented until map lifecycle owns real mmaps.
    mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp,
    /// C++ `sWaypointMgr->GetPath(pathId)` resolver for session-created legacy `WorldCreature`
    /// compatibility objects. The canonical path store is owned by `world-server`.
    waypoint_path_resolver_like_cpp: Option<WaypointPathResolverLikeCpp>,
    /// Session-local representation of `GameObject::m_unique_users` for no-GetLootId chest uses.
    pub(crate) represented_unique_gameobject_uses: std::collections::HashSet<wow_core::ObjectGuid>,
    /// Represented C++ `GameEvents::Trigger` and `TriggeringLinkedGameObject` hook points.
    pub(crate) represented_gameobject_use_effects: Vec<RepresentedGameObjectUseEffect>,
    /// Session-local represented `GameObject` use state until canonical GO runtime ownership lands.
    /// Deterministic iteration order by GUID (not a strict C++ ordering guarantee).
    pub(crate) represented_gameobject_use_states:
        std::collections::BTreeMap<wow_core::ObjectGuid, RepresentedGameObjectUseState>,
    /// C++ `Player::SetPendingBind` represented until `InstanceMap` owns real bind confirmation.
    pub(crate) pending_bind: Option<RepresentedPendingBind>,
    /// Confirmed pending bind ids, used by represented `CMSG_INSTANCE_LOCK_RESPONSE`.
    pub(crate) represented_confirmed_pending_binds: Vec<u32>,
    /// Count of represented `Player::RepopAtGraveyard` calls from rejected pending binds.
    #[cfg(test)]
    pub(crate) represented_repop_at_graveyard_count: u32,
    /// Session-local representation of `GameObject::m_tapList` for personal encounter loot.
    pub(crate) represented_gameobject_tap_lists:
        std::collections::HashMap<wow_core::ObjectGuid, Vec<wow_core::ObjectGuid>>,
    /// Session-local representation of `Player::IsLockedToDungeonEncounter` for encounter loot.
    pub(crate) represented_locked_dungeon_encounters:
        std::collections::HashSet<(wow_core::ObjectGuid, u32)>,
    /// Session-local per-player money for represented personal encounter loot.
    pub(crate) represented_personal_loot_money:
        std::collections::HashMap<(wow_core::ObjectGuid, wow_core::ObjectGuid), u32>,
    /// Owners whose money must be read from `represented_personal_loot_money`.
    pub(crate) represented_personal_loot_owners: std::collections::HashSet<wow_core::ObjectGuid>,

    // ── Dynamic visibility tracking ───────────────────────────────
    /// C++ `Player::m_clientGUIDs`: exact objects currently known by this client.
    /// Updated on login and each visibility refresh (player movement).
    pub(crate) client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp,
    /// C++ `PlayerData::Customizations` loaded before the self CREATE and
    /// retained for non-owner visibility CREATE blocks.
    #[cfg(test)]
    pub(crate) loaded_player_customizations_like_cpp:
        Box<Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>>,
    /// C++ `Player::m_visibleTransports`, maintained by `Map::SendInitTransports`.
    pub(crate) client_visible_transports_like_cpp: std::collections::HashSet<wow_core::ObjectGuid>,
    /// Current C++ `m_movementInfo.transport.guid`, used to exclude the
    /// player's own transport from `Map::SendInitTransports`.
    #[cfg(test)]
    player_transport_login_state_like_cpp: Option<Box<PlayerTransportLoginStateLikeCpp>>,
    /// Login-start delivery guard for creature movement packets.
    ///
    /// The C++ 3.4.3 login baseline does not deliver `SMSG_ON_MONSTER_MOVE`
    /// during the initial enter-world packet burst, even after
    /// `UpdateVisibilityForPlayer()` has repopulated `m_clientGUIDs`. Rust uses
    /// the cutoff to drop movement commands queued before the burst completes
    /// without blocking movement generated after the player is in world.
    pub(crate) suppress_creature_movement_queued_at_or_before_like_cpp: Option<Instant>,
    /// Represented C++ `Player::m_seer`. This slice keeps only the GUID seam:
    /// self/player GUID by default, current viewpoint GUID after valid FAR_SIGHT enable.
    pub(crate) represented_seer_guid_like_cpp: Option<wow_core::ObjectGuid>,
    /// Session-local delivery guard for represented DynamicObject VALUES packets
    /// consumed from the last map-owned `Map::SendObjectUpdates` stable snapshot.
    /// Includes the map update generation so repeated `process_pending()` calls
    /// suppress the same snapshot without blocking later identical bytes.
    represented_dynamic_object_values_updates_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid, u64)>,
    /// Session-local delivery guard for represented GameObjectDespawn packets
    /// consumed from the last map-owned `GameObject::Update` summary.
    represented_gameobject_visual_despawns_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid)>,
    /// Session-local delivery guard for represented CapturePointRemoved packets
    /// consumed from the last map-owned `GameObject::Delete` summary.
    represented_capture_point_removed_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid)>,
    /// Represented C++ `GameObject::GetPhaseShift()` for visible DB-spawned
    /// gameobjects until canonical gameobject map ownership lands.
    pub(crate) represented_gameobject_phase_shifts:
        std::collections::HashMap<wow_core::ObjectGuid, PhaseShift>,
    /// Test-only C++ `Player::GetPhaseShift()` fixture used before a canonical
    /// player handle is installed.
    #[cfg(test)]
    pub(crate) represented_player_phase_shift: PhaseShift,
    /// Position at which visibility was last fully recalculated.
    pub(crate) last_visibility_pos: Option<wow_core::Position>,

    // ── Player-menu interaction state ─────────────────────────────
    /// The represented subset of C++ `PlayerMenu::InteractionData`.
    ///
    /// `PlayerChoiceId` remains unrepresented until the corresponding
    /// player-choice runtime lands. Gossip options deliberately remain
    /// separate because C++ `InteractionData::Reset` and
    /// `PlayerMenu::ClearMenus` are different operations.
    #[cfg(test)]
    player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp,
    /// Active gossip options for the NPC the player is talking to.
    /// Stored when SMSG_GOSSIP_MESSAGE is sent, used when CMSG_GOSSIP_SELECT_OPTION arrives.
    #[cfg(test)]
    pub(crate) gossip_options: Vec<GossipOptionInfo>,

    // ── Area trigger tracking ──────────────────────────────────────
    /// Currently active area trigger ID (to prevent retriggering on same position).
    /// Set to Some(trigger_id) when entered, None when exited.
    pub(crate) active_area_trigger: Option<u32>,
    /// Ownerless legacy fixtures only; production uses Player's teleport state.
    #[cfg(test)]
    pending_teleport: Option<(u32, wow_core::Position)>,

    // ── QueryCreature cache ────────────────────────────────────────
    /// Creature entry IDs for which we've already sent a QueryCreatureResponse.
    /// The client caches the response locally, so we skip duplicates.
    pub(crate) creature_query_cache: std::collections::HashSet<u32>,
}

/// Compatibility name while handler modules move to the Player-owned type.
pub use wow_entities::PlayerGossipOptionLikeCpp as GossipOptionInfo;

/// Compatibility name retained while handlers move onto Player-owned inventory
/// commands and queries. The concrete record is owned by `wow_entities::Player`.
pub use wow_entities::PlayerInventoryItem as InventoryItem;

/// Detached inventory state already reserved by an earlier operation in the
/// same atomic storage plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectInventoryStorageOverlayLikeCpp {
    pub(crate) bag: u8,
    pub(crate) slot: u8,
    pub(crate) entry_id: u32,
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedAutoUnequipOffhandReasonLikeCpp {
    Forced,
    LostDualWield,
    InvalidTwoHandState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAutoUnequipOffhandLikeCpp {
    pub item_guid: ObjectGuid,
    pub item_entry: u32,
    pub reason: RepresentedAutoUnequipOffhandReasonLikeCpp,
    pub stored_destination: Option<(u8, u8)>,
    pub needs_mail_fallback: bool,
}

pub(crate) const MAX_EQUIPMENT_SET_INDEX_LIKE_CPP: u32 = 20;

pub(crate) use wow_entities::{
    PlayerEquipmentSetLikeCpp as RepresentedEquipmentSetLikeCpp,
    PlayerEquipmentSetTypeLikeCpp as RepresentedEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp as RepresentedEquipmentSetUpdateStateLikeCpp,
    PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp,
};

fn represented_equipment_set_from_packet_like_cpp(
    set: wow_packet::packets::misc::EquipmentSetDataLikeCpp,
    guid: u64,
    state: RepresentedEquipmentSetUpdateStateLikeCpp,
) -> Option<RepresentedEquipmentSetLikeCpp> {
    let set_type =
        RepresentedEquipmentSetTypeLikeCpp::handler_branch_from_i32_like_cpp(set.set_type)?;
    Some(RepresentedEquipmentSetLikeCpp {
        raw_set_type: set.set_type,
        set_type,
        guid,
        set_id: set.set_id,
        ignore_mask: set.ignore_mask,
        pieces: set.pieces,
        appearances: set.appearances,
        enchants: set.enchants,
        secondary_shoulder_appearance_id: set.secondary_shoulder_appearance_id,
        secondary_shoulder_slot: set.secondary_shoulder_slot,
        secondary_weapon_appearance_id: set.secondary_weapon_appearance_id,
        secondary_weapon_slot: set.secondary_weapon_slot,
        assigned_spec_index: set.assigned_spec_index,
        set_name: set.set_name,
        set_icon: set.set_icon,
        state,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedEquipmentSetSavedLikeCpp {
    pub(crate) guid: u64,
    pub(crate) set_type: RepresentedEquipmentSetTypeLikeCpp,
    pub(crate) raw_set_type: i32,
    pub(crate) set_id: u32,
    pub(crate) generated_new_guid: bool,
}

/// Current finite stock for a vendor item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorItemCount {
    pub count: u32,
    pub last_increment_time: u64,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorBuyItemTestOverrideLikeCpp {
    pub(crate) item_id: u32,
    pub(crate) item_type: i32,
    pub(crate) max_count: u32,
    pub(crate) incr_time: u32,
    pub(crate) player_condition_id: u32,
    pub(crate) has_vendor_conditions: bool,
    pub(crate) extended_cost: u32,
    pub(crate) buy_price: u64,
    pub(crate) max_durability: u32,
    pub(crate) buy_count: u32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerConditionContextLikeCpp {
    spells: Vec<u32>,
    items: Vec<PlayerConditionCountLikeCpp>,
    currencies: Vec<PlayerConditionCountLikeCpp>,
    completed_quests: Vec<u32>,
    current_quests: Vec<u32>,
    complete_quests: Vec<u32>,
    auras: Vec<PlayerConditionAuraLikeCpp>,
    skills: Vec<PlayerConditionSkillLikeCpp>,
    reputations: Vec<PlayerConditionReputationLikeCpp>,
    explored_area_ids: Vec<u16>,
    parent_area_ids: Vec<u32>,
    achievements: Vec<u16>,
    lfg_values: Vec<PlayerConditionCountLikeCpp>,
    modifier_tree_ids: Vec<u32>,
    quest_kills: Vec<PlayerConditionQuestKillLikeCpp>,
    avg_item_level: f32,
    avg_equipped_item_level: f32,
    mainhand_weapon_subclass: Option<u8>,
}

/// Represented subset of C++ `Player::m_unitData` item-level cap fields
/// consumed by `Item::GetItemLevel(Player const*)`.
pub(crate) type RepresentedItemLevelCapsLikeCpp = wow_entities::PlayerItemLevelCapsLikeCpp;

impl RepresentedPlayerConditionContextLikeCpp {
    pub(crate) fn as_context<'a>(
        &'a self,
        session: &'a WorldSession,
    ) -> Option<PlayerConditionContextLikeCpp<'a>> {
        let class = session.player_class_like_cpp();
        let (_, area_id) = session.player_zone_area_like_cpp()?;
        Some(PlayerConditionContextLikeCpp {
            race: session.player_race_like_cpp(),
            class_mask: if class == 0 {
                0
            } else {
                1u32 << u32::from(class.saturating_sub(1))
            },
            gender: session.player_gender_like_cpp(),
            native_gender: session.player_gender_like_cpp(),
            power_type: -1,
            power: 0,
            max_power: 0,
            primary_specialization_id: session
                .represented_primary_specialization_id_like_cpp()
                .filter(|spec_id| *spec_id != 0),
            skills: &self.skills,
            language_skill: 0,
            reputations: &self.reputations,
            current_pvp_faction: 0,
            pvp_medals_mask: 0,
            lifetime_max_pvp_rank: 0,
            movement_flags: [0, 0],
            mainhand_weapon_subclass: self.mainhand_weapon_subclass,
            party_status: if session.resolved_group_guid_like_cpp().is_some() {
                PlayerConditionPartyStatusLikeCpp::InParty
            } else {
                PlayerConditionPartyStatusLikeCpp::Solo
            },
            completed_quests: &self.completed_quests,
            current_quests: &self.current_quests,
            complete_quests: &self.complete_quests,
            spells: &self.spells,
            items: &self.items,
            currencies: &self.currencies,
            explored_area_ids: &self.explored_area_ids,
            auras: &self.auras,
            weather_id: 0,
            achievements: &self.achievements,
            lfg_values: &self.lfg_values,
            area_id,
            parent_area_ids: &self.parent_area_ids,
            expansion: session.expansion as i8,
            server_expansion: session.account_expansion as i8,
            is_game_master: session.security > 0,
            phase_satisfied: true,
            quest_kill_id: 0,
            quest_kills: &self.quest_kills,
            avg_item_level: self.avg_item_level,
            avg_equipped_item_level: self.avg_equipped_item_level,
            modifier_tree_ids: &self.modifier_tree_ids,
            chr_specializations: session.chr_specialization_store().map(Arc::as_ref),
            world_state_expressions: None,
            world_state_expression_context: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerCurrencyDelta {
    pub currency_id: u32,
    pub quantity: u32,
    pub amount: u32,
    pub weekly_quantity: Option<u32>,
    pub max_quantity: Option<u32>,
    pub total_earned: Option<u32>,
    pub suppress_chat_log: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestObjectiveProgressEventLikeCpp {
    MoneyChanged {
        old_money: u64,
        new_money: u64,
    },
    #[allow(dead_code)]
    CurrencyChanged {
        currency_id: u32,
        change: i32,
    },
    #[allow(dead_code)]
    ReputationChanged {
        faction_id: u32,
        change: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurrencyGainSourceLikeCpp {
    QuestReward = 3,
    QuestRewardIgnoreCaps = 29,
    WorldQuestReward = 35,
    WorldQuestRewardIgnoreCaps = 36,
    DailyQuestReward = 38,
    WeeklyQuestReward = 40,
}

pub(crate) fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}

const FIRST_LOGIN_START_REPUTATION_STANDING_LIKE_CPP: i32 = 42_999;
const FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP: &[u32] = &[
    942, 935, 936, 1011, 970, 967, 989, 932, 934, 1038, 1077, 1106, 1104, 1090, 1098, 1156, 1073,
    1105, 1119, 1091,
];
const FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP: &[u32] = &[
    72, 47, 69, 930, 730, 978, 54, 946, 1037, 1068, 1126, 1094, 1050,
];
const FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP: &[u32] = &[
    76, 68, 81, 911, 729, 941, 530, 947, 1052, 1067, 1124, 1064, 1085,
];

const WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP: u8 = 80;

fn player_team_id_for_race_cpp(race: u8) -> u32 {
    match player_team_for_race_cpp(race) {
        Team::Horde => 1,
        _ => 0,
    }
}

fn currency_max_quantity_cpp(entry: &CurrencyTypesEntry, currency: &PlayerCurrency) -> u32 {
    if !entry.has_max_quantity(false, false) {
        return 0;
    }

    let mut max_quantity = entry.max_qty;
    if entry.flags.contains(CurrencyTypesFlags::DYNAMIC_MAXIMUM) {
        max_quantity = max_quantity.saturating_add(currency.increased_cap_quantity);
    }
    max_quantity
}

pub use wow_entities::AuraApplicationLikeCpp as AuraApplication;

type CanonicalThreatAuraSnapshotLikeCpp = wow_entities::AuraThreatSnapshotLikeCpp;

pub use wow_entities::{RepresentedAuraEffectAmountLikeCpp, RepresentedAuraEffectLikeCpp};

fn represented_aura_effect_amounts_like_cpp(
    effect: &wow_data::SpellEffectInfo,
) -> Vec<RepresentedAuraEffectAmountLikeCpp> {
    let Some(effect_index) = u8::try_from(effect.effect_index).ok() else {
        return Vec::new();
    };
    vec![RepresentedAuraEffectAmountLikeCpp {
        effect_index,
        amount: effect.effect_base_points,
    }]
}

fn unit_owned_apply_aura_effect_mask_like_cpp(spell: &wow_data::SpellInfo) -> u32 {
    use wow_data::spell::spell_effect_types::{
        SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
        SPELL_EFFECT_APPLY_AREA_AURA_OWNER, SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
        SPELL_EFFECT_APPLY_AREA_AURA_PET, SPELL_EFFECT_APPLY_AREA_AURA_RAID,
        SPELL_EFFECT_APPLY_AURA, SPELL_EFFECT_APPLY_AURA_ON_PET,
    };

    const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
    const SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM: u32 = 271;

    spell.effects().iter().fold(0, |mask, effect| {
        let unit_owned = matches!(
            effect.effect,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
        );
        if unit_owned && effect.effect_index < u32::BITS {
            mask | (1u32 << effect.effect_index)
        } else {
            mask
        }
    })
}

const AFLAG_NOCASTER_LIKE_CPP: u32 = 0x0000_0001;
pub(crate) const AFLAG_SCALABLE_LIKE_CPP: u32 = 0x0000_0008;

pub(crate) const SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP: u32 = 0x0000_0800;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_ANIM_LIKE_CPP: u32 = 0x0000_0020;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP: u32 = 0x0000_0008;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP: u32 = 0x0000_0010;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP: u32 =
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP | SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP: u32 = 0x0200_0000;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP: u32 = 0x0000_0020;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP: u32 = 0x0000_4000;
pub(crate) const PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP: u32 = 0x0000_8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ApplyEffectSummonObjectWildSessionStatusLikeCpp {
    InvalidTemplateEntry,
    MissingTemplateStore,
    MissingTemplate,
    MissingExplicitDestination,
    MissingCaster,
    MissingCasterPosition,
    MissingCanonicalMapManager,
    MissingCanonicalPlayerMap,
    MissingManagedMap,
    MapResolved,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) struct ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
    pub status: ApplyEffectSummonObjectWildSessionStatusLikeCpp,
    pub template_entry: Option<u32>,
    pub duration_ms: Option<i32>,
    pub explicit_destination_used: bool,
    pub close_point_fallback_represented: bool,
    pub map_outcome: Option<wow_map::map::SpellEffectSummonObjectWildOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ApplyEffectSummonObjectSlotSessionStatusLikeCpp {
    InvalidSlot,
    InvalidTemplateEntry,
    MissingTemplateStore,
    MissingTemplate,
    MissingCaster,
    MissingCasterPosition,
    MissingCanonicalMapManager,
    MissingCanonicalPlayerMap,
    MissingManagedMap,
    MapResolved,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) struct ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
    pub status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp,
    pub slot: Option<usize>,
    pub template_entry: Option<u32>,
    pub duration_ms: Option<i32>,
    pub explicit_destination_used: bool,
    pub close_point_fallback_represented: bool,
    pub cleanup_outcome: Option<wow_map::map::GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp>,
    pub map_outcome: Option<wow_map::map::GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedSpellFocusObjectLikeCpp {
    pub guid: ObjectGuid,
    pub map_key: wow_map::MapKey,
    pub position: Position,
    pub source: wow_entities::SpellFocusUseSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepresentedMountSpellCheckOutcomeLikeCpp {
    CastFailed(SpellCastResult),
    DontReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReputationGainSourceLikeCpp {
    Kill,
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
    Spell,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MovementFallDamageEvent {
    pub z_diff: f32,
    pub damage: u32,
    pub final_damage: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MovementUnderMapDamageEvent {
    pub z: f32,
    pub min_height: f32,
    pub damage: u32,
}

/// Parameters for spawning nearby creatures after login.
pub struct PendingCreatureSpawn {
    pub map_id: u16,
    pub position: wow_core::Position,
    pub zone_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingCreatureKillRewardLikeCpp {
    killer_guid: ObjectGuid,
    creature_guid: ObjectGuid,
    creature_entry: u32,
    creature_level: u8,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedCreatureKillEventLikeCpp {
    KillerProc {
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    TapperTargetDiesProc {
        tapper_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    VictimDeathProc {
        victim_guid: ObjectGuid,
    },
    DeliveredKillingBlowCriteria {
        player_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        quantity: u32,
    },
    DeathStateJustDied {
        victim_guid: ObjectGuid,
    },
    ZoneScriptUnitDeath {
        unit_guid: ObjectGuid,
    },
    TapperPetKilledUnitAi {
        tapper_guid: ObjectGuid,
        pet_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    LootFlagsApplied {
        creature_guid: ObjectGuid,
        lootable: bool,
        can_skin: bool,
        skinnable: bool,
    },
    CreatureOnHealthDepletedAi {
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        is_kill: bool,
    },
    CreatureJustDiedAi {
        creature_guid: ObjectGuid,
        killer_guid: ObjectGuid,
    },
    ScriptMgrOnCreatureKill {
        killer_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    },
    CreatureKillReputationAwarded {
        creature_guid: ObjectGuid,
        faction_id: u32,
        reputation: i32,
        spillover_only: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CreatureCreateModelScalarsLikeCpp {
    pub display_scale: f32,
    pub native_x_display_scale: f32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub hover_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CreatureCreateDisplaySelectionLikeCpp {
    pub display_id: u32,
    pub display_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureCreateStatsLikeCpp {
    pub health: i64,
    pub max_health: i64,
    pub power_type: PowerType,
    pub power: i32,
    pub max_power: i32,
    pub base_mana: i32,
}

fn is_represented_bag_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

fn player_class_mask_for_transmog_like_cpp(class_id: u8) -> u32 {
    if class_id == 0 || class_id > 32 {
        0
    } else {
        1_u32 << u32::from(class_id - 1)
    }
}

fn player_class_mask_for_talent_like_cpp(class_id: u8) -> Option<u32> {
    if class_id == 0 || class_id > 32 {
        None
    } else {
        Some(1_u32 << u32::from(class_id - 1))
    }
}

fn player_class_by_armor_subclass_like_cpp(subclass: u32) -> u32 {
    match subclass {
        x if x == ItemSubClassArmor::Miscellaneous as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Cloth as u32 => {
            (1 << (5 - 1)) | (1 << (8 - 1)) | (1 << (9 - 1))
        }
        x if x == ItemSubClassArmor::Leather as u32 => {
            (1 << (4 - 1)) | (1 << (10 - 1)) | (1 << (11 - 1)) | (1 << (12 - 1))
        }
        x if x == ItemSubClassArmor::Mail as u32 => (1 << (3 - 1)) | (1 << (7 - 1)),
        x if x == ItemSubClassArmor::Plate as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (6 - 1))
        }
        x if x == ItemSubClassArmor::Cosmetic as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Shield as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (7 - 1))
        }
        x if x == ItemSubClassArmor::Libram as u32 => 1 << (2 - 1),
        x if x == ItemSubClassArmor::Idol as u32 => 1 << (11 - 1),
        x if x == ItemSubClassArmor::Totem as u32 => 1 << (7 - 1),
        x if x == ItemSubClassArmor::Sigil as u32 => 1 << (6 - 1),
        x if x == ItemSubClassArmor::Relic as u32 => {
            (1 << (2 - 1)) | (1 << (6 - 1)) | (1 << (7 - 1)) | (1 << (11 - 1))
        }
        _ => 0,
    }
}

#[allow(dead_code)]
impl WorldSession {
    const MIN_ITEM_LEVEL_LIKE_CPP: u32 = 1;
    const MAX_ITEM_LEVEL_LIKE_CPP: u32 = 1300;

    /// Create a new session with the given account info and channels.
    pub fn new(
        account_id: u32,
        account_name: String,
        security: u8,
        expansion: u8,
        account_expansion: u8,
        build: u32,
        session_key: Vec<u8>,
        locale: String,
        packet_rx: flume::Receiver<WorldPacket>,
        send_tx: flume::Sender<Vec<u8>>,
    ) -> Self {
        let (session_command_tx, session_command_rx) = flume::bounded(256);

        // The instance endpoint keeps the pre-#297 default; the kernel does not
        // hardcode a world-server address of its own.
        let mut connection = wow_session::SessionConnection::new(send_tx, packet_rx);
        connection.set_instance_endpoint([127, 0, 0, 1], 8086);

        Self {
            quests: crate::catalogs::quest::QuestCatalogsLikeCpp::default(),
            chr: crate::catalogs::chr::ChrCatalogsLikeCpp::default(),
            creatures: crate::catalogs::creature::CreatureCatalogsLikeCpp::default(),
            factions: crate::catalogs::faction::FactionCatalogsLikeCpp::default(),
            gameobjects: crate::catalogs::gameobject::GameObjectCatalogsLikeCpp::default(),
            items: crate::catalogs::item::ItemCatalogsLikeCpp::default(),
            maps: crate::catalogs::map::MapCatalogsLikeCpp::default(),
            spell_catalogs: crate::catalogs::spell::SpellCatalogsLikeCpp::default(),
            account_id,
            battlenet_account_id: account_id,
            realm_list_secret_like_cpp: [0; 32],
            recruiter_id_like_cpp: 0,
            is_a_recruiter_like_cpp: false,
            account_name,
            security,
            expansion,
            account_expansion,
            server_expansion_like_cpp: 2,
            #[cfg(test)]
            characters_per_realm_like_cpp: 60,
            #[cfg(test)]
            declined_names_used_like_cpp: false,
            #[cfg(test)]
            feature_system_bpay_store_enabled_like_cpp: false,
            #[cfg(test)]
            feature_system_character_undelete_enabled_like_cpp: false,
            instance_ignore_raid_like_cpp: false,
            instance_ignore_level_like_cpp: false,
            max_instances_per_hour_like_cpp: 5,
            #[cfg(test)]
            start_all_explored_like_cpp: false,
            #[cfg(test)]
            start_all_reputation_like_cpp: false,
            #[cfg(test)]
            start_all_spells_like_cpp: false,
            #[cfg(test)]
            player_create_info_store_like_cpp: None,
            #[cfg(test)]
            player_create_cast_spell_store_like_cpp: None,
            #[cfg(test)]
            player_create_custom_spell_store_like_cpp: None,
            build,
            session_key,
            locale,
            mute_time_like_cpp: 0,
            connection,
            session_command_tx,
            session_command_rx,
            durable_creature_runtime_commands_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Arc::new(AtomicBool::new(false)),
            state: SessionState::Authed,
            last_packet_time: Instant::now(),
            socket_timeouts_like_cpp: SocketTimeoutsLikeCpp::default(),
            socket_timeout_deadline_like_cpp: Instant::now()
                + Duration::from_secs(SocketTimeoutsLikeCpp::default().unauthenticated_secs),
            packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp::default(),
            packet_throttling_like_cpp: HashMap::new(),
            remote_address_like_cpp: None,
            pending_packet_spoof_ban_like_cpp: None,
            legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp::default(),
            represented_runtime_rng_like_cpp: StdRng::from_entropy(),
            dispatch_table: build_dispatch_table(),
            homebind_persistence_tx_like_cpp: None,
            persistence_ports_like_cpp: Box::default(),
            trainer_store_like_cpp: None,
            #[cfg(test)]
            bank_bag_slot_prices_store: None,
            currency_types_store: None,
            #[cfg(test)]
            import_price_stores: None,
            #[cfg(test)]
            emotes_store: None,
            #[cfg(test)]
            emotes_text_store: None,
            #[cfg(test)]
            item_class_store: None,
            #[cfg(test)]
            item_currency_cost_store: None,
            trinity_string_store: None,
            heirloom_store: None,
            toy_store: None,
            #[cfg(test)]
            battle_pet_breed_quality_store: None,
            #[cfg(test)]
            battle_pet_breed_state_store: None,
            #[cfg(test)]
            battle_pet_species_store: None,
            #[cfg(test)]
            battle_pet_species_state_store: None,
            #[cfg(test)]
            battle_pet_selection_store_like_cpp: None,
            #[cfg(test)]
            battle_pet_purchase_selection_override_like_cpp: None,
            #[cfg(test)]
            battle_pet_xp_game_table: None,
            combat_ratings_game_table: None,
            shield_block_regular_game_table: None,
            transmog_set_item_store: None,
            #[cfg(test)]
            item_price_base_store: None,
            player_stats: None,
            #[cfg(test)]
            represented_item_level_caps_like_cpp: RepresentedItemLevelCapsLikeCpp::default(),
            #[cfg(test)]
            represented_using_pvp_item_levels_like_cpp: false,
            pvp_item_store: None,
            durability_costs_store: None,
            durability_quality_store: None,
            item_template_addon_quest_log_item_ids_like_cpp: HashMap::new(),
            rand_prop_points_store: None,
            #[cfg(test)]
            item_disenchant_loot_store: None,
            loot_stores: None,
            condition_store: None,
            player_condition_store: None,
            #[cfg(test)]
            adventure_map_poi_store: None,
            content_tuning_store: None,
            curve_store: None,
            curve_point_store: None,
            scaling_stat_distribution_store: None,
            scaling_stat_values_store: None,
            disable_mgr: None,
            difficulty_store: None,
            lock_store: None,
            gem_properties_store: None,
            #[cfg(test)]
            tact_key_store: None,
            skill_store: None,
            trait_definition_store: None,
            skill_line_store: None,
            skill_tiers_store: None,
            area_table_store: None,
            fishing_base_skill_store: None,
            #[cfg(test)]
            area_trigger_db2_store: None,
            #[cfg(test)]
            area_trigger_store: None,
            #[cfg(test)]
            area_trigger_script_store: None,
            #[cfg(test)]
            area_trigger_script_dispatcher_like_cpp: None,
            #[cfg(test)]
            give_player_xp_script_dispatcher_like_cpp: None,
            #[cfg(test)]
            driver_phase_trace_like_cpp: Vec::new(),
            #[cfg(test)]
            tavern_area_trigger_store: None,
            #[cfg(test)]
            graveyard_store: None,
            world_safe_loc_store_like_cpp: None,
            access_requirement_store: None,
            lfg_dungeons_store: None,
            #[cfg(test)]
            lfg_dungeon_store_like_cpp: None,
            #[cfg(test)]
            battlemaster_list_store: None,
            #[cfg(test)]
            represented_dungeon_difficulty_id_like_cpp: DIFFICULTY_NORMAL_LIKE_CPP,
            #[cfg(test)]
            represented_raid_difficulty_id_like_cpp: DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            #[cfg(test)]
            represented_legacy_raid_difficulty_id_like_cpp: DIFFICULTY_10_N_LIKE_CPP,
            #[cfg(test)]
            represented_player_recent_instances_like_cpp: HashMap::new(),
            friendship_rep_reaction_store: None,
            paragon_reputation_store: None,
            reputation_reward_rate_store: None,
            reputation_spillover_template_store: None,
            #[cfg(test)]
            championing_faction_like_cpp: 0,
            #[cfg(test)]
            creature_equipment_store_like_cpp: None,
            #[cfg(test)]
            creature_addon_store_like_cpp: None,
            #[cfg(test)]
            creature_difficulty_store_like_cpp: None,
            #[cfg(test)]
            creature_base_stats_store_like_cpp: None,
            #[cfg(test)]
            creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp::default(),
            mount_store: None,
            mount_definition_store_like_cpp: None,
            mount_capability_store: None,
            mount_type_x_capability_store: None,
            mount_x_display_store: None,
            vehicle_store: None,
            vehicle_seat_store: None,
            #[cfg(test)]
            vehicle_template_store: None,
            vehicle_accessory_store: None,
            terrain_swap_store: None,
            phase_store: None,
            phase_group_store: None,
            player_registry: None,
            game_event_quest_complete_tx: None,
            group_registry: None,
            pending_invites: None,
            #[cfg(test)]
            group_guid: None,
            #[cfg(test)]
            represented_subgroup_like_cpp: None,
            #[cfg(test)]
            represented_group_update_sequences_like_cpp: std::array::from_fn(|_| {
                Default::default()
            }),
            #[cfg(test)]
            pass_on_group_loot: false,
            #[cfg(test)]
            represented_enchanting_skill: 0,
            #[cfg(test)]
            player_skill_values_like_cpp: HashMap::new(),
            #[cfg(test)]
            player_skill_records_like_cpp: HashMap::new(),
            #[cfg(test)]
            player_skill_non_durable_tombstones_like_cpp: BTreeSet::new(),
            #[cfg(test)]
            player_skill_records_loaded_like_cpp: false,
            #[cfg(test)]
            player_skill_records_complete_like_cpp: false,
            #[cfg(test)]
            player_skill_occupied_slots_like_cpp: None,
            #[cfg(test)]
            represented_gray_level_script_overrides_like_cpp: HashMap::new(),
            realm_id: 1,
            realm_region: 1,
            realm_battlegroup: 1,
            realm_names_like_cpp: BTreeMap::from([(
                0x0101_0001,
                ("RustyCore".to_string(), "RustyCore".to_string()),
            )]),
            #[cfg(test)]
            guid_generator: None,
            #[cfg(test)]
            item_guid_generator_like_cpp: None,
            #[cfg(test)]
            equipment_set_guid_generator_like_cpp: None,
            #[cfg(test)]
            void_storage_item_id_generator_like_cpp: None,
            legit_characters: Vec::new(),
            pending_packets: VecDeque::new(),
            character_rename_callbacks: Default::default(),
            player_loading: None,
            player_login_claim_like_cpp: None,
            player_logout_like_cpp: false,
            finalization: None,
            session_mgr: None,
            time_sync_next_counter: 0,
            time_sync_timer_ms: 0,
            time_sync_pending_requests: HashMap::new(),
            time_sync_clock_delta_queue: VecDeque::with_capacity(6),
            time_sync_clock_delta: 0,
            logout_time: None,
            login_time: None,
            player_save_interval_ms_like_cpp: DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
            next_player_save_ms_like_cpp: DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
            pending_periodic_player_save_like_cpp: false,
            total_played_time: 0,
            level_played_time: 0,
            max_player_level_config_like_cpp: 80,
            max_primary_trade_skills_like_cpp:
                crate::profession::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP,
            is_pvp_realm_like_cpp: false,
            is_ffa_pvp_realm_like_cpp: false,
            max_recruit_a_friend_bonus_player_level_like_cpp: 85,
            max_recruit_a_friend_bonus_player_level_difference_like_cpp: 4,
            #[cfg(test)]
            represented_rest_bonus_xp_like_cpp: 0.0,
            #[cfg(test)]
            represented_rest_state_xp_like_cpp: REST_STATE_NORMAL_LIKE_CPP,
            #[cfg(test)]
            represented_rest_flag_mask_like_cpp: 0,
            #[cfg(test)]
            represented_rest_location_initialized_like_cpp: false,
            #[cfg(test)]
            represented_defer_rest_flag_sync_like_cpp: false,
            #[cfg(test)]
            represented_deferred_rest_flag_update_dirty_like_cpp: false,
            #[cfg(test)]
            represented_inn_area_trigger_id_like_cpp: 0,
            #[cfg(test)]
            represented_rest_time_secs_like_cpp: 0,
            #[cfg(test)]
            represented_loaded_player_flags_like_cpp: None,
            #[cfg(test)]
            represented_loaded_player_flags_ex_like_cpp: None,
            #[cfg(test)]
            represented_loaded_player_flags_applied_like_cpp: false,
            #[cfg(test)]
            rest_offline_wilderness_rate_like_cpp: 1.0,
            #[cfg(test)]
            rest_offline_tavern_or_city_rate_like_cpp: 1.0,
            #[cfg(test)]
            rest_ingame_rate_like_cpp: 1.0,
            #[cfg(test)]
            player_gold: 0,
            #[cfg(test)]
            represented_talent_reset_cost_like_cpp: 0,
            #[cfg(test)]
            represented_talent_reset_time_secs_like_cpp: 0,
            #[cfg(test)]
            player_bank_bag_slot_count_like_cpp: 0,
            #[cfg(test)]
            player_inventory_slot_count_like_cpp: INVENTORY_DEFAULT_SIZE,
            #[cfg(test)]
            player_character_points_like_cpp: 0,
            #[cfg(test)]
            represented_player_powers_like_cpp: empty_character_power_snapshot_like_cpp(),
            #[cfg(test)]
            represented_player_max_powers_like_cpp: empty_character_power_snapshot_like_cpp(),
            #[cfg(test)]
            represented_player_base_mana_like_cpp: 0,
            #[cfg(test)]
            represented_bank_bag_slot_flags_like_cpp: [0; 7],
            #[cfg(test)]
            represented_bank_item_moves_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_guild_bank_inventory_moves_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_guild_bank_list_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_guild_bank_money_moves_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_guild_bank_tab_actions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_auction_replicate_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_auction_place_bids_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_auction_remove_items_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_auction_sell_items_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_auto_unequip_offhand_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            player_xp: 0,
            #[cfg(test)]
            player_next_level_xp: 400,
            #[cfg(test)]
            player_xp_table: None,
            #[cfg(test)]
            exploration_base_xp_store: None,
            #[cfg(test)]
            exploration_xp_rate_like_cpp: 1.0,
            min_quest_scaled_xp_ratio_like_cpp: 0,
            #[cfg(test)]
            min_discovered_scaled_xp_ratio_like_cpp: 0,
            #[cfg(test)]
            selection_guid: None,
            player_guid: None,
            recent_player_guid_low_like_cpp: 0,
            #[cfg(test)]
            player_bootstrap_attached_like_cpp: false,
            account_data_like_cpp: default_account_data_like_cpp(),
            tutorials_like_cpp: [0; 8],
            tutorials_loaded_from_db_like_cpp: false,
            tutorials_loaded_coherently_like_cpp: false,
            tutorials_changed_like_cpp: false,
            pending_creature_spawn: None,
            pending_creature_kill_loot_like_cpp: Vec::new(),
            pending_creature_kill_rewards_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_creature_kill_events_like_cpp: Vec::new(),
            #[cfg(test)]
            inventory_items: HashMap::new(),
            #[cfg(test)]
            buyback_items: HashMap::new(),
            #[cfg(test)]
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            #[cfg(test)]
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            #[cfg(test)]
            current_buyback_slot: BUYBACK_SLOT_START,
            #[cfg(test)]
            represented_item_mod_reapply_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_item_bonus_actions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_item_bonus_state_like_cpp: RepresentedItemBonusStateLikeCpp::default(),
            #[cfg(test)]
            represented_item_set_effects_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_item_set_spell_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_item_set_aura_refresh_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_combat_stat_recalculations_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_titan_grip_penalty_actions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_avg_equipped_item_level_updates_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_guild_id_like_cpp: 0,
            #[cfg(test)]
            represented_guild_id_authority_complete_like_cpp: false,
            #[cfg(test)]
            represented_guild_id_invited_like_cpp: 0,
            #[cfg(test)]
            represented_guild_accept_invites_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_calendar_community_invites_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_calendar_add_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_calendar_remove_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_arena_team_id_invited_like_cpp: 0,
            #[cfg(test)]
            represented_wargame_invite_acceptances_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_active_trade_partner_like_cpp: None,
            #[cfg(test)]
            represented_trade_accepted_like_cpp: false,
            #[cfg(test)]
            represented_partner_trade_server_state_index_like_cpp: 0,
            #[cfg(test)]
            represented_trade_client_state_index_like_cpp: 1,
            #[cfg(test)]
            represented_trade_server_state_index_like_cpp: 1,
            #[cfg(test)]
            represented_trade_items_like_cpp: [None; TRADE_SLOT_COUNT_LIKE_CPP as usize],
            #[cfg(test)]
            represented_trade_money_like_cpp: 0,
            #[cfg(test)]
            represented_trade_spell_like_cpp: 0,
            #[cfg(test)]
            represented_trade_spell_cast_item_like_cpp: None,
            #[cfg(test)]
            represented_trade_cancel_statuses_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_sign_petitions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_decline_petitions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_query_petitions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_silence_party_talker_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_can_duel_spell_casts_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_duel_arbiter_guid_like_cpp: None,
            #[cfg(test)]
            represented_duel_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_force_deselects_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_duel_accepts_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_duel_cancels_like_cpp: Vec::new(),
            represented_guild_repair_bank_state_like_cpp: None,
            #[cfg(test)]
            represented_guild_repair_bank_withdraws_like_cpp: Vec::new(),
            #[cfg(test)]
            player_currencies: HashMap::new(),
            represented_quest_objective_progress_events_like_cpp: VecDeque::new(),
            represented_quest_objective_progress_draining_like_cpp: false,
            #[cfg(test)]
            inventory_item_objects: HashMap::new(),
            current_map_id: 0,
            player_race: 0,
            player_class: 0,
            player_level: 0,
            player_gender: 0,
            #[cfg(test)]
            player_create_mode_like_cpp: wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,
            #[cfg(test)]
            represented_shapeshift_form_like_cpp: 0,
            #[cfg(test)]
            loot_specialization_id: 0,
            #[cfg(test)]
            represented_primary_specialization_id_like_cpp: 0,
            #[cfg(test)]
            known_spells: Vec::new(),
            #[cfg(test)]
            represented_player_spell_rows_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_player_spell_rows_loaded_like_cpp: false,
            #[cfg(test)]
            represented_player_spell_rows_complete_like_cpp: false,
            #[cfg(test)]
            represented_fallback_player_spell_rows_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_dependent_known_spells_like_cpp: HashSet::new(),
            #[cfg(test)]
            represented_removed_known_spells_like_cpp: HashSet::new(),
            #[cfg(test)]
            represented_favorite_known_spells_like_cpp: HashSet::new(),
            #[cfg(test)]
            represented_spell_trait_definition_ids_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_spell_trait_definition_ids_complete_like_cpp: false,
            #[cfg(test)]
            represented_trait_config_rows_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_trait_config_rows_complete_like_cpp: false,
            #[cfg(test)]
            represented_trait_entry_rows_complete_like_cpp: false,
            #[cfg(test)]
            represented_trait_entry_rows_empty_like_cpp: false,
            #[cfg(test)]
            represented_spell_acquisition_post_commit_actions_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_weapon_proficiency_like_cpp: 0,
            #[cfg(test)]
            represented_armor_proficiency_like_cpp: 0,
            #[cfg(test)]
            account_mounts_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_spell_history_packets_like_cpp: (Vec::new(), Vec::new()),
            #[cfg(test)]
            cuf_profiles_like_cpp: vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP],
            #[cfg(test)]
            cuf_profiles_loaded_like_cpp: false,
            #[cfg(test)]
            player_position: None,
            #[cfg(test)]
            player_movement_flags_like_cpp: MovementFlag::NONE,
            #[cfg(test)]
            represented_can_swim_to_fly_transition_like_cpp: false,
            #[cfg(test)]
            represented_mover_fixed_position_vehicle_like_cpp: false,
            player_name: None,
            registered_addon_prefixes: Vec::new(),
            filter_addon_messages: false,
            creature_tick: 0,
            vendor_item_counts: HashMap::new(),
            #[cfg(test)]
            vendor_buy_item_test_override_like_cpp: None,
            map_manager: None,
            canonical_map_manager: None,
            player_handle_like_cpp: None,
            mmap_pathfinder_like_cpp: None,
            #[cfg(test)]
            combat_target: None,
            combat_tick_last_at_like_cpp: Instant::now(),
            player_swing_error_msg_like_cpp: None,
            #[cfg(test)]
            in_combat: false,
            #[cfg(test)]
            player_alive_like_cpp: true,
            #[cfg(test)]
            player_game_master_like_cpp: false,
            #[cfg(test)]
            player_cheat_god_like_cpp: false,
            #[cfg(test)]
            player_normal_damage_immune_like_cpp: false,
            #[cfg(test)]
            player_environmental_damage_immune_like_cpp: false,
            #[cfg(test)]
            player_health_like_cpp: 100,
            #[cfg(test)]
            player_max_health_like_cpp: 100,
            last_presented_creature_melee_health_state_revision_like_cpp: 0,
            #[cfg(test)]
            player_movement_time_like_cpp: 0,
            #[cfg(test)]
            player_movement_jump_like_cpp: wow_packet::packets::movement::JumpInfo::default(),
            #[cfg(test)]
            last_fall_time_like_cpp: 0,
            #[cfg(test)]
            last_fall_z_like_cpp: 0.0,
            #[cfg(test)]
            fall_damage_events_like_cpp: Vec::new(),
            #[cfg(test)]
            player_out_of_bounds_like_cpp: false,
            #[cfg(test)]
            under_map_damage_events_like_cpp: Vec::new(),
            #[cfg(test)]
            player_stand_state_like_cpp: UnitStandStateType::Stand,
            #[cfg(test)]
            represented_live_applications_like_cpp: Vec::new(),
            #[cfg(test)]
            player_emote_state_like_cpp: 0,
            #[cfg(test)]
            temporary_pet_unsummon_requests_like_cpp: 0,
            #[cfg(test)]
            movement_jump_proc_requests_like_cpp: 0,
            #[cfg(test)]
            active_player_local_flags_like_cpp: 0,
            #[cfg(test)]
            active_player_transport_server_time_like_cpp: 0,
            #[cfg(test)]
            active_player_multi_action_bars_like_cpp: 0,
            #[cfg(test)]
            represented_action_buttons_like_cpp: [0; wow_packet::packets::misc::MAX_ACTION_BUTTONS],
            #[cfg(test)]
            represented_action_buttons_loaded_like_cpp: false,
            advanced_combat_logging_enabled_like_cpp: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            player_moved_unit_guid_like_cpp: ObjectGuid::EMPTY,
            movement_visibility_refresh_requests_like_cpp: 0,
            #[cfg(test)]
            movement_ack_events_like_cpp: Vec::new(),
            #[cfg(test)]
            taxi_destinations_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_activate_taxi_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_alter_appearance_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_confirm_barbers_choice_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_confirm_respec_wipe_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_at_login_flags_like_cpp: 0,
            #[cfg(test)]
            represented_talent_reset_script_hooks_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_at_login_flag_removals_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_talent_respec_visual_spell_casts_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_talent_respec_criteria_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_equipment_sets_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_equipment_sets_loaded_like_cpp: false,
            #[cfg(test)]
            represented_void_storage_items_like_cpp: std::array::from_fn(|_| None),
            #[cfg(test)]
            represented_void_storage_loaded_like_cpp: false,
            #[cfg(test)]
            represented_adventure_map_start_quest_requests_like_cpp: Vec::new(),
            taxi_node_map_ids_like_cpp: HashMap::new(),
            #[cfg(test)]
            taxi_flight_state_like_cpp: None,
            #[cfg(test)]
            taxi_unit_flags_like_cpp: UnitFlags::empty(),
            #[cfg(test)]
            taxi_mounted_like_cpp: false,
            #[cfg(test)]
            player_mount_display_id_like_cpp: 0,
            #[cfg(test)]
            player_mount_vehicle_id_like_cpp: 0,
            #[cfg(test)]
            player_mount_vehicle_kit_like_cpp: None,
            #[cfg(test)]
            player_mount_vehicle_accessories_like_cpp: Vec::new(),
            #[cfg(test)]
            player_mount_vehicle_seat_count_like_cpp: 0,
            #[cfg(test)]
            player_mount_vehicle_usable_seat_count_like_cpp: 0,
            #[cfg(test)]
            player_vehicle_seat_flags_like_cpp: None,
            #[cfg(test)]
            player_vehicle_seat_id_like_cpp: None,
            #[cfg(test)]
            represented_vehicle_seat_change_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_vehicle_seat_spell_click_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_vehicle_enter_requests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_vehicle_dismiss_movements_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_vehicle_base_movements_like_cpp: Vec::new(),
            #[cfg(test)]
            player_battleground_type_id_like_cpp: None,
            #[cfg(test)]
            player_battleground_map_id_like_cpp: None,
            #[cfg(test)]
            represented_battleground_status_like_cpp: None,
            #[cfg(test)]
            represented_battleground_leave_requests_like_cpp: 0,
            #[cfg(test)]
            represented_battlemaster_hellos_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battlefield_lists_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battlemaster_joins_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battlemaster_join_arenas_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battlemaster_join_skirmishes_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battleground_queue_slots_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battlefield_ports_like_cpp: Vec::new(),
            #[cfg(test)]
            area_spirit_healer_guid_like_cpp: ObjectGuid::EMPTY,
            #[cfg(test)]
            represented_pet_guid_like_cpp: None,
            #[cfg(test)]
            represented_temporary_unsummoned_pet_number_like_cpp: 0,
            #[cfg(test)]
            represented_old_pet_spell_like_cpp: 0,
            #[cfg(test)]
            represented_pet_stable_like_cpp: PetStable::default(),
            #[cfg(test)]
            represented_character_pet_rows_empty_authority_complete_like_cpp: false,
            pet_load_query_holder_rows_like_cpp: lifecycle::PetLoadQueryHolderRowsLikeCpp::default(
            ),
            #[cfg(test)]
            represented_pet_created_by_spell_like_cpp: 0,
            #[cfg(test)]
            represented_pet_react_state_like_cpp:
                wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
            #[cfg(test)]
            represented_pet_command_state_like_cpp:
                wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
            #[cfg(test)]
            temporary_mount_pet_react_state_like_cpp: None,
            #[cfg(test)]
            mount_vehicle_create_requests_like_cpp: 0,
            #[cfg(test)]
            mount_vehicle_remove_requests_like_cpp: 0,
            #[cfg(test)]
            mount_cancel_expected_vehicle_aura_packets_like_cpp: 0,
            #[cfg(test)]
            mount_pet_control_disable_requests_like_cpp: 0,
            #[cfg(test)]
            mount_pet_control_enable_requests_like_cpp: 0,
            #[cfg(test)]
            mount_pet_resummon_requests_like_cpp: 0,
            #[cfg(test)]
            mount_collision_height_update_requests_like_cpp: 0,
            #[cfg(test)]
            movement_counter_like_cpp: 0,
            #[cfg(test)]
            player_collision_height_like_cpp: 1.0,
            #[cfg(test)]
            player_object_scale_like_cpp: 1.0,
            #[cfg(test)]
            player_scale_duration_like_cpp: 0,
            #[cfg(test)]
            player_unit_flags_like_cpp: UnitFlags::PLAYER_CONTROLLED,
            #[cfg(test)]
            player_faction_template_like_cpp: None,
            #[cfg(test)]
            player_mounted_like_cpp: false,
            #[cfg(test)]
            player_pvp_hostile_like_cpp: false,
            #[cfg(test)]
            player_pvp_enabled_like_cpp: false,
            #[cfg(test)]
            player_in_pvp_flag_like_cpp: false,
            #[cfg(test)]
            player_pvp_end_timer_like_cpp: None,
            #[cfg(test)]
            player_contested_pvp_timer_like_cpp: 0,
            #[cfg(test)]
            player_zone_id_like_cpp: 0,
            #[cfg(test)]
            player_area_id_like_cpp: 0,
            #[cfg(test)]
            player_zone_area_authority_complete_like_cpp: false,
            #[cfg(test)]
            player_spell_hit_aura_authority_tombstoned_like_cpp: false,
            #[cfg(test)]
            move_spline_done_taxi_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_can_delay_teleport_like_cpp: false,
            #[cfg(test)]
            represented_has_delayed_teleport_like_cpp: false,
            #[cfg(test)]
            near_teleport_pending_like_cpp: false,
            #[cfg(test)]
            represented_far_teleport_pending_like_cpp: false,
            #[cfg(test)]
            near_teleport_destination_like_cpp: None,
            #[cfg(test)]
            represented_delayed_teleport_like_cpp: None,
            #[cfg(test)]
            near_teleport_destination_zone_area_like_cpp: None,
            #[cfg(test)]
            represented_homebind_like_cpp: None,
            #[cfg(test)]
            represented_resurrection_request_like_cpp: None,
            #[cfg(test)]
            represented_delayed_resurrection_after_teleport_like_cpp: None,
            #[cfg(test)]
            represented_self_res_spells_like_cpp: BTreeSet::new(),
            #[cfg(test)]
            represented_override_spells_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_override_spells_complete_like_cpp: false,
            represented_cast_unstuck_enabled_like_cpp: true,
            #[cfg(test)]
            represented_death_timer_active_like_cpp: false,
            #[cfg(test)]
            move_teleport_ack_events_like_cpp: Vec::new(),
            #[cfg(test)]
            temporary_pet_resummon_requests_like_cpp: 0,
            #[cfg(test)]
            delayed_operations_processed_like_cpp: 0,
            #[cfg(test)]
            forced_speed_changes_like_cpp: [0; UnitMoveTypeLikeCpp::COUNT],
            #[cfg(test)]
            movement_speed_rates_like_cpp: [1.0; UnitMoveTypeLikeCpp::COUNT],
            #[cfg(test)]
            represented_pet_movement_speed_rates_like_cpp: [1.0; UnitMoveTypeLikeCpp::COUNT],
            #[cfg(test)]
            represented_pet_speed_propagations_like_cpp: 0,
            #[cfg(test)]
            player_on_transport_like_cpp: false,
            #[cfg(test)]
            movement_force_mod_magnitude_changes_like_cpp: 0,
            #[cfg(test)]
            movement_force_mod_magnitude_like_cpp: 1.0,
            #[cfg(test)]
            movement_speed_ack_events_like_cpp: Vec::new(),
            #[cfg(test)]
            visible_auras: HashMap::new(),
            #[cfg(test)]
            player_aura_authority_complete_like_cpp: false,
            #[cfg(test)]
            player_equipment_inventory_authority_complete_like_cpp: false,
            #[cfg(test)]
            canonical_threat_aura_snapshots_like_cpp: HashMap::new(),
            spell_acquisition_cast_authority_like_cpp: None,
            spell_acquisition_craft_authority_like_cpp: None,
            spell_script_exact_spell_ids_like_cpp: None,
            spell_script_all_rank_root_spell_ids_like_cpp: None,
            legacy_spell_script_spell_ids_like_cpp: None,
            spell_linked_rejected_trigger_spell_ids_like_cpp: None,
            talent_store: None,
            num_talents_at_level_store: None,
            #[cfg(test)]
            power_type_store: None,
            cinematic_sequences_store: None,
            movie_store: None,
            #[cfg(test)]
            represented_cinematic_like_cpp: None,
            #[cfg(test)]
            represented_cinematic_camera_ids_like_cpp: None,
            #[cfg(test)]
            represented_cinematic_camera_index_like_cpp: -1,
            #[cfg(test)]
            represented_cinematic_next_camera_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_cinematic_end_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_movie_like_cpp: None,
            #[cfg(test)]
            represented_movie_complete_events_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_support_enabled_like_cpp: true,
            #[cfg(test)]
            represented_support_tickets_enabled_like_cpp: false,
            #[cfg(test)]
            represented_support_bugs_enabled_like_cpp: false,
            #[cfg(test)]
            represented_support_complaints_enabled_like_cpp: false,
            #[cfg(test)]
            represented_support_suggestions_enabled_like_cpp: false,
            script_name_interner: None,
            #[cfg(test)]
            object_mgr_catalogs_like_cpp: None,
            gameobject_template_lifecycle_store_like_cpp: None,
            quest_poi_store_like_cpp: None,
            #[cfg(test)]
            player_quests: HashMap::new(),
            #[cfg(test)]
            rewarded_quests: std::collections::HashSet::new(),
            #[cfg(test)]
            player_quest_status_authority_complete_like_cpp: false,
            #[cfg(test)]
            represented_rewarded_quest_rows_like_cpp: BTreeSet::new(),
            #[cfg(test)]
            represented_completed_achievements_like_cpp: HashSet::new(),
            represented_instance_reset_times_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            daily_quests_completed_like_cpp: HashSet::new(),
            #[cfg(test)]
            df_quests_like_cpp: HashSet::new(),
            #[cfg(test)]
            weekly_quests_completed_like_cpp: HashSet::new(),
            #[cfg(test)]
            monthly_quests_completed_like_cpp: HashSet::new(),
            #[cfg(test)]
            last_daily_quest_time_like_cpp: 0,
            quest_low_level_hide_diff_like_cpp: 4,
            quest_high_level_hide_diff_like_cpp: 7,
            #[cfg(test)]
            seasonal_quests_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            seasonal_quest_changed_like_cpp: false,
            #[cfg(test)]
            represented_account_heirlooms_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_account_toys_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_item_appearances_like_cpp: HashSet::new(),
            #[cfg(test)]
            represented_item_appearance_blocks_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_temporary_item_appearances_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_favorite_item_appearances_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_transmog_illusions_like_cpp: HashSet::new(),
            #[cfg(test)]
            represented_battle_pets_like_cpp: HashMap::new(),
            battle_pet_account_attachment_like_cpp: None,
            #[cfg(test)]
            represented_battle_pet_journal_lock_like_cpp: false,
            #[cfg(test)]
            represented_battle_pet_slots_like_cpp: std::array::from_fn(|index| {
                RepresentedBattlePetSlotLikeCpp::locked_empty(index as u8)
            }),
            #[cfg(test)]
            represented_battle_pet_slots_authority_complete_like_cpp: false,
            #[cfg(test)]
            represented_summoned_battle_pet_guid_like_cpp: None,
            #[cfg(test)]
            represented_critter_guid_like_cpp: None,
            #[cfg(test)]
            represented_dismissed_critter_guids_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battle_pet_query_companions_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_battle_pet_cage_items_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battle_pet_xp_per_level_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_battle_pet_level_criteria_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battle_pet_active_level_criteria_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battle_pet_unique_owned_criteria_like_cpp: 0,
            #[cfg(test)]
            represented_battle_pet_learned_new_pet_criteria_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_battle_pet_data_updates_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_timed_quest_removals_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_reward_skill_updates_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_reward_spell_casts_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_reward_titles_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_known_titles_like_cpp: HashSet::new(),
            #[cfg(test)]
            represented_chosen_title_like_cpp: 0,
            #[cfg(test)]
            represented_quest_reward_talent_points_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_reward_mails_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_reward_reputations_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_completed_bits_like_cpp: BTreeSet::new(),
            #[cfg(test)]
            represented_explored_zones_like_cpp: [0; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
            #[cfg(test)]
            represented_reveal_world_map_overlay_criteria_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_area_zone_criteria_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_auto_accept_acknowledged_quests_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_pending_quest_sharing_like_cpp: None,
            #[cfg(test)]
            represented_quest_push_result_responses_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_quest_push_result_sender_mismatch_count_like_cpp: 0,
            #[cfg(test)]
            represented_quest_confirm_accepts_like_cpp: Vec::new(),
            represented_quest_complete_status_updates_like_cpp: Vec::new(),
            #[cfg(test)]
            represented_push_quest_to_party_outcomes_like_cpp: Vec::new(),
            #[cfg(test)]
            active_spell_cast: None,
            #[cfg(test)]
            represented_pending_spell_cast_request_like_cpp: None,
            #[cfg(test)]
            last_spell_cast_time: None,
            #[cfg(test)]
            last_spell_cast_time_per_spell: HashMap::new(),
            #[cfg(test)]
            represented_character_spell_cooldowns_like_cpp: HashMap::new(),
            #[cfg(test)]
            represented_character_spell_cooldowns_loaded_like_cpp: false,
            #[cfg(test)]
            represented_character_spell_charges_like_cpp: BTreeMap::new(),
            #[cfg(test)]
            represented_character_spell_charges_loaded_like_cpp: false,
            #[cfg(test)]
            represented_active_talent_group_like_cpp: 0,
            #[cfg(test)]
            represented_bonus_talent_groups_like_cpp: 0,
            #[cfg(test)]
            represented_talents_like_cpp: std::array::from_fn(|_| BTreeMap::new()),
            #[cfg(test)]
            represented_talents_loaded_like_cpp: false,
            #[cfg(test)]
            represented_glyphs_like_cpp: [[0;
                wow_packet::packets::misc::MAX_GLYPH_SLOT_INDEX_LIKE_CPP];
                MAX_SPECIALIZATIONS_LIKE_CPP],
            #[cfg(test)]
            represented_glyphs_loaded_like_cpp: false,
            loot_table: std::collections::HashMap::new(),
            represented_loot_cache_generations_like_cpp: std::collections::HashMap::new(),
            active_loot_guid: ObjectGuid::EMPTY,
            active_loot_view_owners: std::collections::HashSet::new(),
            active_loot_view_generations_like_cpp: std::collections::HashMap::new(),
            active_loot_view_authorities_like_cpp: std::collections::HashMap::new(),
            durable_item_loot_persistence_like_cpp:
                DurableItemLootPersistenceTrackerLikeCpp::default(),
            durable_loot_money_persistence_like_cpp: Arc::new(
                DurableLootMoneyPersistenceTrackerLikeCpp::default(),
            ),
            #[cfg(test)]
            module_registry_like_cpp: None,
            represented_loot_rolls: std::collections::HashMap::new(),
            #[cfg(test)]
            loot_money_persistence_test_result_like_cpp: None,
            #[cfg(test)]
            loot_item_store_test_grants_like_cpp: None,
            #[cfg(test)]
            loot_item_store_test_success_like_cpp: true,
            #[cfg(test)]
            loot_item_store_test_commit_gate_like_cpp: None,
            #[cfg(test)]
            represented_loot_roll_criteria_events: Vec::new(),
            #[cfg(test)]
            represented_gameobject_criteria_events: Vec::new(),
            #[cfg(test)]
            represented_transmog_criteria_events: Vec::new(),
            loot_drop_rates: LootDropRatesLikeCpp::default(),
            reputation_rates: ReputationRatesLikeCpp::default(),
            repair_cost_rate_like_cpp: 1.0,
            reset_schedule_like_cpp: wow_instances::ResetSchedule::default(),
            represented_offhand_check_at_spell_unlearn_like_cpp: true,
            vmap_indoor_check_like_cpp: false,
            #[cfg(test)]
            represented_is_outdoors_like_cpp: None,
            #[cfg(test)]
            reputation_mgr_like_cpp: ReputationMgrLikeCpp::new_like_cpp(),
            #[cfg(test)]
            watched_faction_index_like_cpp: -1,
            enable_ae_loot_like_cpp: false,
            #[cfg(test)]
            addon_channel_like_cpp: true,
            #[cfg(test)]
            chat_fake_message_preventing_like_cpp: false,
            #[cfg(test)]
            party_raid_warnings_like_cpp: false,
            #[cfg(test)]
            allow_gm_group_like_cpp: false,
            #[cfg(test)]
            allow_two_side_interaction_group_like_cpp: false,
            #[cfg(test)]
            party_level_req_like_cpp: 1,
            #[cfg(test)]
            chat_strict_link_checking_kick_like_cpp: false,
            #[cfg(test)]
            chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp::default(),
            #[cfg(test)]
            chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp::default(),
            #[cfg(test)]
            chat_flood_config_like_cpp: ChatFloodConfigLikeCpp::default(),
            chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp::default(); 2],
            mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp::default(),
            waypoint_path_resolver_like_cpp: None,
            represented_unique_gameobject_uses: std::collections::HashSet::new(),
            represented_gameobject_use_effects: Vec::new(),
            represented_gameobject_use_states: std::collections::BTreeMap::new(),
            pending_bind: None,
            represented_confirmed_pending_binds: Vec::new(),
            #[cfg(test)]
            represented_repop_at_graveyard_count: 0,
            represented_gameobject_tap_lists: std::collections::HashMap::new(),
            represented_locked_dungeon_encounters: std::collections::HashSet::new(),
            represented_personal_loot_money: std::collections::HashMap::new(),
            represented_personal_loot_owners: std::collections::HashSet::new(),
            client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp::default(),
            #[cfg(test)]
            loaded_player_customizations_like_cpp: Box::default(),
            client_visible_transports_like_cpp: std::collections::HashSet::new(),
            #[cfg(test)]
            player_transport_login_state_like_cpp: None,
            suppress_creature_movement_queued_at_or_before_like_cpp: None,
            represented_seer_guid_like_cpp: None,
            represented_dynamic_object_values_updates_delivered_like_cpp:
                std::collections::HashSet::new(),
            represented_gameobject_visual_despawns_delivered_like_cpp:
                std::collections::HashSet::new(),
            represented_capture_point_removed_delivered_like_cpp: std::collections::HashSet::new(),
            represented_gameobject_phase_shifts: std::collections::HashMap::new(),
            #[cfg(test)]
            represented_player_phase_shift: PhaseShift::default(),
            last_visibility_pos: None,
            #[cfg(test)]
            player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp::default(),
            #[cfg(test)]
            gossip_options: Vec::new(),
            active_area_trigger: None,
            #[cfg(test)]
            pending_teleport: None,
            creature_query_cache: std::collections::HashSet::new(),
            instance_lock_mgr: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn seed_represented_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.represented_runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }

    pub(crate) fn represented_urand_u32_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        self.represented_runtime_rng_like_cpp.gen_range(min..=max)
    }

    pub(crate) fn represented_runtime_subrng_like_cpp(&mut self) -> StdRng {
        StdRng::seed_from_u64(self.represented_runtime_rng_like_cpp.next_u64())
    }

    fn with_owned_void_storage_like_cpp<R>(
        &self,
        mut f: impl FnMut(&[Option<RepresentedVoidStorageItemLikeCpp>], bool) -> R,
    ) -> Option<R> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            f(&state.void_storage_items, state.void_storage_loaded)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(f(
                &self.represented_void_storage_items_like_cpp,
                self.represented_void_storage_loaded_like_cpp,
            ));
        }
        None
    }

    fn with_owned_void_storage_mut_like_cpp<R>(
        &mut self,
        mut f: impl FnMut(&mut Vec<Option<RepresentedVoidStorageItemLikeCpp>>, &mut bool) -> R,
    ) -> Option<R> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let state = player.gameplay_state_mut();
            if state.void_storage_items.len()
                != wow_entities::PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP
            {
                state.void_storage_items =
                    vec![None; wow_entities::PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP];
            }
            f(
                &mut state.void_storage_items,
                &mut state.void_storage_loaded,
            )
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut fixture = self.represented_void_storage_items_like_cpp.to_vec();
            let result = f(
                &mut fixture,
                &mut self.represented_void_storage_loaded_like_cpp,
            );
            self.represented_void_storage_items_like_cpp = fixture
                .try_into()
                .expect("void-storage fixture preserves its fixed slot count");
            return Some(result);
        }
        None
    }

    pub(crate) fn clear_represented_void_storage_like_cpp(&mut self) {
        let _ = self.with_owned_void_storage_mut_like_cpp(|items, loaded| {
            items.fill(None);
            *loaded = false;
        });
    }

    pub(crate) fn represented_void_storage_free_slots_like_cpp(&self) -> Option<usize> {
        self.with_owned_void_storage_like_cpp(|items, _| {
            items.iter().filter(|item| item.is_none()).count()
        })
    }

    pub(crate) fn represented_void_storage_next_free_slot_like_cpp(&self) -> Option<u8> {
        self.with_owned_void_storage_like_cpp(|items, _| {
            items
                .iter()
                .position(Option::is_none)
                .and_then(|slot| u8::try_from(slot).ok())
        })?
    }

    pub(crate) fn represented_void_storage_item_by_id_like_cpp(
        &self,
        item_id: u64,
    ) -> Option<(u8, RepresentedVoidStorageItemLikeCpp)> {
        self.with_owned_void_storage_like_cpp(|items, _| {
            items.iter().enumerate().find_map(|(slot, item)| {
                let item = item.as_ref()?;
                (item.item_id == item_id).then(|| {
                    (
                        u8::try_from(slot).expect("void-storage slot fits u8"),
                        item.clone(),
                    )
                })
            })
        })?
    }

    pub(crate) fn represented_void_storage_item_at_like_cpp(
        &self,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        self.with_owned_void_storage_like_cpp(|items, _| {
            items.get(usize::from(slot)).cloned().flatten()
        })?
    }

    pub(crate) fn add_represented_void_storage_item_like_cpp(
        &mut self,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> Option<u8> {
        let slot = self.represented_void_storage_next_free_slot_like_cpp()?;
        self.with_owned_void_storage_mut_like_cpp(|items, _| {
            items[usize::from(slot)] = Some(item.clone());
        })?;
        Some(slot)
    }

    pub(crate) fn delete_represented_void_storage_item_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<RepresentedVoidStorageItemLikeCpp> {
        self.with_owned_void_storage_mut_like_cpp(|items, _| {
            items.get_mut(usize::from(slot)).and_then(Option::take)
        })?
    }

    pub(crate) fn swap_represented_void_storage_item_like_cpp(
        &mut self,
        old_slot: u8,
        new_slot: u8,
    ) -> bool {
        let old_slot = usize::from(old_slot);
        let new_slot = usize::from(new_slot);
        if old_slot >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
            || new_slot >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
            || old_slot == new_slot
        {
            return false;
        }
        self.with_owned_void_storage_mut_like_cpp(|items, _| {
            items.swap(old_slot, new_slot);
        })
        .is_some()
    }

    pub(crate) fn next_represented_void_storage_item_id_with_generator_like_cpp(
        &self,
        generator: &VoidStorageItemIdGeneratorLikeCpp,
    ) -> u64 {
        generator.generate()
    }

    #[cfg(test)]
    pub(crate) fn next_represented_void_storage_item_id_like_cpp(&self) -> Option<u64> {
        self.void_storage_item_id_generator_like_cpp
            .as_deref()
            .map(|generator| {
                self.next_represented_void_storage_item_id_with_generator_like_cpp(generator)
            })
    }

    pub(crate) fn represented_void_storage_contents_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::void_storage::VoidStorageContents> {
        self.with_owned_void_storage_like_cpp(|stored, _| {
            let items = stored
                .iter()
                .enumerate()
                .filter_map(|(slot, item)| {
                    let item = item.as_ref()?;
                    Some(self.represented_void_storage_item_packet_like_cpp(slot as u8, item))
                })
                .collect();
            wow_packet::packets::void_storage::VoidStorageContents { items }
        })
    }

    fn find_free_backpack_slot_like_cpp(&self) -> Option<u8> {
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.resolved_player_inventory_slot_count_like_cpp()?)
            .min(INVENTORY_SLOT_ITEM_END);
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        (INVENTORY_SLOT_ITEM_START..inventory_end).find(|slot| !inventory_items.contains_key(slot))
    }

    pub(crate) fn auto_reply_msg_like_cpp(&self) -> Option<String> {
        self.canonical_player_snapshot_like_cpp(|player| {
            player
                .gameplay_state()
                .social
                .auto_reply_msg_like_cpp
                .clone()
        })
    }

    /// Build the initial canonical Player value before a generation-checked
    /// owner exists. This is construction input only: once a handle has been
    /// installed, callers must query or mutate that exact owner in place.
    fn build_initial_player_for_owner_like_cpp(
        &self,
        key: wow_map::MapKey,
        bootstrap_position: Option<Position>,
    ) -> Option<Player> {
        if self.player_handle_like_cpp.is_some() {
            return None;
        }

        let guid = self.player_guid()?;
        let position = bootstrap_position.or_else(|| self.player_position_like_cpp())?;
        let name = self.player_name_like_cpp()?;
        let mut player = Player::new(Some(u64::from(self.account_id)), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name(name);
        player
            .unit_mut()
            .world_mut()
            .set_map(key.map_id, key.instance_id)
            .ok()?;
        player.unit_mut().world_mut().relocate(position);
        // Before the first canonical owner exists production has no phase
        // authority to read. The pre-load bootstrap has always been the C++
        // default empty PhaseShift; tests may provide an explicit fixture.
        #[cfg(not(test))]
        let bootstrap_phase_shift = PhaseShift::default();
        #[cfg(test)]
        let bootstrap_phase_shift = self.represented_player_phase_shift.clone();
        *player.unit_mut().world_mut().phase_shift_mut() = bootstrap_phase_shift;
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.set_race_class_gender(
            self.player_race_like_cpp(),
            self.player_class_like_cpp(),
            gender_from_u8(self.player_gender_like_cpp()),
        );
        if let Some(faction_template) = self.player_faction_template_id_like_cpp() {
            player.unit_mut().set_faction(faction_template);
        }
        player.unit_mut().set_level(self.player_level_like_cpp());
        // Preserve the pre-load Rust bootstrap shape. The Character row later
        // hydrates these values directly into the generation-checked Player;
        // Session no longer stores a second production vital-state authority.
        #[cfg(not(test))]
        {
            player.unit_mut().set_max_health(100);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(100);
        }
        #[cfg(test)]
        {
            player
                .unit_mut()
                .set_max_health(u64::from(self.player_max_health_like_cpp.max(1)));
            if !self.player_alive_like_cpp || self.player_health_like_cpp == 0 {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
            } else {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
            }
            player
                .unit_mut()
                .set_health(u64::from(self.player_health_like_cpp));
        }
        #[cfg(test)]
        self.apply_represented_player_powers_to_canonical_like_cpp(&mut player);
        #[cfg(not(test))]
        {
            // This value is the pre-Character-row bootstrap only. Production
            // immediately hydrates the same owned Player through the setters
            // below; it is never used as a fallback for an unresolved handle.
            player.set_xp(0);
            player.set_next_level_xp(400);
            player.set_character_points_like_cpp(0);
        }
        #[cfg(test)]
        {
            player.set_xp(self.player_xp_like_cpp() as i32);
            player.set_next_level_xp(self.player_next_level_xp_like_cpp() as i32);
            player.set_character_points_like_cpp(self.player_character_points_like_cpp());
        }
        #[cfg(not(test))]
        player.set_scaling_player_level_delta_like_cpp(
            if self.player_level_like_cpp() < WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP {
                -1
            } else {
                0
            },
        );
        #[cfg(test)]
        player.set_scaling_player_level_delta_like_cpp(self.player_scaling_level_delta_like_cpp());
        #[cfg(not(test))]
        player.set_money(0);
        #[cfg(test)]
        player.set_money(self.player_gold_like_cpp());
        #[cfg(not(test))]
        {
            // Pre-character-row bootstrap only. Login hydrates both values
            // into this same owned Player before gameplay admission.
            player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);
            player.set_bank_bag_slot_count(0);
        }
        #[cfg(test)]
        {
            player.set_inventory_slot_count(self.player_inventory_slot_count_like_cpp);
            player.set_bank_bag_slot_count(self.player_bank_bag_slot_count_like_cpp);
        }
        for (category, party_type) in self
            .party_member_party_type_like_cpp()
            .into_iter()
            .enumerate()
        {
            let _ = player.set_party_type_like_cpp(category as u8, party_type);
        }
        #[cfg(test)]
        for (index, value) in self
            .represented_bank_bag_slot_flags_like_cpp
            .iter()
            .copied()
            .enumerate()
        {
            player.set_bank_bag_slot_flag_value_like_cpp(index, value);
        }
        #[cfg(not(test))]
        player.set_watched_faction_index_like_cpp(-1);
        #[cfg(test)]
        player.set_watched_faction_index_like_cpp(self.watched_faction_index_like_cpp);
        #[cfg(test)]
        for quest_bit in &self.represented_quest_completed_bits_like_cpp {
            player.set_quest_completed_bit_like_cpp(*quest_bit, true);
        }
        #[cfg(test)]
        player.set_explored_zones_blocks_like_cpp(&self.represented_explored_zones_like_cpp);
        #[cfg(test)]
        {
            player.gameplay_state_mut().world_local = wow_entities::PlayerWorldLocalState {
                zone_id: self.player_zone_id_like_cpp,
                area_id: self.player_area_id_like_cpp,
                zone_area_authority_complete: self.player_zone_area_authority_complete_like_cpp,
                pvp_hostile: self.player_pvp_hostile_like_cpp,
                pvp_end_timer: self.player_pvp_end_timer_like_cpp,
                contested_pvp_timer: self.player_contested_pvp_timer_like_cpp,
                is_outdoors: self.represented_is_outdoors_like_cpp,
            };
            player.gameplay_state_mut().vehicle_seat_flags =
                self.player_vehicle_seat_flags_like_cpp;
            player.gameplay_state_mut().vehicle_seat_id = self.player_vehicle_seat_id_like_cpp;
            player.gameplay_state_mut().active_local_flags =
                self.active_player_local_flags_like_cpp;
            player.gameplay_state_mut().active_transport_server_time =
                self.active_player_transport_server_time_like_cpp;
            player.gameplay_state_mut().multi_action_bars =
                self.active_player_multi_action_bars_like_cpp;
            player
                .unit_mut()
                .subsystems_mut()
                .control
                .set_moved_unit(Some(if self.player_moved_unit_guid_like_cpp.is_empty() {
                    guid
                } else {
                    self.player_moved_unit_guid_like_cpp
                }));
            player
                .unit_mut()
                .world_mut()
                .set_zone_and_area(self.player_zone_id_like_cpp, self.player_area_id_like_cpp);
            if self.player_pvp_enabled_like_cpp {
                player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
            }
            if self.player_in_pvp_flag_like_cpp {
                player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
            }
        }
        #[cfg(not(test))]
        player
            .unit_mut()
            .subsystems_mut()
            .control
            .set_moved_unit(Some(guid));
        #[cfg(test)]
        player.set_game_master_like_cpp(self.player_game_master_like_cpp);
        #[cfg(test)]
        {
            player.set_cheat_god_like_cpp(self.player_cheat_god_like_cpp);
            player.set_normal_damage_immune_like_cpp(self.player_normal_damage_immune_like_cpp);
            player.set_environmental_damage_immune_like_cpp(
                self.player_environmental_damage_immune_like_cpp,
            );
            *player.resurrection_state_mut_like_cpp() = PlayerResurrectionStateLikeCpp {
                request: self.represented_resurrection_request_like_cpp,
                delayed_after_teleport: self
                    .represented_delayed_resurrection_after_teleport_like_cpp,
                self_res_spells: self.represented_self_res_spells_like_cpp.clone(),
                death_timer_active: self.represented_death_timer_active_like_cpp,
                area_spirit_healer_guid: self.area_spirit_healer_guid_like_cpp,
            };
            *player.teleport_state_mut_like_cpp() = PlayerTeleportStateLikeCpp {
                recovery: Default::default(),
                far_destination: self.pending_teleport,
                post_add: None,
                can_delay: self.represented_can_delay_teleport_like_cpp,
                has_delayed: self.represented_has_delayed_teleport_like_cpp,
                near_pending: self.near_teleport_pending_like_cpp,
                far_pending: self.represented_far_teleport_pending_like_cpp,
                near_destination: self.near_teleport_destination_like_cpp,
                delayed: self.represented_delayed_teleport_like_cpp,
                near_destination_zone_area: self.near_teleport_destination_zone_area_like_cpp,
            };
            *player.pet_lifecycle_state_mut_like_cpp() = PlayerPetLifecycleStateLikeCpp {
                stable: self.represented_pet_stable_like_cpp.clone(),
                character_rows_empty_authority_complete: self
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                temporary_unsummoned_pet_number: self
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                old_pet_spell: self.represented_old_pet_spell_like_cpp,
                temporary_mount_react_state: self.temporary_mount_pet_react_state_like_cpp,
            };
        }
        crate::canonical_player_sync::hydrate_player_presentation_like_cpp(self, &mut player)?;
        #[cfg(test)]
        {
            player.set_create_mode_like_cpp(self.player_create_mode_like_cpp);
            player.set_shapeshift_form_id_like_cpp(self.represented_shapeshift_form_like_cpp);
            player.set_loot_specialization_id_like_cpp(self.loot_specialization_id);
            player.set_primary_specialization(self.represented_primary_specialization_id_like_cpp);
            player.replace_spell_runtime_like_cpp(canonical_player_spell_runtime_like_cpp(
                RepresentedPlayerSpellRuntimeLikeCpp {
                    known_spells: self.known_spells.clone(),
                    rows: self.represented_player_spell_rows_like_cpp.clone(),
                    rows_loaded: self.represented_player_spell_rows_loaded_like_cpp,
                    rows_complete: self.represented_player_spell_rows_complete_like_cpp,
                    fallback_rows: self.represented_fallback_player_spell_rows_like_cpp.clone(),
                    dependent_known_spells: self
                        .represented_dependent_known_spells_like_cpp
                        .clone(),
                    removed_known_spells: self.represented_removed_known_spells_like_cpp.clone(),
                    favorite_known_spells: self.represented_favorite_known_spells_like_cpp.clone(),
                    trait_definition_ids: self
                        .represented_spell_trait_definition_ids_like_cpp
                        .clone(),
                    trait_definition_ids_complete: self
                        .represented_spell_trait_definition_ids_complete_like_cpp,
                    trait_config_rows: self.represented_trait_config_rows_like_cpp.clone(),
                    trait_config_rows_complete: self
                        .represented_trait_config_rows_complete_like_cpp,
                    trait_entry_rows_complete: self.represented_trait_entry_rows_complete_like_cpp,
                    trait_entry_rows_empty: self.represented_trait_entry_rows_empty_like_cpp,
                    override_spells: self.represented_override_spells_like_cpp.clone(),
                    override_spells_complete: self.represented_override_spells_complete_like_cpp,
                },
            ));
            player.gameplay_state_mut().cuf_profiles = self
                .cuf_profiles_like_cpp
                .iter()
                .map(|profile| profile.clone().map(player_cuf_profile_from_packet_like_cpp))
                .collect();
            player.gameplay_state_mut().cuf_profiles_loaded = self.cuf_profiles_loaded_like_cpp;
            player.gameplay_state_mut().equipment_sets =
                self.represented_equipment_sets_like_cpp.clone();
            player.gameplay_state_mut().equipment_sets_loaded =
                self.represented_equipment_sets_loaded_like_cpp;
            player.gameplay_state_mut().void_storage_items =
                self.represented_void_storage_items_like_cpp.to_vec();
            player.gameplay_state_mut().void_storage_loaded =
                self.represented_void_storage_loaded_like_cpp;
            player.gameplay_state_mut().collections = wow_entities::PlayerCollectionStateLikeCpp {
                mounts: self.account_mounts_like_cpp.clone(),
                heirlooms: self.represented_account_heirlooms_like_cpp.clone(),
                toys: self.represented_account_toys_like_cpp.clone(),
                item_appearances: self.represented_item_appearances_like_cpp.clone(),
                item_appearance_blocks: self.represented_item_appearance_blocks_like_cpp.clone(),
                temporary_item_appearances: self
                    .represented_temporary_item_appearances_like_cpp
                    .clone(),
                favorite_item_appearances: self
                    .represented_favorite_item_appearances_like_cpp
                    .clone(),
                transmog_illusions: self.represented_transmog_illusions_like_cpp.clone(),
            };
        }
        player
            .unit_mut()
            .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
        #[cfg(not(test))]
        player
            .unit_mut()
            .set_unit_flags_like_cpp(UnitFlags::PLAYER_CONTROLLED);
        #[cfg(test)]
        player
            .unit_mut()
            .set_unit_flags_like_cpp(self.player_unit_flags_like_cpp);
        if let Some(selection) = self.selection_guid_like_cpp() {
            player.set_selection(selection);
        }
        self.apply_represented_player_unit_shape_to_canonical_like_cpp(&mut player);
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .set_spell_hit_aura_authority_inert_like_cpp(
                self.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
            );
        Some(player)
    }

    #[cfg(test)]
    fn apply_represented_player_powers_to_canonical_like_cpp(&self, player: &mut Player) {
        let powers = self
            .represented_player_powers_like_cpp
            .map(|value| value.unwrap_or(0));
        let max_powers = self
            .represented_player_max_powers_like_cpp
            .map(|value| value.unwrap_or(0));
        player
            .unit_mut()
            .replace_create_power_arrays_like_cpp(powers, max_powers);
        let Some(current) = self.represented_player_powers_like_cpp[0] else {
            return;
        };
        let primary_power_type =
            primary_power_type_for_player_class_like_cpp(self.player_class_like_cpp());
        for raw_power in 0..=25 {
            player.set_power_index(power_type_from_u8_like_cpp(raw_power), None);
        }
        player.set_power_index(primary_power_type, Some(0));
        player.unit_mut().set_display_power(primary_power_type);
        player
            .unit_mut()
            .set_create_mana_like_cpp(self.represented_player_base_mana_like_cpp.max(0));
        if let Some(max) = self.represented_player_max_powers_like_cpp[0] {
            player.unit_mut().set_max_power(primary_power_type, max);
            player.unit_mut().set_power(primary_power_type, current);
        }
    }

    fn apply_represented_player_unit_shape_to_canonical_like_cpp(&self, player: &mut Player) {
        let display_id = crate::handlers::character::default_display_id(
            self.player_race_like_cpp(),
            self.player_gender_like_cpp(),
        );
        #[cfg(not(test))]
        let mount_display_id = 0;
        #[cfg(test)]
        let mount_display_id = u32::try_from(self.player_mount_display_id_like_cpp).unwrap_or(0);
        let unit = player.unit_mut();
        unit.set_display_id(display_id, true);
        unit.set_mount_display_id(mount_display_id);
        #[cfg(not(test))]
        unit.set_collision_height_like_cpp(1.0);
        #[cfg(test)]
        unit.set_collision_height_like_cpp(self.player_collision_height_like_cpp);
        #[cfg(not(test))]
        unit.world_mut().object_mut().set_scale(1.0);
        #[cfg(test)]
        unit.world_mut()
            .object_mut()
            .set_scale(self.player_object_scale_like_cpp);
    }

    fn player_can_never_see_target_like_cpp(&self) -> bool {
        self.active_player_update_state_like_cpp()
            .map(|(flags, _, _)| {
                flags & PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP == 0
            })
            .unwrap_or(true)
    }

    pub(crate) fn set_canonical_chosen_title_like_cpp(
        &mut self,
        title_id: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        self.mutate_canonical_player_like_cpp(|player| {
            player.set_chosen_title_like_cpp(title_id);
            player.values_update(true)
        })
    }

    fn player_world_local_state_like_cpp(&self) -> Option<wow_entities::PlayerWorldLocalState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().world_local);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerWorldLocalState {
                zone_id: self.player_zone_id_like_cpp,
                area_id: self.player_area_id_like_cpp,
                zone_area_authority_complete: self.player_zone_area_authority_complete_like_cpp,
                pvp_hostile: self.player_pvp_hostile_like_cpp,
                pvp_end_timer: self.player_pvp_end_timer_like_cpp,
                contested_pvp_timer: self.player_contested_pvp_timer_like_cpp,
                is_outdoors: self.represented_is_outdoors_like_cpp,
            });
        }
        canonical
    }

    fn player_war_mode_local_active_like_cpp(&self) -> bool {
        self.active_player_update_state_like_cpp()
            .is_some_and(|(flags, _, _)| flags & PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP != 0)
    }

    pub(crate) fn player_is_possessing_like_cpp(&self) -> bool {
        let Some(player_guid) = self.player_guid else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };

        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_some() {
                return;
            }

            let map = managed.map();
            let Some(player) = map.get_typed_player(player_guid) else {
                return;
            };
            let Some(charmed_guid) = player.unit().subsystems().control.charmed_guid else {
                result = Some(false);
                return;
            };

            let target_possessed_by_player = map
                .get_typed_player(charmed_guid)
                .map(|target| {
                    let control = &target.unit().subsystems().control;
                    control.charmer_guid == Some(player_guid) && control.is_possessed()
                })
                .or_else(|| {
                    map.with_creature_like_cpp(charmed_guid, |target| {
                        let control = &target.unit().subsystems().control;
                        control.charmer_guid == Some(player_guid) && control.is_possessed()
                    })
                })
                .unwrap_or(false);

            result = Some(target_possessed_by_player);
        });

        result.unwrap_or(false)
    }

    pub(crate) fn represented_player_charmed_guid_like_cpp(&self) -> ObjectGuid {
        let Some(player_guid) = self.player_guid else {
            return ObjectGuid::EMPTY;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return ObjectGuid::EMPTY;
        };
        let Ok(manager) = manager.lock() else {
            return ObjectGuid::EMPTY;
        };

        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_some() {
                return;
            }

            let Some(player) = managed.map().get_typed_player(player_guid) else {
                return;
            };
            result = Some(
                player
                    .unit()
                    .subsystems()
                    .control
                    .charmed_guid
                    .unwrap_or(ObjectGuid::EMPTY),
            );
        });

        result.unwrap_or(ObjectGuid::EMPTY)
    }

    /// C++ fishing-hole release performs AddUse, MaxOpens comparison, and
    /// SetLootState on one world thread. Keep all three under one map lock so
    /// two concurrent personal releases cannot finish in `Ready` after max.
    pub(crate) fn release_canonical_fishing_hole_like_cpp(
        &mut self,
        guid: ObjectGuid,
        max_opens: Option<u32>,
    ) -> Option<(
        u32,
        wow_entities::LootState,
        wow_map::map::GameObjectSetLootStateOutcomeLikeCpp,
    )> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let map = managed.map_mut();
        let use_count = {
            let gameobject = map.get_typed_game_object_mut(guid)?;
            gameobject.add_use_like_cpp();
            gameobject.use_times()
        };
        let loot_state = if max_opens.is_some_and(|max_opens| use_count >= max_opens) {
            wow_entities::LootState::JustDeactivated
        } else {
            wow_entities::LootState::Ready
        };
        let outcome = map.set_gameobject_loot_state_like_cpp(
            guid,
            loot_state,
            None,
            game_time_secs,
            0,
            false,
        );
        Some((use_count, loot_state, outcome))
    }

    pub fn summon_private_object_owner_like_cpp(
        &self,
        caster_guid: ObjectGuid,
        caster_private_object_owner: ObjectGuid,
        properties: &SummonPropertiesEntry,
    ) -> ObjectGuid {
        let flags = u32::try_from(properties.flags[0]).unwrap_or(0);
        let only_visible_to_summoner =
            flags & SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP != 0;
        let only_visible_to_summoner_group =
            flags & SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP != 0;
        if !only_visible_to_summoner && !only_visible_to_summoner_group {
            return ObjectGuid::EMPTY;
        }

        if !caster_private_object_owner.is_empty() {
            return caster_private_object_owner;
        }

        if only_visible_to_summoner_group && caster_guid.is_player() {
            if let Some(group_guid) = self.resolved_group_guid_like_cpp() {
                return ObjectGuid::create_group(group_guid);
            }
        }

        caster_guid
    }

    #[inline(never)]
    fn initial_player_box_like_cpp(&self, key: wow_map::MapKey) -> Option<Box<Player>> {
        Some(Box::new(
            self.build_initial_player_for_owner_like_cpp(key, None)?,
        ))
    }

    fn access_requirement_abort_like_cpp(
        &self,
        map_id: u32,
        requested_difficulty: u8,
    ) -> Option<(u32, u8, i32)> {
        let downscaled_entries = self
            .create_map_db2_entries_like_cpp(map_id, requested_difficulty as wow_map::Difficulty)?;
        let map_difficulty_id = self
            .maps
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
            .map(|entry| entry.id)
            .unwrap_or(0);
        let map_difficulty_has_message = self
            .maps
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
            .map(|entry| !entry.message.is_empty())
            .unwrap_or(false);

        let failed_map_difficulty_x_condition =
            if self.instance_ignore_level_like_cpp || map_difficulty_id == 0 {
                0
            } else {
                self.maps
                    .difficulty_x_condition_store
                    .as_ref()
                    .zip(self.player_condition_store.as_ref())
                    .and_then(|(difficulty_conditions, player_conditions)| {
                        difficulty_conditions.failed_condition_like_cpp(
                            map_difficulty_id,
                            player_conditions,
                            |condition| {
                                self.represented_player_condition_context_like_cpp()
                                    .as_ref()
                                    .and_then(|context| context.as_context(self))
                                    .is_some_and(|context| {
                                        is_player_meeting_condition_like_cpp(condition, &context)
                                    })
                            },
                        )
                    })
                    .unwrap_or(0)
            };

        let access_requirement = self
            .access_requirement_store
            .as_ref()
            .and_then(|store| store.get(map_id, requested_difficulty));

        let mut level_min = 0;
        let mut level_max = 0;
        let mut missing_item = 0;
        let mut missing_quest = 0;
        let mut missing_achievement = 0;

        if let Some(access_requirement) = access_requirement {
            if !self.instance_ignore_level_like_cpp {
                if access_requirement.level_min != 0
                    && self.player_level_like_cpp() < access_requirement.level_min
                {
                    level_min = access_requirement.level_min;
                }
                if access_requirement.level_max != 0
                    && self.player_level_like_cpp() > access_requirement.level_max
                {
                    level_max = access_requirement.level_max;
                }
            }

            let item_counts = self.represented_inventory_item_counts_like_cpp()?;
            if access_requirement.item != 0 {
                if item_counts
                    .get(&access_requirement.item)
                    .copied()
                    .unwrap_or(0)
                    == 0
                    && (access_requirement.item2 == 0
                        || item_counts
                            .get(&access_requirement.item2)
                            .copied()
                            .unwrap_or(0)
                            == 0)
                {
                    missing_item = access_requirement.item;
                }
            } else if access_requirement.item2 != 0
                && item_counts
                    .get(&access_requirement.item2)
                    .copied()
                    .unwrap_or(0)
                    == 0
            {
                missing_item = access_requirement.item2;
            }

            let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
            match player_team_for_race_cpp(self.player_race_like_cpp()) {
                Team::Alliance
                    if access_requirement.quest_done_a != 0
                        && !quests
                            .rewarded_quest_ids
                            .contains(&access_requirement.quest_done_a) =>
                {
                    missing_quest = access_requirement.quest_done_a;
                }
                Team::Horde
                    if access_requirement.quest_done_h != 0
                        && !quests
                            .rewarded_quest_ids
                            .contains(&access_requirement.quest_done_h) =>
                {
                    missing_quest = access_requirement.quest_done_h;
                }
                _ => {}
            }

            if access_requirement.completed_achievement != 0
                && !self.access_requirement_leader_has_achievement_like_cpp(
                    access_requirement.completed_achievement,
                )
            {
                missing_achievement = access_requirement.completed_achievement;
            }
        }

        if level_min != 0
            || level_max != 0
            || failed_map_difficulty_x_condition != 0
            || missing_item != 0
            || missing_quest != 0
            || missing_achievement != 0
        {
            if missing_quest != 0
                && let Some(access_requirement) = access_requirement
                && !access_requirement.quest_failed_text.is_empty()
            {
                self.send_system_message_like_cpp(&access_requirement.quest_failed_text);
            } else if map_difficulty_has_message || failed_map_difficulty_x_condition != 0 {
                return Some((
                    TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
                    requested_difficulty,
                    failed_map_difficulty_x_condition as i32,
                ));
            } else if missing_item != 0 {
                let level_min_text = level_min.to_string();
                let item_name = self.item_template_name_like_cpp(missing_item);
                let notify_text = trinity_sprintf_like_cpp(
                    self.trinity_string_like_cpp(
                        wow_data::LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP,
                    ),
                    &[level_min_text.as_str(), item_name],
                );
                self.send_notification_like_cpp(notify_text);
            } else if level_min != 0 {
                let level_min_text = level_min.to_string();
                let notify_text = trinity_sprintf_like_cpp(
                    self.trinity_string_like_cpp(wow_data::LANG_LEVEL_MINREQUIRED_LIKE_CPP),
                    &[level_min_text.as_str()],
                );
                self.send_notification_like_cpp(notify_text);
            }
            return Some((TRANSFER_ABORT_ERROR_LIKE_CPP, 0, 0));
        }

        None
    }

    pub(crate) fn represented_gameobject_questgiver_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        // C++ anchor: Player::CanInteractWithQuestGiver(TYPEID_GAMEOBJECT)
        // delegates to GetGameObjectIfCanInteractWith(guid, GAMEOBJECT_TYPE_QUESTGIVER).
        // This represented guard consumes canonical map access plus locally recorded
        // template type/radius from the GO-use path. The C++ IconName == "Point"
        // rejection is represented earlier in handle_game_obj_use before runtime state
        // is registered/consumed here; standalone paths without represented type state
        // fail closed instead of treating canonical existence as interactability.
        let access = self.canonical_gameobject_access_like_cpp(guid)?;
        let state = self.represented_gameobject_use_states.get(&guid)?;
        if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) {
            return None;
        }
        let player_position = self.player_position_like_cpp()?;
        let interaction_distance = state
            .interact_radius_override
            .filter(|value| *value != 0)
            .map_or(5.5555553, |override_hundredths| {
                override_hundredths as f32 / 100.0
            });
        access
            .position
            .is_within_dist(&player_position, interaction_distance)
            .then_some(access)
    }

    fn represented_has_quest_for_gameobject_like_cpp(&self, gameobject_entry: u32) -> bool {
        let Some(store) = self.quests.store.as_ref() else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let object_id = i32::try_from(gameobject_entry).unwrap_or(i32::MAX);
        quests.statuses.values().any(|status| {
            if status.status != crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }
            let Some(quest) = store.get(status.quest_id) else {
                return false;
            };
            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(index, objective)| {
                    objective.obj_type == 2
                        && objective.object_id == object_id
                        && crate::handlers::quest_rules::represented_quest_objective_completable_like_cpp(
                            status, quest, index,
                        )
                        && !crate::handlers::quest_rules::represented_quest_objective_complete_like_cpp(
                            status, quest, objective,
                        )
                })
        })
    }

    pub(crate) fn record_represented_gameobject_template_quest_source_like_cpp(
        &mut self,
        guid: ObjectGuid,
        template: &wow_entities::GameObjectTemplateData,
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        if let Some(source) = template.chest_loot_source_like_cpp() {
            state.chest_loot_source = Some(source);
        }
        if let Some(source) = template.gathering_node_use_source_like_cpp() {
            state.gathering_node_loot_id = Some(source.loot_id);
        }
        state.condition_id1 = (template.get_condition_id1_like_cpp() != 0)
            .then_some(template.get_condition_id1_like_cpp());
    }

    fn represented_gameobject_is_for_quests_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => true,
            // C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:8791-8800
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry)
                    || state
                        .chest_loot_source
                        .is_some_and(|source| source.chest_quest_id != 0)
                    || state.chest_loot_source.is_some_and(|source| {
                        self.represented_gameobject_loot_ids_have_quest_loot_like_cpp(
                            crate::session_rules::represented_gameobject_chest_loot_ids_like_cpp(
                                source,
                            ),
                        )
                    })
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry)
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => state
                .goober_use_source
                .is_some_and(|source| source.quest_id != 0),
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => self
                .represented_gameobject_loot_ids_have_quest_loot_like_cpp(
                    state.gathering_node_loot_id,
                ),
            _ => false,
        }
    }

    fn represented_gameobject_activate_to_quest_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> bool {
        if self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry) {
            return true;
        }

        if !self.represented_gameobject_is_for_quests_like_cpp(gameobject_entry, state) {
            return false;
        }

        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => {
                let status = self.get_represented_quest_giver_status_like_cpp(
                    crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                        entry: gameobject_entry,
                    },
                );
                status != wow_packet::packets::quest::quest_giver_status::NONE
                    && status != wow_packet::packets::quest::quest_giver_status::FUTURE
            }
            // C++ anchor: /home/server/woltk-trinity-legacy/src/server/game/Entities/GameObject/GameObject.cpp:2236-2251
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                state.loot_state != Some(wow_entities::LootState::NotReady)
                    && (state.chest_loot_source.is_some_and(|source| {
                        source.chest_quest_id != 0
                            && self
                                .represented_player_quest_status_like_cpp(source.chest_quest_id)
                                .is_some_and(|status| {
                                    status
                                        == Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                                })
                    }) || state.chest_loot_source.is_some_and(|source| {
                        self.represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
                            crate::session_rules::represented_gameobject_chest_loot_ids_like_cpp(
                                source,
                            ),
                        )
                    }))
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                self.represented_has_quest_for_gameobject_like_cpp(gameobject_entry)
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => {
                state
                    .goober_use_source
                    .is_some_and(|source| source.quest_id != 0)
                    && state.goober_use_source.is_some_and(|source| {
                        self.represented_player_quest_status_like_cpp(source.quest_id)
                            .is_some_and(|status| {
                                status == Some(crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
                            })
                    })
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => self
                .represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
                    state.gathering_node_loot_id,
                ),
            _ => false,
        }
    }

    pub(crate) fn represented_npc_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        npc_flags: u32,
        npc_flags2: u32,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        if guid.is_empty() || !guid.is_any_type_creature() {
            return None;
        }
        let player_guid = self.player_guid()?;
        let player_position = self.player_position_like_cpp()?;
        let target_player_contested_pvp = self
            .canonical_player_has_player_flag_like_cpp(
                player_guid,
                PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
            )
            .unwrap_or(false);
        let player_faction_template_id = self.player_faction_template_id_like_cpp();
        let player_interaction_combat_reach = self.player_interaction_combat_reach_like_cpp();
        if self.resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return None;
        }

        let player_map_key = self
            .current_canonical_player_map_key_like_cpp()
            .unwrap_or_else(|| wow_map::MapKey::new(u32::from(self.player_map_id_like_cpp()), 0));
        let mut canonical_record_found_like_cpp = false;
        let mut canonical_fail_closed_like_cpp = false;
        let mut canonical_reaction_input_like_cpp = None;
        let canonical_access = (|| {
            let manager = self.canonical_map_manager.as_ref()?;
            let Ok(manager) = manager.lock() else {
                return None;
            };
            let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
            let target_player_faction_template_id = player_faction_template_id.or_else(|| {
                map.map()
                    .get_typed_player(player_guid)
                    .and_then(|player| u32::try_from(player.unit().data().faction_template).ok())
                    .filter(|faction| *faction != 0)
            });
            let player_is_alive = if let Some(canonical_player) =
                map.map().get_typed_player(player_guid)
            {
                if !canonical_player.unit().world().object().is_in_world() {
                    canonical_fail_closed_like_cpp = true;
                    return None;
                }
                #[cfg(test)]
                let is_alive = if canonical_player.unit().data().max_health == 0
                    && self.player_handle_like_cpp.is_none()
                {
                    self.player_alive_like_cpp && self.player_health_like_cpp > 0
                } else {
                    canonical_player.unit().is_alive() && canonical_player.unit().data().health > 0
                };
                #[cfg(not(test))]
                let is_alive =
                    canonical_player.unit().is_alive() && canonical_player.unit().data().health > 0;
                is_alive
            } else {
                #[cfg(test)]
                {
                    if self.player_handle_like_cpp.is_some() {
                        canonical_fail_closed_like_cpp = true;
                        return None;
                    }
                    self.player_alive_like_cpp && self.player_health_like_cpp > 0
                }
                #[cfg(not(test))]
                {
                    canonical_fail_closed_like_cpp = true;
                    return None;
                }
            };
            canonical_record_found_like_cpp = map.map().contains_map_object_like_cpp(guid);
            map.map()
                .with_creature_or_pet_like_cpp(guid, |creature, _| {
                    let type_flags = CreatureTypeFlags::from_bits_retain(
                        creature.lifecycle_metadata().type_flags,
                    );
                    if !player_is_alive
                        && !type_flags.contains(CreatureTypeFlags::VISIBLE_TO_GHOSTS)
                    {
                        return None;
                    }
                    if !creature.is_alive()
                        && !type_flags.contains(CreatureTypeFlags::INTERACT_WHILE_DEAD)
                    {
                        return None;
                    }
                    if (npc_flags != 0 || npc_flags2 != 0)
                        && (creature.ai_ownership().npc_flags & npc_flags) == 0
                        && (creature.ai_ownership().npc_flags2 & npc_flags2) == 0
                    {
                        return None;
                    }
                    if creature.unit().subsystems().control.charmer_guid.is_some() {
                        return None;
                    }
                    let unit_flags2 = creature.unit().unit_flags2_like_cpp();
                    if !unit_flags2.contains(UnitFlags2::INTERACT_WHILE_HOSTILE) {
                        canonical_reaction_input_like_cpp =
                            Some(RepresentedGetReactionInputLikeCpp {
                                self_faction_template_id: creature
                                    .unit()
                                    .data()
                                    .faction_template
                                    .max(0)
                                    as u32,
                                target_faction_template_id: target_player_faction_template_id?,
                                same_object: false,
                                attackable_by_summoner: false,
                                same_charmer_or_owner_or_self: false,
                                self_has_player_owner: false,
                                target_has_player_owner: true,
                                target_player_owner_is_current_session: true,
                                target_owner_forced_rank_for_self: None,
                                same_player_owner: false,
                                duel_in_progress: false,
                                same_raid: false,
                                self_unit_player_controlled: false,
                                target_unit_player_controlled: true,
                                self_ffa_pvp: false,
                                target_ffa_pvp: false,
                                self_ignores_reputation: false,
                                target_ignores_reputation: false,
                                target_is_unit: true,
                                target_player_contested_pvp,
                            });
                    }

                    let interaction_distance = creature.unit().world().combat_reach() + 4.0;
                    let in_range = if let Some(player) = map.map().get_typed_player(player_guid) {
                        creature.unit().world().is_within_dist_in_map(
                            player.unit().world(),
                            interaction_distance,
                            true,
                        )
                    } else {
                        let fallback_distance = interaction_distance
                            + creature.unit().world().combat_reach()
                            + player_interaction_combat_reach;
                        creature
                            .unit()
                            .world()
                            .position()
                            .is_within_dist(&player_position, fallback_distance)
                    };
                    if !in_range {
                        return None;
                    }

                    Some(RepresentedCreatureAccessLikeCpp {
                        entry: creature.entry(),
                        position: creature.unit().world().position(),
                        npc_flags: creature.ai_ownership().npc_flags,
                        npc_flags2: creature.ai_ownership().npc_flags2,
                        trainer_class: creature.trainer_class_like_cpp(),
                        faction_template_id: creature.unit().data().faction_template.max(0) as u32,
                    })
                })
                .flatten()
        })();
        if let Some(canonical_access) = canonical_access {
            // Reputation is Player-owned and resolves through the same
            // canonical manager. Evaluate it only after releasing the map
            // guard held by the lookup closure above.
            if canonical_reaction_input_like_cpp.is_some_and(|input| {
                self.represented_get_reaction_to_like_cpp(input)
                    <= wow_data::reputation::ReputationRankLikeCpp::Unfriendly
            }) {
                return None;
            }
            return Some(canonical_access);
        }
        if canonical_record_found_like_cpp || canonical_fail_closed_like_cpp {
            return None;
        }

        self.represented_legacy_npc_can_interact_with_like_cpp(
            guid,
            npc_flags,
            npc_flags2,
            player_position,
            player_map_key.instance_id,
            player_interaction_combat_reach,
            target_player_contested_pvp,
        )
    }

    fn represented_legacy_npc_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        npc_flags: u32,
        npc_flags2: u32,
        player_position: Position,
        player_instance_id: u32,
        player_interaction_combat_reach: f32,
        target_player_contested_pvp: bool,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        let manager = self.map_manager.as_ref()?;
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature =
            manager.find_creature(self.player_map_id_like_cpp(), player_instance_id, guid)?;
        let type_flags =
            CreatureTypeFlags::from_bits_retain(creature.creature.lifecycle_metadata().type_flags);
        if self.resolved_player_is_alive_like_cpp() != Some(true)
            && !type_flags.contains(CreatureTypeFlags::VISIBLE_TO_GHOSTS)
        {
            return None;
        }
        if !creature.is_alive() && !type_flags.contains(CreatureTypeFlags::INTERACT_WHILE_DEAD) {
            return None;
        }
        if (npc_flags != 0 || npc_flags2 != 0)
            && (creature.npc_flags() & npc_flags) == 0
            && (creature.npc_flags2() & npc_flags2) == 0
        {
            return None;
        }
        if creature
            .creature
            .unit()
            .subsystems()
            .control
            .charmer_guid
            .is_some()
        {
            return None;
        }
        if !creature
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::INTERACT_WHILE_HOSTILE)
        {
            let reaction =
                self.represented_get_reaction_to_like_cpp(RepresentedGetReactionInputLikeCpp {
                    self_faction_template_id: creature.faction(),
                    target_faction_template_id: self
                        .player_faction_template_id_like_cpp()
                        .unwrap_or(0),
                    same_object: false,
                    attackable_by_summoner: false,
                    same_charmer_or_owner_or_self: false,
                    self_has_player_owner: false,
                    target_has_player_owner: true,
                    target_player_owner_is_current_session: true,
                    target_owner_forced_rank_for_self: None,
                    same_player_owner: false,
                    duel_in_progress: false,
                    same_raid: false,
                    self_unit_player_controlled: false,
                    target_unit_player_controlled: true,
                    self_ffa_pvp: false,
                    target_ffa_pvp: false,
                    self_ignores_reputation: false,
                    target_ignores_reputation: false,
                    target_is_unit: true,
                    target_player_contested_pvp,
                });
            if reaction <= wow_data::reputation::ReputationRankLikeCpp::Unfriendly {
                return None;
            }
        }
        let interaction_distance = creature.creature.unit().world().combat_reach() + 4.0;
        let interaction_distance_with_radii = interaction_distance
            + creature.creature.unit().world().combat_reach()
            + player_interaction_combat_reach;
        if !creature
            .position()
            .is_within_dist(&player_position, interaction_distance_with_radii)
        {
            return None;
        }

        Some(RepresentedCreatureAccessLikeCpp {
            entry: creature.entry(),
            position: creature.position(),
            npc_flags: creature.npc_flags(),
            npc_flags2: creature.npc_flags2(),
            trainer_class: creature.trainer_class_like_cpp(),
            faction_template_id: creature.faction(),
        })
    }

    pub(crate) fn record_represented_fishing_hole_max_opens_like_cpp(
        &mut self,
        guid: ObjectGuid,
        max_opens: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .fishing_hole_max_opens = Some(max_opens);
    }

    pub(crate) fn record_represented_fishing_hole_radius_like_cpp(
        &mut self,
        guid: ObjectGuid,
        radius: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .fishing_hole_radius = Some(radius as f32);
    }

    pub fn set_realm_id(&mut self, realm_id: u16) {
        self.realm_id = realm_id;
    }

    pub fn set_realm_handle_like_cpp(&mut self, region: u8, battlegroup: u8, realm_id: u16) {
        self.realm_region = region;
        self.realm_battlegroup = battlegroup;
        self.realm_id = realm_id;
    }

    pub fn set_realm_names_like_cpp(
        &mut self,
        names: impl IntoIterator<Item = (u32, String, String)>,
    ) {
        self.realm_names_like_cpp = names
            .into_iter()
            .map(|(address, actual, normalized)| (address, (actual, normalized)))
            .collect();
    }

    /// Compute the Virtual Realm Address: `(Region << 24) | (Battlegroup << 16) | RealmId`.
    ///
    /// Region and Battlegroup come from the active `realmlist` row, matching C++
    /// `Battlenet::RealmHandle{ realm.Id.Region, realm.Id.Site, realm.Id.Realm }.GetAddress()`.
    pub(crate) fn virtual_realm_address(&self) -> u32 {
        (u32::from(self.realm_region) << 24)
            | (u32::from(self.realm_battlegroup) << 16)
            | u32::from(self.realm_id)
    }

    pub(crate) fn realm_names_for_address_like_cpp(
        &self,
        realm_address: u32,
    ) -> Option<(&str, &str)> {
        self.realm_names_like_cpp
            .get(&realm_address)
            .map(|(actual, normalized)| (actual.as_str(), normalized.as_str()))
    }

    /// Set the GUID generator for new characters.
    #[cfg(test)]
    pub fn set_guid_generator(&mut self, generator: Arc<ObjectGuidGenerator>) {
        self.guid_generator = Some(generator);
    }

    /// Install the process-wide C++ `sObjectMgr->GenerateVoidStorageItemId()` mirror.
    #[cfg(test)]
    pub fn set_void_storage_item_id_generator_like_cpp(
        &mut self,
        generator: Arc<VoidStorageItemIdGeneratorLikeCpp>,
    ) {
        self.void_storage_item_id_generator_like_cpp = Some(generator);
    }

    /// Install the Player lifecycle persistence port. Composition supplies the
    /// MariaDB adapter; unit sessions leave it empty and skip durable writes.
    pub fn set_player_lifecycle_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp.player.player_lifecycle = Some(port);
    }

    pub(crate) fn player_lifecycle_port_like_cpp(
        &self,
    ) -> Option<&Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .player_lifecycle
            .as_ref()
    }

    pub(crate) fn set_realm_list_secret_like_cpp(&mut self, secret: [u8; 32]) {
        self.realm_list_secret_like_cpp = secret;
    }

    pub(crate) fn realm_list_secret_like_cpp(&self) -> &[u8; 32] {
        &self.realm_list_secret_like_cpp
    }

    pub fn set_mute_time_like_cpp(&mut self, mute_time: i64) {
        self.mute_time_like_cpp = mute_time;
    }

    pub(crate) fn can_speak_like_cpp(&self) -> bool {
        self.mute_time_like_cpp <= unix_now()
    }

    pub(crate) fn mute_time_remaining_secs_like_cpp(&self) -> Option<u64> {
        let remaining = self.mute_time_like_cpp.saturating_sub(unix_now());
        (remaining > 0).then_some(remaining as u64)
    }

    pub fn set_recruiter_id_like_cpp(&mut self, recruiter_id: u32) {
        self.recruiter_id_like_cpp = recruiter_id;
    }

    pub(crate) fn recruiter_id_like_cpp(&self) -> u32 {
        self.recruiter_id_like_cpp
    }

    pub fn set_is_a_recruiter_like_cpp(&mut self, is_a_recruiter: bool) {
        self.is_a_recruiter_like_cpp = is_a_recruiter;
    }

    pub(crate) fn is_a_recruiter_like_cpp(&self) -> bool {
        self.is_a_recruiter_like_cpp
    }

    pub(crate) fn session_locale_name_like_cpp(&self) -> &str {
        &self.locale
    }

    /// C++ `sImportPriceQualityStore.LookupEntry(quality + 1)`.
    #[cfg(test)]
    pub fn import_price_quality_factor_like_cpp(&self, quality: u32) -> Option<f32> {
        self.item_valuation_catalogs_for_test_like_cpp()
            .import_prices
            .quality
            .get(quality + 1)
            .map(|entry| entry.data)
    }

    pub fn set_shield_block_regular_game_table(
        &mut self,
        table: Arc<ShieldBlockRegularGameTableLikeCpp>,
    ) {
        self.shield_block_regular_game_table = Some(table);
    }

    pub(crate) fn player_collection_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCollectionStateLikeCpp> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().collections.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerCollectionStateLikeCpp {
                mounts: self.account_mounts_like_cpp.clone(),
                heirlooms: self.represented_account_heirlooms_like_cpp.clone(),
                toys: self.represented_account_toys_like_cpp.clone(),
                item_appearances: self.represented_item_appearances_like_cpp.clone(),
                item_appearance_blocks: self.represented_item_appearance_blocks_like_cpp.clone(),
                temporary_item_appearances: self
                    .represented_temporary_item_appearances_like_cpp
                    .clone(),
                favorite_item_appearances: self
                    .represented_favorite_item_appearances_like_cpp
                    .clone(),
                transmog_illusions: self.represented_transmog_illusions_like_cpp.clone(),
            });
        }
        canonical
    }

    fn replace_player_collection_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerCollectionStateLikeCpp,
    ) -> bool {
        let canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player.install_collection_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(test)]
        {
            self.account_mounts_like_cpp = state.mounts.clone();
            self.represented_account_heirlooms_like_cpp = state.heirlooms.clone();
            self.represented_account_toys_like_cpp = state.toys.clone();
            self.represented_item_appearances_like_cpp = state.item_appearances.clone();
            self.represented_item_appearance_blocks_like_cpp = state.item_appearance_blocks.clone();
            self.represented_temporary_item_appearances_like_cpp =
                state.temporary_item_appearances.clone();
            self.represented_favorite_item_appearances_like_cpp =
                state.favorite_item_appearances.clone();
            self.represented_transmog_illusions_like_cpp = state.transmog_illusions.clone();
            if self.player_handle_like_cpp.is_none() {
                return true;
            }
        }
        canonical
    }

    /// C++ `Player::AddHeirloom`, called from `CollectionMgr::AddHeirloom`
    /// after the account collection accepts a new heirloom.
    pub(crate) fn add_player_heirloom_dynamic_fields_like_cpp(
        &mut self,
        item_id: u32,
        flags: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_id = i32::try_from(item_id).ok()?;
        self.mutate_canonical_player_like_cpp(|player| {
            player.add_heirloom_like_cpp(item_id, flags);
            player.values_update(true)
        })
    }

    /// C++ `DB2Manager::IsToyItem`.
    pub(crate) fn is_toy_item_like_cpp(&self, item_id: u32) -> bool {
        self.toy_store
            .as_ref()
            .and_then(|store| store.get_by_item_id_like_cpp(item_id))
            .is_some()
    }

    /// C++ `std::find_if(item->Effects, spellId)` in `HandleUseToy`.
    pub(crate) fn toy_item_has_spell_effect_like_cpp(&self, item_id: u32, spell_id: i32) -> bool {
        self.items
            .effect_store
            .as_ref()
            .and_then(|store| store.effect_for_item_spell_like_cpp(item_id, spell_id))
            .is_some()
    }

    /// Bounded C++ `SpellHistory::GetCooldownDurations(spellInfo, itemId)`.
    ///
    /// C++ lets `ItemEffect` override spell/category cooldowns for item-backed
    /// casts. Rust still lacks full `_categoryCooldowns`, so this represented
    /// helper returns the longest positive item cooldown as the single spell-id
    /// keyed duration used by the current `SpellHistory` seam.
    pub(crate) fn toy_item_spell_cooldown_ms_like_cpp(
        &self,
        item_id: u32,
        spell_id: i32,
        spell_info: &wow_data::SpellInfo,
    ) -> u32 {
        if let Some(effect) = self
            .items
            .effect_store
            .as_ref()
            .and_then(|store| store.effect_for_item_spell_like_cpp(item_id, spell_id))
        {
            if effect.cooldown_msec >= 0 || effect.category_cooldown_msec >= 0 {
                return effect
                    .cooldown_msec
                    .max(effect.category_cooldown_msec)
                    .max(0) as u32;
            }
        }

        spell_info.recovery_time_ms.max(spell_info.cooldown_ms)
    }

    /// C++ `Player::AddToy`, called from `CollectionMgr::AddToy` after the
    /// account collection accepts a new toy.
    pub(crate) fn add_player_toy_dynamic_field_like_cpp(
        &mut self,
        item_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_id = i32::try_from(item_id).ok()?;
        self.mutate_canonical_player_like_cpp(|player| {
            player.add_toy_like_cpp(item_id);
            player.values_update(true)
        })
    }

    /// C++ `CollectionMgr::ToyClearFanfare`.
    pub(crate) fn toy_clear_fanfare_like_cpp(&mut self, item_id: u32) -> bool {
        self.mutate_player_collection_state_like_cpp(|collections| {
            let Some(flags) = collections.toys.get_mut(&item_id) else {
                return false;
            };
            *flags &= !TOY_FLAG_HAS_FANFARE_LIKE_CPP;
            true
        })
        .unwrap_or(false)
    }

    /// C++ `CollectionMgr::ToySetFavorite`.
    pub(crate) fn toy_set_favorite_like_cpp(&mut self, item_id: u32, favorite: bool) -> bool {
        self.mutate_player_collection_state_like_cpp(|collections| {
            let Some(flags) = collections.toys.get_mut(&item_id) else {
                return false;
            };
            if favorite {
                *flags |= TOY_FLAG_FAVORITE_LIKE_CPP;
            } else {
                *flags &= !TOY_FLAG_FAVORITE_LIKE_CPP;
            }
            true
        })
        .unwrap_or(false)
    }

    /// Set the player stats store for this session.
    pub fn set_player_stats(&mut self, store: Arc<PlayerStatsStore>) {
        self.player_stats = Some(store);
    }

    /// Get the player stats store reference.
    pub fn player_stats(&self) -> Option<&Arc<PlayerStatsStore>> {
        self.player_stats.as_ref()
    }

    fn represented_scaling_stat_context_like_cpp(
        &self,
        item_entry: u32,
    ) -> Option<RepresentedScalingStatContextLikeCpp> {
        let item_store = self.items.store.as_ref()?;
        let scaling_stat_distribution_id = item_store.scaling_stat_distribution_id(item_entry);
        let scaling_stat_value = item_store.scaling_stat_value(item_entry);
        if scaling_stat_distribution_id == 0 || scaling_stat_value == 0 {
            return None;
        }
        let distribution_store = self.scaling_stat_distribution_store.as_ref()?;
        let values_store = self.scaling_stat_values_store.as_ref()?;
        let distribution = distribution_store.get(u32::from(scaling_stat_distribution_id))?;
        let character_level = self.represented_scaling_stat_character_level_like_cpp(distribution);
        let values = values_store.get_for_character_level_like_cpp(character_level)?;
        let mask = scaling_stat_value as u32;
        Some(RepresentedScalingStatContextLikeCpp {
            stat_id: distribution.stat_id,
            bonus: distribution.bonus,
            ssd_multiplier: values.ssd_multiplier_like_cpp(mask),
            spell_bonus: values.spell_bonus_like_cpp(mask),
            armor_mod: values.armor_mod_like_cpp(mask),
            dps_mod: values.dps_mod_like_cpp(mask),
            is_two_hand: values.is_two_hand_like_cpp(mask),
        })
    }

    fn represented_scaling_stat_character_level_like_cpp(
        &self,
        distribution: &ScalingStatDistributionEntry,
    ) -> u32 {
        let min_level = u32::try_from(distribution.min_level).unwrap_or(0);
        let max_level = u32::try_from(distribution.max_level).unwrap_or(min_level);
        let (min_level, max_level) = if min_level <= max_level {
            (min_level, max_level)
        } else {
            (max_level, min_level)
        };
        u32::from(self.player_level_like_cpp()).clamp(min_level, max_level)
    }

    pub fn set_reset_schedule_like_cpp(&mut self, schedule: wow_instances::ResetSchedule) {
        self.reset_schedule_like_cpp = schedule;
    }

    pub fn set_represented_is_outdoors_like_cpp(&mut self, is_outdoors: bool) {
        let _ = self.mutate_player_world_local_state_like_cpp(|state| {
            state.is_outdoors = Some(is_outdoors);
        });
    }

    #[cfg(test)]
    pub fn set_start_all_explored_like_cpp(&mut self, enabled: bool) {
        self.start_all_explored_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn start_all_explored_like_cpp(&self) -> bool {
        self.start_all_explored_like_cpp
    }

    pub(crate) const fn reset_schedule_like_cpp(&self) -> wow_instances::ResetSchedule {
        self.reset_schedule_like_cpp
    }

    pub(crate) fn resolved_watched_faction_index_like_cpp(&self) -> Option<i32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.watched_faction_index_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.watched_faction_index_like_cpp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn watched_faction_index_like_cpp(&self) -> i32 {
        self.resolved_watched_faction_index_like_cpp()
            .expect("Player watched-faction owner must resolve")
    }

    pub(crate) fn set_watched_faction_index_like_cpp(&mut self, index: i32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_watched_faction_index_like_cpp(index)
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.watched_faction_index_like_cpp = index;
        }
    }

    pub(crate) fn represented_faction_reaction_to_like_cpp(
        &self,
        input: RepresentedFactionReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        let Some(faction_template_store) = self.factions.template_store.as_ref() else {
            return ReputationRankLikeCpp::Neutral;
        };
        let Some(source_faction_template) =
            faction_template_store.get(input.source_faction_template_id)
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        let Some(target_faction_template) =
            faction_template_store.get(input.target_faction_template_id)
        else {
            return ReputationRankLikeCpp::Neutral;
        };
        let Some(reputation_mgr) = self.with_reputation_mgr_like_cpp(Clone::clone) else {
            return ReputationRankLikeCpp::Neutral;
        };

        if input.target_has_player_owner && input.target_player_owner_is_current_session {
            if source_faction_template.is_contested_guard_faction_like_cpp()
                && input.target_player_contested_pvp
            {
                return ReputationRankLikeCpp::Hostile;
            }
            if let Some(forced_rank) = reputation_mgr
                .forced_rank_by_faction_id_like_cpp(u32::from(source_faction_template.faction))
            {
                return forced_rank;
            }
            if input.target_is_unit
                && !input.target_ignores_reputation
                && let Some(faction_store) = self.factions.store.as_ref()
                && let Some(source_faction_entry) =
                    faction_store.get(u32::from(source_faction_template.faction))
                && source_faction_entry.can_have_reputation_like_cpp()
            {
                let mut rank = reputation_mgr.rank_for_faction_entry_like_cpp(
                    source_faction_entry,
                    self.friendship_rep_reaction_store.as_deref(),
                    self.player_race_like_cpp(),
                    self.player_class_like_cpp(),
                );
                if reputation_mgr.is_at_war_with_faction_like_cpp(source_faction_entry)
                    && rank > ReputationRankLikeCpp::Neutral
                {
                    rank = ReputationRankLikeCpp::Neutral;
                }
                return rank;
            }
        }

        if source_faction_template.is_hostile_to_like_cpp(target_faction_template) {
            return ReputationRankLikeCpp::Hostile;
        }
        if source_faction_template.is_friendly_to_like_cpp(target_faction_template) {
            return ReputationRankLikeCpp::Friendly;
        }
        if target_faction_template.is_friendly_to_like_cpp(source_faction_template) {
            return ReputationRankLikeCpp::Friendly;
        }
        if source_faction_template.is_hostile_by_default_like_cpp() {
            return ReputationRankLikeCpp::Hostile;
        }
        ReputationRankLikeCpp::Neutral
    }

    pub(crate) fn represented_get_reaction_to_like_cpp(
        &self,
        input: RepresentedGetReactionInputLikeCpp,
    ) -> wow_data::reputation::ReputationRankLikeCpp {
        use wow_data::reputation::ReputationRankLikeCpp;

        if input.same_object {
            return ReputationRankLikeCpp::Friendly;
        }
        if input.attackable_by_summoner {
            return ReputationRankLikeCpp::Neutral;
        }
        if input.same_charmer_or_owner_or_self {
            return ReputationRankLikeCpp::Friendly;
        }
        let Some(reputation_mgr) = self.with_reputation_mgr_like_cpp(Clone::clone) else {
            return ReputationRankLikeCpp::Neutral;
        };

        if input.self_has_player_owner {
            if let Some(faction_template_store) = self.factions.template_store.as_ref()
                && let Some(target_faction_template) =
                    faction_template_store.get(input.target_faction_template_id)
                && let Some(forced_rank) = reputation_mgr
                    .forced_rank_by_faction_id_like_cpp(u32::from(target_faction_template.faction))
            {
                return forced_rank;
            }
        } else if input.target_has_player_owner
            && let Some(faction_template_store) = self.factions.template_store.as_ref()
            && faction_template_store
                .get(input.self_faction_template_id)
                .is_some()
            && let Some(forced_rank) = input.target_owner_forced_rank_for_self
        {
            return forced_rank;
        }

        if input.self_unit_player_controlled && input.target_unit_player_controlled {
            if input.self_has_player_owner && input.target_has_player_owner {
                if input.same_player_owner {
                    return ReputationRankLikeCpp::Friendly;
                }
                if input.duel_in_progress {
                    return ReputationRankLikeCpp::Hostile;
                }
                if input.same_raid {
                    return ReputationRankLikeCpp::Friendly;
                }
            }

            if input.self_ffa_pvp && input.target_ffa_pvp {
                return ReputationRankLikeCpp::Hostile;
            }

            if input.self_has_player_owner {
                let Some(faction_template_store) = self.factions.template_store.as_ref() else {
                    return self.represented_faction_reaction_to_like_cpp(
                        RepresentedFactionReactionInputLikeCpp {
                            source_faction_template_id: input.self_faction_template_id,
                            target_faction_template_id: input.target_faction_template_id,
                            target_has_player_owner: input.target_has_player_owner,
                            target_player_owner_is_current_session: input
                                .target_player_owner_is_current_session,
                            target_player_contested_pvp: input.target_player_contested_pvp,
                            target_is_unit: input.target_is_unit,
                            target_ignores_reputation: input.target_ignores_reputation,
                        },
                    );
                };
                if let Some(target_faction_template) =
                    faction_template_store.get(input.target_faction_template_id)
                {
                    if let Some(forced_rank) = reputation_mgr.forced_rank_by_faction_id_like_cpp(
                        u32::from(target_faction_template.faction),
                    ) {
                        return forced_rank;
                    }
                    if !input.self_ignores_reputation
                        && let Some(faction_store) = self.factions.store.as_ref()
                        && let Some(target_faction_entry) =
                            faction_store.get(u32::from(target_faction_template.faction))
                        && target_faction_entry.can_have_reputation_like_cpp()
                    {
                        if target_faction_template.is_contested_guard_faction_like_cpp()
                            && input.target_player_contested_pvp
                        {
                            return ReputationRankLikeCpp::Hostile;
                        }
                        if reputation_mgr.is_at_war_with_faction_like_cpp(target_faction_entry) {
                            return ReputationRankLikeCpp::Hostile;
                        }
                        return ReputationRankLikeCpp::Friendly;
                    }
                }
            }
        }

        self.represented_faction_reaction_to_like_cpp(RepresentedFactionReactionInputLikeCpp {
            source_faction_template_id: input.self_faction_template_id,
            target_faction_template_id: input.target_faction_template_id,
            target_has_player_owner: input.target_has_player_owner,
            target_player_owner_is_current_session: input.target_player_owner_is_current_session,
            target_player_contested_pvp: input.target_player_contested_pvp,
            target_is_unit: input.target_is_unit,
            target_ignores_reputation: input.target_ignores_reputation,
        })
    }

    #[cfg(test)]
    pub fn set_addon_channel_like_cpp(&mut self, enabled: bool) {
        self.addon_channel_like_cpp = enabled;
    }

    pub fn set_socket_timeouts_like_cpp(&mut self, timeouts: SocketTimeoutsLikeCpp) {
        self.socket_timeouts_like_cpp = timeouts;
        self.reset_timeout_time_like_cpp(false);
    }

    pub fn set_server_expansion_like_cpp(&mut self, expansion: u8) {
        self.server_expansion_like_cpp = expansion;
    }

    #[cfg(test)]
    pub fn set_characters_per_realm_like_cpp(&mut self, characters_per_realm: u32) {
        self.characters_per_realm_like_cpp = characters_per_realm;
    }

    #[cfg(test)]
    pub fn set_declined_names_used_like_cpp(&mut self, used: bool) {
        self.declined_names_used_like_cpp = used;
    }

    #[cfg(test)]
    pub(crate) fn declined_names_used_like_cpp(&self) -> bool {
        self.declined_names_used_like_cpp
    }

    #[cfg(test)]
    pub fn set_feature_system_character_undelete_enabled_like_cpp(&mut self, enabled: bool) {
        self.feature_system_character_undelete_enabled_like_cpp = enabled;
    }

    pub fn set_waypoint_path_resolver_like_cpp(&mut self, resolver: WaypointPathResolverLikeCpp) {
        self.waypoint_path_resolver_like_cpp = Some(resolver);
    }

    #[cfg(test)]
    pub(crate) fn addon_channel_like_cpp(&self) -> bool {
        self.addon_channel_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn update_speak_time_like_cpp(&mut self, index: ChatFloodThrottleIndexLikeCpp) {
        self.update_speak_time_with_policy_like_cpp(index, self.chat_flood_config_like_cpp)
    }

    pub(crate) fn player_is_game_master_like_cpp(&self) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(Player::is_game_master_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_game_master_like_cpp);
        }
        canonical
    }

    fn player_unit_presentation_snapshot_like_cpp(&self) -> Option<(UnitFlags, i32, f32)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            (
                player.unit().unit_flags_like_cpp(),
                player.unit().data().mount_display_id,
                player.unit().world().object().scale(),
            )
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.player_unit_flags_like_cpp,
                self.player_mount_display_id_like_cpp,
                self.player_object_scale_like_cpp,
            ));
        }
        canonical
    }

    fn set_player_mount_presentation_like_cpp(&mut self, display_id: i32, mounted: bool) -> bool {
        #[cfg_attr(not(test), allow(unused_mut))]
        let mut canonical = self
            .mutate_player_unit_presentation_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_mount_display_id(u32::try_from(display_id).unwrap_or(0));
                let mut flags = player.unit().unit_flags_like_cpp();
                if mounted {
                    flags.insert(UnitFlags::MOUNT);
                } else {
                    flags.remove(UnitFlags::MOUNT);
                }
                player.unit_mut().set_unit_flags_like_cpp(flags);
            })
            .is_some();
        #[cfg(test)]
        if !canonical
            && self.player_handle_like_cpp.is_none()
            && let Some(guid) = self.player_guid()
        {
            canonical = self
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player
                        .unit_mut()
                        .set_mount_display_id(u32::try_from(display_id).unwrap_or(0));
                    let mut flags = player.unit().unit_flags_like_cpp();
                    if mounted {
                        flags.insert(UnitFlags::MOUNT);
                    } else {
                        flags.remove(UnitFlags::MOUNT);
                    }
                    player.unit_mut().set_unit_flags_like_cpp(flags);
                })
                .is_some();
        }
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_mount_display_id_like_cpp = display_id;
            self.player_mounted_like_cpp = mounted;
            if mounted {
                self.player_unit_flags_like_cpp.insert(UnitFlags::MOUNT);
            } else {
                self.player_unit_flags_like_cpp.remove(UnitFlags::MOUNT);
            }
            return true;
        }
        canonical
    }

    pub(crate) fn clear_buyback_runtime_like_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.buyback_items_mut().clear();
            *inventory.buyback_price_mut() = [0; BUYBACK_SLOT_COUNT];
            *inventory.buyback_timestamp_mut() = [0; BUYBACK_SLOT_COUNT];
            inventory.set_current_buyback_slot(BUYBACK_SLOT_START);
        });
    }

    pub(crate) fn set_current_buyback_slot_like_cpp(&mut self, slot: u8) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.set_current_buyback_slot(slot);
        });
    }

    pub(crate) fn set_buyback_slot_metadata_like_cpp(
        &mut self,
        slot: u8,
        price: u32,
        timestamp: i64,
    ) {
        if !(BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot) {
            return;
        }
        let index = (slot - BUYBACK_SLOT_START) as usize;
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.buyback_price_mut()[index] = price;
            inventory.buyback_timestamp_mut()[index] = timestamp;
        });
    }

    pub(crate) fn clear_buyback_slot_metadata_like_cpp(&mut self, slot: u8) {
        self.set_buyback_slot_metadata_like_cpp(slot, 0, 0);
    }

    pub(crate) fn select_buyback_slot_cpp(&self) -> Option<u8> {
        let buyback_items = self.resolved_buyback_items_like_cpp()?;
        let buyback_timestamp = self.resolved_buyback_timestamp_like_cpp()?;
        let mut slot = self.resolved_current_buyback_slot_like_cpp()?;
        if buyback_items.contains_key(&slot) {
            let mut oldest_slot = BUYBACK_SLOT_START;
            let mut oldest_time = buyback_timestamp[0];

            for candidate in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
                let candidate_index = (candidate - BUYBACK_SLOT_START) as usize;
                if !buyback_items.contains_key(&candidate) {
                    oldest_slot = candidate;
                    break;
                }
                let candidate_time = buyback_timestamp[candidate_index];
                if oldest_time > candidate_time {
                    oldest_time = candidate_time;
                    oldest_slot = candidate;
                }
            }

            slot = oldest_slot;
        }

        Some(slot)
    }

    pub(crate) fn advance_buyback_slot_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            if inventory.current_buyback_slot() < BUYBACK_SLOT_END - 1 {
                inventory.set_current_buyback_slot(inventory.current_buyback_slot() + 1);
            }
        });
    }

    /// Set the C++ DisableMgr store loaded from the `disables` table.
    pub fn set_disable_mgr(&mut self, store: Arc<DisableMgrLikeCpp>) {
        self.disable_mgr = Some(store);
    }

    /// Get the loaded DisableMgr store reference.
    pub fn disable_mgr(&self) -> Option<&Arc<DisableMgrLikeCpp>> {
        self.disable_mgr.as_ref()
    }

    /// C++ `sLockStore.LookupEntry(lockId)`.
    pub fn lock_entry_exists_like_cpp(&self, lock_id: u32) -> bool {
        self.lock_store
            .as_ref()
            .is_some_and(|store| store.contains(lock_id))
    }

    #[cfg(test)]
    fn set_give_player_xp_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: GivePlayerXpScriptDispatcherLikeCpp,
    ) {
        self.give_player_xp_script_dispatcher_like_cpp = Some(dispatcher);
    }

    pub(crate) fn set_championing_faction_like_cpp(&mut self, faction_id: u32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_championing_faction_like_cpp(faction_id);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.championing_faction_like_cpp = faction_id;
        }
    }

    pub(crate) fn resolved_championing_faction_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.gameplay_state().championing_faction_id);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.championing_faction_like_cpp);
        }
        canonical
    }

    /// C++ can load/summon a `character_pet` during the Player lifetime and
    /// pet runtime can cast owner auras. Until those transitions are fully
    /// represented, admit only the complete empty-query state and revoke it
    /// on every represented pet load or mutation.
    fn represented_character_pet_aura_source_is_empty_like_cpp(&self) -> bool {
        let Some(pet_lifecycle) = self.player_pet_lifecycle_state_snapshot_like_cpp() else {
            return false;
        };
        pet_lifecycle.character_rows_empty_authority_complete
            && pet_lifecycle.temporary_unsummoned_pet_number == 0
            && self.player_pet_guid_state_like_cpp() == Some(None)
            && pet_lifecycle.stable.current_pet_index.is_none()
            && pet_lifecycle.stable.active_pets.is_empty()
            && pet_lifecycle.stable.stabled_pets.is_empty()
            && pet_lifecycle.stable.unslotted_pets.is_empty()
    }

    pub(crate) fn spell_spell_group_map_bounds_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| {
                store.spell_spell_group_map_bounds_like_cpp(spell_id, |lookup_spell_id| {
                    self.spell_catalogs
                        .spell_chain_store
                        .as_ref()
                        .map(|spell_chains| {
                            spell_chains.first_spell_in_chain_like_cpp(lookup_spell_id)
                        })
                        .unwrap_or(lookup_spell_id)
                })
            })
            .unwrap_or(&[])
    }

    pub(crate) fn spell_group_spell_map_bounds_like_cpp(&self, group_id: u32) -> &[i32] {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| store.spell_group_spell_map_bounds_like_cpp(group_id))
            .unwrap_or(&[])
    }

    pub(crate) fn is_spell_member_of_spell_group_like_cpp(
        &self,
        spell_id: u32,
        group_id: u32,
    ) -> bool {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| {
                store.is_spell_member_of_spell_group_like_cpp(
                    spell_id,
                    group_id,
                    |lookup_spell_id| {
                        self.spell_catalogs
                            .spell_chain_store
                            .as_ref()
                            .map(|spell_chains| {
                                spell_chains.first_spell_in_chain_like_cpp(lookup_spell_id)
                            })
                            .unwrap_or(lookup_spell_id)
                    },
                )
            })
            .unwrap_or(false)
    }

    pub(crate) fn set_of_spells_in_spell_group_like_cpp(&self, group_id: u32) -> BTreeSet<u32> {
        self.spell_catalogs
            .spell_group_store
            .as_ref()
            .map(|store| store.set_of_spells_in_spell_group_like_cpp(group_id))
            .unwrap_or_default()
    }

    pub(crate) fn spell_group_stack_rule_like_cpp(
        &self,
        group_id: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        self.spell_catalogs
            .spell_group_stack_rule_store
            .as_ref()
            .map(|store| store.spell_group_stack_rule_like_cpp(group_id))
            .unwrap_or(SpellGroupStackRuleLikeCpp::Default)
    }

    pub(crate) fn check_spell_group_stack_rules_like_cpp(
        &self,
        first_rank_spell_id_1: u32,
        first_rank_spell_id_2: u32,
    ) -> SpellGroupStackRuleLikeCpp {
        let Some(stack_rules) = self.spell_catalogs.spell_group_stack_rule_store.as_ref() else {
            return SpellGroupStackRuleLikeCpp::Default;
        };
        let Some(spell_groups) = self.spell_catalogs.spell_group_store.as_ref() else {
            return SpellGroupStackRuleLikeCpp::Default;
        };
        stack_rules.check_spell_group_stack_rules_like_cpp(
            spell_groups,
            first_rank_spell_id_1,
            first_rank_spell_id_2,
        )
    }

    pub(crate) fn pet_aura_like_cpp(
        &self,
        spell_id: u32,
        effect_index: u8,
    ) -> Option<&PetAuraLikeCpp> {
        self.spell_catalogs
            .spell_pet_aura_store
            .as_ref()
            .and_then(|store| store.get_pet_aura_like_cpp(spell_id, effect_index))
    }

    #[cfg(test)]
    pub(crate) fn pet_levelup_spell_list_like_cpp(
        &self,
        pet_family: u32,
    ) -> Option<&PetLevelupSpellSetLikeCpp> {
        self.spell_catalogs
            .pet_levelup_spell_store
            .as_ref()
            .and_then(|store| store.get_pet_levelup_spell_list_like_cpp(pet_family))
    }

    #[cfg(test)]
    pub(crate) fn pet_default_spells_entry_like_cpp(
        &self,
        id: i32,
    ) -> Option<&PetDefaultSpellsEntryLikeCpp> {
        self.spell_catalogs
            .pet_default_spell_store
            .as_ref()
            .and_then(|store| store.get_pet_default_spells_entry_like_cpp(id))
    }

    #[cfg(test)]
    pub(crate) fn pet_family_spells_like_cpp(&self, pet_family: u32) -> Option<Vec<u32>> {
        self.spell_catalogs
            .pet_family_spell_store
            .as_ref()
            .and_then(|store| store.get_pet_family_spells_like_cpp(pet_family))
    }

    #[cfg(test)]
    pub(crate) fn model_for_totem_like_cpp(&self, spell_id: u32, race_id: u8) -> u32 {
        self.spell_catalogs
            .spell_totem_model_store
            .as_ref()
            .map(|store| store.get_model_for_totem_like_cpp(spell_id, race_id))
            .unwrap_or(0)
    }

    pub fn set_script_name_interner(&mut self, store: Arc<ScriptNameInternerLikeCpp>) {
        self.script_name_interner = Some(store);
    }

    #[allow(dead_code)]
    pub(crate) fn script_name_like_cpp(&self, id: ScriptIdLikeCpp) -> &str {
        self.script_name_interner
            .as_ref()
            .map(|store| store.get_script_name_like_cpp(id))
            .unwrap_or("")
    }

    #[allow(dead_code)]
    pub(crate) fn script_id_bound_in_database_like_cpp(&self, id: ScriptIdLikeCpp) -> bool {
        self.script_name_interner
            .as_ref()
            .is_some_and(|store| store.is_script_database_bound_like_cpp(id))
    }

    pub(crate) fn faction_template_for_race_like_cpp(&self, race: u8) -> Option<i32> {
        self.chr
            .races_store
            .as_ref()?
            .get(u32::from(race))
            .map(|entry| i32::from(entry.faction_id))
    }

    fn player_cinematic_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCinematicStateLikeCpp> {
        let canonical = self.with_owned_player_like_cpp(|player| player.gameplay_state().cinematic);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerCinematicStateLikeCpp {
                cinematic_id: self.represented_cinematic_like_cpp,
                camera_ids: self.represented_cinematic_camera_ids_like_cpp,
                camera_index: self.represented_cinematic_camera_index_like_cpp,
                movie_id: self.represented_movie_like_cpp,
            });
        }
        None
    }

    pub(crate) fn opening_cinematic_like_cpp(&mut self) -> Option<u32> {
        if self.resolved_player_xp_like_cpp()? != 0 {
            return None;
        }

        let class_store = self.chr.classes_store.as_ref()?;
        let class_entry = class_store.get(u32::from(self.player_class_like_cpp()))?;
        let cinematic_id = if class_entry.cinematic_sequence_id != 0 {
            u32::from(class_entry.cinematic_sequence_id)
        } else {
            let race_store = self.chr.races_store.as_ref()?;
            race_store
                .get(u32::from(self.player_race_like_cpp()))
                .map(|race_entry| race_entry.cinematic_sequence_id as u32)?
        };

        self.send_represented_cinematic_start_like_cpp(cinematic_id);
        Some(cinematic_id)
    }

    pub(crate) fn complete_represented_cinematic_like_cpp(&mut self) {
        let Some(cinematic_id) = self
            .mutate_player_cinematic_state_like_cpp(|state| {
                let cinematic_id = state.cinematic_id.take();
                state.camera_ids = None;
                state.camera_index = -1;
                cinematic_id
            })
            .flatten()
        else {
            return;
        };
        {
            #[cfg(not(test))]
            let _ = cinematic_id;
            #[cfg(test)]
            self.represented_cinematic_end_events_like_cpp
                .push(cinematic_id);
        }
    }

    pub(crate) fn next_represented_cinematic_camera_like_cpp(&mut self) {
        let Some(Some(camera_id)) = self.mutate_player_cinematic_state_like_cpp(|state| {
            state.cinematic_id?;
            let camera_ids = state.camera_ids?;
            if state.camera_index >= camera_ids.len() as i32 {
                return None;
            }
            state.camera_index += 1;
            // C++ checks the previous index before pre-incrementing. Rust keeps
            // the normal flow but refuses the out-of-bounds edge instead of
            // reproducing undefined behavior.
            camera_ids.get(state.camera_index as usize).copied()
        }) else {
            return;
        };
        if camera_id == 0 {
            return;
        }
        #[cfg(test)]
        self.represented_cinematic_next_camera_events_like_cpp
            .push(camera_id);
    }

    pub(crate) fn complete_represented_movie_like_cpp(&mut self) {
        let Some(movie_id) = self
            .mutate_player_cinematic_state_like_cpp(|state| state.movie_id.take())
            .flatten()
        else {
            return;
        };
        {
            #[cfg(not(test))]
            let _ = movie_id;
            #[cfg(test)]
            self.represented_movie_complete_events_like_cpp
                .push(movie_id);
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.cinematic_id)
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_camera_index_like_cpp(&self) -> i32 {
        self.player_cinematic_state_snapshot_like_cpp()
            .map_or(-1, |state| state.camera_index)
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_next_camera_events_like_cpp(&self) -> &[u16] {
        &self.represented_cinematic_next_camera_events_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_cinematic_end_events_like_cpp(&self) -> &[u32] {
        &self.represented_cinematic_end_events_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_movie_like_cpp(&self) -> Option<u32> {
        self.player_cinematic_state_snapshot_like_cpp()
            .and_then(|state| state.movie_id)
    }

    #[cfg(test)]
    pub(crate) fn represented_movie_complete_events_like_cpp(&self) -> &[u32] {
        &self.represented_movie_complete_events_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_support_enabled_like_cpp(&self) -> bool {
        self.represented_support_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_enabled_like_cpp(&mut self, enabled: bool) {
        self.represented_support_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_support_tickets_enabled_like_cpp(&self) -> bool {
        self.represented_support_tickets_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_tickets_enabled_like_cpp(&mut self, enabled: bool) {
        self.represented_support_tickets_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_support_bugs_enabled_like_cpp(&self) -> bool {
        self.represented_support_bugs_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_bugs_enabled_like_cpp(&mut self, enabled: bool) {
        self.represented_support_bugs_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_bug_system_status_like_cpp(&self) -> bool {
        self.represented_support_enabled_like_cpp && self.represented_support_bugs_enabled_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_support_complaints_enabled_like_cpp(&self) -> bool {
        self.represented_support_complaints_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_complaints_enabled_like_cpp(&mut self, enabled: bool) {
        self.represented_support_complaints_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_complaint_system_status_like_cpp(&self) -> bool {
        self.represented_support_enabled_like_cpp
            && self.represented_support_complaints_enabled_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_support_suggestions_enabled_like_cpp(&self) -> bool {
        self.represented_support_suggestions_enabled_like_cpp
    }

    #[cfg(test)]
    pub fn set_represented_support_suggestions_enabled_like_cpp(&mut self, enabled: bool) {
        self.represented_support_suggestions_enabled_like_cpp = enabled;
    }

    #[cfg(test)]
    pub(crate) fn represented_suggestion_system_status_like_cpp(&self) -> bool {
        self.represented_support_enabled_like_cpp
            && self.represented_support_suggestions_enabled_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn feature_system_status_like_cpp(&self) -> FeatureSystemStatus {
        self.feature_system_status_with_policy_like_cpp(
            &self.support_feature_policy_for_test_like_cpp(),
        )
    }

    #[cfg(test)]
    pub(crate) fn feature_system_status_glue_screen_like_cpp(
        &self,
    ) -> FeatureSystemStatusGlueScreen {
        self.feature_system_status_glue_screen_with_policy_like_cpp(
            &self.support_feature_policy_for_test_like_cpp(),
        )
    }

    /// C++ `Player::IsMaxLevel` reads `ActivePlayerData::MaxLevel`, which
    /// `InitStatsForLevel` derives from both the account's active expansion
    /// and CONFIG_MAX_PLAYER_LEVEL. RestMgr deliberately uses the config-only
    /// check above instead, so keep these two concepts separate.
    pub(crate) fn player_active_max_level_like_cpp(&self) -> u32 {
        let expansion_max = u32::from(max_level_for_expansion_like_cpp(self.expansion));
        let configured_max = self.max_player_level_config_like_cpp;
        if expansion_max == 80 || expansion_max >= configured_max {
            configured_max
        } else {
            expansion_max
        }
    }

    fn player_is_max_level_like_cpp(&self) -> bool {
        u32::from(self.player_level_like_cpp()) >= self.player_active_max_level_like_cpp()
    }

    #[cfg(test)]
    fn can_gain_represented_xp_rest_bonus_like_cpp(&self) -> Option<bool> {
        if self.player_is_at_configured_max_level_like_cpp() {
            return Some(false);
        }

        let next_level_xp = self.resolved_player_next_level_xp_like_cpp()?;
        Some(next_level_xp != 0 && next_level_xp != u32::MAX)
    }

    #[cfg(test)]
    fn represented_xp_rest_bonus_cap_like_cpp(&self) -> Option<f32> {
        Some(
            self.resolved_player_next_level_xp_like_cpp()? as f32
                * REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP,
        )
    }

    pub(crate) fn player_rest_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerRestState> {
        let canonical =
            self.with_owned_player_for_rest_like_cpp(|player| player.rest_state_like_cpp().clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerRestState {
                rest_bonus: self.represented_rest_bonus_xp_like_cpp,
                rest_state: self.represented_rest_state_xp_like_cpp,
                rest_flag_mask: self.represented_rest_flag_mask_like_cpp,
                location_initialized: self.represented_rest_location_initialized_like_cpp,
                defer_flag_sync: self.represented_defer_rest_flag_sync_like_cpp,
                deferred_flag_update_dirty: self
                    .represented_deferred_rest_flag_update_dirty_like_cpp,
                inn_area_trigger_id: self.represented_inn_area_trigger_id_like_cpp,
                rest_time_secs: self.represented_rest_time_secs_like_cpp,
                ..Default::default()
            });
        }
        canonical
    }

    #[cfg(test)]
    fn replace_player_rest_state_like_cpp(&mut self, state: wow_entities::PlayerRestState) -> bool {
        let canonical = self
            .with_owned_player_mut_for_rest_like_cpp(|player| {
                player.set_xp_rest_info_like_cpp(
                    state.rest_bonus.clamp(0.0, u32::MAX as f32) as u32,
                    state.rest_state,
                );
                if state.location_initialized {
                    if state.rest_flag_mask != 0 {
                        player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                    } else {
                        player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                    }
                }
                player.replace_rest_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_rest_bonus_xp_like_cpp = state.rest_bonus;
            self.represented_rest_state_xp_like_cpp = state.rest_state;
            self.represented_rest_flag_mask_like_cpp = state.rest_flag_mask;
            self.represented_rest_location_initialized_like_cpp = state.location_initialized;
            self.represented_defer_rest_flag_sync_like_cpp = state.defer_flag_sync;
            self.represented_deferred_rest_flag_update_dirty_like_cpp =
                state.deferred_flag_update_dirty;
            self.represented_inn_area_trigger_id_like_cpp = state.inn_area_trigger_id;
            self.represented_rest_time_secs_like_cpp = state.rest_time_secs;
            return true;
        }
        canonical
    }

    #[cfg(test)]
    fn set_represented_xp_rest_bonus_like_cpp(&mut self, rest_bonus: f32) -> u8 {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_set_xp_rest_bonus_like_cpp(rest_bonus);
        }
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.set_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
        })
        .unwrap_or(0)
    }

    pub(crate) fn add_represented_xp_rest_bonus_like_cpp(&mut self, rest_bonus: f32) -> u8 {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let Some(current) = self.resolved_xp_rest_bonus_like_cpp() else {
                return 0;
            };
            return self.set_represented_xp_rest_bonus_like_cpp(current + rest_bonus);
        }
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.add_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
        })
        .unwrap_or(0)
    }

    #[cfg(test)]
    fn calc_represented_xp_rest_extra_per_sec_like_cpp(&self, bubble: f32) -> Option<f32> {
        if !self.can_gain_represented_xp_rest_bonus_like_cpp()? {
            return Some(0.0);
        }
        Some(self.resolved_player_next_level_xp_like_cpp()? as f32 / 72_000.0 * bubble)
    }

    #[cfg(test)]
    pub(crate) fn apply_offline_xp_rest_bonus_like_cpp(
        &mut self,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.apply_offline_xp_rest_bonus_with_policy_like_cpp(
            &policy,
            logout_time_secs,
            now_secs,
            was_logout_resting,
        )
    }

    #[cfg(test)]
    fn update_represented_online_xp_rest_bonus_like_cpp(&mut self, now_secs: u64) -> (f32, u8) {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.update_represented_online_xp_rest_bonus_with_policy_like_cpp(&policy, now_secs)
    }

    #[cfg(test)]
    fn tick_represented_online_xp_rest_bonus_with_roll_like_cpp(
        &mut self,
        now_secs: u64,
        update_roll_passed: bool,
    ) {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
            &policy,
            now_secs,
            update_roll_passed,
        );
    }

    #[cfg(test)]
    fn revalidate_represented_tavern_resting_like_cpp(&mut self) {
        let db2 = self
            .area_trigger_db2_store
            .clone()
            .unwrap_or_else(|| Arc::new(AreaTriggerDb2Store::from_entries([])));
        self.revalidate_represented_tavern_resting_with_catalog_like_cpp(db2.as_ref());
    }

    pub(crate) fn represented_player_has_flag_like_cpp(&self, flag: u32) -> bool {
        let canonical = self
            .player_guid()
            .and_then(|guid| self.canonical_player_has_player_flag_like_cpp(guid, flag));
        if let Some(value) = canonical {
            return value;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .represented_loaded_player_flags_like_cpp
                .is_some_and(|flags| (flags & flag) != 0);
        }
        false
    }

    pub(crate) fn represented_player_flags_value_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.canonical_player_snapshot_like_cpp(|player| player.data().player_flags);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.represented_loaded_player_flags_like_cpp;
        }
        canonical
    }

    pub(crate) fn void_storage_is_unlocked_like_cpp(&self) -> bool {
        self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP)
    }

    fn take_represented_xp_rest_bonus_for_gain_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
    ) -> (u32, u8) {
        if victim.is_empty() {
            return (0, 0);
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_take_xp_rest_bonus_like_cpp(xp, victim);
        }
        let Some(pct) = self.resolved_total_represented_aura_modifier_like_cpp(
            RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
        ) else {
            return (0, 0);
        };
        let at_max = self.player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.with_owned_player_mut_like_cpp(|player| {
            player.take_xp_rest_bonus_like_cpp(xp, pct, at_max, raf)
        })
        .unwrap_or((0, 0))
    }

    pub(crate) fn resolved_xp_rest_bonus_like_cpp(&self) -> Option<f32> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_bonus)
    }

    pub(crate) fn resolved_xp_rest_state_like_cpp(&self) -> Option<u8> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_state)
    }

    pub(crate) fn resolved_xp_rest_threshold_like_cpp(&self) -> Option<u32> {
        Some(
            self.resolved_xp_rest_bonus_like_cpp()?
                .clamp(0.0, u32::MAX as f32) as u32,
        )
    }

    #[cfg(test)]
    pub(crate) fn represented_xp_rest_bonus_like_cpp(&self) -> f32 {
        self.resolved_xp_rest_bonus_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_xp_rest_state_like_cpp(&self) -> u8 {
        self.resolved_xp_rest_state_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_xp_rest_threshold_like_cpp(&self) -> u32 {
        self.resolved_xp_rest_threshold_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    pub(crate) fn resolved_is_resting_like_cpp(&self) -> Option<bool> {
        self.player_rest_state_snapshot_like_cpp()
            .map(|state| state.rest_flag_mask != 0)
    }

    #[cfg(test)]
    pub(crate) fn represented_is_resting_like_cpp(&self) -> bool {
        self.resolved_is_resting_like_cpp()
            .expect("test Player rest owner must resolve")
    }

    pub(crate) fn set_represented_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        self.mutate_player_rest_state_like_cpp(|state| {
            state.set_flag_like_cpp(
                rest_flag,
                trigger_id,
                crate::session_rules::current_game_time_secs_like_cpp,
            )
        })
        .unwrap_or(false)
    }

    fn update_represented_rest_flag_like_cpp(&mut self, rest_flag: u32, active: bool) -> bool {
        if active {
            self.set_represented_rest_flag_like_cpp(rest_flag, 0)
        } else {
            self.remove_represented_rest_flag_like_cpp(rest_flag)
        }
    }

    pub(crate) fn set_represented_tavern_resting_like_cpp(
        &mut self,
        trigger_id: u32,
        entered: bool,
    ) -> bool {
        if entered {
            self.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, trigger_id)
        } else {
            self.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP)
        }
    }

    pub(crate) fn resolved_player_flags_for_create_like_cpp(&self) -> Option<(u32, u32)> {
        let player_flags = self.resolved_player_flags_for_rest_state_save_like_cpp()?;
        let canonical_flags_ex =
            self.with_owned_player_for_rest_like_cpp(|player| player.data().player_flags_ex);
        #[cfg(test)]
        let canonical_flags_ex =
            if canonical_flags_ex.is_none() && self.player_handle_like_cpp.is_none() {
                self.represented_loaded_player_flags_ex_like_cpp.or(Some(0))
            } else {
                canonical_flags_ex
            };
        let player_flags_ex = canonical_flags_ex?;
        Some((player_flags, player_flags_ex))
    }

    #[cfg(test)]
    pub(crate) fn represented_player_flags_for_create_like_cpp(&self) -> (u32, u32) {
        self.resolved_player_flags_for_create_like_cpp()
            .unwrap_or_else(|| {
                (
                    self.represented_loaded_player_flags_like_cpp.unwrap_or(0),
                    self.represented_loaded_player_flags_ex_like_cpp
                        .unwrap_or(0),
                )
            })
    }

    fn represented_xp_rest_info_changed_since_like_cpp(
        &self,
        old_rest_bonus: f32,
        old_rest_state: u8,
    ) -> bool {
        self.resolved_xp_rest_bonus_like_cpp()
            .zip(self.resolved_xp_rest_state_like_cpp())
            .is_some_and(|(bonus, state)| {
                bonus.to_bits() != old_rest_bonus.to_bits() || state != old_rest_state
            })
    }

    pub(crate) fn current_played_time_values_like_cpp(&self) -> (u32, u32) {
        let session_secs: u32 = self
            .login_time
            .map(|time| time.elapsed().as_secs() as u32)
            .unwrap_or(0);
        (
            self.total_played_time.saturating_add(session_secs),
            self.level_played_time.saturating_add(session_secs),
        )
    }

    pub(crate) fn represented_action_button_db_context_like_cpp(&self) -> Option<(u8, i32)> {
        // Trait-config-specific action bars are not represented yet. Both load and save must use
        // the same C++ fallback context so an autosave cannot mutate rows it never loaded.
        Some((self.represented_active_talent_group_like_cpp()?, 0))
    }

    pub(crate) fn take_deferred_rest_flag_update_dirty_like_cpp(&mut self) -> bool {
        self.mutate_player_rest_state_like_cpp(|state| {
            std::mem::take(&mut state.deferred_flag_update_dirty)
        })
        .unwrap_or(false)
    }

    /// Apply XP to the live session state, leveling up if threshold reached.
    /// C++ `Player::GiveXP` visible side effects; persistence is handled by async wrappers.
    /// `group_rate` is packet metadata only: C++ `KillRewarder::_RewardXP`
    /// scales `xp` before calling this method and passes `_groupRate` separately.
    pub(crate) fn give_xp_runtime_like_cpp(
        &mut self,
        mut xp: u32,
        victim: wow_core::ObjectGuid,
        group_rate: f32,
    ) -> bool {
        use wow_packet::packets::misc::{LevelUpInfo, LogXpGain};

        if xp == 0 {
            return false;
        }
        let Some(player_is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return false;
        };
        if !player_is_alive && !self.player_in_represented_battleground_like_cpp() {
            return false;
        }
        if self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_NO_XP_GAIN_LIKE_CPP) {
            return false;
        }
        if victim.is_any_type_creature()
            && !self
                .represented_creature_has_loot_recipient_like_cpp(victim)
                .unwrap_or(false)
        {
            return false;
        }

        // C++ captures the pre-hook level, dispatches the mutable PlayerScript
        // amount, and only then checks max level. Do not reapply the xp == 0
        // guard after dispatch: C++ continues when a hook changes the amount
        // to zero.
        let old_level = self.player_level_like_cpp();
        let script_context = wow_script::player::GivePlayerXpContextLikeCpp {
            player_guid: self.player_guid().unwrap_or(wow_core::ObjectGuid::EMPTY),
            victim_guid: victim,
        };
        #[cfg(test)]
        if let Some(dispatcher) = &self.give_player_xp_script_dispatcher_like_cpp {
            dispatcher(script_context, &mut xp);
        } else {
            let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);
        }
        #[cfg(not(test))]
        let _ = wow_script::player::on_give_player_xp_like_cpp(script_context, &mut xp);
        if self.player_is_max_level_like_cpp() {
            return false;
        } // max level

        // Resolve the generation-checked owner before consuming rest state or
        // publishing LogXPGain. A stale session must produce no side effect.
        let (Some(current_xp), Some(_next_level_xp)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
        ) else {
            return false;
        };

        // C++ `Player::GiveXP`: Recruit-A-Friend is mutually exclusive with
        // rested XP and contributes 2 * base XP (3x total).
        let recruit_a_friend = self.gets_recruit_a_friend_xp_bonus_like_cpp();
        let (bonus_xp, rest_info_mask) = if recruit_a_friend {
            (xp.saturating_mul(2), 0)
        } else {
            self.take_represented_xp_rest_bonus_for_gain_like_cpp(xp, victim)
        };
        let total_xp = xp.saturating_add(bonus_xp);

        // Send floating XP text — C++ `WorldPackets::Character::LogXPGain`.
        // C++ opcode registration routes SMSG_LOG_XP_GAIN through
        // CONNECTION_TYPE_REALM, even after the session has switched its
        // default channel to the instance connection.
        self.send_packet_realm(&LogXpGain {
            victim,
            original: total_xp.min(i32::MAX as u32) as i32,
            reason: if victim.is_empty() { 1 } else { 0 },
            amount: xp.min(i32::MAX as u32) as i32,
            group_bonus: group_rate,
        });
        if !self.set_player_xp_like_cpp(current_xp.saturating_add(total_xp)) {
            return false;
        }

        // C++ `Player::GiveXP`: while (newXP >= nextLvlXP && !IsMaxLevel()).
        loop {
            let (Some(current_xp), Some(next_level_xp)) = (
                self.resolved_player_xp_like_cpp(),
                self.resolved_player_next_level_xp_like_cpp(),
            ) else {
                return false;
            };
            if current_xp < next_level_xp || self.player_is_max_level_like_cpp() {
                break;
            }
            if !self.set_player_xp_like_cpp(current_xp - next_level_xp) {
                return false;
            }
            let new_level = self.player_level_like_cpp() + 1;

            info!(account = self.account_id, new_level, "Player leveled up");

            // C++ `Player::GiveLevel` computes these deltas from
            // player_classlevelstats + player_racestats and GtBaseMP before
            // updating the live player level.
            let (base_mana_delta, stat_delta) = self
                .level_up_stat_deltas_like_cpp(new_level)
                .unwrap_or((0, [0; 5]));
            let mut power_delta = [0i32; 10];
            power_delta[0] = base_mana_delta;

            let Some(next_level_xp) = self.resolved_player_xp_for_level_like_cpp(new_level) else {
                return false;
            };

            // Send SMSG_LEVELUP_INFO — "Ding!" popup.
            // C++ registers SMSG_LEVEL_UP_INFO on CONNECTION_TYPE_REALM too.
            self.send_packet_realm(&LevelUpInfo {
                level: new_level as i32,
                health_delta: 0,
                power_delta,
                stat_delta,
                num_new_talents: 0,
            });

            self.set_player_level_like_cpp(new_level);
            self.set_player_next_level_xp_like_cpp(next_level_xp);
            self.send_level_up_stat_update_like_cpp();
        }

        self.sync_represented_xp_level_to_canonical_and_client_like_cpp(
            self.player_level_like_cpp() != old_level,
            rest_info_mask,
        );

        true
    }

    /// Mirror C++ update-field side effects from `SetXP` / `GiveLevel` until
    /// canonical map-owned `SendObjectUpdates` has complete session fanout.
    fn sync_represented_xp_level_to_canonical_and_client_like_cpp(
        &mut self,
        level_changed: bool,
        rest_info_mask: u8,
    ) {
        if self.player_guid().is_none() {
            return;
        }

        let level = self.player_level_like_cpp();
        let (Some(xp), Some(next_level_xp), Some(scaling_player_level_delta)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
            self.resolved_player_scaling_level_delta_like_cpp(),
        ) else {
            return;
        };
        let xp = xp.min(i32::MAX as u32) as i32;
        let next_level_xp = next_level_xp.min(i32::MAX as u32) as i32;
        // `GiveLevel` may publish and clear its stat delta before C++'s final
        // `SetXP(newXP)`. Preserve that final unconditional ModifyValue mark
        // without writing a second progression value back into the owner.
        let owner_marked = self
            .with_owned_player_mut_like_cpp(|player| {
                player.mark_xp_changed_like_cpp();
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(not(test))]
        if !owner_marked {
            return;
        }
        #[cfg(test)]
        if !owner_marked && self.player_handle_like_cpp.is_some() {
            // A stale handle is an unknown owner in tests too. Only legacy
            // handle-less fixtures may exercise the isolated packet adapter.
            return;
        }

        // C++ mutates RestInfo and XP on the same Player update mask before
        // `Map::SendObjectUpdates`. Build one isolated transitional delta so
        // the explicit session fanout neither splits nor duplicates RestInfo,
        // and does not leak unrelated canonical dirty fields.
        let mut delta = Player::new(None, false);
        delta.clear_data_changes();
        if level_changed {
            delta.unit_mut().set_level(level);
            delta.set_next_level_xp(next_level_xp);
        }
        delta.set_xp(xp);
        delta.mark_xp_changed_like_cpp();
        delta.set_scaling_player_level_delta_like_cpp(scaling_player_level_delta);
        delta.mark_scaling_player_level_delta_changed_like_cpp();
        if rest_info_mask != 0 {
            let (Some(rest_threshold), Some(rest_state)) = (
                self.resolved_xp_rest_threshold_like_cpp(),
                self.resolved_xp_rest_state_like_cpp(),
            ) else {
                return;
            };
            delta.prepare_rest_info_values_update_like_cpp(
                0,
                rest_threshold,
                rest_state,
                rest_info_mask,
            );
        }
        let update = delta.values_update(true);
        self.send_player_values_update_like_cpp(&update);
    }

    /// Give XP to the player, leveling up if threshold reached.
    /// C++ `Player::GiveXP(xp, victim, group_rate)`.
    pub(crate) async fn give_xp(&mut self, xp: u32, victim: wow_core::ObjectGuid, group_rate: f32) {
        let old_level = self.player_level_like_cpp();
        let (Some(old_rest_bonus), Some(old_rest_state)) = (
            self.resolved_xp_rest_bonus_like_cpp(),
            self.resolved_xp_rest_state_like_cpp(),
        ) else {
            return;
        };
        if !self.give_xp_runtime_like_cpp(xp, victim, group_rate) {
            return;
        }

        let (Some(guid), Some(port)) = (
            self.player_guid(),
            self.player_lifecycle_port_like_cpp().map(Arc::clone),
        ) else {
            return;
        };
        let Some(request) = self.resolved_current_player_xp_persistence_request_like_cpp(
            self.player_level_like_cpp() != old_level,
            self.represented_xp_rest_info_changed_since_like_cpp(old_rest_bonus, old_rest_state),
            guid.counter() as u64,
        ) else {
            return;
        };
        match port.persist_xp_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    guid = guid.counter(),
                    "Failed to atomically persist represented XP/rest state: {reason}"
                );
            }
        }
    }

    /// Level at which mobs give 0 XP ("gray") — C++ `Trinity::XP::GetGrayLevel`.
    pub(crate) fn gray_level(&self, pl: u8) -> u8 {
        let level = if pl < 7 {
            0
        } else if pl < 35 {
            let count = (15..=pl).filter(|level| level % 5 == 0).count() as u8;
            (pl - 7).saturating_sub(count.saturating_sub(1))
        } else {
            pl.saturating_sub(10)
        };
        #[cfg(test)]
        let level = self
            .represented_gray_level_script_overrides_like_cpp
            .get(&pl)
            .copied()
            .unwrap_or(level);
        level
    }

    #[cfg(test)]
    pub(crate) fn set_represented_gray_level_script_override_like_cpp(
        &mut self,
        player_level: u8,
        gray_level: u8,
    ) {
        self.represented_gray_level_script_overrides_like_cpp
            .insert(player_level, gray_level);
        if self.player_level_like_cpp() == player_level {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                player.gameplay_state_mut().gray_level = gray_level;
            });
        }
    }

    /// Zero-difference table — C++ `Trinity::XP::GetZeroDifference`.
    fn zero_difference(&self, pl: u8) -> u8 {
        match pl {
            0..=3 => 5,
            4..=9 => 6,
            10..=11 => 7,
            12..=15 => 8,
            16..=19 => 9,
            20..=29 => 11,
            30..=39 => 12,
            40..=44 => 13,
            45..=49 => 14,
            50..=54 => 15,
            55..=59 => 16,
            _ => 17,
        }
    }

    fn represented_championing_faction_for_kill_like_cpp(&self) -> Option<u32> {
        let championing_faction = self.resolved_championing_faction_like_cpp()?;
        if championing_faction == 0 {
            return None;
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        if !self
            .map_store()
            .and_then(|store| store.get(map_id))
            .is_some_and(|entry| entry.is_non_raid_dungeon_like_cpp())
        {
            return None;
        }
        let difficulty_id = self.current_map_difficulty_id_like_cpp();
        let is_wrath_max_level_lfg = self
            .lfg_dungeons_store
            .as_ref()
            .and_then(|store| store.get_by_map_and_difficulty_like_cpp(map_id, difficulty_id))
            .is_some_and(|dungeon| {
                dungeon.target_level == WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP
            });
        is_wrath_max_level_lfg.then_some(championing_faction)
    }

    pub(crate) async fn killed_player_credit_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        victim_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_PLAYERKILLS with ObjectID 0.
        self.update_represented_storing_value_quest_objective_progress_like_cpp(
            item_guid_generator,
            QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
            0,
            1,
            victim_guid,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn killed_player_credit_like_cpp(
        &mut self,
        victim_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.killed_player_credit_with_generator_like_cpp(generator.as_ref(), victim_guid)
            .await;
    }

    pub(crate) async fn kill_credit_criteria_tree_objective_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        criteria_tree_id: u32,
    ) {
        // C++ QUEST_OBJECTIVE_CRITERIA_TREE.
        self.update_represented_storing_flag_quest_objective_progress_like_cpp(
            item_guid_generator,
            14,
            criteria_tree_id as i32,
            1,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn kill_credit_criteria_tree_objective_like_cpp(
        &mut self,
        criteria_tree_id: u32,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.kill_credit_criteria_tree_objective_with_generator_like_cpp(
            generator.as_ref(),
            criteria_tree_id,
        )
        .await;
    }

    /// Set the player XP table (xp required per level).
    #[cfg(test)]
    pub fn set_player_xp_table(&mut self, table: Arc<Vec<u32>>) {
        self.player_xp_table = Some(table);
        self.refresh_next_level_xp();
    }

    #[cfg(test)]
    pub fn set_exploration_xp_rate_like_cpp(&mut self, rate: f32) {
        self.exploration_xp_rate_like_cpp = rate.max(0.0);
    }

    #[cfg(test)]
    pub fn set_min_discovered_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.min_discovered_scaled_xp_ratio_like_cpp = ratio.min(100);
    }

    #[cfg(test)]
    pub(crate) fn refresh_next_level_xp(&mut self) {
        let catalogs = self.progression_catalogs_for_test_like_cpp();
        self.refresh_next_level_xp_with_catalogs_like_cpp(&catalogs);
    }

    /// C++ `Player::SetXP` updates this field every time XP changes. It uses
    /// the client build's compile-time `MAX_LEVEL`, not the configurable or
    /// account-expansion-specific active maximum.
    pub(crate) fn resolved_player_scaling_level_delta_like_cpp(&self) -> Option<i32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.active_data().scaling_player_level_delta);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                if self.player_level_like_cpp() < WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP
                    && self.player_xp < self.player_next_level_xp / 2
                {
                    -1
                } else {
                    0
                },
            );
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_scaling_level_delta_like_cpp(&self) -> i32 {
        self.resolved_player_scaling_level_delta_like_cpp()
            .expect("test Player progression owner must resolve")
    }

    /// Set the shared player registry (used for broadcast).
    pub fn set_player_registry(&mut self, registry: Arc<PlayerRegistry>) {
        if let Some(manager) = &self.canonical_map_manager {
            let _ = registry.bind_canonical_map_manager(Arc::clone(manager));
        } else if let Some(manager) = registry.canonical_map_manager_like_cpp() {
            self.canonical_map_manager = Some(manager);
        }
        self.player_registry = Some(registry);
    }

    /// Get a reference to the shared player registry.
    pub fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.player_registry.as_ref()
    }

    /// Get a reference to the shared pending invites map.
    pub fn pending_invites(&self) -> Option<&Arc<PendingInvites>> {
        self.pending_invites.as_ref()
    }

    pub(crate) fn player_is_in_world_for_registry_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        if let Some(manager) = &self.canonical_map_manager
            && let Ok(manager) = manager.lock()
        {
            let mut canonical_in_world = None;
            manager.do_for_all_maps(|managed| {
                if canonical_in_world.is_none()
                    && let Some(player) = managed.map().get_typed_player(guid)
                {
                    canonical_in_world = Some(player.unit().world().object().is_in_world());
                }
            });
            if let Some(is_in_world) = canonical_in_world {
                return is_in_world;
            }
        }

        // Registry insertion happens only after successful character login; logout/disconnect
        // unregisters instead of leaving a false/stale row behind.
        true
    }

    pub(crate) fn player_is_strictly_in_world_like_cpp(&self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self
            .canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
        else {
            return false;
        };
        let mut is_in_world = None;
        manager.do_for_all_maps(|managed| {
            if is_in_world.is_none()
                && let Some(player) = managed.map().get_typed_player(guid)
            {
                is_in_world = Some(player.unit().world().object().is_in_world());
            }
        });
        is_in_world.unwrap_or(false)
    }

    pub(crate) fn player_has_unit_state_like_cpp(&self, state: UnitState) -> bool {
        self.canonical_player_snapshot_like_cpp(|player| player.unit().unit_state())
            .is_some_and(|unit_state| unit_state & state.bits() != 0)
    }

    /// Register this session in the player registry.
    /// Called after player login is complete (player_guid + position both set).
    pub(crate) fn register_in_player_registry(&self) {
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        let (Some(guid), Some(pos), Some(name), Some(reg)) = (
            self.player_guid(),
            self.player_position_like_cpp(),
            self.player_name_like_cpp(),
            &self.player_registry,
        ) else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let gender = self.player_gender_like_cpp();
        let level = self.player_level_like_cpp();
        let Some(is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return;
        };
        // Fallback to 0 (world/default instance) when no canonical map key is
        // available — mirrors C++ world-map phase where instance_id == 0.
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        let active_loot_rolls = self
            .represented_loot_rolls
            .values()
            .map(|state| state.command_identity.clone())
            .collect();
        reg.register_or_replace(
            guid,
            PlayerSessionRegistrationLikeCpp {
                identity: crate::session::directory::PlayerDirectoryIdentityLikeCpp::new(
                    name,
                    self.account_id,
                    self.recruiter_id_like_cpp,
                    race,
                    class,
                    gender,
                    self.expansion,
                ),
                placement: crate::session::directory::PlayerDirectoryPlacementLikeCpp {
                    map_id,
                    instance_id,
                    position: pos,
                    is_in_world: self.player_is_in_world_for_registry_like_cpp(),
                    level,
                    is_alive,
                },
                active_loot_rolls,
                send_tx: self.send_tx().clone(),
                realm_send_tx: self.realm_route_tx().clone(),
                command_tx: self.session_command_tx.clone(),
                durable_creature_runtime_commands_like_cpp: Arc::clone(
                    &self.durable_creature_runtime_commands_like_cpp,
                ),
                client_visible_guids_like_cpp: self.client_visible_guids_like_cpp.clone(),
                advanced_combat_logging_enabled_like_cpp: Arc::clone(
                    &self.advanced_combat_logging_enabled_like_cpp,
                ),
                visibility_refresh_pending_like_cpp: Arc::clone(
                    &self.visibility_refresh_pending_like_cpp,
                ),
            },
            Arc::clone(&self.durable_loot_money_persistence_like_cpp),
        );
        // Production already has the canonical Player before publication. The
        // explicit owner-installing test harness creates it while registering,
        // so repeat the one-way hydration after that seam as well.
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        self.sync_player_registry_party_member_party_type_like_cpp();
        debug!(
            "Registered player {:?} ({}) in broadcast registry (map {})",
            guid, name, map_id
        );
    }

    pub(crate) fn sync_player_registry_state_like_cpp(&self) {
        let (Some(guid), Some(registry)) = (self.player_guid(), &self.player_registry) else {
            return;
        };
        self.update_registry_position();
        #[cfg(test)]
        crate::canonical_player_sync::hydrate_player_directory_fixture_like_cpp(self);
        registry.replace_loot_rolls_for_control_channel(
            guid,
            &self.session_command_tx,
            self.represented_loot_rolls
                .values()
                .map(|state| state.command_identity.clone())
                .collect(),
        );
        self.sync_player_registry_party_member_party_type_like_cpp();
    }

    /// Get the realm ID.
    pub fn realm_id(&self) -> u16 {
        self.realm_id
    }

    /// Get the GUID generator test fixture.
    #[cfg(test)]
    pub fn guid_generator(&self) -> Option<&Arc<ObjectGuidGenerator>> {
        self.guid_generator.as_ref()
    }

    /// Set the session manager for ConnectTo flow.
    pub fn set_session_mgr(&mut self, mgr: Arc<SessionManager>) {
        self.session_mgr = Some(mgr);
    }

    /// Get the session manager reference.
    pub fn session_mgr(&self) -> Option<&Arc<SessionManager>> {
        self.session_mgr.as_ref()
    }

    pub(crate) fn is_addon_registered_like_cpp(&self, prefix: &str) -> bool {
        // C++ WorldSession::IsAddonRegistered: if the registration filter is
        // disabled (initial state or softcap exceeded), all prefixes pass.
        if !self.filter_addon_messages {
            return true;
        }

        !self.registered_addon_prefixes.is_empty()
            && self
                .registered_addon_prefixes
                .iter()
                .any(|registered| registered == prefix)
    }

    #[cfg(test)]
    pub(crate) fn seed_empty_seasonal_event_bucket_like_cpp(&mut self, event_id: u16) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.seasonal_quests.entry(event_id).or_default();
        });
    }

    /// Set the list of legitimate characters for this account.
    pub fn set_legit_characters(&mut self, guids: Vec<ObjectGuid>) {
        self.legit_characters = guids;
    }

    /// Check if a GUID is in the legit characters list.
    pub fn is_legit_character(&self, guid: &ObjectGuid) -> bool {
        self.legit_characters.contains(guid)
    }

    // ── Aura system ───────────────────────────────────────────────

    fn apply_represented_battle_pet_xp_pct_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        let multiplier = 1.0 + (effect.effect_base_points as f32 / 100.0);
        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::ModBattlePetXpPct),
            represented_amount: effect.effect_base_points,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: None,
            represented_multiplier: multiplier,
            applied_at: Instant::now(),
        };

        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        self.send_aura_update_applied(spell_id, slot, caster_guid, 30_000, 0x0000_0001, 0x1);

        Ok(())
    }

    fn update_player_collision_height_like_cpp(&mut self) {
        let Some((_, mount_display_id, object_scale)) =
            self.player_unit_presentation_snapshot_like_cpp()
        else {
            return;
        };
        let computed_height = if let (Some(display_store), Some(model_store)) = (
            self.creatures.display_info_store.as_ref(),
            self.creatures.model_data_store.as_ref(),
        ) {
            let native_display_id = crate::handlers::character::default_display_id(
                self.player_race_like_cpp(),
                self.player_gender_like_cpp(),
            );
            let mount_display_id = u32::try_from(mount_display_id).ok().filter(|id| *id != 0);
            wow_data::unit_collision_height_like_cpp(
                object_scale,
                native_display_id,
                mount_display_id,
                display_store,
                model_store,
            )
        } else {
            None
        };

        let mount_display_id = u32::try_from(mount_display_id).unwrap_or(0);
        let _canonical_height = self.with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_mount_display_id(mount_display_id);
            if let Some(height) = computed_height {
                unit.set_collision_height_like_cpp(height);
            }
            unit.collision_height_like_cpp()
        });
        #[cfg(test)]
        if let Some(height) = _canonical_height.or(computed_height)
            && (_canonical_height.is_some() || self.player_handle_like_cpp.is_none())
        {
            self.player_collision_height_like_cpp = height;
        }
    }

    pub(crate) fn reset_time_sync_like_cpp(&mut self) {
        self.time_sync_next_counter = 0;
        self.time_sync_pending_requests.clear();
    }

    pub(crate) fn record_time_sync_response_like_cpp(
        &mut self,
        sequence_index: u32,
        client_time: u32,
    ) {
        let Some(server_time_at_sent) = self.time_sync_pending_requests.remove(&sequence_index)
        else {
            return;
        };

        let received_time = crate::session_rules::game_time_ms_like_cpp();
        let round_trip_duration = received_time.wrapping_sub(server_time_at_sent);
        let lag_delay = round_trip_duration / 2;
        let clock_delta =
            i64::from(server_time_at_sent) + i64::from(lag_delay) - i64::from(client_time);
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            info!(
                account = self.account_id,
                sequence_index,
                client_time,
                server_time_at_sent,
                received_time,
                round_trip_duration,
                lag_delay,
                clock_delta,
                "RUST_LOGIN_TRACE time_sync_response"
            );
        }

        if self.time_sync_clock_delta_queue.len() == 6 {
            self.time_sync_clock_delta_queue.pop_front();
        }
        self.time_sync_clock_delta_queue
            .push_back((clock_delta, round_trip_duration));
        self.compute_new_clock_delta_like_cpp();
    }

    fn compute_new_clock_delta_like_cpp(&mut self) {
        if self.time_sync_clock_delta_queue.is_empty() {
            return;
        }

        let mut latencies: Vec<u32> = self
            .time_sync_clock_delta_queue
            .iter()
            .map(|(_, round_trip_duration)| *round_trip_duration)
            .collect();
        latencies.sort_unstable();
        let latency_median = rounded_median_u32(&latencies);
        let latency_mean =
            latencies.iter().map(|v| f64::from(*v)).sum::<f64>() / latencies.len() as f64;
        let latency_variance = latencies
            .iter()
            .map(|v| {
                let diff = f64::from(*v) - latency_mean;
                diff * diff
            })
            .sum::<f64>()
            / latencies.len() as f64;
        let latency_standard_deviation = latency_variance.sqrt().round() as u32;

        let latency_threshold = latency_standard_deviation.saturating_add(latency_median);
        let mut clock_delta_sum = 0i64;
        let mut sample_size_after_filtering = 0u32;
        for (clock_delta, round_trip_duration) in &self.time_sync_clock_delta_queue {
            if *round_trip_duration < latency_threshold {
                clock_delta_sum += *clock_delta;
                sample_size_after_filtering += 1;
            }
        }

        if sample_size_after_filtering != 0 {
            let mean_clock_delta =
                (clock_delta_sum as f64 / f64::from(sample_size_after_filtering)).round() as i64;
            if (mean_clock_delta - self.time_sync_clock_delta).abs() > 25 {
                self.time_sync_clock_delta = mean_clock_delta;
            }
        } else if self.time_sync_clock_delta == 0 {
            self.time_sync_clock_delta = self
                .time_sync_clock_delta_queue
                .back()
                .map(|(clock_delta, _)| *clock_delta)
                .unwrap_or_default();
        }
    }

    /// Share the process-wide trusted module registry with this session.
    ///
    /// Composition calls this once after construction. A session that never
    /// receives one keeps the zero-module no-op path.
    #[cfg(test)]
    pub fn set_module_registry_like_cpp(&mut self, registry: Arc<wow_module_api::ModuleRegistry>) {
        self.module_registry_like_cpp = Some(registry);
    }

    fn trinity_string_like_cpp(&self, entry: u32) -> &str {
        self.trinity_string_store
            .as_ref()
            .map(|store| store.get_like_cpp(entry, &self.locale))
            .unwrap_or("<error>")
    }

    /// Set the logged-in player GUID.
    pub fn set_player_guid(&mut self, guid: Option<ObjectGuid>) {
        let previous_player_guid = self.player_guid;
        let player_changed = self.player_guid != guid;
        if player_changed {
            let _ = self.mutate_player_world_local_state_like_cpp(|state| {
                state.zone_area_authority_complete = false;
            });
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.player_guid = guid;
        if player_changed {
            self.last_presented_creature_melee_health_state_revision_like_cpp = 0;
            // Visible auras and their completeness proof belong to the C++
            // Player, not the authenticated WorldSession. Clear both at the
            // identity boundary so a later character cannot inherit positive
            // or negative aura-spell authority from the previous one.
            let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.clear_runtime_applications_like_cpp();
                auras.reset_player_aura_source_authority_like_cpp();
            });
            self.begin_player_equipment_inventory_authority_load_like_cpp();
            #[cfg(test)]
            {
                self.player_quest_status_authority_complete_like_cpp = false;
                self.represented_rewarded_quest_rows_like_cpp.clear();
                self.represented_loaded_player_flags_like_cpp = None;
                self.represented_loaded_player_flags_ex_like_cpp = None;
                self.represented_loaded_player_flags_applied_like_cpp = false;
            }
            #[cfg(test)]
            {
                self.represented_guild_id_like_cpp = 0;
                self.represented_guild_id_authority_complete_like_cpp = false;
            }
            let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.trait_config_rows.clear();
                runtime.trait_config_rows_complete = false;
                runtime.trait_entry_rows_complete = false;
                runtime.trait_entry_rows_empty = false;
            });
            let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                state.character_rows_empty_authority_complete = false;
            });
            // C++ owns PlayerMenu (and therefore both InteractionData and its
            // menus) under Player. A WorldSession can survive character
            // logout, so no player-menu state may cross that lifetime here.
            self.reset_player_interaction_data_like_cpp();
            self.clear_player_gossip_options_like_cpp();
            #[cfg(test)]
            self.represented_spell_acquisition_post_commit_actions_like_cpp
                .clear();
            if previous_player_guid.is_some() {
                let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                    runtime.fallback_rows.clear();
                });
            }
            // `_SaveSkills` tombstones belong to the current C++ Player's
            // update-field slots, not to the authenticated WorldSession.
            if previous_player_guid.is_some() {
                self.clear_player_skill_tombstones_like_cpp();
            }
        }
        if let Some(guid) = guid {
            self.recent_player_guid_low_like_cpp = guid.counter() as u64;
            self.represented_seer_guid_like_cpp = Some(guid);
        }
        if guid.is_none() {
            #[cfg(test)]
            {
                self.player_bootstrap_attached_like_cpp = false;
            }
            self.represented_seer_guid_like_cpp = None;
            // Old registry clones remain permanently closed; a later character
            // selected on this authenticated session receives a fresh fence.
            self.durable_loot_money_persistence_like_cpp =
                Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        }
    }

    pub(crate) fn set_tutorial_int_like_cpp(&mut self, index: usize, value: u32) -> bool {
        let Some(current) = self.tutorials_like_cpp.get_mut(index) else {
            return false;
        };

        if *current != value {
            *current = value;
            self.tutorials_changed_like_cpp = true;
        }
        true
    }

    pub(crate) fn apply_tutorial_action_like_cpp(
        &mut self,
        action: u8,
        tutorial_bit: Option<u32>,
    ) -> bool {
        match action {
            wow_packet::packets::misc::TUTORIAL_ACTION_UPDATE_LIKE_CPP => {
                let Some(tutorial_bit) = tutorial_bit else {
                    return false;
                };
                let index = (tutorial_bit >> 5) as usize;
                if index >= self.tutorials_like_cpp.len() {
                    return false;
                }
                let flag = self.tutorials_like_cpp[index] | (1u32 << (tutorial_bit & 0x1F));
                self.set_tutorial_int_like_cpp(index, flag)
            }
            wow_packet::packets::misc::TUTORIAL_ACTION_CLEAR_LIKE_CPP => {
                for index in 0..self.tutorials_like_cpp.len() {
                    self.set_tutorial_int_like_cpp(index, u32::MAX);
                }
                true
            }
            wow_packet::packets::misc::TUTORIAL_ACTION_RESET_LIKE_CPP => {
                for index in 0..self.tutorials_like_cpp.len() {
                    self.set_tutorial_int_like_cpp(index, 0);
                }
                true
            }
            _ => false,
        }
    }

    /// C++ `Player::SetFactionForRace`: `Player::LoadFromDB` resolves the
    /// player's live faction template from `ChrRacesEntry::FactionID` before
    /// the player is added to the map or published through ObjectAccessor.
    fn set_player_faction_for_race_like_cpp(&mut self, race: u8) {
        let Some(chr_races_store) = self.chr.races_store.as_ref() else {
            return;
        };
        let faction_template = chr_races_store
            .get(u32::from(race))
            .and_then(|entry| u32::try_from(entry.faction_id).ok())
            .filter(|faction_template| *faction_template != 0);
        let faction_template = faction_template.unwrap_or(0);
        let _canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_faction(faction_template);
        });
        #[cfg(test)]
        if _canonical.is_some() || self.player_handle_like_cpp.is_none() {
            self.player_faction_template_like_cpp =
                (faction_template != 0).then_some(faction_template);
        }
    }

    pub(crate) fn attach_player_controller_like_cpp(
        &mut self,
        controller: SessionPlayerController,
    ) {
        let controller_position = controller.position();
        self.set_player_guid(Some(controller.guid()));
        self.player_name = Some(controller.name().to_string());
        #[cfg(test)]
        {
            self.player_position = Some(controller_position);
        }
        self.current_map_id = controller.map_id();
        self.player_race = controller.race();
        self.player_class = controller.class();
        self.player_level = controller.level();
        self.player_gender = controller.gender();
        self.represented_seer_guid_like_cpp = Some(controller.guid());
        #[cfg(test)]
        {
            self.player_bootstrap_attached_like_cpp = true;
        }
        self.initialize_reputation_mgr_like_cpp();
        self.set_fall_information_like_cpp(0, controller_position.z);
        // Production receives MapManager at session construction, so consume
        // the login bootstrap immediately. Unit fixtures historically inject
        // or replace their synthetic manager after attachment; they exercise
        // the same ownership transition through
        // `ensure_canonical_player_owner_for_map_like_cpp` instead.
        #[cfg(not(test))]
        let _ = self.install_detached_canonical_player_from_session_like_cpp(controller_position);
        self.set_player_moved_unit_guid_like_cpp(controller.guid());
    }

    pub(crate) fn set_player_liquid_status_like_cpp(&mut self, status: u32) {
        crate::canonical_player_sync::sync_player_liquid_status_like_cpp(self, status);
    }

    pub(crate) fn set_player_level_like_cpp(&mut self, level: u8) {
        self.player_level = level;
        let gray_level = self.gray_level(level);
        crate::canonical_player_sync::sync_player_level_like_cpp(self, level, gray_level);
        self.refresh_represented_talent_points_like_cpp();
    }

    #[cfg(test)]
    pub(crate) fn set_player_class_like_cpp(&mut self, class: u8) {
        if self.player_class_like_cpp() != class {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.player_class = class;
        self.refresh_represented_talent_points_like_cpp();
    }

    pub(crate) fn set_player_create_mode_like_cpp(&mut self, create_mode: u8) -> bool {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_create_mode_like_cpp(create_mode))
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.player_create_mode_like_cpp = create_mode;
            return true;
        }
        _canonical
    }

    pub(crate) fn set_player_gold_like_cpp(&mut self, gold: u64) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_money(gold))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_gold = gold;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    #[cfg(test)]
    pub(crate) fn set_player_character_points_like_cpp(&mut self, points: i32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_character_points_like_cpp(points);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_character_points_like_cpp = points;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    fn resolved_player_interaction_data_like_cpp(&self) -> Option<PlayerInteractionDataLikeCpp> {
        let canonical =
            self.with_owned_player_like_cpp(|player| *player.interaction_data_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_interaction_data_like_cpp);
        }
        canonical
    }

    pub(crate) fn reset_player_interaction_data_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.reset_interaction_data_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp.reset();
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_interaction_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_interaction_source_like_cpp(source_guid);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_source(source_guid);
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_trainer_interaction_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trainer_interaction_like_cpp(source_guid, trainer_id);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_trainer(source_guid, trainer_id);
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn reset_player_interaction_if_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.reset_interaction_if_source_like_cpp(source_guid)
        });
        #[cfg(test)]
        if canonical.is_some() || self.player_handle_like_cpp.is_none() {
            let fixture = self
                .player_interaction_data_like_cpp
                .reset_if_source(source_guid);
            return canonical.unwrap_or(fixture);
        }
        canonical.unwrap_or(false)
    }

    pub(crate) fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let interaction = self.resolved_player_interaction_data_like_cpp()?;
        (!interaction.source_guid.is_empty()).then_some(interaction.source_guid)
    }

    pub(crate) fn resolved_player_interaction_trainer_id_like_cpp(&self) -> Option<u32> {
        self.resolved_player_interaction_data_like_cpp()
            .map(|interaction| interaction.trainer_id)
    }

    #[cfg(test)]
    pub(crate) fn player_interaction_trainer_id_like_cpp(&self) -> u32 {
        self.resolved_player_interaction_trainer_id_like_cpp()
            .unwrap_or(0)
    }

    pub(crate) fn player_trainer_interaction_matches_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> bool {
        self.resolved_player_interaction_data_like_cpp()
            .is_some_and(|interaction| interaction.trainer_matches(source_guid, trainer_id))
    }

    pub(crate) fn set_player_xp_like_cpp(&mut self, xp: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let xp = xp.min(i32::MAX as u32) as i32;
                player.set_xp(xp);
                player.mark_xp_changed_like_cpp();
                let scaling_level_delta = if player.unit().data().level
                    < i32::from(WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP)
                    && xp < player.active_data().next_level_xp / 2
                {
                    -1
                } else {
                    0
                };
                player.set_scaling_player_level_delta_like_cpp(scaling_level_delta);
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_xp = xp;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_next_level_xp_like_cpp(&mut self, xp: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let next_level_xp = xp.min(i32::MAX as u32) as i32;
                player.set_next_level_xp(next_level_xp);
                // Rust hydrates the Character row before its XP table refresh,
                // while C++ has NextLevelXP ready before `SetXP`. Recompute the
                // dependent SetXP field here so the final canonical value is
                // independent of that transitional load ordering.
                let scaling_level_delta = if player.unit().data().level
                    < i32::from(WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP)
                    && player.active_data().xp < next_level_xp / 2
                {
                    -1
                } else {
                    0
                };
                player.set_scaling_player_level_delta_like_cpp(scaling_level_delta);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_next_level_xp = xp;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_selection_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_selection(guid.unwrap_or_default()))
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.selection_guid = guid;
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_represented_pet_mode_state_with_spell_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
        react_state: u8,
        command_state: u8,
        created_by_spell: u32,
    ) {
        if !self.set_player_pet_guid_like_cpp(pet_guid) {
            return;
        }
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let canonical = pet_guid.is_some_and(|pet_guid| {
            self.with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
                pet.set_created_by_spell_id_like_cpp(created_by_spell);
                pet.creature_mut()
                    .set_react_state(react_state_from_db_like_cpp(react_state));
                pet.creature_mut()
                    .unit_mut()
                    .subsystems_mut()
                    .control
                    .init_charm_info()
                    .command_state = command_state;
            })
            .is_some()
        });
        #[cfg(test)]
        if !canonical {
            self.represented_pet_created_by_spell_like_cpp = created_by_spell;
            self.represented_pet_react_state_like_cpp = react_state;
            self.represented_pet_command_state_like_cpp = command_state;
        }
        #[cfg(not(test))]
        let _ = canonical;
        if pet_guid.is_none() {
            let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                state.temporary_unsummoned_pet_number = 0;
                state.old_pet_spell = 0;
            });
            #[cfg(test)]
            {
                self.represented_pet_movement_speed_rates_like_cpp =
                    [1.0; UnitMoveTypeLikeCpp::COUNT];
            }
        }
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_mount_react_state = None;
        });
    }

    pub(crate) fn load_represented_pet_spell_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellRowLikeCpp>,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let spells: Vec<_> = rows.into_iter().filter(|row| row.spell_id != 0).collect();
        let loaded = spells.len();
        if loaded == 0 {
            self.pet_load_query_holder_rows_like_cpp
                .spells
                .remove(&pet_number);
        } else {
            self.pet_load_query_holder_rows_like_cpp
                .spells
                .insert(pet_number, spells);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_spell_cooldown_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellCooldownRowLikeCpp>,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let spell_store = self.spell_store().cloned();
        let cooldowns: Vec<_> = rows
            .into_iter()
            .filter(|row| {
                row.spell_id != 0
                    && spell_store
                        .as_ref()
                        .is_none_or(|store| store.get(row.spell_id as i32).is_some())
            })
            .collect();
        let loaded = cooldowns.len();
        if loaded == 0 {
            self.pet_load_query_holder_rows_like_cpp
                .spell_cooldowns
                .remove(&pet_number);
        } else {
            self.pet_load_query_holder_rows_like_cpp
                .spell_cooldowns
                .insert(pet_number, cooldowns);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_spell_charge_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellChargeRowLikeCpp>,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let spell_category_store = self.spell_catalogs.spell_category_store().cloned();
        let charges: Vec<_> = rows
            .into_iter()
            .filter(|row| {
                row.category_id != 0
                    && spell_category_store
                        .as_ref()
                        .is_none_or(|store| store.get(row.category_id).is_some())
            })
            .collect();
        let loaded = charges.len();
        if loaded == 0 {
            self.pet_load_query_holder_rows_like_cpp
                .spell_charges
                .remove(&pet_number);
        } else {
            self.pet_load_query_holder_rows_like_cpp
                .spell_charges
                .insert(pet_number, charges);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_aura_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
    ) -> usize {
        self.load_represented_pet_aura_rows_with_timediff_like_cpp(pet_number, rows, 0)
    }

    pub(crate) fn load_represented_pet_aura_rows_with_timediff_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let spell_store = self.spell_store().cloned();
        let difficulty_store = self.difficulty_store().cloned();
        let aura_options_store = self.spell_catalogs.spell_aura_options_store.clone();
        let spell_misc_store = self.spell_catalogs.spell_misc_store().cloned();
        let auras: Vec<_> = rows
            .into_iter()
            .filter(|row| {
                row.spell_id != 0
                    && spell_store
                        .as_ref()
                        .is_none_or(|store| store.get(row.spell_id as i32).is_some())
                    && (row.difficulty == 0
                        || difficulty_store
                            .as_ref()
                            .is_none_or(|store| store.contains(u32::from(row.difficulty))))
            })
            .filter_map(|mut row| {
                let aura_expires_offline = spell_misc_store
                    .as_ref()
                    .and_then(|store| store.get_by_spell_id(row.spell_id))
                    .is_some_and(|misc| {
                        (misc.attributes[4] as u32
                            & wow_data::spell::attributes::SPELL_ATTR4_AURA_EXPIRES_OFFLINE)
                            != 0
                    });
                if let Some(remain_time_ms) = adjusted_represented_pet_aura_remain_time_like_cpp(
                    row.remain_time_ms,
                    timediff_secs,
                    true,
                    aura_expires_offline,
                ) {
                    row.remain_time_ms = remain_time_ms;
                } else {
                    return None;
                }

                if let Some(store) = aura_options_store.as_ref() {
                    let proc_charges = store.proc_charges_like_cpp(row.spell_id, row.difficulty);
                    row.remain_charges = if proc_charges == 0 {
                        0
                    } else if row.remain_charges == 0 {
                        proc_charges
                    } else {
                        row.remain_charges
                    };
                }
                Some(row)
            })
            .collect();
        let loaded = auras.len();
        if loaded == 0 {
            self.pet_load_query_holder_rows_like_cpp
                .auras
                .remove(&pet_number);
        } else {
            self.pet_load_query_holder_rows_like_cpp
                .auras
                .insert(pet_number, auras);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_aura_effect_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraEffectRowLikeCpp>,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let effects: Vec<_> = rows
            .into_iter()
            .filter(|row| {
                row.spell_id != 0
                    && u32::from(row.effect_index)
                        < wow_data::conditions::MAX_SPELL_EFFECTS_LIKE_CPP
            })
            .collect();
        let loaded = effects.len();
        if loaded == 0 {
            self.pet_load_query_holder_rows_like_cpp
                .aura_effects
                .remove(&pet_number);
        } else {
            self.pet_load_query_holder_rows_like_cpp
                .aura_effects
                .insert(pet_number, effects);
        }
        loaded
    }

    pub(crate) fn mount_set_favorite_like_cpp(
        &mut self,
        mount_spell_id: u32,
        is_favorite: bool,
    ) -> bool {
        let Ok(spell_id) = i32::try_from(mount_spell_id) else {
            return false;
        };
        let Some(updated_flags) = self
            .mutate_player_collection_state_like_cpp(|collections| {
                let flags = collections.mounts.get_mut(&spell_id)?;
                if is_favorite {
                    *flags |= 0x01;
                } else {
                    *flags &= !0x01;
                }
                Some(*flags)
            })
            .flatten()
        else {
            return false;
        };

        self.send_packet(&AccountMountUpdate::partial(vec![AccountMount {
            spell_id,
            flags: updated_flags,
        }]));
        true
    }

    pub(crate) fn record_represented_titan_grip_penalty_action_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let main_template = self
                .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_MAINHAND)
                .and_then(|item| self.item_storage_template(item.entry_id));
            let off_template = self
                .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)
                .and_then(|item| self.item_storage_template(item.entry_id));
            let using_two_handed_weapon_in_one_hand =
                two_handed_in_one_hand_like_cpp(main_template.as_ref(), off_template.as_ref());

            let Some(action) = self.canonical_player_snapshot_like_cpp(|player| {
                let penalty_spell_id = player.titan_grip_penalty_spell_id();
                let has_penalty_aura = penalty_spell_id > 0
                    && self
                        .visible_auras
                        .values()
                        .any(|aura| aura.spell_id == penalty_spell_id as i32);

                player.check_titan_grip_penalty_action(
                    using_two_handed_weapon_in_one_hand,
                    has_penalty_aura,
                )
            }) else {
                return;
            };

            if action != TitanGripPenaltyAction::None {
                self.represented_titan_grip_penalty_actions_like_cpp
                    .push(action);
            }
        }
    }

    pub(crate) fn set_player_currencies_like_cpp(
        &mut self,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_currencies_like_cpp(currencies.clone());
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_currencies = currencies;
            return true;
        }
        canonical
    }

    pub(crate) fn clear_player_currencies_like_cpp(&mut self) -> bool {
        self.set_player_currencies_like_cpp(HashMap::new())
    }

    pub(crate) fn player_name_like_cpp(&self) -> Option<&str> {
        self.player_name.as_deref()
    }

    pub(crate) fn player_faction_template_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            u32::try_from(player.unit().data().faction_template)
                .ok()
                .filter(|faction| *faction != 0)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.player_faction_template_like_cpp;
        }
        canonical.flatten()
    }

    #[cfg(test)]
    pub(crate) fn represented_can_swim_to_fly_transition_like_cpp(&self) -> bool {
        self.resolved_can_swim_to_fly_transition_like_cpp()
            .expect("test Player movement owner must resolve")
    }

    fn resolved_can_swim_to_fly_transition_like_cpp(&self) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .movement_control
                .can_swim_to_fly_transition
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_can_swim_to_fly_transition_like_cpp);
        }
        canonical
    }

    fn resolved_player_scale_duration_like_cpp(&self) -> Option<i32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.gameplay_state().movement_control.scale_duration
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_scale_duration_like_cpp);
        }
        canonical
    }

    pub(crate) fn player_liquid_status_like_cpp(&self) -> Option<u32> {
        self.canonical_player_snapshot_like_cpp(|player| player.gameplay_state().liquid_status)
    }

    pub(crate) fn player_race_like_cpp(&self) -> u8 {
        self.player_race
    }

    pub(crate) fn player_class_like_cpp(&self) -> u8 {
        self.player_class
    }

    pub(crate) fn player_create_mode_like_cpp(&self) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(Player::create_mode_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_create_mode_like_cpp);
        }
        canonical
    }

    pub(crate) fn player_level_like_cpp(&self) -> u8 {
        self.player_level
    }

    pub(crate) fn player_gender_like_cpp(&self) -> u8 {
        self.player_gender
    }

    pub(crate) fn represented_shapeshift_form_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(Player::shapeshift_form_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_shapeshift_form_like_cpp);
        }
        canonical
    }

    pub(crate) fn set_represented_shapeshift_form_like_cpp(&mut self, form_id: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_shapeshift_form_id_like_cpp(form_id)
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_shapeshift_form_like_cpp = form_id;
            return true;
        }
        canonical
    }

    pub(crate) fn represented_primary_specialization_id_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(Player::primary_specialization_id_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_primary_specialization_id_like_cpp);
        }
        canonical
    }

    pub(crate) fn set_represented_primary_specialization_id_like_cpp(
        &mut self,
        spec_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_primary_specialization(spec_id))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_primary_specialization_id_like_cpp = spec_id;
            return true;
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_gold_like_cpp(&self) -> u64 {
        self.resolved_player_money_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_gold)
            })
            .expect("test Player money owner must resolve")
    }

    pub(crate) fn resolved_player_character_points_like_cpp(&self) -> Option<i32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.active_data().character_points);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_character_points_like_cpp);
        }
        canonical
    }

    pub(crate) fn resolved_player_xp_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.active_data().xp.max(0) as u32);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_xp);
        }
        canonical
    }

    fn resolved_player_xp_for_level_like_cpp(&self, level: u8) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.player_xp_for_level_like_cpp(level))
            .flatten();
        #[cfg(test)]
        if canonical.is_none() {
            return self
                .player_xp_table
                .as_ref()
                .and_then(|table| table.get(usize::from(level)).copied())
                .or_else(|| self.resolved_player_next_level_xp_like_cpp());
        }
        canonical
    }

    pub(crate) fn resolved_player_next_level_xp_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.active_data().next_level_xp.max(0) as u32);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_next_level_xp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_character_points_like_cpp(&self) -> i32 {
        self.resolved_player_character_points_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_character_points_like_cpp)
            })
            .expect("test Player progression owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn player_xp_like_cpp(&self) -> u32 {
        self.resolved_player_xp_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_xp)
            })
            .expect("test Player progression owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn player_next_level_xp_like_cpp(&self) -> u32 {
        self.resolved_player_next_level_xp_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_next_level_xp)
            })
            .expect("test Player progression owner must resolve")
    }

    #[allow(dead_code)]
    pub(crate) fn selection_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let canonical = self.with_owned_player_like_cpp(|player| player.unit().data().target);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.selection_guid;
        }
        canonical.filter(|guid| !guid.is_empty())
    }

    pub(crate) fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().currencies.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_currencies.clone());
        }
        canonical
    }

    pub(crate) fn represented_player_condition_context_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        let spells = self
            .known_spells_like_cpp()
            .iter()
            .filter_map(|spell_id| u32::try_from(*spell_id).ok())
            .collect();
        let items = self
            .represented_inventory_item_counts_like_cpp()?
            .into_iter()
            .map(|(id, count)| PlayerConditionCountLikeCpp { id, count })
            .collect();
        let currencies = self
            .player_currencies_like_cpp()?
            .iter()
            .map(|(&id, currency)| PlayerConditionCountLikeCpp {
                id,
                count: currency.quantity,
            })
            .collect();
        let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
        let completed_quests = quests.rewarded_quest_ids.iter().copied().collect();
        let current_quests = quests
            .statuses
            .iter()
            .filter_map(|(&quest_id, status)| {
                (status.status == crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    || status.status == crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                    .then_some(quest_id)
            })
            .collect();
        let complete_quests = quests
            .statuses
            .iter()
            .filter_map(|(&quest_id, status)| {
                (status.status == crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                    .then_some(quest_id)
            })
            .collect();
        let auras = self
            .resolved_player_visible_auras_like_cpp()?
            .into_values()
            .filter_map(|aura| {
                Some(PlayerConditionAuraLikeCpp {
                    spell_id: u32::try_from(aura.spell_id).ok()?,
                    stacks: aura.stack_count,
                })
            })
            .collect();
        let skills = self
            .resolved_player_skill_values_like_cpp()?
            .iter()
            .map(|(&id, &value)| PlayerConditionSkillLikeCpp { id, value })
            .collect();
        let (_, area_id) = self.player_zone_area_like_cpp()?;
        let explored_zones = self.player_explored_zones_snapshot_like_cpp()?;
        let (explored_area_ids, parent_area_ids) = self
            .area_table_store
            .as_ref()
            .map(|store| {
                (
                    store.explored_area_ids_from_blocks_like_cpp(&explored_zones),
                    store.parent_area_ids_like_cpp(area_id),
                )
            })
            .unwrap_or_default();

        let mut mainhand_weapon_subclass = None;
        for (&slot, inventory_item) in &self.resolved_inventory_items_like_cpp()? {
            if slot == EQUIPMENT_SLOT_MAINHAND {
                mainhand_weapon_subclass = self
                    .items
                    .store
                    .as_ref()
                    .and_then(|store| store.get(inventory_item.entry_id))
                    .map(|record| record.subclass_id);
            }
        }

        Some(RepresentedPlayerConditionContextLikeCpp {
            spells,
            items,
            currencies,
            completed_quests,
            current_quests,
            complete_quests,
            auras,
            skills,
            explored_area_ids,
            parent_area_ids,
            avg_item_level: self.represented_avg_total_item_level_like_cpp()?,
            avg_equipped_item_level: self.represented_avg_equipped_item_level_like_cpp()?,
            mainhand_weapon_subclass,
            ..Default::default()
        })
    }

    pub(crate) fn represented_meets_player_condition_id_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        if player_condition_id == 0 {
            return true;
        }

        let Some(store) = self.player_condition_store.as_ref() else {
            return false;
        };
        let Some(condition) = store.get(player_condition_id) else {
            return true;
        };

        let Some(context) = self.represented_player_condition_context_like_cpp() else {
            return false;
        };
        context
            .as_context(self)
            .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
    }

    #[allow(dead_code)]
    pub(crate) fn represented_mount_x_display_usable_like_cpp(
        &self,
        player_condition_id: u32,
    ) -> bool {
        self.represented_meets_player_condition_id_like_cpp(player_condition_id)
    }

    #[allow(dead_code)]
    fn represented_mount_capability_selection_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Result<wow_data::MountCapabilityEntry, wow_data::MountCapabilityRejectLikeCpp> {
        let capability_store = self
            .mount_capability_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingCapabilityRow)?;
        let type_store = self
            .mount_type_x_capability_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingMountTypeCapabilities)?;
        let area_store = self
            .area_table_store
            .as_ref()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;

        let map_id = u32::from(self.player_map_id_like_cpp());
        let map = self.maps.store.as_ref().and_then(|store| store.get(map_id));
        let (_, area_id) = self
            .player_zone_area_like_cpp()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;
        let mount_flags = mount_restriction_flags.unwrap_or_else(|| {
            area_store
                .get(area_id)
                .map(|area| area.mount_flags as u8)
                .unwrap_or_else(|| {
                    if area_id == 0 {
                        // C++ reaches this check after TerrainMgr resolved the
                        // player's area. Until Rust has that full terrain bridge,
                        // an area 0 placeholder should not make ordinary ground
                        // mounts fail SPELL_FAILED_NOT_HERE.
                        wow_data::AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS
                    } else {
                        0
                    }
                })
        });
        let context = wow_data::MountCapabilityContextLikeCpp {
            riding_skill,
            mount_flags,
            is_submerged,
            is_in_water,
            map_id: map_id as i32,
            cosmetic_parent_map_id: map
                .map(|entry| i32::from(entry.cosmetic_parent_map_id))
                .unwrap_or(-1),
            parent_map_id: map
                .map(|entry| i32::from(entry.parent_map_id))
                .unwrap_or(-1),
        };
        let visible_auras = self
            .resolved_player_visible_auras_like_cpp()
            .ok_or(wow_data::MountCapabilityRejectLikeCpp::Aura)?;

        capability_store
            .select_for_mount_type_with_reject_like_cpp(
                type_store,
                mount_type_id,
                &context,
                |required_area_id| {
                    area_store.is_in_area_like_cpp(area_id, u32::from(required_area_id))
                },
                |aura_id| {
                    visible_auras
                        .values()
                        .any(|aura| u32::try_from(aura.spell_id).ok() == Some(aura_id))
                },
                |spell_id| self.known_spells_like_cpp().contains(&spell_id),
            )
            .copied()
    }

    pub(crate) fn represented_mount_capability_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Option<wow_data::MountCapabilityEntry> {
        self.represented_mount_capability_selection_for_type_like_cpp(
            mount_type_id,
            riding_skill,
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
        .ok()
    }

    #[allow(dead_code)]
    pub(crate) fn represented_mount_capability_for_type_from_session_like_cpp(
        &self,
        mount_type_id: u16,
        mount_restriction_flags: Option<u8>,
    ) -> Option<wow_data::MountCapabilityEntry> {
        let (is_submerged, is_in_water) = self.represented_player_mount_liquid_state_like_cpp()?;
        self.represented_mount_capability_for_type_like_cpp(
            mount_type_id,
            u32::from(self.resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)?),
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
    }

    pub(crate) fn represented_player_mount_liquid_state_like_cpp(&self) -> Option<(bool, bool)> {
        let liquid_status = self.player_liquid_status_like_cpp()?;
        let is_submerged = liquid_status & LIQUID_MAP_UNDER_WATER_LIKE_CPP != 0
            || self
                .resolved_player_movement_flags_like_cpp()?
                .contains(MovementFlag::SWIMMING);
        let is_in_water =
            liquid_status & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP) != 0;
        Some((is_submerged, is_in_water))
    }

    pub(crate) fn resolved_buyback_price_like_cpp(&self) -> Option<[u32; BUYBACK_SLOT_COUNT]> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| *inventory.buyback_price())
    }

    pub(crate) fn resolved_buyback_timestamp_like_cpp(&self) -> Option<[i64; BUYBACK_SLOT_COUNT]> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| *inventory.buyback_timestamp())
    }

    pub(crate) fn resolved_current_buyback_slot_like_cpp(&self) -> Option<u8> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.current_buyback_slot())
    }

    #[cfg(test)]
    pub(crate) fn buyback_price_like_cpp(&self) -> &[u32; BUYBACK_SLOT_COUNT] {
        &self.buyback_price
    }

    #[cfg(test)]
    pub(crate) fn buyback_timestamp_like_cpp(&self) -> &[i64; BUYBACK_SLOT_COUNT] {
        &self.buyback_timestamp
    }

    #[cfg(test)]
    pub(crate) fn current_buyback_slot_like_cpp(&self) -> u8 {
        self.current_buyback_slot
    }

    pub fn set_player_alive_like_cpp(&mut self, alive: bool) {
        let resolved = self.with_owned_player_mut_like_cpp(|player| {
            if !alive {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
                player.unit_mut().set_health(0);
            } else {
                let max_health = player.unit().data().max_health.max(1);
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
                if player.unit().data().health == 0 {
                    player.unit_mut().set_health(max_health);
                }
            }
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player
                    .unit()
                    .data()
                    .max_health
                    .clamp(1, u64::from(u32::MAX)) as u32,
                player.unit().is_alive(),
            )
        });
        #[cfg(test)]
        {
            let (health, max_health, is_alive) = resolved.unwrap_or_else(|| {
                let max_health = self.player_max_health_like_cpp.max(1);
                let health = if alive {
                    self.player_health_like_cpp.max(1).min(max_health)
                } else {
                    0
                };
                (health, max_health, alive)
            });
            self.player_health_like_cpp = health;
            self.player_max_health_like_cpp = max_health;
            self.player_alive_like_cpp = is_alive;
        }
        if resolved.is_some() || cfg!(test) && self.player_handle_like_cpp.is_none() {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub(crate) fn resolved_player_vitals_like_cpp(&self) -> Option<(u32, u32, bool)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let health = player.unit().data().health.min(u64::from(max_health)) as u32;
            (health, max_health, player.unit().is_alive() && health > 0)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.player_health_like_cpp,
                self.player_max_health_like_cpp.max(1),
                self.player_alive_like_cpp && self.player_health_like_cpp > 0,
            ));
        }
        canonical
    }

    pub(crate) fn resolved_player_is_alive_like_cpp(&self) -> Option<bool> {
        self.resolved_player_vitals_like_cpp()
            .map(|(_, _, alive)| alive)
    }

    #[cfg(test)]
    pub(crate) fn player_is_alive_like_cpp(&self) -> bool {
        self.resolved_player_is_alive_like_cpp().unwrap()
    }

    pub(crate) fn player_has_ghost_flag_like_cpp(&self) -> bool {
        self.player_guid()
            .and_then(|guid| {
                self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_GHOST_LIKE_CPP)
            })
            .unwrap_or(false)
    }

    pub(crate) fn set_player_ghost_flag_like_cpp(&mut self, ghost: bool) {
        if self.player_guid().is_some() {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                if ghost {
                    player.set_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
                }
            });
        }
        self.sync_player_registry_state_like_cpp();
    }

    #[cfg(test)]
    pub(crate) fn set_player_faction_template_like_cpp(&mut self, faction_template: u32) {
        self.player_faction_template_like_cpp = (faction_template != 0).then_some(faction_template);
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_faction(faction_template);
        });
    }

    fn player_battleground_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerBattlegroundState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().battleground.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerBattlegroundState {
                represented_type_id: self.player_battleground_type_id_like_cpp,
                represented_map_id: self.player_battleground_map_id_like_cpp,
                represented_status: self.represented_battleground_status_like_cpp,
                represented_queue_slots: self.represented_battleground_queue_slots_like_cpp.clone(),
                arena_team_id_invited: self.represented_arena_team_id_invited_like_cpp,
                ..Default::default()
            });
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn set_player_battleground_type_id_like_cpp(&mut self, bg_type_id: u32) -> bool {
        self.mutate_player_battleground_state_like_cpp(|state| {
            state.represented_type_id = (bg_type_id != 0).then_some(bg_type_id);
        })
        .is_some()
    }

    #[cfg(test)]
    pub(crate) fn set_player_battleground_context_like_cpp(
        &mut self,
        bg_type_id: u32,
        bg_map_id: u32,
    ) -> bool {
        self.mutate_player_battleground_state_like_cpp(|state| {
            state.represented_type_id = (bg_type_id != 0).then_some(bg_type_id);
            state.represented_map_id = (bg_map_id != 0).then_some(bg_map_id);
        })
        .is_some()
    }

    pub(crate) fn set_represented_battleground_status_like_cpp(&mut self, status: Option<u8>) {
        let _ = self.mutate_player_battleground_state_like_cpp(|state| {
            state.represented_status = status;
        });
    }

    pub(crate) fn player_in_represented_battleground_like_cpp(&self) -> bool {
        self.player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.represented_type_id.is_some())
    }

    pub(crate) fn represented_battleground_status_is_wait_leave_like_cpp(&self) -> bool {
        self.player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.represented_status == Some(4))
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_hello_like_cpp(&mut self, unit: ObjectGuid) -> bool {
        let Some((npc_flags, entry)) =
            self.mutate_world_creature(unit, |creature| (creature.npc_flags(), creature.entry()))
        else {
            return false;
        };

        if (npc_flags & wow_constants::unit::NPCFlags1::BATTLE_MASTER.bits()) == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_battlemaster_hellos_like_cpp
            .push(RepresentedBattlemasterHelloLikeCpp { unit, entry });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlefield_list_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        list_id: i32,
    ) -> bool {
        let Ok(list_id) = u32::try_from(list_id) else {
            return false;
        };
        if battlemaster_lists.get(list_id).is_none() {
            return false;
        }

        #[cfg(test)]
        self.represented_battlefield_lists_like_cpp
            .push(RepresentedBattlefieldListLikeCpp { list_id });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        queue_ids: &[u64],
        roles: u8,
        blacklist_map: [i32; 2],
    ) -> bool {
        let Some(&packed_queue_id) = queue_ids.first() else {
            return false;
        };
        let queue_type_id = battleground_queue_type_id_from_packed_like_cpp(packed_queue_id);
        if !self.is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id) {
            return false;
        }
        if battlemaster_lists
            .is_internal_only_like_cpp(u32::from(queue_type_id.battlemaster_list_id))
        {
            return false;
        }
        if self
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    u32::from(queue_type_id.battlemaster_list_id),
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }
        if self.player_in_represented_battleground_like_cpp() {
            return false;
        }

        #[cfg(test)]
        self.represented_battlemaster_joins_like_cpp
            .push(RepresentedBattlemasterJoinLikeCpp {
                packed_queue_id,
                queue_type_id,
                roles,
                blacklist_map,
            });
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_arena_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        team_size_index: u8,
        roles: u8,
    ) -> bool {
        if self.player_in_represented_battleground_like_cpp() {
            return false;
        }

        let Some(arena_type) = arena_team_type_by_slot_like_cpp(team_size_index) else {
            return false;
        };
        let queue_type_id = RepresentedBattlegroundQueueTypeIdLikeCpp {
            battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
            queue_type: 1,
            rated: true,
            team_size: arena_type,
        };
        if !self.is_valid_battleground_queue_type_id_like_cpp(battlemaster_lists, queue_type_id) {
            return false;
        }
        if self
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    wow_data::BATTLEGROUND_AA_LIKE_CPP,
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }

        let (Some(player_guid), Some(group_guid), Some(group_registry)) = (
            self.player_guid(),
            self.resolved_group_guid_like_cpp(),
            self.group_registry.as_ref(),
        ) else {
            return false;
        };
        let is_group_leader = group_registry
            .get(&group_guid)
            .map(|group| {
                group.members.contains(&player_guid) && group.is_leader_like_cpp(player_guid)
            })
            .unwrap_or(false);
        if !is_group_leader {
            return false;
        }

        // C++ continues with Player::GetArenaTeamId, ArenaTeamMgr::GetArenaTeamById,
        // Group::CanJoinBattlegroundQueue, AddGroup and status packets. Rust
        // does not have the live rated-arena team/queue manager in this seam yet,
        // so the bounded port records the accepted intent after the representable
        // gates above without pretending that the queue was live.
        #[cfg(test)]
        self.represented_battlemaster_join_arenas_like_cpp.push(
            RepresentedBattlemasterJoinArenaLikeCpp {
                team_size_index,
                roles,
                arena_type,
                group_guid,
                queue_type_id,
            },
        );
        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlemaster_join_skirmish_like_cpp(
        &mut self,
        battlemaster_lists: &BattlemasterListStore,
        bg_type_id: u32,
        bracket_id: u32,
        as_group: u8,
        is_rated: u8,
    ) -> bool {
        if self.player_in_represented_battleground_like_cpp() {
            return false;
        }

        let arena_type = arena_skirmish_type_like_cpp(bg_type_id, bracket_id);
        let Some(entry) = battlemaster_lists.get(wow_data::BATTLEGROUND_AA_LIKE_CPP) else {
            return false;
        };
        if entry.instance_type != wow_data::MAP_ARENA_LIKE_CPP {
            return false;
        }
        if self
            .disable_mgr
            .as_ref()
            .map(|disable_mgr| {
                disable_mgr.is_disabled_for_like_cpp(
                    DISABLE_TYPE_BATTLEGROUND,
                    wow_data::BATTLEGROUND_AA_LIKE_CPP,
                    None,
                    0,
                    None,
                )
            })
            .unwrap_or(false)
        {
            return false;
        }

        let join_as_group = as_group != 0;
        let group_guid = if join_as_group {
            let (Some(player_guid), Some(group_guid), Some(group_registry)) = (
                self.player_guid(),
                self.resolved_group_guid_like_cpp(),
                self.group_registry.as_ref(),
            ) else {
                return false;
            };
            let is_group_leader = group_registry
                .get(&group_guid)
                .map(|group| {
                    group.members.contains(&player_guid) && group.is_leader_like_cpp(player_guid)
                })
                .unwrap_or(false);
            if !is_group_leader {
                return false;
            }
            Some(group_guid)
        } else {
            None
        };

        let queue_type_id = RepresentedBattlegroundQueueTypeIdLikeCpp {
            battlemaster_list_id: wow_data::BATTLEGROUND_AA_LIKE_CPP as u16,
            queue_type: 4,
            rated: false,
            team_size: arena_type,
        };

        // C++ continues with PVPDifficulty lookup, BattlegroundQueue::AddGroup,
        // solo queue-slot checks, Group::CanJoinBattlegroundQueue, status packet
        // fanout and ScheduleQueueUpdate. Rust records the bounded intent after
        // the currently represented gates without pretending that live queueing exists.
        #[cfg(test)]
        self.represented_battlemaster_join_skirmishes_like_cpp.push(
            RepresentedBattlemasterJoinSkirmishLikeCpp {
                bg_type_id,
                bracket_id,
                as_group: join_as_group,
                is_rated_packet_value: is_rated,
                arena_type,
                group_guid,
                queue_type_id,
            },
        );
        true
    }

    #[cfg(test)]
    pub(crate) fn add_represented_battleground_queue_slot_like_cpp(
        &mut self,
        slot: u32,
        queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
        invited_instance_guid: u32,
    ) {
        let _ = self.mutate_player_battleground_state_like_cpp(|state| {
            let updated = RepresentedBattlegroundQueueSlotLikeCpp {
                slot,
                queue_type_id,
                invited_instance_guid,
            };
            if let Some(existing) = state
                .represented_queue_slots
                .iter_mut()
                .find(|queued| queued.slot == slot)
            {
                *existing = updated;
            } else {
                state.represented_queue_slots.push(updated);
            }
        });
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn battlefield_port_like_cpp(
        &mut self,
        ticket: wow_packet::packets::misc::LfgRideTicket,
        accepted_invite: bool,
    ) -> bool {
        let Some(state) = self.player_battleground_state_snapshot_like_cpp() else {
            return false;
        };
        if state.represented_queue_slots.is_empty() {
            return false;
        }
        let Some(queued) = state
            .represented_queue_slots
            .iter()
            .copied()
            .find(|queued| queued.slot == ticket.id)
        else {
            return false;
        };
        if accepted_invite && queued.invited_instance_guid == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_battlefield_ports_like_cpp
            .push(RepresentedBattlefieldPortLikeCpp {
                ticket,
                accepted_invite,
                queue_type_id: queued.queue_type_id,
                invited_instance_guid: queued.invited_instance_guid,
            });
        true
    }

    fn is_valid_battleground_queue_type_id_like_cpp(
        &self,
        battlemaster_lists: &BattlemasterListStore,
        queue_type_id: RepresentedBattlegroundQueueTypeIdLikeCpp,
    ) -> bool {
        let Some(entry) = battlemaster_lists.get(u32::from(queue_type_id.battlemaster_list_id))
        else {
            return false;
        };

        match queue_type_id.queue_type {
            0 => {
                entry.instance_type == wow_data::MAP_BATTLEGROUND_LIKE_CPP
                    && queue_type_id.team_size == 0
            }
            1 => {
                entry.instance_type == wow_data::MAP_ARENA_LIKE_CPP
                    && queue_type_id.rated
                    && queue_type_id.team_size != 0
            }
            2 => !queue_type_id.rated,
            4 => {
                entry.instance_type == wow_data::MAP_ARENA_LIKE_CPP
                    && queue_type_id.rated
                    && queue_type_id.team_size == 3
            }
            _ => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_hellos_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterHelloLikeCpp] {
        &self.represented_battlemaster_hellos_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlefield_lists_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldListLikeCpp] {
        &self.represented_battlefield_lists_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_joins_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinLikeCpp] {
        &self.represented_battlemaster_joins_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_join_arenas_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinArenaLikeCpp] {
        &self.represented_battlemaster_join_arenas_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlemaster_join_skirmishes_like_cpp(
        &self,
    ) -> &[RepresentedBattlemasterJoinSkirmishLikeCpp] {
        &self.represented_battlemaster_join_skirmishes_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn represented_battlefield_ports_like_cpp(
        &self,
    ) -> &[RepresentedBattlefieldPortLikeCpp] {
        &self.represented_battlefield_ports_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn accept_represented_wargame_invite_like_cpp(&mut self, inviter_name: &str) {
        let (
            Some(player_guid),
            Some(player_group_guid),
            Some(player_registry),
            Some(group_registry),
        ) = (
            self.player_guid(),
            self.resolved_group_guid_like_cpp(),
            self.player_registry.as_ref(),
            self.group_registry.as_ref(),
        )
        else {
            return;
        };

        let Some(inviter) = player_registry.social_recipient_by_name(inviter_name) else {
            return;
        };
        let inviter_guid = inviter.guid;

        let Some(player_group) = group_registry.get(&player_group_guid) else {
            return;
        };
        if !player_group.members.contains(&player_guid) {
            return;
        }
        let player_group_size = player_group.members.len();
        drop(player_group);

        let Some(inviter_group) = group_registry
            .snapshots()
            .into_iter()
            .find(|group| group.members.contains(&inviter_guid))
        else {
            return;
        };
        let inviter_group_guid = inviter_group.group_guid;
        let inviter_group_size = inviter_group.members.len();

        if player_group_size != inviter_group_size {
            return;
        }

        #[cfg(test)]
        self.represented_wargame_invite_acceptances_like_cpp.push(
            RepresentedWargameInviteAcceptanceLikeCpp {
                inviter_name: inviter_name.to_string(),
                inviter_guid,
                player_group_guid,
                inviter_group_guid,
                group_size: player_group_size,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn represented_wargame_invite_acceptances_like_cpp(
        &self,
    ) -> &[RepresentedWargameInviteAcceptanceLikeCpp] {
        &self.represented_wargame_invite_acceptances_like_cpp
    }

    #[cfg(test)]
    pub(crate) fn set_player_game_master_like_cpp(&mut self, is_game_master: bool) {
        let mut canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_game_master_like_cpp(is_game_master)
            })
            .is_some();
        if !canonical
            && self.player_handle_like_cpp.is_none()
            && let Some(guid) = self.player_guid()
        {
            canonical = self
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.set_game_master_like_cpp(is_game_master)
                })
                .is_some();
        }
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_game_master_like_cpp = is_game_master;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_player_mounted_like_cpp(&mut self, mounted: bool) {
        let display_id = if mounted { 1 } else { 0 };
        let _ = self.set_player_mount_presentation_like_cpp(display_id, mounted);
    }

    fn resolved_player_mounted_like_cpp(&self) -> Option<bool> {
        self.player_unit_presentation_snapshot_like_cpp()
            .map(|(flags, _, _)| flags.contains(UnitFlags::MOUNT))
    }

    #[cfg(test)]
    pub(crate) fn player_mounted_like_cpp(&self) -> bool {
        self.resolved_player_mounted_like_cpp()
            .expect("test Player presentation owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn set_player_cheat_god_like_cpp(&mut self, enabled: bool) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_cheat_god_like_cpp(enabled))
            .is_some();
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_cheat_god_like_cpp = enabled;
        }
    }

    fn resolved_represented_total_stat_multiplier_for_stat_like_cpp(
        &self,
        stat: usize,
        uses_misc_value_b: bool,
    ) -> Option<f32> {
        let spell_store = self.spell_store()?;
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let aura_type = wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE;
        let mut multiplier = 1.0f32;
        let mut same_effect_spell_groups = BTreeMap::<u32, i32>::new();

        for aura in visible_auras.values() {
            let Some(spell) = spell_store.get(aura.spell_id) else {
                continue;
            };

            for effect in spell.effects().iter().filter(|effect| {
                1u32.checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                    && effect.effect_aura == aura_type
                    && if uses_misc_value_b {
                        effect.effect_misc_value_2 == 0
                            || effect.effect_misc_value_2 & (1 << stat) != 0
                    } else {
                        effect.effect_misc_value_1 == -1
                            || effect.effect_misc_value_1 == stat as i32
                    }
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| u32::from(represented.effect_index) == effect.effect_index)
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());

                let same_effect_group = self
                    .spell_spell_group_map_bounds_like_cpp(aura.spell_id as u32)
                    .iter()
                    .copied()
                    .find(|group_id| {
                        self.same_effect_stack_rule_aura_types_like_cpp(*group_id)
                            .is_some_and(|aura_types| aura_types.contains(&aura_type))
                    });
                if let Some(group_id) = same_effect_group {
                    same_effect_spell_groups
                        .entry(group_id)
                        .and_modify(|current| {
                            if current.unsigned_abs() < amount.unsigned_abs() {
                                *current = amount;
                            }
                        })
                        .or_insert(amount);
                } else {
                    multiplier += multiplier * amount as f32 / 100.0;
                }
            }
        }

        for amount in same_effect_spell_groups.into_values() {
            multiplier += multiplier * amount as f32 / 100.0;
        }
        Some(multiplier)
    }

    pub(crate) fn resolved_represented_total_stat_multipliers_like_cpp(&self) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier =
                self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, true)?;
        }
        Some(multipliers)
    }

    pub(crate) fn resolved_represented_total_stat_buff_multipliers_like_cpp(
        &self,
    ) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier =
                self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, false)?;
        }
        Some(multipliers)
    }

    #[cfg(test)]
    pub(crate) fn represented_total_stat_multipliers_like_cpp(&self) -> [f32; 5] {
        self.resolved_represented_total_stat_multipliers_like_cpp()
            .expect("test Player aura owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_total_stat_buff_multipliers_like_cpp(&self) -> [f32; 5] {
        self.resolved_represented_total_stat_buff_multipliers_like_cpp()
            .expect("test Player aura owner must resolve")
    }

    pub(crate) fn player_min_height_like_cpp(&self, position: wow_core::Position) -> f32 {
        let map_id = self.player_map_id_like_cpp();
        self.map_manager
            .as_ref()
            .and_then(|manager| {
                manager
                    .read()
                    .ok()
                    .map(|manager| manager.min_height_like_cpp(map_id, 0, position.x, position.y))
            })
            .unwrap_or(crate::map_manager::DEFAULT_MIN_HEIGHT_LIKE_CPP)
    }

    #[cfg(test)]
    pub(crate) fn player_out_of_bounds_like_cpp(&self) -> bool {
        self.player_out_of_bounds_like_cpp
    }

    pub(crate) fn set_player_stand_state_like_cpp(&mut self, state: UnitStandStateType) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_stand_state_like_cpp(state)
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.player_stand_state_like_cpp = state;
        }
    }

    /// Session-owned represented->live boundary.
    ///
    /// Packet handlers construct a typed intent only after completing their
    /// C++ validation. The bridge applies against the authoritative live owner
    /// first and records evidence only when that application succeeds.
    pub(crate) fn apply_represented_live_intent_like_cpp(
        &mut self,
        intent: RepresentedLiveIntentLikeCpp,
    ) -> RepresentedLiveIntentApplyOutcomeLikeCpp {
        let outcome = match &intent {
            RepresentedLiveIntentLikeCpp::StandStateChanged(change) => {
                self.apply_represented_stand_state_changed_live_like_cpp(change)
            }
        };

        if matches!(
            outcome,
            RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(_)
        ) {
            debug!(?intent, ?outcome, "represented->live intent applied");
            #[cfg(test)]
            self.represented_live_applications_like_cpp
                .push(RepresentedLiveApplicationLikeCpp { intent, outcome });
        }

        outcome
    }

    /// C++ `Unit::SetStandState` (`Unit.cpp:9966-9977`).
    ///
    /// The canonical `Player::Unit` is the live owner. Session-local stand and
    /// aura state remain mirrors until their remaining consumers migrate.
    fn apply_represented_stand_state_changed_live_like_cpp(
        &mut self,
        change: &RepresentedStandStateChangedLikeCpp,
    ) -> RepresentedLiveIntentApplyOutcomeLikeCpp {
        let Some(player_guid) = self.player_guid() else {
            return RepresentedLiveIntentApplyOutcomeLikeCpp::RejectedMissingPlayer;
        };

        let state = change.state;
        let spell_store = self.spell_catalogs.spell_store.as_ref().map(Arc::clone);
        let difficulty_store = self.difficulty_store.as_ref().map(Arc::clone);
        let Some(represented_visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return RepresentedLiveIntentApplyOutcomeLikeCpp::RejectedMissingCanonicalPlayer;
        };
        let spell_difficulty_id = self
            .current_canonical_player_map_difficulty_id_like_cpp()
            .unwrap_or(0);
        let standing_flag = wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP;
        let Some((
            canonical_field_changed,
            canonical_removed_auras,
            canonical_removed_visible_slots,
            mut channel_cancellation_boundary,
            canonical_interrupted_spell_ids,
        )) = self.mutate_canonical_player_like_cpp(|player| {
            let previous = player.unit().stand_state_like_cpp();
            let unit = player.unit_mut();
            unit.set_stand_state_like_cpp(state);

            let mut removed_auras = Vec::new();
            let mut removed_visible_slots = Vec::new();
            if unit.is_stand_state_like_cpp() {
                // Keep the locally registered masks as a
                // fallback for transitional/test state, and use the real
                // SpellInterrupts.db2 metadata for live auras whose older
                // represented materializers still carry zero masks.
                let metadata_matches: Vec<_> = {
                    let auras = &unit.subsystems().auras;
                    auras
                        .applied_auras
                        .iter()
                        .copied()
                        .filter(|aura| {
                            if let Some(known) = auras.aura_interrupt_flags.get(aura) {
                                // Presence means this applied aura already
                                // carries resolved metadata, including a known
                                // non-Standing mask. Never override it with a
                                // DB2 row selected from the current map.
                                return known.0 & standing_flag != 0;
                            }
                            let store_matches = i32::try_from(aura.spell_id)
                                .ok()
                                .and_then(|spell_id| {
                                    spell_store
                                        .as_ref()?
                                        .aura_interrupt_flags_for_difficulty_like_cpp(
                                            spell_id,
                                            spell_difficulty_id,
                                            difficulty_store.as_deref(),
                                        )
                                })
                                .is_some_and(|known| known[0] & standing_flag != 0);
                            // No canonical entry means this older materializer
                            // supplied no interrupt metadata; hydrate only that
                            // missing case through the current difficulty's C++
                            // fallback chain.
                            store_matches
                        })
                        .collect()
                };
                for aura in metadata_matches {
                    let was_visible = unit
                        .subsystems()
                        .auras
                        .visible_auras
                        .get(&aura.slot)
                        .is_some_and(|visible| *visible == aura.aura_ref());
                    if unit
                        .subsystems_mut()
                        .auras
                        .unapply_aura(aura, wow_entities::AURA_REMOVE_BY_INTERRUPT_LIKE_CPP)
                    {
                        // C++ Unit::RemoveAura removes the locally owned Aura
                        // base after unapplying its application when this Unit
                        // is the owner. Presence in the local owned store is
                        // the bounded owner identity available to this model.
                        // Cross-Unit applications still require the full Aura
                        // runtime/fanout tracked by the represented boundary.
                        unit.subsystems_mut()
                            .auras
                            .remove_owned_by_aura_ref_like_cpp(aura.aura_ref());
                        removed_auras.push(aura);
                        if was_visible {
                            unit.subsystems_mut().auras.clear_visible(aura.slot);
                            removed_visible_slots.push(aura.slot);
                        }
                    }
                }
            } else {
                debug_assert!(removed_auras.is_empty());
            }

            // C++ evaluates the current channel only after the Standing aura
            // removal loop and calls InterruptNonMeleeSpells(false). Reuse the
            // canonical Unit implementation so generic, autorepeat, and
            // channeled slots follow the same interruption ordering. Packet
            // and owned-effect cleanup still remains a typed boundary.
            let mut interrupted_spell_ids = Vec::new();
            let channel_cancellation_boundary = if unit.is_stand_state_like_cpp()
                && let Some(current) = unit.current_spell(wow_entities::CurrentSpellSlot::Channeled)
                && current.state == wow_constants::SpellState::Casting
            {
                let channel_flags = i32::try_from(current.spell_id).ok().and_then(|spell_id| {
                    spell_store
                        .as_ref()?
                        .channel_interrupt_flags_for_difficulty_like_cpp(
                            spell_id,
                            spell_difficulty_id,
                            difficulty_store.as_deref(),
                        )
                });
                match channel_flags {
                    Some(flags) if flags[0] & standing_flag != 0 => {
                        let interrupted = unit.interrupt_non_melee_spells(None, false, true);
                        interrupted_spell_ids
                            .extend(interrupted.iter().map(|(_, spell)| spell.spell_id));
                        Some(RepresentedStandChannelCancellationBoundary::Interrupted {
                            spell_id: current.spell_id,
                            canonical_spells_interrupted: interrupted.len(),
                            session_cast_interrupted: false,
                        })
                    }
                    Some(_) => None,
                    None => Some(
                        RepresentedStandChannelCancellationBoundary::UnknownInterruptMetadata {
                            spell_id: current.spell_id,
                        },
                    ),
                }
            } else {
                None
            };
            (
                previous != state,
                removed_auras,
                removed_visible_slots,
                channel_cancellation_boundary,
                interrupted_spell_ids,
            )
        })
        else {
            return RepresentedLiveIntentApplyOutcomeLikeCpp::RejectedMissingCanonicalPlayer;
        };

        let interrupted_cast = self
            .mutate_cast_execution_like_cpp(|execution| {
                let interrupted = execution.active.as_ref().is_some_and(|active| {
                    u32::try_from(active.spell_id)
                        .ok()
                        .is_some_and(|spell_id| canonical_interrupted_spell_ids.contains(&spell_id))
                });
                if interrupted {
                    execution.take_interrupted_cast(None)
                } else {
                    None
                }
            })
            .flatten();
        let session_cast_interrupted = interrupted_cast.is_some();
        if let Some(cast) = interrupted_cast {
            self.publish_player_cast_interruption_like_cpp(cast);
        }
        if let Some(RepresentedStandChannelCancellationBoundary::Interrupted {
            session_cast_interrupted: recorded,
            ..
        }) = &mut channel_cancellation_boundary
        {
            *recorded = session_cast_interrupted;
        }

        let mut represented_removed_slots = Vec::new();
        if state == UnitStandStateType::Stand {
            represented_removed_slots.extend(
                represented_visible_auras
                    .values()
                    .filter(|aura| {
                        let snapshot_matches = aura.aura_interrupt_flags & standing_flag != 0;
                        if aura.aura_interrupt_flags != 0 || aura.aura_interrupt_flags2 != 0 {
                            return snapshot_matches;
                        }
                        let store_matches = spell_store
                            .as_ref()
                            .and_then(|store| {
                                store.aura_interrupt_flags_for_difficulty_like_cpp(
                                    aura.spell_id,
                                    spell_difficulty_id,
                                    difficulty_store.as_deref(),
                                )
                            })
                            .is_some_and(|known| known[0] & standing_flag != 0);
                        store_matches
                    })
                    .map(|aura| aura.slot),
            );
            represented_removed_slots.sort_unstable();
            for slot in represented_removed_slots.iter().copied() {
                let _ = self.remove_aura(slot);
            }
        }

        self.set_player_stand_state_like_cpp(state);

        use wow_packet::ServerPacket;
        let mut removed_slots: Vec<u8> = canonical_removed_visible_slots
            .into_iter()
            .chain(represented_removed_slots.iter().copied())
            .collect();
        removed_slots.sort_unstable();
        removed_slots.dedup();
        for slot in removed_slots {
            let aura_update = wow_packet::packets::misc::AuraUpdate {
                unit_guid: player_guid,
                update_all: false,
                auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
                    slot,
                    aura_data: None,
                }],
            };
            if !represented_removed_slots.contains(&slot) {
                self.send_packet(&aura_update);
            }
            self.broadcast_to_movement_set_like_cpp(aura_update.to_bytes(), false);
        }

        // Opcodes.cpp registers SMSG_STAND_STATE_UPDATE on
        // CONNECTION_TYPE_REALM even though the CMSG is handled in-world.
        self.send_packet_realm(&wow_packet::packets::misc::StandStateUpdate {
            anim_kit_id: 0,
            stand_state: state as u8,
        });

        // `UnitData::StandState` is `UpdateField<uint8, 32, 56>` in the C++
        // generated fields. Until map-owned SendObjectUpdates has real fanout,
        // mirror this one live delta through the existing visibility registry.
        // Keep the canonical dirty bit set: only canonical object-update
        // processing may consume it, including when session routing is absent.
        if canonical_field_changed {
            let mut values = wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
            values.unit_data_mask[1] = (1 << (32 - 32)) | (1 << (56 - 32));
            values.stand_state = state as u8;
            let update = wow_packet::packets::update::UpdateObject::unit_values_update(
                player_guid,
                self.player_map_id_like_cpp(),
                values,
            );
            let bytes = update.to_bytes();
            self.send_raw_packet(&bytes);
            self.broadcast_to_movement_set_like_cpp(bytes, false);
        }

        RepresentedLiveIntentApplyOutcomeLikeCpp::Applied(
            RepresentedLiveIntentAppliedLikeCpp::StandStateChanged {
                canonical_field_changed,
                canonical_auras_removed: canonical_removed_auras.len(),
                represented_auras_removed: represented_removed_slots.len(),
                channel_cancellation_boundary,
            },
        )
    }

    #[cfg(test)]
    pub(crate) fn represented_live_applications_like_cpp(
        &self,
    ) -> &[RepresentedLiveApplicationLikeCpp] {
        &self.represented_live_applications_like_cpp
    }

    fn resolved_player_stand_state_like_cpp(&self) -> Option<UnitStandStateType> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().stand_state_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_stand_state_like_cpp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_stand_state_like_cpp(&self) -> UnitStandStateType {
        self.resolved_player_stand_state_like_cpp()
            .expect("test Player stand-state owner must resolve")
    }

    pub(crate) fn player_is_sit_state_like_cpp(&self) -> bool {
        self.resolved_player_stand_state_like_cpp()
            .is_some_and(|state| {
                matches!(
                    state,
                    UnitStandStateType::Sit
                        | UnitStandStateType::SitChair
                        | UnitStandStateType::SitLowChair
                        | UnitStandStateType::SitMediumChair
                        | UnitStandStateType::SitHighChair
                )
            })
    }

    pub(crate) fn represented_is_on_barber_chair_like_cpp(&self) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(current_stand_state) = self
            .resolved_player_stand_state_like_cpp()
            .and_then(|state| num_traits::ToPrimitive::to_u32(&state))
        else {
            return false;
        };

        self.represented_gameobject_use_effects
            .iter()
            .rev()
            .any(|effect| {
                matches!(
                    effect,
                    RepresentedGameObjectUseEffect::BarberChairUsed {
                        player_guid: effect_player_guid,
                        stand_state,
                        ..
                    } if *effect_player_guid == player_guid && *stand_state == current_stand_state
                )
            })
    }

    #[cfg(test)]
    pub(crate) fn represented_titan_grip_penalty_actions_like_cpp(
        &self,
    ) -> &[TitanGripPenaltyAction] {
        &self.represented_titan_grip_penalty_actions_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn calendar_community_invite_like_cpp(
        &mut self,
        min_level: u8,
        max_level: u8,
        max_rank_order: u8,
    ) -> bool {
        let Some(guild_id) = self.resolved_represented_guild_id_like_cpp() else {
            return false;
        };
        if guild_id == 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_calendar_community_invites_like_cpp.push(
            RepresentedCalendarCommunityInviteLikeCpp {
                guild_id,
                min_level,
                max_level,
                max_rank_order,
            },
        );
        true
    }

    #[cfg(test)]
    pub(crate) fn represented_calendar_community_invites_like_cpp(
        &self,
    ) -> &[RepresentedCalendarCommunityInviteLikeCpp] {
        &self.represented_calendar_community_invites_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn calendar_add_event_like_cpp(
        &mut self,
        club_id: u64,
        event_type: u8,
        texture_id: i32,
        time_packed: u32,
        flags: u32,
        invite_count: usize,
        title: String,
        description: String,
        max_size: u32,
    ) -> bool {
        const CALENDAR_FLAG_WITHOUT_INVITES_LIKE_CPP: u32 = 0x040;
        const CALENDAR_FLAG_GUILD_EVENT_LIKE_CPP: u32 = 0x400;

        let guild_scoped = (flags
            & (CALENDAR_FLAG_GUILD_EVENT_LIKE_CPP | CALENDAR_FLAG_WITHOUT_INVITES_LIKE_CPP))
            != 0;
        let guild_id = if guild_scoped {
            let Some(resolved_guild_id) = self.resolved_represented_guild_id_like_cpp() else {
                return false;
            };
            if resolved_guild_id == 0 {
                return false;
            }
            Some(resolved_guild_id)
        } else {
            None
        };

        #[cfg(test)]
        self.represented_calendar_add_events_like_cpp
            .push(RepresentedCalendarAddEventLikeCpp {
                guild_id,
                club_id,
                event_type,
                texture_id,
                time_packed,
                flags,
                invite_count,
                title,
                description,
                max_size,
            });
        true
    }

    #[cfg(test)]
    pub(crate) fn represented_calendar_add_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarAddEventLikeCpp] {
        &self.represented_calendar_add_events_like_cpp
    }

    pub(crate) fn set_represented_arena_team_id_invited_like_cpp(
        &mut self,
        arena_team_id: u32,
    ) -> bool {
        self.mutate_player_battleground_state_like_cpp(|state| {
            state.arena_team_id_invited = arena_team_id;
        })
        .is_some()
    }

    #[cfg(test)]
    pub(crate) fn represented_arena_team_id_invited_like_cpp(&self) -> u32 {
        self.player_battleground_state_snapshot_like_cpp()
            .expect("test Player battleground owner must resolve")
            .arena_team_id_invited
    }

    #[cfg(test)]
    pub(crate) fn represented_trade_item_like_cpp(&self, slot: u8) -> Option<ObjectGuid> {
        (slot < TRADE_SLOT_COUNT_LIKE_CPP)
            .then(|| {
                self.player_trade_state_snapshot_like_cpp()
                    .flatten()
                    .and_then(|state| state.items[slot as usize])
            })
            .flatten()
    }

    pub(crate) fn clear_represented_trade_item_like_cpp(&mut self, trade_slot: u8) {
        use wow_packet::ServerPacket;

        let Some(Some(mut trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        trade.client_state_index = trade.client_state_index.wrapping_add(1);

        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            let _ = self.mutate_player_trade_state_like_cpp(|state| *state = Some(trade));
            return;
        }

        let slot = trade_slot as usize;
        if trade.items[slot].is_none() {
            let _ = self.mutate_player_trade_state_like_cpp(|state| *state = Some(trade));
            return;
        }

        trade.items[slot] = None;
        trade.accepted = false;
        trade.server_state_index = trade.server_state_index.wrapping_add(1);
        if self
            .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
            .is_none()
        {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.send_raw_packet(&packet_bytes);

        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::UnacceptRepresentedTradeLikeCpp(
                crate::session::mailbox::UnacceptRepresentedTradeLikeCppCommand { packet_bytes },
            ),
        );
    }

    pub(crate) fn set_represented_trade_item_like_cpp(
        &mut self,
        trade_slot: u8,
        pack_slot: u8,
        item_slot_in_pack: u8,
    ) {
        use wow_packet::ServerPacket;

        let Some(Some(mut trade)) = self.player_trade_state_snapshot_like_cpp() else {
            return;
        };
        let partner_guid = trade.partner_guid;

        if trade_slot >= TRADE_SLOT_COUNT_LIKE_CPP {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        }

        let Some(item) = self.get_inventory_item_by_pos(pack_slot, item_slot_in_pack) else {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        };

        if trade.items.contains(&Some(item.guid)) {
            let packet_bytes =
                TradeStatus::cancel_like_cpp(TRADE_STATUS_CANCELLED_LIKE_CPP).to_bytes();
            self.send_raw_packet(&packet_bytes);
            return;
        }

        trade.client_state_index = trade.client_state_index.wrapping_add(1);
        trade.items[trade_slot as usize] = Some(item.guid);
        trade.accepted = false;
        trade.server_state_index = trade.server_state_index.wrapping_add(1);
        if self
            .mutate_player_trade_state_like_cpp(|state| *state = Some(trade))
            .is_none()
        {
            return;
        }

        let packet_bytes =
            TradeStatus::status_only_like_cpp(TRADE_STATUS_UNACCEPTED_LIKE_CPP).to_bytes();
        self.send_raw_packet(&packet_bytes);

        self.try_send_connected_player_command_like_cpp(
            partner_guid,
            SessionCommand::UnacceptRepresentedTradeLikeCpp(
                crate::session::mailbox::UnacceptRepresentedTradeLikeCppCommand { packet_bytes },
            ),
        );
    }

    #[cfg(test)]
    pub(crate) fn represented_force_deselects_like_cpp(
        &self,
    ) -> &[RepresentedForceDeselectLikeCpp] {
        &self.represented_force_deselects_like_cpp
    }

    pub(crate) fn represented_eject_passenger_like_cpp(
        &mut self,
        passenger_guid: ObjectGuid,
    ) -> bool {
        if !passenger_guid.is_unit() {
            return false;
        }

        let passenger_type_id = if passenger_guid.is_player() {
            TypeId::Player
        } else {
            TypeId::Unit
        };
        self.mutate_player_mount_vehicle_kit_like_cpp(|kit| {
            let Some(vehicle_kit) = kit.as_mut() else {
                return false;
            };
            if !vehicle_kit
                .seat_info_for_passenger_like_cpp(passenger_guid)
                .is_some_and(|seat| seat.ejectable)
            {
                return false;
            }
            vehicle_kit
                .remove_passenger_plan_like_cpp(
                    passenger_guid,
                    passenger_type_id,
                    false,
                    false,
                    false,
                    false,
                )
                .is_some()
        })
        .unwrap_or(false)
    }

    fn has_recently_dropped_flag_debuff_like_cpp(&self) -> Option<bool> {
        const SPELL_RECENTLY_DROPPED_ALLIANCE_FLAG: i32 = 42_792;
        const SPELL_RECENTLY_DROPPED_HORDE_FLAG: i32 = 50_326;
        const SPELL_RECENTLY_DROPPED_NEUTRAL_FLAG: i32 = 50_327;

        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras.values().any(|aura| {
                matches!(
                    aura.spell_id,
                    SPELL_RECENTLY_DROPPED_ALLIANCE_FLAG
                        | SPELL_RECENTLY_DROPPED_HORDE_FLAG
                        | SPELL_RECENTLY_DROPPED_NEUTRAL_FLAG
                )
            })
        })
    }

    pub(crate) fn represented_player_can_use_battleground_object_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let gameobject_faction = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.faction_template);
        if let (Some(player_faction), Some(gameobject_faction), Some(store)) = (
            self.player_faction_template_id_like_cpp(),
            gameobject_faction,
            self.factions.template_store.as_ref(),
        ) && let (Some(player_entry), Some(gameobject_entry)) =
            (store.get(player_faction), store.get(gameobject_faction))
            && !player_entry.is_friendly_to_like_cpp(gameobject_entry)
        {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::UnfriendlyFaction,
                },
            );
            return false;
        }

        let Some((player_unit_flags, _, _)) = self.player_unit_presentation_snapshot_like_cpp()
        else {
            return false;
        };
        if player_unit_flags.contains(UnitFlags::IMMUNE) {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::DamageImmune,
                },
            );
            return false;
        }

        let Some(has_recently_dropped_flag_debuff) =
            self.has_recently_dropped_flag_debuff_like_cpp()
        else {
            return false;
        };
        if has_recently_dropped_flag_debuff {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::RecentlyDroppedFlag,
                },
            );
            return false;
        }

        let Some(player_is_alive) = self.resolved_player_is_alive_like_cpp() else {
            return false;
        };
        if !player_is_alive {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                    gameobject_guid,
                    player_guid,
                    reason: RepresentedBattlegroundObjectUseRejection::Dead,
                },
            );
            return false;
        }

        true
    }

    /// Finish C++ `Player::DestroyItem` after the Login DB pet/receipt commit.
    ///
    /// The receipt makes this second database phase retryable: a reconnect may
    /// observe the already-created pet and finish deleting the cage without
    /// publishing another pet or visual.
    async fn destroy_uncaged_battle_pet_item_durable_like_cpp(
        &mut self,
        source_item_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let player_db_guid = player_guid.counter() as u64;
        let item_db_guid = source_item_guid.counter() as u64;

        let (owner_guid, inventory_linked) = match self
            .uncage_item_state_like_cpp(player_db_guid, item_db_guid)
            .await
        {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state) => {
                (state.owner_guid, state.inventory_linked)
            }
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    item_guid = item_db_guid,
                    %reason,
                    "Failed to inspect the uncaged battle-pet item"
                );
                return false;
            }
        };
        if owner_guid.is_some_and(|owner_guid| owner_guid != player_db_guid) {
            warn!(
                account = self.account_id,
                item_guid = item_db_guid,
                durable_owner = ?owner_guid,
                player_guid = player_db_guid,
                "Refusing to destroy an uncaged item owned by another character"
            );
            return false;
        }
        let Some((bag, slot, item)) = self.get_inventory_item_by_guid_like_cpp(source_item_guid)
        else {
            return owner_guid.is_none() && !inventory_linked;
        };
        let runtime_item = self.resolved_inventory_item_object_like_cpp(source_item_guid);
        self.destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
            bag,
            slot,
            item,
            runtime_item,
            owner_guid.map(|_| player_db_guid),
            "BattlePetUncage",
        )
        .await
    }

    #[cfg(test)]
    pub(crate) fn battle_pet_grant_battle_pet_experience_with_owner_auras_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        xp: u16,
        xp_source: RepresentedBattlePetXpSourceLikeCpp,
    ) -> RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
        let multiplier = if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
            self.total_represented_aura_multiplier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModBattlePetXpPct,
            )
        } else {
            1.0
        };

        self.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid, xp, xp_source, multiplier,
        )
    }

    pub(crate) fn set_represented_critter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        if self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_critter_guid_like_cpp(guid);
            })
            .is_some()
        {
            return;
        }
        #[cfg(test)]
        {
            self.represented_critter_guid_like_cpp = guid;
        }
    }

    pub(crate) fn represented_critter_guid_like_cpp(&self) -> Option<ObjectGuid> {
        if let Some(guid) =
            self.with_owned_player_like_cpp(|player| player.unit().critter_guid_like_cpp())
        {
            return guid;
        }
        #[cfg(test)]
        {
            self.represented_critter_guid_like_cpp
        }
        #[cfg(not(test))]
        {
            None
        }
    }

    /// C++ `WorldSession::HandleDismissCritter`, represented at the ownership gate.
    ///
    /// Full `ObjectAccessor::GetCreatureOrPetOrVehicle`, `Unit::IsSummon` and
    /// `TempSummon::UnSummon` remain part of the live companion runtime. This
    /// represented path preserves the C++ no-response semantics and only acts
    /// when the requested GUID is the player's active critter.
    pub(crate) fn represented_dismiss_critter_like_cpp(
        &mut self,
        critter_guid: ObjectGuid,
    ) -> bool {
        if self.represented_critter_guid_like_cpp() != Some(critter_guid) {
            return false;
        }

        if let Some(companion) = self.represented_battle_pet_query_companion_like_cpp(critter_guid)
            && let Some(battle_pet_guid) = companion.battle_pet_companion_guid
            && self.represented_summoned_battle_pet_guid_like_cpp() == Some(battle_pet_guid)
        {
            let _ = self.set_represented_summoned_battle_pet_guid_like_cpp(None);
        }

        self.set_represented_critter_guid_like_cpp(None);
        #[cfg(test)]
        self.represented_dismissed_critter_guids_like_cpp
            .push(critter_guid);
        true
    }

    #[cfg(test)]
    pub(crate) fn represented_dismissed_critter_guids_like_cpp(&self) -> &[ObjectGuid] {
        &self.represented_dismissed_critter_guids_like_cpp
    }

    fn represented_player_battleground_type_id_or_reject_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<u32> {
        let state = self.player_battleground_state_snapshot_like_cpp()?;
        if let Some(bg_type_id) = state.represented_type_id {
            return Some(bg_type_id);
        }

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::NotInBattleground,
            },
        );
        None
    }

    fn record_represented_capture_point_update_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: wow_entities::CapturePointUseSource,
        state: RepresentedCapturePointStateLikeCpp,
        broadcast_text_id: u32,
        event_id: u32,
        assault_timer_ms: u32,
    ) {
        let (custom_anim, spell_visual_id) = state.custom_anim_and_spell_visual_like_cpp(source);
        if let Some(position) = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.position)
        {
            self.send_packet(&wow_packet::packets::misc::UpdateCapturePoint {
                guid: gameobject_guid,
                position,
                state: state.packet_state_like_cpp(),
                capture_time_ms: assault_timer_ms,
                capture_total_duration_ms: source.capture_time_ms,
            });
        }
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CapturePointUpdated {
                gameobject_guid,
                state,
                broadcast_text_id,
                event_id,
                world_state_id: source.world_state_id,
                spell_visual_id,
                custom_anim,
                assault_timer_ms,
            },
        );
    }

    pub(crate) fn apply_represented_new_flag_state_command_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: Option<ObjectGuid>,
        new_state: RepresentedNewFlagStateRequest,
        respawn_time_ms: u32,
    ) -> bool {
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        let old_state = state
            .new_flag_state
            .unwrap_or(RepresentedNewFlagStateRequest::InBase);
        if old_state == new_state {
            return false;
        }

        state.new_flag_state = Some(new_state);
        state.new_flag_carrier_guid = if new_state == RepresentedNewFlagStateRequest::Taken {
            player_guid
        } else {
            None
        };

        if new_state == RepresentedNewFlagStateRequest::Taken
            && old_state == RepresentedNewFlagStateRequest::InBase
        {
            state.new_flag_taken_from_base_game_time_ms =
                Some(crate::session_rules::game_time_ms_like_cpp());
        } else if matches!(
            new_state,
            RepresentedNewFlagStateRequest::InBase | RepresentedNewFlagStateRequest::Respawning
        ) {
            state.new_flag_taken_from_base_game_time_ms = None;
        }

        state.new_flag_respawn_until = (new_state == RepresentedNewFlagStateRequest::Respawning)
            .then(|| Instant::now() + Duration::from_millis(u64::from(respawn_time_ms)));

        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                gameobject_guid,
                player_guid: player_guid.unwrap_or(ObjectGuid::EMPTY),
                state: new_state,
            },
        );
        true
    }

    fn lookup_represented_fishing_hole_around_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        const FISHING_HOLE_SEARCH_RANGE_LIKE_CPP: f32 =
            20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP;

        let source = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let source_position = source.position?;
        let source_map_id = source.map_id;
        let now = Instant::now();
        let mut nearest: Option<(ObjectGuid, f32)> = None;

        for (candidate_guid, candidate) in &self.represented_gameobject_use_states {
            if *candidate_guid == gameobject_guid {
                continue;
            }
            if candidate.go_type != Some(wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE as u8) {
                continue;
            }
            if source_map_id.is_some() && candidate.map_id != source_map_id {
                continue;
            }
            if candidate
                .per_player_despawn_until
                .is_some_and(|until| until > now)
            {
                continue;
            }
            let Some(candidate_position) = candidate.position else {
                continue;
            };
            let Some(fishing_hole_radius) = candidate.fishing_hole_radius else {
                continue;
            };
            if !source_position
                .is_within_dist(&candidate_position, FISHING_HOLE_SEARCH_RANGE_LIKE_CPP)
                || !source_position.is_within_dist(&candidate_position, fishing_hole_radius)
            {
                continue;
            }
            let distance = source_position.distance(&candidate_position);
            if nearest.is_none_or(|(_, nearest_distance)| distance < nearest_distance) {
                nearest = Some((*candidate_guid, distance));
            }
        }

        nearest.map(|(guid, _)| guid)
    }

    pub(crate) fn use_represented_creature_questgiver_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        creature_entry: u32,
    ) -> bool {
        let Some(quest_store) = self.quests.store.as_ref().map(Arc::clone) else {
            return false;
        };

        let menu_items =
            self.represented_creature_quest_menu_items_like_cpp(&quest_store, creature_entry);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let ender_candidates = quest_store
            .quests_for_ender(creature_entry)
            .iter()
            .map(|quest| {
                (
                    quest.id,
                    quest.log_title.clone(),
                    quest.allowable_races,
                    quest.allowable_classes,
                    quest.min_level,
                    quest.max_level,
                    quests.statuses.get(&quest.id).map(|status| status.status),
                    quests.rewarded_quest_ids.contains(&quest.id),
                )
            })
            .collect::<Vec<_>>();
        let starter_candidates = quest_store
            .quests_for_starter(creature_entry)
            .iter()
            .map(|quest| {
                (
                    quest.id,
                    quest.log_title.clone(),
                    quest.allowable_races,
                    quest.allowable_classes,
                    quest.min_level,
                    quest.max_level,
                    quest.is_available_for(
                        self.player_race_like_cpp(),
                        self.player_class_like_cpp(),
                        self.player_level_like_cpp(),
                    ),
                    self.can_take_quest(quest),
                    quests.statuses.get(&quest.id).map(|status| status.status),
                    quests.rewarded_quest_ids.contains(&quest.id),
                )
            })
            .collect::<Vec<_>>();
        info!(
            creature_entry,
            race = self.player_race_like_cpp(),
            class = self.player_class_like_cpp(),
            level = self.player_level_like_cpp(),
            ender_candidates = ?ender_candidates,
            starter_candidates = ?starter_candidates,
            menu_items = ?self.represented_quest_menu_item_log_rows_like_cpp(&menu_items),
            "Prepared creature questgiver fallback menu like C++"
        );
        if menu_items.is_empty() {
            return false;
        }

        self.send_represented_prepared_quest_like_cpp(creature_guid, menu_items);
        true
    }

    pub(crate) fn use_represented_gameobject_questgiver_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::QuestgiverUseSource,
    ) -> bool {
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::SendGossip {
                gameobject_guid,
                player_guid,
                gossip_id: source.gossip_id,
            });

        let Some(quest_store) = self.quests.store.as_ref().map(Arc::clone) else {
            return true;
        };

        let menu_items =
            self.represented_gameobject_quest_menu_items_like_cpp(&quest_store, gameobject_entry);
        if !menu_items.is_empty() {
            self.send_represented_prepared_quest_like_cpp(gameobject_guid, menu_items);
        }

        true
    }

    fn represented_quest_giver_query_creature_menu_item_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        creature_entry: u32,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
        if quest_store.creature_has_ender_relation_like_cpp(creature_entry, quest.id) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                return Some(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
            // C++ `HandleQuestgiverQueryQuestOpcode` accepts hasQuest OR
            // hasInvolvedQuest; `PrepareQuestMenu` still checks starters after
            // skipping inactive involved quests on the same source.
        }

        if quest_store.creature_has_starter_relation_like_cpp(creature_entry, quest.id) {
            return self.represented_starter_quest_menu_item_like_cpp(quest);
        }

        None
    }

    fn represented_quest_giver_query_gameobject_menu_item_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        gameobject_entry: u32,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
        if quest_store.gameobject_has_ender_relation_like_cpp(gameobject_entry, quest.id) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                return Some(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
            // C++ `HandleQuestgiverQueryQuestOpcode` accepts hasQuest OR
            // hasInvolvedQuest; `PrepareQuestMenu` still checks starters after
            // skipping inactive involved quests on the same source.
        }

        if quest_store.gameobject_has_starter_relation_like_cpp(gameobject_entry, quest.id) {
            return self.represented_starter_quest_menu_item_like_cpp(quest);
        }

        None
    }

    fn represented_creature_quest_menu_items_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        creature_entry: u32,
    ) -> Vec<RepresentedPreparedQuestMenuItemLikeCpp> {
        let mut menu_items = Vec::new();

        // C++ `Player::PrepareQuestMenu`: involved/ender relations first, icon 4 for
        // represented COMPLETE/INCOMPLETE local quest status.
        for quest in quest_store.quests_for_ender(creature_entry) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                menu_items.push(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
        }

        // Starter relations second, preserving C++ quest-icon selection for later
        // `SendPreparedQuest` single-item auto-open.
        for quest in quest_store.quests_for_starter(creature_entry) {
            if let Some(menu_item) = self.represented_starter_quest_menu_item_like_cpp(quest) {
                menu_items.push(menu_item);
            }
        }

        menu_items
    }

    fn represented_gameobject_quest_menu_items_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        gameobject_entry: u32,
    ) -> Vec<RepresentedPreparedQuestMenuItemLikeCpp> {
        let mut menu_items = Vec::new();

        // C++ `Player::PrepareQuestMenu`: GO involved/ender relations first, then
        // starters; do not contaminate GameObject sources with Creature relations.
        for quest in quest_store.quests_for_gameobject_ender(gameobject_entry) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                menu_items.push(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
        }

        for quest in quest_store.quests_for_gameobject_starter(gameobject_entry) {
            if let Some(menu_item) = self.represented_starter_quest_menu_item_like_cpp(quest) {
                menu_items.push(menu_item);
            }
        }

        menu_items
    }

    fn represented_starter_quest_menu_item_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
        if !self.can_take_quest(quest) {
            return None;
        }

        let quest_icon = if quest.is_turn_in_like_cpp()
            && (!quest.is_repeatable()
                || quest.is_daily_like_cpp()
                || quest.is_weekly_like_cpp()
                || quest.is_monthly_like_cpp())
        {
            QUEST_MENU_ICON_TURN_IN_LIKE_CPP
        } else if quest.is_turn_in_like_cpp() {
            QUEST_MENU_ICON_COMPLETE_LIKE_CPP
        } else if self.represented_player_quest_status_is_none_like_cpp(quest.id) {
            QUEST_MENU_ICON_AVAILABLE_LIKE_CPP
        } else {
            return None;
        };

        Some(RepresentedPreparedQuestMenuItemLikeCpp {
            quest: quest.clone(),
            quest_icon,
            has_starter_relation: true,
            has_involved_relation: false,
        })
    }

    fn represented_quest_menu_item_log_rows_like_cpp(
        &self,
        menu_items: &[RepresentedPreparedQuestMenuItemLikeCpp],
    ) -> Vec<(u32, String, u8, bool, bool, u32, u32)> {
        menu_items
            .iter()
            .map(|item| {
                (
                    item.quest.id,
                    item.quest.log_title.clone(),
                    item.quest_icon,
                    item.has_starter_relation,
                    item.has_involved_relation,
                    item.quest.allowable_classes,
                    item.quest.flags,
                )
            })
            .collect()
    }

    fn quest_list_entry_from_menu_item_like_cpp(
        &self,
        menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
    ) -> QuestListEntry {
        let quest = &menu_item.quest;
        QuestListEntry {
            quest_id: quest.id,
            // C++ `PlayerMenu::SendQuestGiverQuestListMessage` writes
            // `QuestMenuItem::QuestIcon`, not `Quest::GetQuestType`.
            quest_type: menu_item.quest_icon,
            quest_level: 0,
            quest_max_scaling_level: 0,
            quest_flags: quest.flags,
            quest_flags_ex: quest.flags_ex,
            repeatable: quest.is_turn_in_like_cpp()
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
                && !quest.is_monthly_like_cpp(),
            important: self.represented_quest_is_important_like_cpp(quest),
            title: quest.log_title.clone(),
        }
    }

    fn active_player_update_state_like_cpp(&self) -> Option<(u32, i32, u8)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            (
                state.active_local_flags,
                state.active_transport_server_time,
                state.multi_action_bars,
            )
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.active_player_local_flags_like_cpp,
                self.active_player_transport_server_time_like_cpp,
                self.active_player_multi_action_bars_like_cpp,
            ));
        }
        canonical
    }

    fn mutate_active_player_update_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerGameplayState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut state = wow_entities::PlayerGameplayState {
                active_local_flags: self.active_player_local_flags_like_cpp,
                active_transport_server_time: self.active_player_transport_server_time_like_cpp,
                multi_action_bars: self.active_player_multi_action_bars_like_cpp,
                ..Default::default()
            };
            let result = mutate(&mut state);
            self.active_player_local_flags_like_cpp = state.active_local_flags;
            self.active_player_transport_server_time_like_cpp = state.active_transport_server_time;
            self.active_player_multi_action_bars_like_cpp = state.multi_action_bars;
            return Some(result);
        }
        self.with_owned_player_mut_like_cpp(|player| mutate(player.gameplay_state_mut()))
    }

    #[cfg(test)]
    pub(crate) fn active_player_local_flags_like_cpp(&self) -> u32 {
        self.active_player_update_state_like_cpp()
            .expect("test active Player owner must resolve")
            .0
    }

    #[cfg(test)]
    pub(crate) fn set_active_player_local_flags_like_cpp(&mut self, flags: u32) {
        let _ = self.mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags = flags;
        });
        self.sync_current_player_session_visibility_detection_like_cpp();
    }

    pub(crate) fn represented_set_action_bar_toggles_like_cpp(&mut self, mask: u8) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };

        if self
            .mutate_active_player_update_state_like_cpp(|state| state.multi_action_bars = mask)
            .is_none()
        {
            return false;
        }
        self.send_active_player_multi_action_bars_update_like_cpp(guid);
        true
    }

    #[cfg(test)]
    pub(crate) fn active_player_multi_action_bars_like_cpp(&self) -> u8 {
        self.active_player_update_state_like_cpp()
            .expect("test active Player owner must resolve")
            .2
    }

    pub(crate) fn represented_set_action_button_like_cpp(
        &mut self,
        index: u8,
        packed_action: u32,
    ) -> bool {
        let action = action_button_action_like_cpp(packed_action);
        let action_type = action_button_type_like_cpp(packed_action);

        // C++ delegates deeper validation to `Player::AddActionButton`
        // (SpellMgr/ObjectMgr/Mount/BattlePet stores). Those runtime stores are
        // not unified here yet, so this represented path preserves the exact
        // packed action/type split and slot bounds while leaving store-backed
        // validation explicit.
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_action_button_like_cpp(index, action, action_type)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let Some(button) = self
                .represented_action_buttons_like_cpp
                .get_mut(usize::from(index))
            else {
                return false;
            };
            *button = make_action_button_like_cpp(action, action_type);
            return true;
        }
        canonical
    }

    pub(crate) fn reset_represented_action_buttons_like_cpp(&mut self) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(Player::reset_action_buttons_for_load_like_cpp)
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_action_buttons_like_cpp =
                [0; wow_packet::packets::misc::MAX_ACTION_BUTTONS];
            self.represented_action_buttons_loaded_like_cpp = false;
        }
    }

    pub(crate) fn represented_action_buttons_snapshot_like_cpp(
        &self,
    ) -> Option<[u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS]> {
        let canonical = self.with_owned_player_like_cpp(Player::action_buttons_snapshot_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_action_buttons_like_cpp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn represented_action_button_like_cpp(&self, index: u8) -> Option<u32> {
        if let Some(canonical) =
            self.with_owned_player_like_cpp(|player| player.action_button_like_cpp(index))
        {
            return canonical;
        }
        self.player_handle_like_cpp
            .is_none()
            .then(|| {
                self.represented_action_buttons_like_cpp
                    .get(usize::from(index))
                    .copied()
            })
            .flatten()
    }

    pub(crate) fn clear_represented_cuf_profiles_like_cpp(&mut self) {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.reset_cuf_profiles_like_cpp();
        });
        if canonical.is_some() {
            return;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.cuf_profiles_like_cpp =
                vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP];
            self.cuf_profiles_loaded_like_cpp = false;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_cuf_profiles_like_cpp(
        &self,
    ) -> &[Option<wow_packet::packets::misc::CufProfile>] {
        &self.cuf_profiles_like_cpp
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_confirm_barbers_choice_like_cpp(
        &mut self,
        request: RepresentedConfirmBarbersChoiceLikeCpp,
    ) {
        #[cfg(test)]
        {
            self.represented_confirm_barbers_choice_requests_like_cpp
                .push(request);
        }
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_confirm_respec_wipe_like_cpp(
        &mut self,
        request: RepresentedConfirmRespecWipeLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_confirm_respec_wipe_requests_like_cpp
            .push(request);
    }

    #[cfg(test)]
    pub(crate) fn delayed_operations_processed_like_cpp(&self) -> u32 {
        self.delayed_operations_processed_like_cpp
    }

    fn set_represented_can_fly_like_cpp(&mut self, enable: bool) -> bool {
        let Some(mut movement_flags) = self.resolved_player_movement_flags_like_cpp() else {
            return false;
        };
        let currently_enabled = movement_flags.contains(MovementFlag::CAN_FLY);
        if enable == currently_enabled {
            return false;
        }

        if enable {
            movement_flags.insert(MovementFlag::CAN_FLY);
            movement_flags.remove(MovementFlag::SWIMMING | MovementFlag::SPLINE_ELEVATION);
        } else {
            movement_flags.remove(MovementFlag::CAN_FLY | MovementFlag::MASK_MOVING_FLY);
            if let Some(position) = self.player_position_like_cpp() {
                self.set_fall_information_like_cpp(0, position.z);
            }
        }
        self.set_player_movement_flags_like_cpp(movement_flags);

        self.send_player_move_set_flag_like_cpp(if enable {
            ServerOpcodes::MoveSetCanFly
        } else {
            ServerOpcodes::MoveUnsetCanFly
        });
        true
    }

    fn set_represented_can_swim_to_fly_transition_like_cpp(&mut self, enable: bool) -> bool {
        let canonical_changed = self.with_owned_player_mut_like_cpp(|player| {
            player.set_can_transition_between_swim_and_fly_like_cpp(enable)
        });
        #[cfg(test)]
        let changed = canonical_changed.unwrap_or_else(|| {
            if self.player_handle_like_cpp.is_some()
                || self.represented_can_swim_to_fly_transition_like_cpp == enable
            {
                return false;
            }
            self.represented_can_swim_to_fly_transition_like_cpp = enable;
            true
        });
        #[cfg(not(test))]
        let Some(changed) = canonical_changed else {
            return false;
        };
        if !changed {
            return false;
        }

        #[cfg(test)]
        if canonical_changed.is_some() {
            self.represented_can_swim_to_fly_transition_like_cpp = enable;
        }
        self.send_player_move_set_flag_like_cpp(if enable {
            ServerOpcodes::MoveEnableTransitionBetweenSwimAndFly
        } else {
            ServerOpcodes::MoveDisableTransitionBetweenSwimAndFly
        });
        true
    }

    pub(crate) fn trace_anticheat_violation_like_cpp(
        &self,
        rule: &'static str,
        opcode: Option<ClientOpcodes>,
        severity: &'static str,
    ) {
        trace!(
            target: "anticheat.violation",
            rule,
            account = self.account_id,
            character = ?self.player_guid(),
            ?opcode,
            severity,
            "anticheat.violation"
        );
    }

    pub(crate) fn destroy_represented_totem_like_cpp(
        &mut self,
        client_slot: u8,
        requested_totem_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if self.player_moved_unit_guid_like_cpp() != Some(player_guid) {
            return false;
        }

        let slot_id = usize::from(client_slot).saturating_add(wow_entities::UNIT_SUMMON_SLOT_TOTEM);
        if slot_id >= wow_entities::MAX_UNIT_TOTEM_SLOT {
            return false;
        }

        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let Some(managed) = manager.find_map_mut(map_id, instance_id.unwrap_or(0)) else {
            return false;
        };
        let map = managed.map_mut();
        let Some(slot_totem_guid) = map
            .get_typed_player(player_guid)
            .map(|player| player.unit().subsystems().control.summon_slots[slot_id])
        else {
            return false;
        };
        if slot_totem_guid.is_empty() {
            return false;
        }

        let matches_cpp_totem = map
            .with_creature_like_cpp(slot_totem_guid, |totem| {
                totem.is_totem_unit_type_like_cpp()
                    && (requested_totem_guid.is_empty() || totem.guid() == requested_totem_guid)
            })
            .unwrap_or(false);
        if !matches_cpp_totem {
            return false;
        }

        let destroyed = match map.remove_from_map_like_cpp(slot_totem_guid, true) {
            Ok(_) => true,
            Err(wow_map::RemoveFromMapError::ObjectNotFound { .. }) => false,
            Err(_) => false,
        };
        if !destroyed {
            return false;
        }

        if let Some(player) = map.get_typed_player_mut(player_guid) {
            let _ = player
                .unit_mut()
                .subsystems_mut()
                .control
                .clear_summon_slot(slot_id);
        }
        drop(manager);
        true
    }

    pub(crate) fn cancel_represented_pet_aura_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        spell_id: u32,
    ) -> bool {
        if self
            .spell_store()
            .and_then(|store| store.get(spell_id as i32))
            .is_none()
        {
            return false;
        }

        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let Some(managed) = manager.find_map_mut(map_id, instance_id.unwrap_or(0)) else {
            return false;
        };
        let map = managed.map_mut();
        let owned_or_charmed = map.get_typed_player(player_guid).is_some_and(|player| {
            let control = &player.unit().subsystems().control;
            control.pet_guid() == pet_guid || control.charmed_guid == Some(pet_guid)
        });
        if !owned_or_charmed {
            return false;
        }

        if let Some(pet) = map.get_typed_pet_mut(pet_guid) {
            if !pet.creature().is_alive() {
                drop(manager);
                self.send_packet(&wow_packet::packets::pet::PetActionFeedback {
                    spell_id: 0,
                    response: wow_packet::packets::pet::PET_ACTION_FEEDBACK_DEAD_LIKE_CPP,
                });
                return false;
            }
            let removed = !pet
                .creature_mut()
                .unit_mut()
                .subsystems_mut()
                .auras
                .remove_auras_due_to_spell_like_cpp(spell_id, ObjectGuid::EMPTY, 0)
                .is_empty();
            return removed;
        }

        if let Some(creature) = map.get_typed_creature_mut(pet_guid) {
            if !creature.is_alive() {
                drop(manager);
                self.send_packet(&wow_packet::packets::pet::PetActionFeedback {
                    spell_id: 0,
                    response: wow_packet::packets::pet::PET_ACTION_FEEDBACK_DEAD_LIKE_CPP,
                });
                return false;
            }
            let removed = !creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .remove_auras_due_to_spell_like_cpp(spell_id, ObjectGuid::EMPTY, 0)
                .is_empty();
            return removed;
        }

        false
    }

    pub(crate) fn represented_learn_title_like_cpp(&mut self, title_id: u32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| player.learn_title_like_cpp(title_id))
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.represented_known_titles_like_cpp.insert(title_id);
        }
    }

    pub(crate) fn represented_has_title_like_cpp(&self, title_id: u32) -> bool {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.has_title_like_cpp(title_id));
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.represented_known_titles_like_cpp.contains(&title_id);
        }
        canonical.unwrap_or(false)
    }

    pub(crate) fn represented_set_chosen_title_like_cpp(&mut self, title_id: i32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_chosen_title_like_cpp(title_id))
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.represented_chosen_title_like_cpp = title_id;
        }
    }

    #[cfg(test)]
    pub(crate) fn represented_chosen_title_like_cpp(&self) -> i32 {
        let canonical = self.with_owned_player_like_cpp(|player| player.data().player_title);
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.represented_chosen_title_like_cpp;
        }
        canonical.expect("test Player title owner must resolve")
    }

    /// Get the logged-in player GUID.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.player_guid
    }

    pub(crate) fn apply_far_sight_like_cpp(&mut self, enable: bool) {
        if !enable {
            if let Some(player_guid) = self.player_guid() {
                self.represented_seer_guid_like_cpp = Some(player_guid);
            }
            return;
        }

        let Some(target) = self.current_canonical_farsight_object_like_cpp() else {
            debug!("CMSG_FAR_SIGHT enable requested with no current viewpoint");
            return;
        };
        if self.canonical_map_has_seer_like_object_like_cpp(target) {
            self.represented_seer_guid_like_cpp = Some(target);
        } else {
            debug!("CMSG_FAR_SIGHT enable target {:?} is not resoluble", target);
        }
    }

    /// Get the current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Set the session state (e.g., after character login).
    pub fn set_state(&mut self, state: SessionState) {
        let entered_world = state == SessionState::LoggedIn && self.state != SessionState::LoggedIn;
        self.state = state;
        if entered_world {
            self.apply_represented_ffa_pvp_login_state_like_cpp();
        }
    }

    /// Time since the last packet was received.
    pub fn idle_time(&self) -> std::time::Duration {
        self.last_packet_time.elapsed()
    }

    /// Whether the session is disconnecting.
    pub fn is_disconnecting(&self) -> bool {
        self.state == SessionState::Disconnecting
    }
}

// ── Creature movement step helper ────────────────────────────────

/// Maps a bridge-built [`CreaturePathQueryLikeCpp`] onto a worker request.
///
/// The map/instance/phase identity belongs to the tick, while every
/// query-sensitive input — filter, owner reads, retained corridor and
/// `forceDest` — is supplied by the generator bridge at query time, so it cannot
/// be sampled before the bridge's own state transitions.
fn creature_path_request_like_cpp(
    query: crate::map_manager::CreaturePathQueryLikeCpp,
    source_map_id: u32,
    source_instance_id: u32,
    phase_shift: &wow_entities::PhaseShift,
) -> crate::map_manager::WorldMMapPathRequestLikeCpp {
    crate::map_manager::WorldMMapPathRequestLikeCpp {
        start: query.start,
        destination: query.destination,
        mesh_map_id: source_map_id,
        instance_map_id: source_map_id,
        instance_id: source_instance_id,
        filter_context: query.filter_context,
        owner: query.owner,
        previous_poly_refs: query.previous_poly_refs,
        force_destination: query.force_destination,
        point_path_limit: query.point_path_limit,
        phase_shift: phase_shift.clone(),
    }
}

/// Resolves one creature path request through the off-thread Detour worker with
/// C++ `PathGenerator::CalculatePath` semantics.
///
/// A missing worker, or `Ok(None)` from it, both mean "this map has no usable
/// navmesh for this query" — no `.mmap` map data, no per-instance
/// `dtNavMeshQuery`, or no `.mmtile` covering the endpoints. C++
/// `PathGenerator::CalculatePath` (`PathGenerator.cpp:79-86`) answers that with
/// `BuildShortcut()` and `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`, i.e. a
/// launchable direct path rather than a failure, so creatures keep moving on
/// unmeshed terrain instead of retrying forever.
///
/// A query error has no C++ counterpart (C++ would already be inside
/// `BuildPolyPath`, which answers failures with `BuildShortcut()` +
/// `PATHFIND_NOPATH`), so it stays a failure and the caller retries like the
/// C++ `!result` / `PATHFIND_NOPATH` branch.
fn resolve_creature_detour_path_like_cpp(
    mmap_pathfinder: Option<&crate::map_manager::WorldMMapPathfinderWorkerLikeCpp>,
    guid: wow_core::ObjectGuid,
    request: crate::map_manager::WorldMMapPathRequestLikeCpp,
) -> Option<wow_recastdetour::DetourPolyPath> {
    let start = request.start;
    let destination = request.destination;
    let Some(worker) = mmap_pathfinder else {
        return Some(crate::map_manager::detour_path_without_navmesh_like_cpp(
            start,
            destination,
        ));
    };

    match worker.calculate_path_like_cpp(request) {
        Ok(Some(path)) => Some(path),
        Ok(None) => Some(crate::map_manager::detour_path_without_navmesh_like_cpp(
            start,
            destination,
        )),
        Err(error) => {
            tracing::warn!(
                "mmap pathfinding failed for creature {:?}: {:?}",
                guid,
                error
            );
            None
        }
    }
}

fn trace_monster_move_packet_like_cpp(
    source: &'static str,
    guid: wow_core::ObjectGuid,
    creature: &crate::map_manager::WorldCreature,
    move_spline: &wow_movement::MoveSpline,
    packet_spline: &wow_packet::packets::movement::MovementMonsterSpline,
    bytes: &[u8],
) {
    if std::env::var_os("RUSTYCORE_MONSTER_MOVE_TRACE").is_none() {
        return;
    }

    let hex_len = bytes.len().min(160);
    let hex = bytes[..hex_len]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let suffix = if bytes.len() > hex_len { " ..." } else { "" };
    let path_points = move_spline.create_object_path_points_like_cpp();
    let (path_z_min, path_z_max) = path_points.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min_z, max_z), point| (min_z.min(point.z), max_z.max(point.z)),
    );
    let path_z_min = path_z_min.is_finite().then_some(path_z_min);
    let path_z_max = path_z_max.is_finite().then_some(path_z_max);

    info!(
        source,
        ?guid,
        high = ?guid.high_type(),
        realm = guid.realm_id(),
        server = guid.server_id(),
        map = guid.map_id(),
        entry = guid.entry(),
        counter = guid.counter(),
        creature_entry = creature.entry(),
        creature_map = creature.map_id(),
        creature_state = ?creature.state(),
        flags = packet_spline.movement.flags,
        move_time = packet_spline.movement.move_time,
        points = packet_spline.movement.points.len(),
        packed_deltas = packet_spline.movement.packed_deltas.len(),
        face = ?packet_spline.movement.face,
        spline_id = packet_spline.id,
        spline_duration = move_spline.duration_ms(),
        spline_flags = move_spline.flags().bits(),
        spline_final = ?move_spline.final_destination(),
        spline_path_points = path_points.len(),
        spline_path_z_min = ?path_z_min,
        spline_path_z_max = ?path_z_max,
        packet_len = bytes.len(),
        packet_hex = format!("{hex}{suffix}"),
        "RUST_MONSTER_MOVE"
    );
}

fn check_no_gray_aggro_config_like_cpp(
    config: &LegacyCreatureAggroConfigLikeCpp,
    player_level: u8,
    player_gray_level: u8,
    creature_level: u8,
) -> bool {
    if creature_level > player_gray_level {
        return false;
    }

    let not_above = config.no_gray_aggro_above;
    let not_below = config.no_gray_aggro_below;
    if not_above == 0 && not_below == 0 {
        return false;
    }

    let player_level = u32::from(player_level);
    player_level <= not_below || (not_above > 0 && player_level >= not_above)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyCreatureAggroVisibilityDecisionLikeCpp {
    Allowed,
    Rejected,
    Unrepresented,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LegacyCreatureAiSelectionDecisionLikeCpp {
    Selected(CreatureAiKindLikeCpp),
    ScriptRegistryUnrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyCreatureAiCanAttackDecisionLikeCpp {
    Allowed,
    Rejected,
    Unrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyCreatureCanAttackLeashDecisionLikeCpp {
    Allowed,
    OwnerPositionUnrepresented,
    HomeRangeRejected,
}

struct CreatureMeleeVictimSyncIdentityLikeCpp {
    authority: OwnedLootAuthority,
    health_state_revision_authority: wow_entities::HealthStateRevisionAuthorityLikeCpp,
    spawn_id: u64,
    loot_lifecycle_revision_before: u64,
    loot_lifecycle_revision_after: u64,
    death_state_before: wow_constants::DeathState,
    death_state_after: wow_constants::DeathState,
    ai_state_before: wow_entities::CreatureAiState,
    ai_state_after: wow_entities::CreatureAiState,
}

struct CreatureMeleeVictimSyncStateLikeCpp {
    applied_damage: u32,
    victim_health_before: u64,
    victim_health_after: u64,
    victim_health_state_revision_before: u64,
    victim_health_state_revision_after: u64,
    identity: CreatureMeleeVictimSyncIdentityLikeCpp,
}

enum CreatureMeleeApplyResultLikeCpp {
    Ready,
    Hit {
        victim_applied_damage: u32,
        victim_health_before: u64,
        victim_health_after: u64,
        victim_health_state_revision_before: u64,
        victim_health_state_revision_after: u64,
        victim_creature_sync_identity: Option<CreatureMeleeVictimSyncIdentityLikeCpp>,
        over_damage: i32,
        target_level: u8,
        events: Vec<RuntimeEvent>,
    },
    OutOfRange,
    BadFacing,
    AttackerStateRejected,
    LosRejected,
    AttackerUnavailable,
    VictimNotAlive,
    MissingVictim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CreatureAiSpellTargetLikeCpp {
    SelfTarget,
    Victim,
    Enemy,
    Buff,
    Debuff,
}

impl CreatureAiSpellTargetLikeCpp {
    fn requires_random_threat_selection_like_cpp(self) -> bool {
        matches!(self, Self::Enemy | Self::Debuff)
    }

    fn resolve_for_single_player_threat_list_like_cpp(
        self,
        caster_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    ) -> ObjectGuid {
        match self {
            Self::SelfTarget | Self::Buff => caster_guid,
            Self::Victim | Self::Enemy | Self::Debuff => victim_guid,
        }
    }
}

fn creature_ai_spell_target_like_cpp(
    spell_id: u32,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> CreatureAiSpellTargetLikeCpp {
    const TARGET_UNIT_TARGET_ENEMY_LIKE_CPP: u32 = 6;
    const TARGET_UNIT_DEST_AREA_ENEMY_LIKE_CPP: u32 = 16;
    const TARGET_DEST_TARGET_ENEMY_LIKE_CPP: u32 = 53;

    // C++ does not inspect implicit targets when hostile max range is zero.
    let has_max_range = config
        .spell_misc_store
        .as_ref()
        .and_then(|store| {
            store.entry_for_spell_difficulty_with_fallback_like_cpp(
                spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
            )
        })
        .and_then(|misc| {
            config
                .spell_range_store
                .as_ref()
                .and_then(|store| store.get(u32::from(misc.range_index)))
        })
        .is_some_and(|range| range.range_max[0] != 0.0);
    if !has_max_range {
        return CreatureAiSpellTargetLikeCpp::SelfTarget;
    }

    let positive = crate::session_rules::represented_spell_is_positive_like_cpp(spell);
    spell.effects().iter().fold(
        CreatureAiSpellTargetLikeCpp::SelfTarget,
        |selected, effect| {
            // C++ `FillAISpellInfo` intentionally considers TargetA only.
            let target_a = effect.implicit_target_1;
            let mut candidate = match target_a {
                TARGET_UNIT_TARGET_ENEMY_LIKE_CPP | TARGET_DEST_TARGET_ENEMY_LIKE_CPP => {
                    CreatureAiSpellTargetLikeCpp::Victim
                }
                TARGET_UNIT_DEST_AREA_ENEMY_LIKE_CPP => CreatureAiSpellTargetLikeCpp::Enemy,
                _ => CreatureAiSpellTargetLikeCpp::SelfTarget,
            };
            if effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA {
                if target_a == TARGET_UNIT_TARGET_ENEMY_LIKE_CPP {
                    candidate = CreatureAiSpellTargetLikeCpp::Debuff;
                } else if positive {
                    candidate = CreatureAiSpellTargetLikeCpp::Buff;
                }
            }
            selected.max(candidate)
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureAiSpellConditionLikeCpp {
    Aggro,
    Combat,
    Die,
}

fn creature_ai_spell_condition_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> CreatureAiSpellConditionLikeCpp {
    const SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD_LIKE_CPP: u32 = 0x0080_0000;
    if i32::try_from(spell_id).ok().is_some_and(|spell_id| {
        config.spell_store.as_ref().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
                0,
                SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD_LIKE_CPP,
            )
        })
    }) {
        return CreatureAiSpellConditionLikeCpp::Die;
    }
    let Some(misc) = config.spell_misc_store.as_ref().and_then(|store| {
        store.entry_for_spell_difficulty_with_fallback_like_cpp(
            spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )
    }) else {
        return CreatureAiSpellConditionLikeCpp::Combat;
    };
    if misc.is_passive_like_cpp()
        || spell_duration_ms_like_cpp(
            u32::from(misc.duration_index),
            config.spell_duration_store.as_deref(),
        ) == -1
    {
        CreatureAiSpellConditionLikeCpp::Aggro
    } else {
        CreatureAiSpellConditionLikeCpp::Combat
    }
}

fn creature_ai_spell_difficulty_chain_like_cpp(
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Vec<u8> {
    let mut chain = Vec::new();
    let mut visited = [false; 256];
    let mut current = difficulty_id;
    loop {
        if visited[usize::from(current)] {
            break;
        }
        visited[usize::from(current)] = true;
        chain.push(current);
        if current == 0 {
            break;
        }
        current = config
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(u32::from(current)))
            .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
    }
    chain
}

fn creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // C++ resolves one effective `SpellTargetRestrictions` row through the
    // active difficulty fallback chain before `CheckTargetCreatureType`.
    // Creature/player type checks are not represented by M2.6, so missing
    // authority or any effective nonzero mask must keep publication closed.
    let Some(store) = config.spell_target_restrictions_store.as_ref() else {
        return true;
    };
    const REPRESENTED_HOSTILE_UNIT_TARGETS_LIKE_CPP: u32 = 0x0000_0002 | 0x0000_0080;

    store
        .resolved_for_difficulty_chain_like_cpp(
            spell_id,
            creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config)
                .into_iter()
                .map(u32::from),
        )
        .is_some_and(|restriction| {
            restriction.target_creature_type_mask_like_cpp() != 0
                // C++ seeds ExplicitTargetMask from this signed DB2 field,
                // then synthesizes any required source/destination payload.
                // This slice serializes only one hostile living Unit target;
                // fail closed for every other validation or wire requirement.
                || (restriction.targets as u32 & !REPRESENTED_HOSTILE_UNIT_TARGETS_LIKE_CPP != 0)
        })
}

fn creature_ai_spell_cooldowns_entry_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<&wow_data::SpellCooldownsEntry> {
    let store = config.spell_cooldowns_store.as_ref()?;
    creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config)
        .into_iter()
        .find_map(|difficulty_id| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
                .max_by_key(|entry| entry.id)
        })
}

#[derive(Debug, Clone, Copy)]
struct CreatureSpellCooldownProfileLikeCpp {
    spell_id: u32,
    category_id: u32,
    recovery_time_ms: u64,
    category_recovery_time_ms: u64,
    passive: bool,
}

fn creature_ai_spell_cooldown_profile_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<CreatureSpellCooldownProfileLikeCpp> {
    let signed_spell_id = i32::try_from(spell_id).ok()?;
    let spell_store = config.spell_store.as_deref()?;
    let metadata = spell_store.hit_metadata_for_difficulty_like_cpp(
        signed_spell_id,
        difficulty_id,
        config.difficulty_store.as_deref(),
    )?;
    let cooldowns = creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config);
    Some(CreatureSpellCooldownProfileLikeCpp {
        spell_id,
        category_id: metadata.category_id,
        recovery_time_ms: cooldowns
            .and_then(|entry| u64::try_from(entry.recovery_time).ok())
            .unwrap_or(0),
        category_recovery_time_ms: cooldowns
            .and_then(|entry| u64::try_from(entry.category_recovery_time).ok())
            .unwrap_or(0),
        passive: spell_store.is_passive_like_cpp(signed_spell_id),
    })
}

fn creature_ai_spell_has_represented_cooldown_semantics_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    const SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP: u32 = 0x0200_0000;
    const SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT_LIKE_CPP: i32 = 0x04;

    let Ok(signed_spell_id) = i32::try_from(spell_id) else {
        return false;
    };
    let Some(spell_store) = config.spell_store.as_deref() else {
        return false;
    };
    let Some(metadata) = spell_store.hit_metadata_for_difficulty_like_cpp(
        signed_spell_id,
        difficulty_id,
        config.difficulty_store.as_deref(),
    ) else {
        return false;
    };
    if spell_store.is_passive_like_cpp(signed_spell_id) {
        return true;
    }
    // C++ consumes charges instead of starting the normal spell/category
    // cooldown. M2.6 does not yet own creature charge recovery.
    if metadata.charge_category_id != 0 {
        return false;
    }
    if spell_store.has_attribute_for_difficulty_like_cpp(
        signed_spell_id,
        difficulty_id,
        config.difficulty_store.as_deref(),
        0,
        SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP,
    ) {
        return false;
    }
    if metadata.category_id == 0 {
        return true;
    }
    config
        .spell_category_store
        .as_deref()
        .and_then(|store| store.get(metadata.category_id))
        .is_some_and(|category| {
            category.flags & SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT_LIKE_CPP == 0
        })
}

fn creature_ai_spell_is_combat_forbidden_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    i32::try_from(spell_id).ok().is_none_or(|spell_id| {
        config.spell_store.as_ref().is_none_or(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
                0,
                wow_data::spell::attributes::SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL,
            )
        })
    })
}

fn creature_ai_effective_spell_info_like_cpp(
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> wow_data::SpellInfo {
    let mut effective = spell.clone();
    if let Some(effects) = config.spell_store.as_ref().and_then(|store| {
        store.effects_for_difficulty_like_cpp(
            spell.spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )
    }) {
        effective.effects = effects.to_vec();
    }
    effective
}

fn creature_ai_has_temporally_unrepresented_noninstant_spell_like_cpp(
    spells: &[u32],
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // M2.6 has no cast-completion/cancellation state. If one template slot
    // could start a non-instant Aggro/Combat cast, merely dropping that slot
    // would leave Rust free to emit another slot while C++ still owns
    // UNIT_STATE_CASTING. Suppress this creature's whole spell surface until
    // M3.1 can represent that temporal state.
    spells
        .iter()
        .copied()
        .filter(|spell_id| *spell_id != 0)
        .any(|spell_id| {
            let Some(spell_store) = config.spell_store.as_ref() else {
                return false;
            };
            let Ok(spell_id_i32) = i32::try_from(spell_id) else {
                return false;
            };
            let Some(spell) = spell_store.get(spell_id_i32) else {
                return false;
            };
            let spell = creature_ai_effective_spell_info_like_cpp(spell, difficulty_id, config);
            spell.cast_time_ms != 0
                && matches!(
                    creature_ai_spell_condition_like_cpp(spell_id, difficulty_id, config),
                    CreatureAiSpellConditionLikeCpp::Aggro
                        | CreatureAiSpellConditionLikeCpp::Combat
                )
        })
}

fn creature_ai_spell_initial_cooldown_like_cpp(
    spell_id: u32,
    _spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u64 {
    // C++ `AISpellInfoType` starts at AI_DEFAULT_COOLDOWN=5000 and
    // `FillAISpellInfo` raises it with raw `RecoveryTime`, deliberately not
    // `GetRecoveryTime()`/CategoryRecoveryTime.
    let recovery_time_ms =
        creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config)
            .and_then(|entry| u64::try_from(entry.recovery_time).ok())
            .unwrap_or(0);
    recovery_time_ms.max(5_000)
}

fn creature_ai_spell_repeat_cooldown_like_cpp(
    spell_id: u32,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u64 {
    // C++ `CombatAI::UpdateAI` re-schedules in [cooldown, cooldown*2].
    creature_ai_spell_initial_cooldown_like_cpp(spell_id, spell, difficulty_id, config)
}

fn creature_ai_spell_x_spell_visual_id_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Result<u32, ()> {
    // C++ falls back only when the current difficulty has no visual rows at
    // all. It then orders the selected vector by caster-player condition and
    // evaluates both caster conditions. Viewer conditions likewise make the
    // emitted visual row viewer-specific. This slice can prove only a single
    // unconditional row; a conditional or ambiguous selected vector must not
    // silently fall through to another difficulty or pick a HashMap order.
    let Some(store) = config.spell_x_spell_visual_store.as_ref() else {
        return Err(());
    };
    for difficulty_id in creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config) {
        let rows = store
            .entries_like_cpp()
            .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        if rows.len() != 1
            || rows[0].viewer_player_condition_id != 0
            || rows[0].viewer_unit_condition_id != 0
            || rows[0].caster_player_condition_id != 0
            || rows[0].caster_unit_condition_id != 0
        {
            return Err(());
        }
        return Ok(rows[0].id);
    }
    Ok(0)
}

fn creature_ai_spell_go_cast_flags_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u32 {
    const CAST_FLAG_UNKNOWN_9_LIKE_CPP: u32 = 0x0000_0100;
    const CAST_FLAG_NO_GCD_LIKE_CPP: u32 = 0x0004_0000;
    let start_recovery_time =
        creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config)
            .map_or(0, |entry| entry.start_recovery_time);
    CAST_FLAG_UNKNOWN_9_LIKE_CPP
        | (start_recovery_time == 0)
            .then_some(CAST_FLAG_NO_GCD_LIKE_CPP)
            .unwrap_or(0)
}

/// The exact creature incarnation a cast plan was captured from.
///
/// A GUID and an engagement epoch cannot separate a replacement that reused
/// both, so carry the same three-part identity the melee synchronization path
/// proves: spawn ID, loot-storage authority and health-state revision
/// authority. Holding the two authorities keeps their allocations alive, so
/// neither identity can be recycled while the plan is in flight.
#[derive(Clone)]
struct CreatureSpellCasterIncarnationLikeCpp {
    spawn_id: u64,
    authority: OwnedLootAuthority,
    health_state_revision_authority: wow_entities::HealthStateRevisionAuthorityLikeCpp,
}

impl CreatureSpellCasterIncarnationLikeCpp {
    fn capture_like_cpp(creature: &wow_entities::Creature) -> Self {
        Self {
            spawn_id: creature.spawn_id(),
            authority: creature.loot_authority_like_cpp().clone(),
            health_state_revision_authority: creature
                .unit()
                .health_state_revision_authority_like_cpp(),
        }
    }

    fn matches_like_cpp(&self, creature: &wow_entities::Creature) -> bool {
        creature.spawn_id() == self.spawn_id
            && creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&self.authority)
            && creature
                .unit()
                .shares_health_state_revision_authority_like_cpp(
                    &self.health_state_revision_authority,
                )
    }
}

impl std::fmt::Debug for CreatureSpellCasterIncarnationLikeCpp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `OwnedLootAuthority` is deliberately not `Debug`: its identity is the
        // shared allocation, not a printable value.
        f.debug_struct("CreatureSpellCasterIncarnationLikeCpp")
            .field("spawn_id", &self.spawn_id)
            .field(
                "health_state_revision_authority",
                &self.health_state_revision_authority,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct CreatureSpellCastPlanLikeCpp {
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    spell_id: i32,
    spell_x_spell_visual_id: u32,
    cast_time_ms: u32,
    spell_go_cast_flags: u32,
    engagement_epoch: u64,
    caster_incarnation: CreatureSpellCasterIncarnationLikeCpp,
}

fn creature_ai_successful_untriggered_spell_resets_combat_timers_like_cpp(
    command: &CreatureSpellCastPlanLikeCpp,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // C++ `Spell::IsAutoActionResetSpell` rejects triggered casts and these
    // two attribute cases. CreatureAI `DoCast`/`CastSpell` is untriggered in
    // this slice, so a successful represented cast resets the base swing when
    // neither applicable opt-out is present (Spell.cpp:3839-3840, 8056-8064).
    const SPELL_ATTR2_DO_NOT_RESET_COMBAT_TIMERS_LIKE_CPP: u32 = 0x0002_0000;
    const SPELL_ATTR6_DOESNT_RESET_SWING_TIMER_IF_INSTANT_LIKE_CPP: u32 = 0x0200_0000;

    let has_attribute = |word, attribute| {
        config.spell_store.as_ref().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                command.spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
                word,
                attribute,
            )
        })
    };
    !has_attribute(2, SPELL_ATTR2_DO_NOT_RESET_COMBAT_TIMERS_LIKE_CPP)
        && (command.cast_time_ms != 0
            || !has_attribute(6, SPELL_ATTR6_DOESNT_RESET_SWING_TIMER_IF_INSTANT_LIKE_CPP))
}

fn creature_ai_spell_plan_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    spell_id: u32,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Result<CreatureSpellCastPlanLikeCpp, ()> {
    Ok(CreatureSpellCastPlanLikeCpp {
        caster_guid,
        target_guid,
        map_id,
        instance_id,
        spell_id: spell.spell_id,
        spell_x_spell_visual_id: creature_ai_spell_x_spell_visual_id_like_cpp(
            spell_id,
            difficulty_id,
            config,
        )?,
        cast_time_ms: spell.cast_time_ms,
        spell_go_cast_flags: creature_ai_spell_go_cast_flags_like_cpp(
            spell_id,
            difficulty_id,
            config,
        ),
        engagement_epoch: creature.creature_spell_engagement_epoch_like_cpp(),
        caster_incarnation: CreatureSpellCasterIncarnationLikeCpp::capture_like_cpp(
            &creature.creature,
        ),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureAiSpellRepresentationRejectionLikeCpp {
    NonInstant,
    ProjectileOrAmmo,
    EffectOrTarget,
}

fn creature_ai_spell_requires_projectile_payload_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // C++ `Spell::SendSpellStart` / `SendSpellGo` add
    // `CAST_FLAG_PROJECTILE` and `SpellCastData::AmmoDisplayID` for any of
    // these three attributes (Spell.cpp:4678-4679, 4750-4751, 4779-4780).
    // The represented packet currently serializes neither optional ammo field,
    // so accepting one of these spells would produce a structurally different
    // START+GO pair.
    const SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP: u32 = 0x0000_0002;
    const SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP: u32 = 0x0000_0004;
    const SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP: u32 = 0x0008_0000;

    let Some(spell_id_i32) = i32::try_from(spell_id).ok() else {
        return true;
    };
    let db2_requires_projectile = config.spell_store.as_ref().is_some_and(|store| {
        store.has_attribute_for_difficulty_like_cpp(
            spell_id_i32,
            difficulty_id,
            config.difficulty_store.as_deref(),
            0,
            SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP,
        ) || store.has_attribute_for_difficulty_like_cpp(
            spell_id_i32,
            difficulty_id,
            config.difficulty_store.as_deref(),
            10,
            SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP,
        )
    });
    let custom_requires_ammo = config
        .spell_custom_attribute_store
        .as_ref()
        .is_some_and(|store| {
            creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config)
                .into_iter()
                .any(|difficulty_id| {
                    store.attributes_for_spell_difficulty_like_cpp(
                        spell_id,
                        u32::from(difficulty_id),
                    ) & SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP
                        != 0
                })
        });
    db2_requires_projectile || custom_requires_ammo
}

fn creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(
    spell: &wow_data::SpellInfo,
) -> bool {
    // `SpellInfo::PowerCosts` retains zero-valued SpellPower rows. Their mere
    // presence does not make `Spell::m_powerCost` nonzero; in particular live
    // 15691 has a type-3 row whose flat, per-level, periodic, percentage,
    // max-percentage, periodic-percentage and optional values are all zero.
    // Any nonzero cost input still fails closed until Creature power
    // calculation/check/deduction is represented.
    spell.power_costs.iter().any(|cost| {
        cost.mana_cost != 0
            || cost.mana_cost_per_level != 0
            || cost.mana_per_second != 0
            || cost.power_cost_pct != 0.0
            || cost.power_cost_max_pct != 0.0
            || cost.power_pct_per_second != 0.0
            || cost.required_aura_spell_id != 0
            || cost.optional_cost != 0
    })
}

fn creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp(
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
    creature: &crate::map_manager::WorldCreature,
) -> bool {
    const SPELL_ATTR1_USE_ALL_MANA_LIKE_CPP: u32 = 0x0000_0002;
    const SPELL_ATTR4_WEAPON_SPEED_COST_SCALING_LIKE_CPP: u32 = 0x0000_0400;
    const SPELL_AURA_MOD_ADDITIONAL_POWER_COST_LIKE_CPP: i32 = 63;

    if spell.power_costs.is_empty()
        || creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(spell)
    {
        return false;
    }
    let Some(attributes) = config.spell_store.as_ref().and_then(|store| {
        store.misc_attributes_for_difficulty_like_cpp(
            spell.spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )
    }) else {
        // A zero-valued DB2 row is safe only when the attributes that can turn
        // it into a real cost are themselves represented and absent.
        return true;
    };
    if attributes[1] & SPELL_ATTR1_USE_ALL_MANA_LIKE_CPP != 0
        || attributes[4] & SPELL_ATTR4_WEAPON_SPEED_COST_SCALING_LIKE_CPP != 0
    {
        return true;
    }

    let auras = &creature.creature.unit().subsystems().auras;
    auras.has_aura_type_like_cpp(SPELL_AURA_MOD_ADDITIONAL_POWER_COST_LIKE_CPP)
        || auras
            .has_aura_type_like_cpp(wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL)
        || auras.has_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT,
        )
}

fn creature_ai_spell_single_unit_topology_like_cpp(
    spell: &wow_data::SpellInfo,
    target_guid: ObjectGuid,
    recipient_guid: ObjectGuid,
    requires_projectile_payload: bool,
) -> Result<(), CreatureAiSpellRepresentationRejectionLikeCpp> {
    // Cast-time completion belongs to M3.1. M2.6 must fail closed rather than
    // emitting START and an immediate, premature GO for a non-instant spell.
    if spell.cast_time_ms != 0 {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::NonInstant);
    }
    if requires_projectile_payload {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo);
    }
    if spell.requires_spell_focus != 0
        || creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(spell)
    {
        // Focus discovery and Creature power-cost calculation/deduction are
        // not part of M2.6. Emitting GO when C++ CheckCast/CheckPower would
        // fail would be a false successful cast, so keep this slice closed.
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
    }
    if target_guid != recipient_guid {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
    }

    // M2.6 owns AI selection, cooldowns and cast wire. Damage calculation is
    // deliberately left to M3.2: raw EffectBasePoints is not CalcValue and
    // omits dice, scaling, bonuses, mitigation, hit, absorb and resist. Until
    // that pipeline exists, only a topology whose START/GO target lists can be
    // proven from hydrated metadata is emitted, with no fabricated health
    // mutation. TargetA=6 is C++ TARGET_UNIT_TARGET_ENEMY; TargetB must be
    // empty, and chained/radius/triggered effects would add unsupported target
    // or follow-up topology.
    let mut represented_effects = 0usize;
    for effect in spell.effects().iter().filter(|effect| effect.effect != 0) {
        if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(effect.effect) {
            continue;
        }
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
            || effect.implicit_target_1 != 6
            || effect.implicit_target_2 != 0
            || effect.chain_targets != 0
            || effect.effect_radius_index_1 != 0
            || effect.effect_trigger_spell != 0
        {
            return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
        }
        represented_effects += 1;
    }
    if represented_effects == 0 {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureSpellHitProfileLikeCpp {
    NoAttackMissAfterRequiredRoll,
    BaseMeleeMiss {
        miss_threshold_per_ten_thousand: u32,
    },
}

fn represented_creature_spell_hit_profile_like_cpp(
    metadata: &wow_data::spell::SpellHitMetadataLikeCpp,
    active_effect_indices: &[u32],
    attributes: [u32; 15],
) -> Option<CreatureSpellHitProfileLikeCpp> {
    const SPELL_DAMAGE_CLASS_MELEE_LIKE_CPP: i8 = 2;
    const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: u8 = 0x01;
    const SPELL_ATTR0_IS_ABILITY_LIKE_CPP: u32 = 0x0000_0010;
    const SPELL_ATTR3_NO_AVOIDANCE_LIKE_CPP: u32 = 0x0000_0040;
    const SPELL_ATTR3_ALWAYS_HIT_LIKE_CPP: u32 = 0x0004_0000;
    const SPELL_ATTR7_ALLOW_SPELL_REFLECTION_LIKE_CPP: u32 = 0x0000_0001;
    const SPELL_ATTR7_NO_ATTACK_MISS_LIKE_CPP: u32 = 0x0200_0000;

    if metadata.defense_type != SPELL_DAMAGE_CLASS_MELEE_LIKE_CPP
        || metadata.school_mask != SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP
        || metadata.spell_mechanic != 0
        || active_effect_indices.is_empty()
        || active_effect_indices
            .iter()
            .any(|effect_index| metadata.effect_mechanics.get(effect_index).copied() != Some(0))
        || attributes[0] & SPELL_ATTR0_IS_ABILITY_LIKE_CPP == 0
        || attributes[3] & (SPELL_ATTR3_NO_AVOIDANCE_LIKE_CPP | SPELL_ATTR3_ALWAYS_HIT_LIKE_CPP)
            != 0
        || attributes[7] & SPELL_ATTR7_ALLOW_SPELL_REFLECTION_LIKE_CPP != 0
    {
        return None;
    }

    Some(
        if attributes[7] & SPELL_ATTR7_NO_ATTACK_MISS_LIKE_CPP != 0 {
            CreatureSpellHitProfileLikeCpp::NoAttackMissAfterRequiredRoll
        } else {
            CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
                miss_threshold_per_ten_thousand: creature_melee_spell_miss_threshold_3_3_5_like_cpp(
                ),
            }
        },
    )
}

fn resolve_creature_spell_hit_profile_like_cpp(
    profile: CreatureSpellHitProfileLikeCpp,
    roll: Option<u32>,
) -> Option<CreatureSpellTargetHitResultLikeCpp> {
    // C++ `Unit::MeleeSpellHitResult` draws `urand(0, 9999)` before applying
    // NO_ATTACK_MISS to the miss-chance bucket. Both currently represented
    // profiles therefore require exactly one authoritative draw.
    let roll = roll.filter(|roll| *roll <= 9_999)?;
    match profile {
        CreatureSpellHitProfileLikeCpp::NoAttackMissAfterRequiredRoll => {
            Some(CreatureSpellTargetHitResultLikeCpp::Hit)
        }
        CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
            miss_threshold_per_ten_thousand,
        } => Some(if roll < miss_threshold_per_ten_thousand {
            CreatureSpellTargetHitResultLikeCpp::Miss
        } else {
            CreatureSpellTargetHitResultLikeCpp::Hit
        }),
    }
}

fn append_committed_creature_spell_packets_like_cpp(
    plan: &mut RuntimePlan,
    command: &CreatureSpellCastPlanLikeCpp,
    cast_id: ObjectGuid,
    hit_result: CreatureSpellTargetHitResultLikeCpp,
    source_position: Position,
    visibility_range: f32,
    full_log_data: &wow_packet::packets::spell::SpellCastLogData,
) {
    use wow_packet::ServerPacket;
    use wow_packet::packets::spell::{
        SpellCastVisual, SpellGoPkt, SpellMissReason, SpellMissTarget, SpellStartPkt,
        SpellTargetData,
    };

    let visual = SpellCastVisual {
        spell_visual_id: command.spell_x_spell_visual_id,
        script_visual_id: 0,
    };
    let target = SpellTargetData {
        flags: 0x2,
        unit: command.target_guid,
        item: ObjectGuid::EMPTY,
        ..Default::default()
    };
    let start = SpellStartPkt {
        cast_data: Default::default(),
        caster: command.caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: command.spell_id,
        visual: visual.clone(),
        cast_flags: 0x0000_0002,
        cast_flags_ex: 0,
        cast_time_ms: command.cast_time_ms,
        target: target.clone(),
    };
    let (hit_targets, miss_targets) = match hit_result {
        CreatureSpellTargetHitResultLikeCpp::Hit => (vec![command.target_guid], Vec::new()),
        CreatureSpellTargetHitResultLikeCpp::Miss => (
            Vec::new(),
            vec![SpellMissTarget::new(
                command.target_guid,
                SpellMissReason::Miss,
            )],
        ),
    };
    let go = SpellGoPkt {
        cast_data: Default::default(),
        caster: command.caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: command.spell_id,
        visual,
        cast_flags: command.spell_go_cast_flags,
        cast_flags_ex: 0,
        cast_time_ms: crate::session_rules::game_time_ms_like_cpp(),
        target,
        hit_targets,
        miss_targets,
    };
    plan.events.push(RuntimeEvent {
        source_guid: command.caster_guid,
        recipients: RecipientRule::NearbyVisibleDurableSpellCast {
            source_guid: command.caster_guid,
            map_id: command.map_id,
            instance_id: command.instance_id,
            source_position,
            range: visibility_range,
            required_3d: false,
            basic_go_packet_bytes: go.to_bytes(),
            full_go_packet_bytes: go.to_full_log_bytes_like_cpp(full_log_data),
        },
        packet_bytes: start.to_bytes(),
    });
}

fn creature_spell_cast_log_data_like_cpp(
    caster: &wow_entities::Creature,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
) -> Option<wow_packet::packets::spell::SpellCastLogData> {
    use wow_packet::packets::spell::{SpellCastLogData, SpellLogPowerData};

    // The enclosing M2.6 cast path admits only base-difficulty spells and zero
    // effective costs. Keep the cost/aura portions independently fail-closed
    // so later callers cannot fabricate a complete log snapshot.
    if difficulty_id != 0
        || creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(spell)
        || !caster
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    {
        return None;
    }

    // C++ `SpellInfo::CalcPowerCost` retains one row per power type even when
    // its effective amount is zero. That includes signed enum sentinels such
    // as `POWER_ALL=127`: the unknown-power rejection is reached only by
    // cost-bearing branches (for example percentage or use-all-power), all of
    // which M2.6 rejects before this helper. Preserve the remaining zero rows
    // in DB2 order and deduplicate them by their raw signed type.
    let mut power_data = Vec::new();
    for power in &spell.power_costs {
        let power_type = i32::from(power.power_type);
        if power_data
            .iter()
            .any(|known: &SpellLogPowerData| known.power_type == power_type)
        {
            continue;
        }
        // C++ retains the signed raw enum value in the wire row. Values that
        // cannot address its Unit power array keep an amount of zero without
        // changing or dropping the row (including HEALTH=-2 and POWER_ALL).
        let amount = <PowerType as num_traits::FromPrimitive>::from_i8(power.power_type)
            .map(|represented_power| caster.unit().get_power(represented_power))
            .unwrap_or(0);
        power_data.push(SpellLogPowerData {
            power_type,
            amount,
            cost: 0,
        });
    }

    let primary_power = caster.power_type();
    let primary_power_type = primary_power as i32;
    if !power_data
        .iter()
        .any(|power| power.power_type == primary_power_type)
    {
        power_data.insert(
            0,
            SpellLogPowerData {
                power_type: primary_power_type,
                amount: caster.unit().get_power(primary_power),
                cost: 0,
            },
        );
    }

    let stats = caster.combat_log_stats_like_cpp();
    Some(SpellCastLogData {
        // Mirrors the C++ `uint64 GetHealth()` assignment to the signed wire
        // field. DB-backed creature health originates in a u32 and always fits.
        health: caster.unit().data().health as i64,
        attack_power: caster.combat_log_attack_power_like_cpp(),
        spell_power: stats.spell_power,
        armor: stats.armor,
        power_data,
    })
}

enum CreatureSpellCastValidationResultLikeCpp {
    Ready(CreatureSpellTargetHitResultLikeCpp),
    OutOfRange,
    LosRejected,
    MissingTarget,
    TargetRejected,
    CooldownRejected,
    HitResultUnrepresented,
    RuntimeRngAuthorityRejected,
    CasterIncarnationRejected,
}

#[cfg(test)]
fn creature_spell_target_accepts_npc_attack_like_cpp(
    target_flags: UnitFlags,
    spell_attributes: &[u32; 15],
) -> bool {
    const SPELL_ATTR6_CAN_TARGET_UNTARGETABLE_LIKE_CPP: u32 = 0x0100_0000;

    !target_flags.intersects(
        UnitFlags::NON_ATTACKABLE
            | UnitFlags::UNINTERACTIBLE
            | UnitFlags::ON_TAXI
            | UnitFlags::NOT_ATTACKABLE_1
            | UnitFlags::IMMUNE_TO_NPC,
    ) && (!target_flags.contains(UnitFlags::NON_ATTACKABLE_2)
        || spell_attributes[6] & SPELL_ATTR6_CAN_TARGET_UNTARGETABLE_LIKE_CPP != 0)
}

fn creature_spell_target_is_valid_attack_target_like_cpp(
    caster: &wow_entities::Creature,
    victim: &wow_entities::Player,
    spell_attributes: &[u32; 15],
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    if !caster.unit().world().object().is_in_world()
        || !victim.unit().world().object().is_in_world()
        || !caster.unit().is_alive()
        || !victim.unit().is_alive()
        || victim.is_game_master_like_cpp()
        || victim.unit().unit_state() & (UnitState::DIED | UnitState::IN_FLIGHT).bits() != 0
        || !caster
            .unit()
            .can_see_or_detect_unit_like_cpp(victim.unit(), false, false, false)
    {
        return false;
    }

    let Some(faction_templates) = config.faction_template_store.as_deref() else {
        return false;
    };
    let Ok(caster_faction_template_id) = u32::try_from(caster.unit().data().faction_template)
    else {
        return false;
    };
    let Ok(victim_faction_template_id) = u32::try_from(victim.unit().data().faction_template)
    else {
        return false;
    };
    let (Some(caster_faction_template), Some(victim_faction_template)) = (
        faction_templates.get(caster_faction_template_id),
        faction_templates.get(victim_faction_template_id),
    ) else {
        return false;
    };

    let mut victim_flags = victim.unit().unit_flags_like_cpp();
    if spell_attributes[6] & 0x0100_0000 != 0 {
        victim_flags.remove(UnitFlags::NON_ATTACKABLE_2);
    }
    let mut context = wow_entities::UnitAttackContextLikeCpp {
        victim_is_game_master_player: victim.is_game_master_like_cpp(),
        visibility_represented: true,
        attacker_can_see_or_detect_target: true,
        victim_unit_state: victim.unit().unit_state(),
        attacker_unit_flags: caster.unit().unit_flags_like_cpp().bits(),
        victim_unit_flags: victim_flags.bits(),
        relation_represented: true,
        attacker_is_hostile_to_victim: caster_faction_template
            .is_hostile_to_like_cpp(victim_faction_template),
        victim_is_hostile_to_attacker: victim_faction_template
            .is_hostile_to_like_cpp(caster_faction_template),
        attacker_is_friendly_to_victim: caster_faction_template
            .is_friendly_to_like_cpp(victim_faction_template),
        victim_is_friendly_to_attacker: victim_faction_template
            .is_friendly_to_like_cpp(caster_faction_template),
        victim_has_affecting_player: true,
        ..Default::default()
    };

    let creature_faction_id = u32::from(caster_faction_template.faction);
    if creature_faction_id != 0 {
        if victim.has_forced_reputation_rank_like_cpp(creature_faction_id) {
            // The canonical player currently retains only the presence of a
            // forced reaction, not its rank. Its exact reaction is therefore
            // unrepresented at this cast-time boundary.
            return false;
        }
        let Some(factions) = config.faction_store.as_deref() else {
            return false;
        };
        let Some(creature_faction) = factions.get(creature_faction_id) else {
            return false;
        };
        if creature_faction.can_have_reputation_like_cpp()
            && victim.has_reputation_state_like_cpp(creature_faction_id)
        {
            context.player_creature_reputation_represented = true;
            context.creature_is_contested_guard =
                caster_faction_template.is_contested_guard_faction_like_cpp();
            context.player_has_contested_pvp_flag =
                victim.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
            context.player_at_war_with_creature_faction =
                victim.is_at_war_with_faction_like_cpp(creature_faction_id);
        }
    }

    wow_entities::Unit::is_valid_attack_target_represented_like_cpp(&context)
}

/// Resolve the effective `SpellRange` row C++ `Spell::GetMinMaxRange` reads.
///
/// C++ walks the difficulty-specific `SpellMisc.RangeIndex` into
/// `sSpellRangeStore`, whose rows already carry official/custom SQL overlays
/// and the final `hotfix_data` removals. Both the cast-time range gate and the
/// TurretAI attempt gate must read that same authority or a hotfixed range
/// silently applies to one of them only.
fn creature_ai_effective_spell_range_like_cpp<'a>(
    spell_id: u32,
    difficulty_id: u8,
    config: &'a LegacyCreatureAggroConfigLikeCpp,
) -> Option<&'a wow_data::SpellRangeEntry> {
    let misc = config
        .spell_misc_store
        .as_ref()?
        .entry_for_spell_difficulty_with_fallback_like_cpp(
            spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )?;
    config
        .spell_range_store
        .as_ref()?
        .get(u32::from(misc.range_index))
}

/// A TurretAI cast attempt that a represented `Spell::CheckCast` rejection
/// stopped before any plan was built.
struct TurretRejectedCastAttemptLikeCpp {
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    engagement_epoch: u64,
    spell_id: u32,
    difficulty_id: u8,
}

/// Consume the BASE_ATTACK swing of a TurretAI attempt C++ would have made.
///
/// C++ `UnitAI::DoSpellAttackIfReady` runs the strict
/// `IsWithinCombatRange(GetMaxRange(false))` gate, then calls `CastSpell` and
/// `resetAttackTimer` inside that branch. The reset therefore happens even when
/// `Spell::CheckCast` rejects the cast — for a `disables` row, for instance —
/// but never when the victim is out of that raw range or exactly on its bound.
/// Returns whether the swing was consumed.
fn apply_turret_rejected_cast_attempt_like_cpp(
    canonical_map_manager: &SharedCanonicalMapManager,
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    attempt: &TurretRejectedCastAttemptLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    let Ok(manager) = canonical_map_manager.lock() else {
        return false;
    };
    // Same canonical -> legacy lock order the cast validation path uses.
    let mut legacy_guard = legacy_map_manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(legacy_caster) =
        legacy_guard.find_creature(attempt.map_id, attempt.instance_id, attempt.caster_guid)
    else {
        return false;
    };
    if !legacy_caster.is_alive()
        || legacy_caster.state() != wow_entities::CreatureAiState::InCombat
        || legacy_caster.creature.ai_ownership().combat_target != Some(attempt.target_guid)
        || legacy_caster.creature_spell_engagement_epoch_like_cpp() != attempt.engagement_epoch
    {
        return false;
    }
    let Some(range) =
        creature_ai_effective_spell_range_like_cpp(attempt.spell_id, attempt.difficulty_id, config)
    else {
        return false;
    };
    let Some(managed) = manager.find_map(u32::from(attempt.map_id), attempt.instance_id) else {
        return false;
    };
    let within_raw_combat_range = {
        let map = managed.map();
        let Some(caster) = map.creature_transform_vitals_snapshot_like_cpp(attempt.caster_guid)
        else {
            return false;
        };
        let Some(victim) = map.get_typed_player(attempt.target_guid) else {
            return false;
        };
        if !caster.is_alive || !victim.unit().is_alive() {
            return false;
        }
        let reach_sum =
            caster.combat_reach.max(0.0) + victim.unit().world().combat_reach().max(0.0);
        let turret_combat_maximum = range.range_max[0].max(0.0) + reach_sum;
        let distance_sq = caster
            .position
            .distance_sq(&victim.unit().world().position());
        distance_sq < turret_combat_maximum * turret_combat_maximum
    };
    if !within_raw_combat_range {
        return false;
    }
    let Some(creature) =
        legacy_guard.find_creature_mut(attempt.map_id, attempt.instance_id, attempt.caster_guid)
    else {
        return false;
    };
    creature.record_swing();
    true
}

/// One C++ `Unit::DoMeleeAttackIfReady` pass over a canonical player.
///
/// Lifted out of `impl WorldSession` by #28: the body was already exactly one
/// closure over `&mut Player`, and the global legacy loop reaches the same
/// player through the canonical map rather than through a session. Behaviour,
/// argument order and C++ anchors are unchanged.
pub(in crate::session) fn take_canonical_player_attack_swings_like_cpp(
    player: &mut wow_entities::Player,
    diff_ms: u32,
    in_melee_range: bool,
    facing_target: bool,
    within_los: bool,
) -> Option<(Vec<u32>, Option<Option<u8>>)> {
    let unit = player.unit_mut();
    let spell_pauses_combat_timer = [
        wow_entities::CurrentSpellSlot::Generic,
        wow_entities::CurrentSpellSlot::Channeled,
    ]
    .into_iter()
    .any(|slot| {
        unit.current_spell(slot)
            .is_some_and(|spell| spell.delay_combat_timer_during_cast)
    });
    if !spell_pauses_combat_timer {
        unit.update_attack_timers_like_cpp(diff_ms);
    }
    let mut swings = Vec::new();
    let mut processed_ready_attack = false;
    let mut base_attack_error_update = None;
    // C++: Unit::DoMeleeAttackIfReady, Unit.cpp:2087 exits before
    // processing swings unless UNIT_STATE_MELEE_ATTACKING is present.
    if !unit.has_unit_state(UnitState::MELEE_ATTACKING.bits()) {
        return None;
    }
    // C++: Unit::DoMeleeAttackIfReady, Unit.cpp:2090 exits while charging.
    if unit.has_unit_state(UnitState::CHARGING.bits()) {
        return None;
    }
    // C++: Unit::DoMeleeAttackIfReady returns while casting unless
    // the active channeled spell explicitly allows actions.
    if unit.has_unit_state(UnitState::CASTING.bits()) {
        let channeled = unit.current_spell(wow_entities::CurrentSpellSlot::Channeled);
        if !channeled.is_some_and(|spell| spell.allow_actions_during_channel) {
            return None;
        }
    }
    let has_auto_attack_error = !in_melee_range || !facing_target;
    let melee_state_update_allowed =
        within_los && unit.can_attacker_state_update_melee_like_cpp(false);

    if unit.is_attack_ready_like_cpp(WeaponAttackType::BaseAttack) {
        processed_ready_attack = true;
        if has_auto_attack_error {
            base_attack_error_update = Some(Some(if !in_melee_range { 0 } else { 1 }));
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 100);
        } else {
            base_attack_error_update = Some(None);
            if unit.can_dual_wield_like_cpp()
                && unit.attack_timer(WeaponAttackType::OffAttack) < ATTACK_DISPLAY_DELAY_LIKE_CPP_MS
            {
                unit.set_attack_timer(
                    WeaponAttackType::OffAttack,
                    ATTACK_DISPLAY_DELAY_LIKE_CPP_MS,
                );
            }
            if melee_state_update_allowed {
                unit.remove_attacking_interrupt_auras_like_cpp();
                if unit
                    .current_spell(wow_entities::CurrentSpellSlot::Melee)
                    .is_some()
                {
                    let _ = unit.finish_spell(wow_entities::CurrentSpellSlot::Melee);
                } else {
                    let [min_damage, max_damage] = unit.weapon_damage(WeaponAttackType::BaseAttack);
                    swings.push(min_damage.max(1.0).min(max_damage.max(1.0)).round() as u32);
                }
            }
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        }
    }

    if unit.can_dual_wield_like_cpp() && unit.is_attack_ready_like_cpp(WeaponAttackType::OffAttack)
    {
        processed_ready_attack = true;
        if has_auto_attack_error {
            unit.set_attack_timer(WeaponAttackType::OffAttack, 100);
        } else {
            if unit.attack_timer(WeaponAttackType::BaseAttack) < ATTACK_DISPLAY_DELAY_LIKE_CPP_MS {
                unit.set_attack_timer(
                    WeaponAttackType::BaseAttack,
                    ATTACK_DISPLAY_DELAY_LIKE_CPP_MS,
                );
            }
            if melee_state_update_allowed {
                unit.remove_attacking_interrupt_auras_like_cpp();
                let [min_damage, max_damage] = unit.weapon_damage(WeaponAttackType::OffAttack);
                swings.push(min_damage.max(1.0).min(max_damage.max(1.0)).round() as u32);
            }
            unit.reset_attack_timer_like_cpp(WeaponAttackType::OffAttack);
        }
    }

    processed_ready_attack.then_some((swings, base_attack_error_update))
}

/// C++ `CombatManager::SetInCombatWith` for a player attacker, on an already
/// locked map.
///
/// Lifted by #28 so the global loop can begin a combat reference without a
/// session. The session variant keeps the lock acquisition and delegates here.
fn begin_combat_ref_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    relation_represented: bool,
    attacker_is_friendly_to_victim: bool,
    victim_is_friendly_to_attacker: bool,
) -> bool {
    let Some(attacker) = map.get_typed_player(attacker_guid) else {
        return false;
    };
    let attacker_unit = attacker.unit();
    let attacker_world = attacker_unit.world();
    let attacker_combat = &attacker_unit.subsystems().combat;

    let (context, both_player_controlled) = if let Some(victim) = map.get_typed_player(victim_guid)
    {
        let victim_unit = victim.unit();
        let victim_world = victim_unit.world();
        let victim_combat = &victim_unit.subsystems().combat;
        (
            wow_entities::CombatBeginContextLikeCpp {
                same_unit: attacker_guid == victim_guid,
                attacker_in_world: attacker_world.object().is_in_world(),
                victim_in_world: victim_world.object().is_in_world(),
                attacker_alive: attacker_unit.is_alive(),
                victim_alive: victim_unit.is_alive(),
                same_map: attacker_world.is_in_map(victim_world),
                same_phase: attacker_world.in_same_phase(victim_world),
                attacker_unit_state: attacker_unit.unit_state(),
                victim_unit_state: victim_unit.unit_state(),
                attacker_combat_disallowed: attacker_combat.combat_disallowed,
                victim_combat_disallowed: victim_combat.combat_disallowed,
                relation_represented,
                attacker_is_friendly_to_victim,
                victim_is_friendly_to_attacker,
                attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                victim_or_owner_player_is_game_master: victim.is_game_master_like_cpp(),
            },
            true,
        )
    } else if let Some(result) = map.with_creature_like_cpp(victim_guid, |victim| {
        let victim_unit = victim.unit();
        let victim_world = victim_unit.world();
        let victim_combat = &victim_unit.subsystems().combat;
        (
            wow_entities::CombatBeginContextLikeCpp {
                same_unit: false,
                attacker_in_world: attacker_world.object().is_in_world(),
                victim_in_world: victim_world.object().is_in_world(),
                attacker_alive: attacker_unit.is_alive(),
                victim_alive: victim_unit.is_alive(),
                same_map: attacker_world.is_in_map(victim_world),
                same_phase: attacker_world.in_same_phase(victim_world),
                attacker_unit_state: attacker_unit.unit_state(),
                victim_unit_state: victim_unit.unit_state(),
                attacker_combat_disallowed: attacker_combat.combat_disallowed,
                victim_combat_disallowed: victim_combat.combat_disallowed,
                relation_represented,
                attacker_is_friendly_to_victim,
                victim_is_friendly_to_attacker,
                attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                victim_or_owner_player_is_game_master: false,
            },
            false,
        )
    }) {
        result
    } else {
        return false;
    };

    if !wow_entities::CombatSubsystem::can_begin_combat_like_cpp(context) {
        return false;
    }

    let Some(attacker) = map.get_typed_player_mut(attacker_guid) else {
        return false;
    };
    let attacker_started = attacker
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(victim_guid, both_player_controlled, false);

    let victim_started = if let Some(victim) = map.get_typed_player_mut(victim_guid) {
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, both_player_controlled, false)
    } else if let Some(victim) = map.get_typed_creature_mut(victim_guid) {
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, both_player_controlled, false)
    } else {
        false
    };

    attacker_started && victim_started
}

/// Apply one player's melee swings to a canonical player victim.
///
/// Lifted out of `run_combat_tick` by #28: the body was already one closure over
/// `&mut Player`, and whoever owns the tick resolves the same transition. The
/// arithmetic — `max(1)` per swing, saturating health, `-1` unless the swing
/// overkills — is unchanged.
fn apply_player_melee_to_canonical_player_like_cpp(
    victim: &mut wow_entities::Player,
    damages: &[u32],
) -> Option<(Vec<(u32, i32)>, u8)> {
    if !victim.unit().is_alive() {
        return None;
    }
    let target_level = victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8;
    let mut sent_swings = Vec::new();
    for dmg in damages {
        let damage = (*dmg).max(1);
        let health_before = victim.unit().data().health;
        let health_after = health_before.saturating_sub(u64::from(damage));
        victim.unit_mut().set_health(health_after);
        let over_damage = if health_after == 0 {
            u64::from(damage).saturating_sub(health_before) as i32
        } else {
            -1
        };
        sent_swings.push((damage, over_damage));
    }
    Some((sent_swings, target_level))
}

/// What one player's melee pass did to a legacy creature.
///
/// `move_stop` carries the stop position and spline id rather than serialised
/// bytes: whoever owns the tick applies the transition, and the session that
/// owns the receiver builds the packet. Keeping construction at the session is
/// what makes the bytes identical — `MonsterMoveStop` is viewer-independent,
/// but the values update beside it is not (#28).
#[derive(Clone, Debug)]
pub(crate) struct PlayerMeleeCreatureHitLikeCpp {
    /// `(damage, killed, over_damage)` per swing, in swing order.
    pub swings: Vec<(u32, bool, i32)>,
    pub entry: u32,
    pub level: u8,
    pub died: bool,
    pub move_stop: Option<(Position, u32)>,
    pub values_update: wow_entities::UnitValuesUpdate,
}

/// Melee geometry, decided the same way whoever owns the tick.
///
/// These were `impl WorldSession` associated functions taking no `self`. The
/// global legacy loop has no session, so #28 lifts them to module level
/// unchanged; the arithmetic and the C++ anchors are untouched.
fn is_within_melee_range_like_cpp(
    attacker_position: Position,
    attacker_combat_reach: f32,
    target_position: Position,
    target_combat_reach: f32,
) -> bool {
    let melee_range = (attacker_combat_reach.max(0.0) + target_combat_reach.max(0.0) + 4.0 / 3.0)
        .max(NOMINAL_MELEE_RANGE_LIKE_CPP);
    attacker_position.distance(&target_position) <= melee_range
}

fn is_within_target_boundary_radius_like_cpp(
    attacker_position: Position,
    attacker_combat_reach: f32,
    target_position: Position,
    target_combat_reach: f32,
    target_bounding_radius: f32,
) -> bool {
    let boundary_radius = target_bounding_radius.max(MIN_MELEE_REACH_LIKE_CPP)
        + attacker_combat_reach.max(0.0)
        + target_combat_reach.max(0.0);
    attacker_position.distance(&target_position) < boundary_radius
}

fn is_unit_facing_target_for_melee_like_cpp(
    unit_position: Position,
    target_position: Position,
) -> bool {
    let dx = target_position.x - unit_position.x;
    let dy = target_position.y - unit_position.y;
    if dx.abs() <= f32::EPSILON && dy.abs() <= f32::EPSILON {
        return true;
    }

    let target_angle = dy.atan2(dx);
    let mut diff = (target_angle - unit_position.orientation).rem_euclid(std::f32::consts::TAU);
    if diff > std::f32::consts::PI {
        diff = std::f32::consts::TAU - diff;
    }
    diff <= std::f32::consts::PI / 3.0
}

/// How often the map-wide combat-reference sweep runs, per map, in the player
/// melee phase.
///
/// The session did this every combat tick — roughly every 100 ms
/// (`driver/mod.rs`, every second pass). The loop runs at the map update
/// interval, so calling it per tick would promote an O(combat units) map-wide
/// sweep from 10 Hz to 100 Hz. #28 preserves the cadence instead of the call
/// site.
const PLAYER_MELEE_COMBAT_REF_REVALIDATE_INTERVAL_MS: u32 = 100;

/// Accumulated time per map key, so the sweep above keeps its cadence across
/// ticks. Owned by the loop task, not by any map guard.
#[derive(Debug, Default)]
pub struct PlayerMeleePhaseStateLikeCpp {
    revalidate_accumulated_ms: HashMap<(u16, u32), u32>,
}

/// One attacker the tick owner will resolve this frame.
///
/// Built from the player registry before any map lock is taken, so the phase
/// never needs a session to know who is swinging.
#[derive(Clone, Debug)]
pub struct PlayerMeleeAttackerSnapshotLikeCpp {
    pub registration: crate::session::directory::PlayerRegistration,
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    /// The attacker's published combat mirror, used to notice that a session
    /// still believes it is fighting a victim the map resolved away.
    pub in_combat_mirror: bool,
    pub tap_group_guids: Vec<ObjectGuid>,
}

/// One resolved victim, carried between the collect and execute phases with no
/// guard held.
#[derive(Clone, Debug)]
struct PendingPlayerSwingLikeCpp {
    attacker: PlayerMeleeAttackerSnapshotLikeCpp,
    victim_guid: ObjectGuid,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyPlayerMeleeTickOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub attackers_seen: usize,
    pub maps_seen: usize,
    pub combat_ref_revalidations: usize,
    pub victims_resolved: usize,
    pub swings_ready: usize,
    pub creature_hits: usize,
    pub player_hits: usize,
    pub creature_kills: usize,
    pub victim_missing: usize,
    pub victim_not_alive: usize,
    pub attacker_unavailable: usize,
    pub canonical_mirror_rejections: usize,
    pub in_combat_reconciles: usize,
    pub commands: Vec<crate::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand>,
}

// ── Creature AI / Combat tick methods ────────────────────────────

#[allow(dead_code)]
impl WorldSession {}

fn represented_spell_click_school_damage_amount_like_cpp(
    spell_info: &wow_data::SpellInfo,
) -> Option<u32> {
    let direct_spell_effects_like_cpp: Vec<(u32, i32)> = if spell_info.effects().is_empty() {
        vec![(spell_info.effect_type, spell_info.effect_base_points)]
    } else {
        spell_info
            .effects()
            .iter()
            .filter(|effect| effect.effect != 0)
            .map(|effect| (effect.effect, effect.effect_base_points))
            .collect()
    };

    let mut damage_amount = 0_u32;
    for (effect_type, effect_base_points) in direct_spell_effects_like_cpp {
        if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(effect_type) {
            continue;
        }
        if effect_type != wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE {
            return None;
        }
        damage_amount = damage_amount.checked_add(u32::try_from(effect_base_points).ok()?)?;
    }

    Some(damage_amount)
}

/// Current Unix timestamp (seconds since epoch).
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn create_map_instance_lock_token_like_cpp(
    owner_guid: ObjectGuid,
    entries: &wow_instances::MapDb2Entries,
    lock: &wow_instances::InstanceLock,
) -> u64 {
    fn mix(hash: &mut u64, value: u64) {
        *hash ^= value;
        *hash = hash.wrapping_mul(0x1000_0000_01b3);
    }

    let mut hash = 0xcbf2_9ce4_8422_2325;
    mix(&mut hash, owner_guid.high_value() as u64);
    mix(&mut hash, owner_guid.low_value() as u64);
    mix(&mut hash, u64::from(entries.map_id));
    mix(&mut hash, u64::from(entries.lock_id));
    mix(&mut hash, u64::from(entries.difficulty_id));
    mix(&mut hash, u64::from(lock.map_id));
    mix(&mut hash, u64::from(lock.difficulty_id));
    hash
}

fn create_map_decision_difficulty_id_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::Difficulty> {
    match decision {
        wow_map::CreateMapDecision::Existing { difficulty_id, .. }
        | wow_map::CreateMapDecision::Create { difficulty_id, .. } => Some(*difficulty_id),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

fn create_map_decision_key_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::MapKey> {
    match decision {
        wow_map::CreateMapDecision::Existing { key, .. }
        | wow_map::CreateMapDecision::Create { key, .. } => Some(*key),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

/// Available race/class combinations from `class_expansion_requirement` table.
///
/// Data mirrors C++ `ObjectMgr::LoadClassExpansionRequirements` fallback rows.
/// ActiveExpansionLevel/AccountExpansionLevel: 0 for all except Death Knight (class 6)
/// which requires WotLK (active=2). MinActiveExpansionLevel is the minimum active
/// expansion across all races for that class.
fn default_available_classes() -> Vec<wow_packet::packets::auth::RaceClassAvailability> {
    use wow_packet::packets::auth::{ClassAvailability, RaceClassAvailability};

    // (race_id, &[(class_id, active_expansion_level, account_expansion_level)])
    let data: &[(u8, &[(u8, u8, u8)])] = &[
        (
            1,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Human
        (
            2,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Orc
        (
            3,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Dwarf
        (
            4,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (11, 0, 0),
            ],
        ), // Night Elf
        (
            5,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Undead
        (
            6,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (11, 0, 0),
            ],
        ), // Tauren
        (
            7,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Gnome
        (
            8,
            &[
                (1, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
                (9, 0, 0),
                (11, 0, 0),
            ],
        ), // Troll
        (
            10,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (4, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (8, 0, 0),
                (9, 0, 0),
            ],
        ), // Blood Elf
        (
            11,
            &[
                (1, 0, 0),
                (2, 0, 0),
                (3, 0, 0),
                (5, 0, 0),
                (6, 2, 0),
                (7, 0, 0),
                (8, 0, 0),
            ],
        ), // Draenei
    ];

    // MinActiveExpansionLevel per class = min across all races for that class
    // All classes have active=0 across all races except class 6 (DK) which is always 2
    let min_active = |class_id: u8| -> u8 { if class_id == 6 { 2 } else { 0 } };

    data.iter()
        .map(|&(race_id, classes)| RaceClassAvailability {
            race_id,
            classes: classes
                .iter()
                .map(|&(class_id, active_exp, account_exp)| ClassAvailability {
                    class_id,
                    active_expansion_level: active_exp,
                    account_expansion_level: account_exp,
                    min_active_expansion_level: min_active(class_id),
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
#[path = "../session_tests.rs"]
mod tests;
