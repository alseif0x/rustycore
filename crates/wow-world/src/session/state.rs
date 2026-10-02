// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! State: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::AtomicUsize;
#[cfg(any(test, feature = "test-fixtures"))]
use super::BattlePetTestFixtureLikeCpp;
use super::PlayerConditionStore;
use super::PlayerCurrency;
use super::RepresentedBankItemMoveLikeCpp;
use super::RepresentedBattlefieldListLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedBattlegroundQueueSlotLikeCpp;
use super::RepresentedBattlemasterJoinSkirmishLikeCpp;
#[cfg(test)]
use super::RepresentedCreatureKillEventLikeCpp;
#[cfg(test)]
use super::RepresentedGameObjectCriteriaEvent;
#[cfg(test)]
use super::RepresentedGuildRepairBankWithdrawLikeCpp;
use super::RepresentedHomebindLikeCpp;
use super::RepresentedPendingSpellCastRequestLikeCpp;
use super::RepresentedQueryPetitionLikeCpp;
use super::RepresentedQuestCompleteStatusUpdateLikeCpp;
use super::RepresentedQuestObjectiveProgressEventLikeCpp;
use super::RepresentedSignPetitionLikeCpp;
#[cfg(test)]
use super::RepresentedSilencePartyTalkerLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedVehicleSeatSpellClickRequestLikeCpp;
#[cfg(test)]
use super::instances::test_fixtures::InstanceTestFixtureLikeCpp;
#[cfg(test)]
use super::persistence::test_fixtures::LoadedPlayerFlagsTestFixtureLikeCpp;
#[cfg(test)]
use super::player_items::test_fixtures::PlayerItemTestFixtureLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::progression::PlayerSkillTestFixtureLikeCpp;
#[cfg(test)]
use super::quest::test_fixtures::QuestTestFixtureLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::rest_progression::RestMgrTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::CalendarTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::DuelTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::GuildTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::TradeTestFixtureLikeCpp;
#[cfg(test)]
use super::spell_state::PlayerSpellAndTraitTestFixtureLikeCpp;
#[cfg(test)]
use super::support_features::test_fixtures::SupportFeatureTestFixtureLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::test_support::test_fixtures::PlayerBootstrapCatalogTestFixtureLikeCpp;
#[cfg(test)]
use super::visibility::test_fixtures::VisibilityTestFixtureLikeCpp;
use super::{AccessRequirementStoreLikeCpp, AccountDataLikeCpp, AccountHeirloomDataLikeCpp};
use super::{AdventureMapPoiStore, Arc, AreaTableStore, AreaTriggerDb2Store};
use super::{AreaTriggerScriptDispatcherLikeCpp, AreaTriggerScriptStoreLikeCpp, AreaTriggerStore};
use super::{AuraApplication, BTreeMap, BTreeSet};
use super::{BankBagSlotPricesStore, BattlePetAccountAttachmentLikeCpp};
use super::{BattlemasterListStore, CanonicalThreatAuraSnapshotLikeCpp};
use super::{CharacterPowerSnapshotLikeCpp, ChatFloodConfigLikeCpp, ChatFloodThrottleDataLikeCpp};
use super::{ChatLevelRequirementsLikeCpp, ChatListenRangesLikeCpp, CinematicSequencesStore};
use super::{ClientOpcodes, CombatRatingsGameTableLikeCpp, ConditionEntriesByTypeStore};
use super::{ContentTuningStore, CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp};
use super::{CreatureClassificationHealthRatesLikeCpp, CreatureDifficultyStoreLikeCpp};
use super::{CreatureEquipmentStoreLikeCpp, CurrencyTypesStore, CurvePointStore, CurveStore};
use super::{DifficultyStore, DisableMgrLikeCpp, DungeonEncounterStore, DurabilityCostsStore};
use super::{DurabilityQualityStore, DurableItemLootPersistenceTrackerLikeCpp};
use super::{DurableLootMoneyPersistenceTrackerLikeCpp, EmotesStore, EmotesTextStore};
use super::ExplorationBaseXpStoreLikeCpp;
use super::{FavoriteAppearanceStateLikeCpp, FishingBaseSkillStoreLikeCpp};
use super::FriendshipRepReactionStore;
use super::{GameObjectTemplateLifecycleStoreLikeCpp, GemPropertiesStore, GossipOptionInfo};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{GivePlayerXpScriptDispatcherLikeCpp, MoveSplineDoneTaxiEventLikeCpp};
use super::{GraveyardStore, HashMap, HashSet, HeirloomStore};
use super::{HomebindPersistenceJobLikeCpp, ImportPriceStores, Instant, Item};
use super::{ItemClassStore, ItemCurrencyCostStore, ItemDisenchantLootStore, ItemPriceBaseStore};
use super::{
    LegacyCreatureAggroConfigLikeCpp, LfgDungeonStoreLikeCpp, LfgDungeonsStore, LockStore,
};
use super::{LootDropRatesLikeCpp, LootStores, MAX_SPECIALIZATIONS_LIKE_CPP};
use super::{MMapRuntimeConfigLikeCpp, MountCapabilityStore, MountDefinitionStoreLikeCpp};
use super::{MountStore, MountTypeXCapabilityStore, MountXDisplayStore, MovementAckEventLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{MoveTeleportAckEventLikeCpp, PlayerTransportLoginStateLikeCpp};
use super::{MovementFallDamageEvent, MovementFlag, MovementSpeedAckEventLikeCpp};
use super::{MovementUnderMapDamageEvent, MovieStore, NUM_ACCOUNT_DATA_TYPES};
use super::{NumTalentsAtLevelStore, ObjectGuid, ObjectMgrCatalogsLikeCpp};
use super::{OwnedLootAuthority, PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP};
use super::PacketHandlerEntry;
use super::{ParagonReputationStore, PendingCreatureKillRewardLikeCpp, PendingCreatureSpawn};
use super::{PetStable, PhaseGroupStore, PhaseShift, PhaseStore};
use super::PlayerInteractionDataLikeCpp;
use super::{PlayerResurrectionRequestLikeCpp, PlayerStatsStore, PowerTypeStore, PvpItemStore};
use super::{RandPropPointsStore, RegenGameTablesLikeCpp, RepSpilloverTemplateStoreLikeCpp};
use super::{RepresentedActivateTaxiLikeCpp, RepresentedAdventureMapStartQuestLikeCpp};
use super::{RepresentedAlterAppearanceLikeCpp, RepresentedAuctionPlaceBidLikeCpp};
#[cfg(test)]
use super::{RepresentedAreaZoneCriteriaLikeCpp, RepresentedAtLoginFlagRemovalLikeCpp};
use super::{RepresentedAuctionRemoveItemLikeCpp, RepresentedAuctionReplicateRequestLikeCpp};
use super::{RepresentedAuctionSellItemLikeCpp, RepresentedAutoUnequipOffhandLikeCpp};
use super::{RepresentedBattlefieldPortLikeCpp, RepresentedBattlemasterHelloLikeCpp};
use super::{RepresentedBattlemasterJoinArenaLikeCpp, RepresentedBattlemasterJoinLikeCpp};
use super::{RepresentedCharacterSpellChargeLikeCpp, RepresentedCharacterSpellCooldownLikeCpp};
use super::{RepresentedConfirmBarbersChoiceLikeCpp, RepresentedConfirmRespecWipeLikeCpp};
use super::{RepresentedDeclinePetitionLikeCpp, RepresentedGameObjectUseEffect};
use super::{RepresentedGameObjectUseState, RepresentedGuildRepairBankStateLikeCpp};
#[cfg(test)]
use super::{RepresentedGuildBankInventoryMoveLikeCpp, RepresentedGuildBankListRequestLikeCpp};
#[cfg(test)]
use super::{RepresentedGuildBankMoneyMoveLikeCpp, RepresentedGuildBankTabActionLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{RepresentedLiveApplicationLikeCpp, RepresentedLootRollCriteriaEvent};
use super::{RepresentedLootRollState, RepresentedPendingBind};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp};
use super::{RepresentedTalentRespecVisualSpellCastLikeCpp, RepresentedVoidStorageItemLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{RepresentedTaxiFlightStateLikeCpp, RepresentedTransmogCriteriaEvent};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{RepresentedVehicleEnterRequestLikeCpp, RepresentedVehicleSeatChangeRequestLikeCpp};
use super::{RepresentedWargameInviteAcceptanceLikeCpp, ReputationRatesLikeCpp};
use super::{ReputationRewardRateStoreLikeCpp, ScalingStatDistributionStore};
use super::{ScalingStatValuesStore, ScriptNameInternerLikeCpp};
use super::SessionPersistencePortsLikeCpp;
use super::{ShieldBlockRegularGameTableLikeCpp, SkillLineStore};
use super::{SkillStore, SkillTiersStoreLikeCpp, SpellCastState};
use super::{SpellChargeEntry, SpellHistoryEntry, TactKeyStore};
use super::{TalentStore, TavernAreaTriggerStoreLikeCpp, TeleportToOptionsLikeCpp, ToyStore};
use super::{TrainerStoreLikeCpp, TraitDefinitionStore, TransmogSetItemStore};
use super::{TrinityStringStoreLikeCpp, UnitFlags, UnitMoveTypeLikeCpp, UnitStandStateType};
use super::{VecDeque, Vehicle, VehicleAccessory, VehicleAccessoryStoreLikeCpp, VehicleSeatStore};
use super::{VehicleStore, VendorItemCount, WaypointPathResolverLikeCpp};
#[cfg(any(test, feature = "test-fixtures"))]
use super::{VehicleTemplateStoreLikeCpp, VendorBuyItemTestOverrideLikeCpp};
use super::{WorldSafeLocStore, driver, lifecycle};

#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use fixtures::SessionFixtures;
mod hub;
pub(crate) use hub::{
    HubMut, HubRef, InventoryCx, InventoryCxRef, LifecycleCx, LifecycleCxRef, LootCx, LootCxRef,
    PetsCx, PetsCxRef, QuestStateCx, QuestStateCxRef, cx_inventory, cx_inventory_ref, cx_lifecycle,
    cx_lifecycle_ref, cx_loot, cx_loot_ref, cx_pets, cx_pets_ref, cx_quest_state,
    cx_quest_state_ref, hub_mut, hub_ref, split_instances_mut, split_instances_ref,
    split_interaction, split_interaction_ref, split_inventory_mut, split_inventory_ref,
    split_lifecycle_mut, split_lifecycle_ref, split_loot_mut, split_loot_ref,
    split_quest_state_mut, split_quest_state_ref, split_social_mut, split_social_ref,
    split_spell_state_mut, split_spell_state_ref, split_visibility_mut, split_visibility_ref,
    split_world_entities_mut, split_world_entities_ref,
};
mod session_core;
pub(crate) use session_core::SessionCore;
mod driver_phase;
pub(crate) use driver_phase::SessionDriverPhaseLikeCpp;
pub(crate) mod hub_support;
mod loot;
pub(crate) use loot::LootState;
mod catalogs;
pub(crate) use catalogs::SessionCatalogs;
mod config;
pub(in crate::session) use config::SessionWorldConfig;
#[cfg(any(test, feature = "test-fixtures"))]
mod identity;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use identity::PlayerIdentityState;
mod inventory_state;
pub(crate) use inventory_state::InventoryState;
#[cfg(any(test, feature = "test-fixtures"))]
mod collections;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use collections::CollectionsState;
#[cfg(any(test, feature = "test-fixtures"))]
mod auras;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use auras::AuraState;
#[cfg(any(test, feature = "test-fixtures"))]
mod progression;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use progression::ProgressionState;
#[cfg(any(test, feature = "test-fixtures"))]
mod combat;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use combat::CombatState;
#[cfg(any(test, feature = "test-fixtures"))]
mod movement;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use movement::MovementState;
#[cfg(any(test, feature = "test-fixtures"))]
mod teleport;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use teleport::TeleportState;
#[cfg(any(test, feature = "test-fixtures"))]
mod vehicles;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use vehicles::TaxiVehicleState;
#[cfg(any(test, feature = "test-fixtures"))]
mod pets;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use pets::PetState;
#[cfg(any(test, feature = "test-fixtures"))]
mod battleground;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use battleground::BattlegroundState;
mod instances;
pub(crate) use instances::InstanceState;
mod world_entities;
pub(crate) use world_entities::WorldEntitiesState;
mod visibility;
pub(crate) use visibility::VisibilityState;
#[cfg(any(test, feature = "test-fixtures"))]
mod presentation;
#[cfg(any(test, feature = "test-fixtures"))]
pub(in crate::session) use presentation::PlayerPresentationState;
mod interaction;
pub(crate) use interaction::InteractionState;

pub(in crate::session) use wow_world_core::session::state::{
    SessionDirectory, SessionDriverServices,
};

/// Social admission limits the session applies: the C++ Recruit-A-Friend XP
/// level gates and the chat anti-flood throttle state charged per message.
#[derive(Default)]
pub(crate) struct SessionSocialLimits {
    /// C++ Recruit-A-Friend XP level gates used by `Player::GetsRecruitAFriendBonus(true)`.
    pub(in crate::session) max_recruit_a_friend_bonus_player_level_like_cpp: u32,
    pub(in crate::session) max_recruit_a_friend_bonus_player_level_difference_like_cpp: u32,
    /// C++ `WorldSession::m_chatFloodData` accumulators.
    pub(in crate::session) chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp; 2],
    /// Addon chat filtering state shared with the chat handlers.
    pub(crate) addon_filter: SessionAddonFilter,

    // Test-only compatibility for pre-#578 fixtures. Production group
    // membership and Player-owned update sequences live on canonical Player.
    #[cfg(test)]
    pub(crate) group_guid: Option<u64>,
    #[cfg(test)]
    pub(in crate::session) represented_subgroup_like_cpp: Option<u8>,
    #[cfg(test)]
    pub(in crate::session) represented_group_update_sequences_like_cpp:
        [wow_entities::PlayerGroupUpdateSequenceLikeCpp;
            wow_social::group::MAX_GROUP_CATEGORY_LIKE_CPP as usize],

    /// Detached guild membership and invitation state used only by tests.
    #[cfg(test)]
    pub(in crate::session) guild_test_fixture_like_cpp: GuildTestFixtureLikeCpp,
    /// Calendar request evidence used only by detached Session tests.
    #[cfg(test)]
    pub(in crate::session) calendar_test_fixture_like_cpp: CalendarTestFixtureLikeCpp,
    /// Detached trade inputs used only by Session tests.
    #[cfg(test)]
    pub(in crate::session) trade_test_fixture_like_cpp: TradeTestFixtureLikeCpp,
    #[cfg(test)]
    pub(in crate::session) represented_sign_petitions_like_cpp: Vec<RepresentedSignPetitionLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_decline_petitions_like_cpp:
        Vec<RepresentedDeclinePetitionLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_query_petitions_like_cpp:
        Vec<RepresentedQueryPetitionLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_silence_party_talker_like_cpp:
        Vec<RepresentedSilencePartyTalkerLikeCpp>,
    /// Detached duel state and evidence used only by tests.
    #[cfg(test)]
    pub(in crate::session) duel_test_fixture_like_cpp: DuelTestFixtureLikeCpp,
}

/// The session's view of the world it is in: the active area trigger, the taxi
/// travel map lookup, the combat-tick bookkeeping and the realm PvP flags.
pub(in crate::session) struct SessionWorldView {
    /// C++ `World::IsPvPRealm()` classification.
    pub(in crate::session) is_pvp_realm_like_cpp: bool,
    /// C++ `World::IsFFAPvPRealm()` classification.
    pub(in crate::session) is_ffa_pvp_realm_like_cpp: bool,
    /// Last represented player melee tick used to decrement C++ `m_attackTimer`.
    pub(in crate::session) combat_tick_last_at_like_cpp: Instant,
    /// High-water mark for map-owned creature-melee presentation commands.
    /// Canonical health/death authority lives on `wow-map`; this suppresses
    /// durable FIFO replay without writing delayed values back to that owner.
    pub(in crate::session) last_presented_creature_melee_health_state_revision_like_cpp: u64,
    /// Minimal TaxiNodes.db2 map lookup used by represented `MoveSplineDone` taxi transitions.
    pub(in crate::session) taxi_node_map_ids_like_cpp: HashMap<u32, u16>,
    /// Currently active area trigger ID, set when entered and cleared when exited.
    pub(in crate::session) active_area_trigger: Option<u32>,
    #[cfg(test)]
    pub(in crate::session) area_trigger_script_dispatcher_like_cpp:
        Option<AreaTriggerScriptDispatcherLikeCpp>,
}

/// Addon chat filtering: C++ `WorldSession::_registeredAddonPrefixes` and
/// `_filterAddonMessages`. Read by the chat handlers, which is why the filter
/// keeps crate visibility instead of narrowing to the session tree.
#[derive(Default)]
pub(crate) struct SessionAddonFilter {
    pub(crate) registered_addon_prefixes: Vec<String>,
    pub(crate) filter_addon_messages: bool,
}

/// The canonical producer's phase rail for this session (#787), separate from
/// the command mailbox because a phase pass drains that mailbox.
pub(in crate::session) struct SessionPhaseRail {
    pub(in crate::session) tx: flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    pub(in crate::session) rx: flume::Receiver<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
}

/// The session's spell-side represented state: the cached spell-script id sets
/// the startup audit installs, the spell-acquisition authorities, the execute-log
/// effects and the offhand re-check switch, until the owning Player runtime and the
/// spell-acquisition module take them over.
pub(crate) struct SessionSpellState {
    pub(in crate::session) legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) spell_linked_rejected_trigger_spell_ids_like_cpp:
        Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) spell_script_all_rank_root_spell_ids_like_cpp:
        Option<Arc<BTreeSet<u32>>>,
    /// Effective C++ spell-script hooks. These remain optional so a session
    /// constructed without the startup audit fails closed.
    pub(in crate::session) spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    /// C++ `CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN` represented switch.
    pub(in crate::session) represented_offhand_check_at_spell_unlearn_like_cpp: bool,
    pub(in crate::session) represented_spell_execute_log_effects_like_cpp:
        Vec<wow_packet::packets::combat::SpellLogEffect>,
    pub(crate) spell_acquisition_cast_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp>>,
    pub(crate) spell_acquisition_craft_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp>>,
    /// Handle-less test fixture for Player spell and trait data.
    #[cfg(test)]
    pub(in crate::session) player_spell_test_fixture_like_cpp:
        PlayerSpellAndTraitTestFixtureLikeCpp,
    /// Test-only causal trace. Production applies every represented
    /// post-commit action immediately; retaining a second action history on
    /// the Session would be audit state, not C++ runtime authority.
    #[cfg(test)]
    pub(in crate::session) represented_spell_acquisition_post_commit_actions_like_cpp:
        Vec<crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp>,
    /// Login snapshot of the player's spell history + charge packets. C++ reads these
    /// live from `Player::GetSpellHistory()` in `SendInitialPacketsBeforeAddToMap`; Rust
    /// persists the login snapshot so the before-add helper can re-send it on far teleport
    /// without a DB round trip. #NEXT.R8.ENTITIES.1229.
    #[cfg(test)]
    pub(in crate::session) represented_spell_history_packets_like_cpp:
        (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>),
    /// C++ `ActivePlayerData::SelfResSpells`, represented until update-field
    /// ownership is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_self_res_spells_like_cpp: BTreeSet<i32>,
    /// C++ `Player::m_overrideSpells`, represented until active player spell
    /// cast resolution owns override lookup.
    #[cfg(test)]
    pub(in crate::session) represented_override_spells_like_cpp: HashMap<i32, BTreeSet<i32>>,
    /// True only when all C++ `Player::m_overrideSpells` edges were replaced
    /// from a complete source rather than accumulated opportunistically.
    #[cfg(test)]
    pub(in crate::session) represented_override_spells_complete_like_cpp: bool,
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
    pub(in crate::session) represented_character_spell_cooldowns_like_cpp:
        HashMap<u32, RepresentedCharacterSpellCooldownLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_character_spell_cooldowns_loaded_like_cpp: bool,
    #[cfg(test)]
    pub(in crate::session) represented_character_spell_charges_like_cpp:
        BTreeMap<u32, Vec<RepresentedCharacterSpellChargeLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) represented_character_spell_charges_loaded_like_cpp: bool,
}

/// The session's quest-side represented state: the level-gap thresholds that
/// decide quest visibility, the completed-quest status updates and objective
/// progress the player owner drains, and the visibility refreshes those
/// transitions request.
pub(crate) struct SessionQuestState {
    pub(crate) min_quest_scaled_xp_ratio_like_cpp: u32,
    pub(crate) quest_high_level_hide_diff_like_cpp: u32,
    pub(crate) quest_low_level_hide_diff_like_cpp: u32,
    /// Evidence for represented `Player::CompleteQuest` status-update side effects.
    pub(crate) represented_quest_complete_status_updates_like_cpp:
        Vec<RepresentedQuestCompleteStatusUpdateLikeCpp>,
    pub(in crate::session) represented_quest_objective_progress_draining_like_cpp: bool,
    pub(in crate::session) represented_quest_objective_progress_events_like_cpp:
        VecDeque<RepresentedQuestObjectiveProgressEventLikeCpp>,
    /// Count of visibility refreshes requested by movement initialization.
    pub(in crate::session) movement_visibility_refresh_requests_like_cpp: u32,
    #[cfg(test)]
    pub(crate) quest_test_fixture_like_cpp: QuestTestFixtureLikeCpp,
}

/// The session's persistence and lifecycle timeline: the login/logout instants
/// and the periodic-save schedule, the player loading and logout claims, the
/// tutorial and account-data state it persists, the persistence ports and the
/// finalization rail it hands work to, and the pet-load and loot trackers.
pub(crate) struct SessionLifecycleState {
    /// C++ `WorldSession::_accountData`, represented in-memory until DB load/save is wired.
    pub(in crate::session) account_data_like_cpp: [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES],
    pub(in crate::session) battle_pet_account_attachment_like_cpp:
        Option<BattlePetAccountAttachmentLikeCpp>,
    pub(in crate::session) character_rename_callbacks: driver::RenameCallbacks,
    /// Detached durable loot grants and their post-commit runtime
    /// publications. This covers claimed world-owner items plus Item-owner
    /// items/money; Item owners have no map-owned loot authority.
    pub(in crate::session) durable_item_loot_persistence_like_cpp:
        DurableItemLootPersistenceTrackerLikeCpp,
    /// Per-character fence published to remote loot sources before they begin
    /// mutating this character's durable balance.
    pub(in crate::session) durable_loot_money_persistence_like_cpp:
        Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    pub(in crate::session) finalization: Option<crate::finalization::SessionFinalization>,
    pub(in crate::session) homebind_persistence_tx_like_cpp:
        Option<tokio::sync::mpsc::UnboundedSender<HomebindPersistenceJobLikeCpp>>,
    /// Time played at current level loaded from DB (seconds).
    pub(crate) level_played_time: u32,
    /// Timestamp set when the player enters the world (PlayerLogin).
    pub(crate) login_time: Option<Instant>,
    /// When set, the session is counting down to logout (20s timer).
    /// `None` means no logout is pending.
    pub(crate) logout_time: Option<Instant>,
    /// C++ `Player::m_nextSave` countdown in milliseconds; 0 disables autosave.
    pub(in crate::session) next_player_save_ms_like_cpp: u32,
    /// Set by the sync update loop when the autosave countdown expires.
    pub(in crate::session) pending_periodic_player_save_like_cpp: bool,
    /// Typed database capabilities live behind one indirection so adding a
    /// persistence workflow does not keep growing this already-large session;
    /// `wow-database` supplies the concrete adapters.
    pub(crate) persistence_ports_like_cpp: Box<SessionPersistencePortsLikeCpp>,
    /// Per-character asynchronous C++ `PetLoadQueryHolder` result lifetime.
    pub(in crate::session) pet_load_query_holder_rows_like_cpp:
        lifecycle::PetLoadQueryHolderRowsLikeCpp,
    /// GUID of the character being logged in (set during PlayerLogin).
    pub(in crate::session) player_loading: Option<ObjectGuid>,
    /// Strong identity for this session's process-wide live-character claim.
    pub(in crate::session) player_login_claim_like_cpp: Option<(ObjectGuid, Arc<()>)>,
    /// C++ `WorldSession::m_playerLogout`: true only while the logout routine is executing.
    pub(in crate::session) player_logout_like_cpp: bool,
    /// C++ `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` in milliseconds.
    pub(in crate::session) player_save_interval_ms_like_cpp: u32,
    /// Total played time loaded from DB (seconds).
    pub(crate) total_played_time: u32,
    pub(in crate::session) tutorials_changed_like_cpp: bool,
    /// C++ `WorldSession::_tutorials`, account-scoped tutorial completion flags.
    pub(in crate::session) tutorials_like_cpp: [u32; 8],
    pub(in crate::session) tutorials_loaded_coherently_like_cpp: bool,
    pub(in crate::session) tutorials_loaded_from_db_like_cpp: bool,
    /// Detached loaded Player flag values used only by persistence tests.
    #[cfg(test)]
    pub(in crate::session) player_flags_test_fixture_like_cpp: LoadedPlayerFlagsTestFixtureLikeCpp,
    /// C++ `Player::m_atLoginFlags`, represented for reset-on-login side effects.
    #[cfg(test)]
    pub(in crate::session) represented_at_login_flags_like_cpp: u16,
    /// Represented persistent `RemoveAtLoginFlag` calls until direct character DB execution is live.
    #[cfg(test)]
    pub(in crate::session) represented_at_login_flag_removals_like_cpp:
        Vec<RepresentedAtLoginFlagRemovalLikeCpp>,
    /// Explicit test seam for persistence-sensitive loot-money paths. Production
    /// never bypasses the character database.
    #[cfg(test)]
    pub(crate) loot_money_persistence_test_result_like_cpp: Option<bool>,
    /// C++ `PlayerData::Customizations` loaded before the self CREATE and
    /// retained for non-owner visibility CREATE blocks.
    #[cfg(test)]
    pub(crate) loaded_player_customizations_like_cpp:
        Box<Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>>,
}

pub(crate) use wow_world_core::session::state::{
    SessionAccountState, SessionAdmissionState, SessionRealmPolicy, SessionSharedFlags,
    SessionTransport,
};

// Declaration order is drop order (#1241 F2). These side-effecting members must
// keep this relative order: `core.session_command_tx`/`session_command_rx`
// (command rail peers observe the close), `core.directory` (quest-complete
// sender), `core.transport` (socket send/receive channels and write fences),
// then `lifecycle` (battle-pet attachment release, rename task aborts, homebind
// sender, live-character claim, loot persistence trackers), then `phase` (the
// producer observes the phase rail closing). Shared `Arc` handles in `core` and
// the loot authorities in `loot` only become observable if this session is the
// last owner, which production composition never makes it.
pub struct WorldSession {
    /// Hub state every domain reads: account and realm identity, the session state and command
    /// rails, the selected-player binding, the map/registry/instance handles, the id generators and
    /// module registry, and the nested transport, admission, driver, directory, account and realm
    /// sub-states.
    pub(crate) core: SessionCore,

    /// Persistence and lifecycle state shared with the lifecycle and handler code.
    pub(crate) lifecycle: SessionLifecycleState,

    /// The producer-addressed phase rail the driver parks on between phases.
    pub(in crate::session) phase: SessionPhaseRail,
    /// Loot windows and AE-loot views with their authorities and generations, loot rolls, personal
    /// loot money and the loot test hooks.
    pub(crate) loot: LootState,
    /// Immutable catalog bundles and DB2/world-DB store handles injected at composition; read-only
    /// after construction.
    pub(crate) catalogs: SessionCatalogs,
    /// Immutable world configuration and rate values (C++ `sWorld` config subsets) and the script
    /// dispatchers injected at composition.
    pub(in crate::session) config: SessionWorldConfig,
    /// Test-only fixture groups (#1241 F3-0): the 11 cfg(test) domain groups, nested unchanged so
    /// an F3 context borrows one member instead of eleven.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fixtures: SessionFixtures,
    /// Player items, bank and equipment sets, money and currencies, and the represented bank,
    /// guild-bank and auction request sinks.
    pub(in crate::session) inventory: InventoryState,

    /// Spell-side represented state shared with the spell and acquisition adapters.
    pub(crate) spell_state: SessionSpellState,

    /// Social admission limits and chat anti-flood throttle state.
    pub(crate) social: SessionSocialLimits,
    /// Instance binds and reset times, the instance fixture, exploration and area-zone criteria and
    /// adventure-map quest starts.
    pub(crate) instances: InstanceState,
    /// Session-local represented creature and gameobject runtime: creature auras, kills, spawn and
    /// tick, gameobject use and phase state.
    pub(crate) world_entities: WorldEntitiesState,
    /// Per-client visibility and publication fences: visible transports, last visibility position
    /// and farsight, delivered-update guards.
    pub(crate) visibility: VisibilityState,
    /// NPC interaction: C++ `PlayerInteractionData`, gossip options, vendor stock and the support-
    /// feature fixture.
    pub(crate) interaction: InteractionState,

    /// Quest-side represented state shared with the quest handlers.
    pub(crate) quest_state: SessionQuestState,

    /// The session's view of its world: area trigger, taxi, combat and realm flags.
    pub(in crate::session) view: SessionWorldView,
    /// Opcode -> registered handler; `&'static` entries only, so its drop has no side effect.
    pub(in crate::session) dispatch_table: HashMap<ClientOpcodes, &'static PacketHandlerEntry>,
}
