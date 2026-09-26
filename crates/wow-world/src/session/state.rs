// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! State: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::AtomicUsize;
#[cfg(test)]
use super::BattlePetTestFixtureLikeCpp;
use super::PlayerConditionStore;
use super::PlayerCurrency;
use super::RepresentedBankItemMoveLikeCpp;
use super::RepresentedBattlefieldListLikeCpp;
#[cfg(test)]
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
#[cfg(test)]
use super::RepresentedVehicleSeatSpellClickRequestLikeCpp;
#[cfg(test)]
use super::instances::test_fixtures::InstanceTestFixtureLikeCpp;
#[cfg(test)]
use super::persistence::test_fixtures::LoadedPlayerFlagsTestFixtureLikeCpp;
#[cfg(test)]
use super::player_items::test_fixtures::PlayerItemTestFixtureLikeCpp;
#[cfg(test)]
use super::progression::PlayerSkillTestFixtureLikeCpp;
#[cfg(test)]
use super::quest::test_fixtures::QuestTestFixtureLikeCpp;
#[cfg(test)]
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
#[cfg(test)]
use super::test_support::test_fixtures::PlayerBootstrapCatalogTestFixtureLikeCpp;
use super::time_synchronization::TimeSynchronizationStateLikeCpp;
#[cfg(test)]
use super::visibility::test_fixtures::VisibilityTestFixtureLikeCpp;
use super::{AccessRequirementStoreLikeCpp, AccountDataLikeCpp, AccountHeirloomDataLikeCpp};
use super::{AdventureMapPoiStore, Arc, AreaTableStore, AreaTriggerDb2Store};
use super::{AreaTriggerScriptDispatcherLikeCpp, AreaTriggerScriptStoreLikeCpp, AreaTriggerStore};
use super::{AtomicBool, AuraApplication, BTreeMap, BTreeSet};
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
use super::{EquipmentSetGuidGeneratorLikeCpp, ExplorationBaseXpStoreLikeCpp};
use super::{FavoriteAppearanceStateLikeCpp, FishingBaseSkillStoreLikeCpp};
use super::{FriendshipRepReactionStore, GameEventQuestCompleteCommandLikeCpp};
use super::{GameObjectTemplateLifecycleStoreLikeCpp, GemPropertiesStore, GossipOptionInfo};
#[cfg(test)]
use super::{GivePlayerXpScriptDispatcherLikeCpp, MoveSplineDoneTaxiEventLikeCpp};
use super::{GraveyardStore, GroupRegistry, HashMap, HashSet, HeirloomStore};
use super::{HomebindPersistenceJobLikeCpp, ImportPriceStores, Instant, Item};
use super::{ItemClassStore, ItemCurrencyCostStore, ItemDisenchantLootStore, ItemPriceBaseStore};
use super::{
    LegacyCreatureAggroConfigLikeCpp, LfgDungeonStoreLikeCpp, LfgDungeonsStore, LockStore,
};
use super::{LootDropRatesLikeCpp, LootStores, MAX_SPECIALIZATIONS_LIKE_CPP};
use super::{MMapRuntimeConfigLikeCpp, MountCapabilityStore, MountDefinitionStoreLikeCpp};
use super::{MountStore, MountTypeXCapabilityStore, MountXDisplayStore, MovementAckEventLikeCpp};
#[cfg(test)]
use super::{MoveTeleportAckEventLikeCpp, PlayerTransportLoginStateLikeCpp};
use super::{MovementFallDamageEvent, MovementFlag, MovementSpeedAckEventLikeCpp};
use super::{MovementUnderMapDamageEvent, MovieStore, NUM_ACCOUNT_DATA_TYPES};
use super::{NumTalentsAtLevelStore, ObjectGuid, ObjectGuidGenerator, ObjectMgrCatalogsLikeCpp};
use super::{OwnedLootAuthority, PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, PacketCounterLikeCpp};
use super::{PacketHandlerEntry, PacketSpoofConfigLikeCpp, PacketSpoofPendingBanLikeCpp};
use super::{ParagonReputationStore, PendingCreatureKillRewardLikeCpp, PendingCreatureSpawn};
use super::{PendingInvites, PetStable, PhaseGroupStore, PhaseShift, PhaseStore};
use super::{PlayerIdentityBootstrapLikeCpp, PlayerInteractionDataLikeCpp, PlayerRegistry};
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
#[cfg(test)]
use super::{RepresentedLiveApplicationLikeCpp, RepresentedLootRollCriteriaEvent};
use super::{RepresentedLootRollState, RepresentedPendingBind};
#[cfg(test)]
use super::{RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp};
use super::{RepresentedTalentRespecVisualSpellCastLikeCpp, RepresentedVoidStorageItemLikeCpp};
#[cfg(test)]
use super::{RepresentedTaxiFlightStateLikeCpp, RepresentedTransmogCriteriaEvent};
#[cfg(test)]
use super::{RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp};
#[cfg(test)]
use super::{RepresentedVehicleEnterRequestLikeCpp, RepresentedVehicleSeatChangeRequestLikeCpp};
use super::{RepresentedWargameInviteAcceptanceLikeCpp, ReputationRatesLikeCpp};
use super::{ReputationRewardRateStoreLikeCpp, ScalingStatDistributionStore};
use super::{ScalingStatValuesStore, ScriptNameInternerLikeCpp, SessionCommand, SessionManager};
use super::{SessionPersistencePortsLikeCpp, SessionState, SharedCanonicalMapManager};
use super::{SharedClientVisibleGuidsLikeCpp, ShieldBlockRegularGameTableLikeCpp, SkillLineStore};
use super::{SkillStore, SkillTiersStoreLikeCpp, SocketTimeoutsLikeCpp, SpellCastState};
use super::{SpellChargeEntry, SpellHistoryEntry, StdRng, TactKeyStore};
use super::{TalentStore, TavernAreaTriggerStoreLikeCpp, TeleportToOptionsLikeCpp, ToyStore};
use super::{TrainerStoreLikeCpp, TraitDefinitionStore, TransmogSetItemStore};
use super::{TrinityStringStoreLikeCpp, UnitFlags, UnitMoveTypeLikeCpp, UnitStandStateType};
use super::{VecDeque, Vehicle, VehicleAccessory, VehicleAccessoryStoreLikeCpp, VehicleSeatStore};
use super::{VehicleStore, VendorItemCount, VoidStorageItemIdGeneratorLikeCpp};
#[cfg(test)]
use super::{VehicleTemplateStoreLikeCpp, VendorBuyItemTestOverrideLikeCpp};
use super::{WaypointPathResolverLikeCpp, WorldMMapPathfinderWorkerLikeCpp, WorldPacket};
use super::{WorldSafeLocStore, driver, lifecycle};

/// Shared registries and the game-event channel the session coordinates through.
#[derive(Default)]
pub(in crate::session) struct SessionDirectory {
    /// Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    pub(in crate::session) game_event_quest_complete_tx:
        Option<flume::Sender<GameEventQuestCompleteCommandLikeCpp>>,
    /// Shared group registry for party management.
    pub(in crate::session) group_registry: Option<Arc<GroupRegistry>>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub(in crate::session) pending_invites: Option<Arc<PendingInvites>>,
}

pub struct WorldSession {
    /// The realm/instance transport, owned by `wow-session` (#297).
    ///
    /// The first piece of this type to earn its own crate: it compiles without
    /// gameplay, databases or catalogs, so the compiler now prevents transport
    /// decisions from reaching a `Player`, a `Map` or a query.
    pub(in crate::session) connection: wow_session::SessionConnection,
    // Account info
    pub account_id: u32,
    pub(in crate::session) battlenet_account_id: u32,
    pub(in crate::session) realm_list_secret_like_cpp: [u8; 32],
    pub(in crate::session) recruiter_id_like_cpp: u32,
    pub(in crate::session) is_a_recruiter_like_cpp: bool,
    pub account_name: String,
    pub security: u8,
    pub expansion: u8,
    pub account_expansion: u8,
    pub(in crate::session) server_expansion_like_cpp: u8,
    #[cfg(test)]
    pub(in crate::session) characters_per_realm_like_cpp: u32,
    #[cfg(test)]
    pub(in crate::session) declined_names_used_like_cpp: bool,
    #[cfg(test)]
    pub(in crate::session) feature_system_bpay_store_enabled_like_cpp: bool,
    #[cfg(test)]
    pub(in crate::session) feature_system_character_undelete_enabled_like_cpp: bool,
    pub(in crate::session) instance_ignore_raid_like_cpp: bool,
    pub(in crate::session) instance_ignore_level_like_cpp: bool,
    pub(in crate::session) max_instances_per_hour_like_cpp: u32,
    /// Detached Player bootstrap-catalog inputs used only by tests.
    #[cfg(test)]
    pub(in crate::session) player_bootstrap_catalog_test_fixture_like_cpp:
        PlayerBootstrapCatalogTestFixtureLikeCpp,
    pub build: u32,
    pub session_key: Vec<u8>,
    pub locale: String,
    pub(in crate::session) mute_time_like_cpp: i64,

    // Inbound packet queue (from WorldSocket)

    // Outbound channel (serialized bytes back to WorldSocket)
    // FIFO completion fence paired with the current physical send channel.

    // Cross-session commands executed by this session's own update loop.
    pub(in crate::session) session_command_tx: flume::Sender<SessionCommand>,
    pub(in crate::session) session_command_rx: flume::Receiver<SessionCommand>,
    /// The canonical producer's phase rail for this session (#787), separate
    /// from the command mailbox because a phase pass drains that mailbox.
    pub(in crate::session) session_phase_tx:
        flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    pub(in crate::session) session_phase_rx:
        flume::Receiver<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    /// The producer and step this session last accepted, per phase (#787).
    ///
    /// C++ has one caller and needs no such watermark. Here it is what rejects
    /// a foreign producer, a retired step and a replay of one already served,
    /// none of which the identity of the player can distinguish. It is kept per
    /// phase because one step legitimately issues the world phase and then the
    /// map phase under the same epoch (`World.cpp:2704` then `World.cpp:2748`).
    pub(in crate::session) last_phase_authority_like_cpp: [Option<(u64, u64)>; 2],
    pub(in crate::session) durable_creature_runtime_commands_like_cpp:
        Arc<std::sync::Mutex<crate::session::mailbox::DurableCreatureRuntimeCommandsLikeCpp>>,
    pub(in crate::session) visibility_refresh_pending_like_cpp: Arc<AtomicBool>,

    // State
    pub(in crate::session) state: SessionState,
    pub(in crate::session) last_packet_time: Instant,
    pub(in crate::session) socket_timeouts_like_cpp: SocketTimeoutsLikeCpp,
    pub(in crate::session) socket_timeout_deadline_like_cpp: Instant,
    pub(in crate::session) packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp,
    pub(in crate::session) packet_throttling_like_cpp: HashMap<u16, PacketCounterLikeCpp>,
    pub(in crate::session) remote_address_like_cpp: Option<String>,
    pub(in crate::session) pending_packet_spoof_ban_like_cpp: Option<PacketSpoofPendingBanLikeCpp>,
    pub(in crate::session) legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp,
    /// Session-owned RNG for represented gameplay choices that C++ resolves through
    /// `urand`/`SelectRandomContainerElement` while the owning Player/Map runtime is
    /// still being split out of `WorldSession`.
    pub(in crate::session) represented_runtime_rng_like_cpp: StdRng,

    // Dispatch table (built once, shared ref)
    pub(in crate::session) dispatch_table: HashMap<ClientOpcodes, &'static PacketHandlerEntry>,

    // FIFO sender for C++ CharacterDatabase.Execute-style detached homebind
    // writes. Its single worker drains queued jobs after session teardown and
    // preserves call order.
    pub(in crate::session) homebind_persistence_tx_like_cpp:
        Option<tokio::sync::mpsc::UnboundedSender<HomebindPersistenceJobLikeCpp>>,

    /// Typed database capabilities live behind one indirection so adding a
    /// persistence workflow does not keep growing this already-large session;
    /// `wow-database` supplies the concrete adapters.
    pub(crate) persistence_ports_like_cpp: Box<SessionPersistencePortsLikeCpp>,

    // C++ ObjectMgr trainer definitions and creature bindings.
    pub(in crate::session) trainer_store_like_cpp: Option<Arc<TrainerStoreLikeCpp>>,

    // BankBagSlotPrices.db2 store used by C++ HandleBuyBankSlotOpcode.
    #[cfg(test)]
    pub(in crate::session) bank_bag_slot_prices_store: Option<Arc<BankBagSlotPricesStore>>,

    // Currency types store (CurrencyTypes.db2 data)
    pub(in crate::session) currency_types_store: Option<Arc<CurrencyTypesStore>>,

    // Import price stores (ImportPrice*.db2 data)
    #[cfg(test)]
    pub(in crate::session) import_price_stores: Option<Arc<ImportPriceStores>>,

    // Emotes.db2 / EmotesText.db2 stores used by C++ chat text-emote handling.
    #[cfg(test)]
    pub(in crate::session) emotes_store: Option<Arc<EmotesStore>>,
    #[cfg(test)]
    pub(in crate::session) emotes_text_store: Option<Arc<EmotesTextStore>>,

    // Item class store (ItemClass.db2 data)
    #[cfg(test)]
    pub(in crate::session) item_class_store: Option<Arc<ItemClassStore>>,

    // Item currency cost store (ItemCurrencyCost.db2 data)
    #[cfg(test)]
    pub(in crate::session) item_currency_cost_store: Option<Arc<ItemCurrencyCostStore>>,

    /// Item template and item-data catalogs a session reads. Owned by one type (#670).
    pub(crate) items: crate::catalogs::item::ItemCatalogsLikeCpp,

    // Trinity strings loaded from world DB `trinity_string`.
    pub(in crate::session) trinity_string_store: Option<Arc<TrinityStringStoreLikeCpp>>,

    // Heirloom store (Heirloom.db2 data)
    pub(in crate::session) heirloom_store: Option<Arc<HeirloomStore>>,

    // Toy store (Toy.db2 data)
    pub(in crate::session) toy_store: Option<Arc<ToyStore>>,

    // C++ `sCombatRatingsGameTable` used by `Player::GetRatingMultiplier`.
    pub(in crate::session) combat_ratings_game_table: Option<Arc<CombatRatingsGameTableLikeCpp>>,

    // C++ `sRegenMPPerSptGameTable` / `sRegenHPPerSptGameTable` /
    // `sOCTRegenHPGameTable` consumed by `Player::Regenerate*`.
    pub(in crate::session) regen_game_tables: Option<Arc<RegenGameTablesLikeCpp>>,

    // C++ `sShieldBlockRegularGameTable` used by `ItemTemplate::GetShieldBlockValue`.
    pub(in crate::session) shield_block_regular_game_table:
        Option<Arc<ShieldBlockRegularGameTableLikeCpp>>,

    // Creature auras this session applied, with the wall-clock deadline the
    // represented duration expires at (`Aura::Update`).
    pub(in crate::session) represented_creature_auras_like_cpp:
        Vec<crate::session::world_entities::RepresentedCreatureAuraLikeCpp>,

    // C++ `Spell::_executeLogEffects` (`Spell.h:519`, `Spell.cpp:5048-5095`):
    // the current cast's execute-log effects, published once by
    // `Spell::FinishTargetProcessing`.
    pub(in crate::session) represented_spell_execute_log_effects_like_cpp:
        Vec<wow_packet::packets::combat::SpellLogEffect>,

    // Transmog set item store (TransmogSetItem.db2 data)
    pub(in crate::session) transmog_set_item_store: Option<Arc<TransmogSetItemStore>>,

    // Item price base store (ItemPriceBase.db2 data)
    #[cfg(test)]
    pub(in crate::session) item_price_base_store: Option<Arc<ItemPriceBaseStore>>,

    // Player level stats store (race/class/level → base stats)
    pub(in crate::session) player_stats: Option<Arc<PlayerStatsStore>>,
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::_usePvpItemLevels` lives on the canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_using_pvp_item_levels_like_cpp: bool,

    pub(in crate::session) pvp_item_store: Option<Arc<PvpItemStore>>,
    /// Every spell and aura catalog slot, owned by one type (#668).
    pub(crate) spell_catalogs: crate::catalogs::spell::SpellCatalogsLikeCpp,
    pub(in crate::session) durability_costs_store: Option<Arc<DurabilityCostsStore>>,
    pub(in crate::session) durability_quality_store: Option<Arc<DurabilityQualityStore>>,
    pub(in crate::session) item_template_addon_quest_log_item_ids_like_cpp: HashMap<u32, u32>,

    // RandPropPoints store (RandPropPoints.db2 data)
    pub(in crate::session) rand_prop_points_store: Option<Arc<RandPropPointsStore>>,

    // ItemDisenchantLoot store (ItemDisenchantLoot.db2 data)
    #[cfg(test)]
    pub(in crate::session) item_disenchant_loot_store: Option<Arc<ItemDisenchantLootStore>>,

    // C++ LootTemplates_* store foundation.
    pub(in crate::session) loot_stores: Option<Arc<LootStores>>,

    // C++ ConditionMgr condition store loaded from world.conditions.
    pub(in crate::session) condition_store: Option<Arc<ConditionEntriesByTypeStore>>,

    // C++ PlayerCondition.db2 store used by ConditionMgr player-condition checks.
    pub(in crate::session) player_condition_store: Option<Arc<PlayerConditionStore>>,

    // C++ AdventureMapPOI.db2 store used by Adventure Map quest starts.
    #[cfg(test)]
    pub(in crate::session) adventure_map_poi_store: Option<Arc<AdventureMapPoiStore>>,

    // C++ ContentTuning.db2 store used by level gates such as Meeting Stone.
    pub(in crate::session) content_tuning_store: Option<Arc<ContentTuningStore>>,
    pub(in crate::session) curve_store: Option<Arc<CurveStore>>,
    pub(in crate::session) curve_point_store: Option<Arc<CurvePointStore>>,
    pub(in crate::session) scaling_stat_distribution_store:
        Option<Arc<ScalingStatDistributionStore>>,
    pub(in crate::session) scaling_stat_values_store: Option<Arc<ScalingStatValuesStore>>,

    // C++ DisableMgr store loaded from world.disables.
    pub(in crate::session) disable_mgr: Option<Arc<DisableMgrLikeCpp>>,

    // C++ Difficulty.db2 store used by sDifficultyStore difficulty changes.
    pub(in crate::session) difficulty_store: Option<Arc<DifficultyStore>>,

    // Lock store (Lock.db2 data)
    pub(in crate::session) lock_store: Option<Arc<LockStore>>,

    pub(in crate::session) gem_properties_store: Option<Arc<GemPropertiesStore>>,

    #[cfg(test)]
    pub(in crate::session) tact_key_store: Option<Arc<TactKeyStore>>,

    // Skill store (auto-learned spells from SkillLineAbility.db2 + SkillRaceClassInfo.db2)
    pub(in crate::session) skill_store: Option<Arc<SkillStore>>,

    // TraitDefinition.db2 store used by represented PlayerSpell::TraitDefinitionId cleanup.
    pub(in crate::session) trait_definition_store: Option<Arc<TraitDefinitionStore>>,

    // C++ TraitMgr profession tree projection used by trait-config login validation.
    pub(in crate::session) trait_tree_skill_line_index:
        Option<Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>>,

    // SkillLine.db2 store for C++ parent/expansion skill resolution.
    pub(in crate::session) skill_line_store: Option<Arc<SkillLineStore>>,

    // C++ ObjectMgr::_skillTiers loaded from world.skill_tiers.
    pub(in crate::session) skill_tiers_store: Option<Arc<SkillTiersStoreLikeCpp>>,

    // Area table store (area hierarchy + mount flags)
    pub(in crate::session) area_table_store: Option<Arc<AreaTableStore>>,

    // C++ ObjectMgr fishing base skill levels loaded from skill_fishing_base_level.
    pub(in crate::session) fishing_base_skill_store: Option<Arc<FishingBaseSkillStoreLikeCpp>>,

    // Area-trigger catalogs are process-owned and borrowed for each
    // production session pass. These retained fields are test fixtures only.
    #[cfg(test)]
    pub(in crate::session) area_trigger_db2_store: Option<Arc<AreaTriggerDb2Store>>,
    #[cfg(test)]
    pub(in crate::session) area_trigger_store: Option<Arc<AreaTriggerStore>>,
    #[cfg(test)]
    pub(in crate::session) area_trigger_script_store: Option<Arc<AreaTriggerScriptStoreLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) area_trigger_script_dispatcher_like_cpp:
        Option<AreaTriggerScriptDispatcherLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) give_player_xp_script_dispatcher_like_cpp:
        Option<GivePlayerXpScriptDispatcherLikeCpp>,
    /// Ordered record of the driver phases this session has run. Test-only:
    /// it exists so tests assert on the production sequence in
    /// `session::driver` instead of reimplementing it.
    #[cfg(test)]
    pub(in crate::session) driver_phase_trace_like_cpp:
        Vec<crate::session::driver::phases::SessionDriverPhaseLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) tavern_area_trigger_store: Option<Arc<TavernAreaTriggerStoreLikeCpp>>,

    // C++ ObjectMgr::GraveyardStore loaded from graveyard_zone plus attached conditions.
    #[cfg(test)]
    pub(in crate::session) graveyard_store: Option<Arc<GraveyardStore>>,

    /// Character race/class catalogs a session reads. Owned by one type (#670).
    pub(crate) chr: crate::catalogs::chr::ChrCatalogsLikeCpp,

    /// Map and map-difficulty catalogs a session reads. Owned by one type (#670).
    pub(crate) maps: crate::catalogs::map::MapCatalogsLikeCpp,
    /// C++ `sDungeonEncounterStore`, shared immutable catalog used by
    /// `Player::IsLockedToDungeonEncounter` during encounter loot filtering.
    pub(in crate::session) dungeon_encounter_store: Option<Arc<DungeonEncounterStore>>,
    pub(in crate::session) world_safe_loc_store_like_cpp: Option<Arc<WorldSafeLocStore>>,
    pub(in crate::session) access_requirement_store: Option<Arc<AccessRequirementStoreLikeCpp>>,
    pub(in crate::session) lfg_dungeons_store: Option<Arc<LfgDungeonsStore>>,
    #[cfg(test)]
    pub(in crate::session) lfg_dungeon_store_like_cpp: Option<Arc<LfgDungeonStoreLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) battlemaster_list_store: Option<Arc<BattlemasterListStore>>,
    /// Detached instance inputs used only by Session tests.
    #[cfg(test)]
    pub(crate) instance_test_fixture_like_cpp: InstanceTestFixtureLikeCpp,
    /// Faction and reputation catalogs a session reads. Owned by one type (#670).
    pub(crate) factions: crate::catalogs::faction::FactionCatalogsLikeCpp,
    pub(in crate::session) friendship_rep_reaction_store: Option<Arc<FriendshipRepReactionStore>>,
    pub(in crate::session) paragon_reputation_store: Option<Arc<ParagonReputationStore>>,
    pub(in crate::session) reputation_reward_rate_store:
        Option<Arc<ReputationRewardRateStoreLikeCpp>>,
    /// Creature template and creature-data catalogs a session reads. Owned by one type (#670).
    pub(crate) creatures: crate::catalogs::creature::CreatureCatalogsLikeCpp,
    pub(in crate::session) reputation_spillover_template_store:
        Option<Arc<RepSpilloverTemplateStoreLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) championing_faction_like_cpp: u32,
    #[cfg(test)]
    pub(in crate::session) creature_equipment_store_like_cpp:
        Option<Arc<CreatureEquipmentStoreLikeCpp>>,
    /// GameObject template catalogs a session reads. Owned by one type (#670).
    pub(crate) gameobjects: crate::catalogs::gameobject::GameObjectCatalogsLikeCpp,
    #[cfg(test)]
    pub(in crate::session) creature_addon_store_like_cpp: Option<Arc<CreatureAddonStoreLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) creature_difficulty_store_like_cpp:
        Option<Arc<CreatureDifficultyStoreLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) creature_base_stats_store_like_cpp:
        Option<Arc<CreatureBaseStatsStoreLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp,
    pub(in crate::session) mount_store: Option<Arc<MountStore>>,
    pub(in crate::session) mount_definition_store_like_cpp:
        Option<Arc<MountDefinitionStoreLikeCpp>>,
    pub(in crate::session) mount_capability_store: Option<Arc<MountCapabilityStore>>,
    pub(in crate::session) mount_type_x_capability_store: Option<Arc<MountTypeXCapabilityStore>>,
    pub(in crate::session) mount_x_display_store: Option<Arc<MountXDisplayStore>>,
    pub(in crate::session) vehicle_store: Option<Arc<VehicleStore>>,
    pub(in crate::session) vehicle_seat_store: Option<Arc<VehicleSeatStore>>,
    #[cfg(test)]
    pub(in crate::session) vehicle_template_store: Option<Arc<VehicleTemplateStoreLikeCpp>>,
    pub(in crate::session) vehicle_accessory_store: Option<Arc<VehicleAccessoryStoreLikeCpp>>,
    pub(in crate::session) terrain_swap_store: Option<Arc<wow_data::TerrainSwapStore>>,
    pub(in crate::session) phase_store: Option<Arc<PhaseStore>>,
    pub(in crate::session) phase_group_store: Option<Arc<PhaseGroupStore>>,

    // Shared player registry for broadcasting to nearby sessions
    pub(in crate::session) player_registry: Option<Arc<PlayerRegistry>>,

    /// Party registries and the world-event channel, grouped by the B4 split.
    pub(in crate::session) directory: SessionDirectory,

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
    #[cfg(test)]
    pub(crate) pass_on_group_loot: bool,
    #[cfg(test)]
    pub(crate) represented_enchanting_skill: u16,
    /// Handle-less test fixture for Player-owned skill values and persistence rows.
    #[cfg(test)]
    pub(in crate::session) player_skill_test_fixture_like_cpp: PlayerSkillTestFixtureLikeCpp,
    #[cfg(test)]
    pub(in crate::session) represented_gray_level_script_overrides_like_cpp: HashMap<u8, u8>,

    // Realm ID for GUID creation
    pub(in crate::session) realm_id: u16,
    pub(in crate::session) realm_region: u8,
    pub(in crate::session) realm_battlegroup: u8,
    pub(in crate::session) realm_names_like_cpp: BTreeMap<u32, (String, String)>,

    // Process-owned GUID generators retained only as test fixtures.
    #[cfg(test)]
    pub(in crate::session) guid_generator: Option<Arc<ObjectGuidGenerator>>,
    // Process-wide C++ ObjectMgr generator retained only as a test fixture.
    #[cfg(test)]
    pub(in crate::session) item_guid_generator_like_cpp: Option<Arc<ObjectGuidGenerator>>,
    // Process-wide C++ ObjectMgr generator shared by equipment and transmog sets.
    #[cfg(test)]
    pub(in crate::session) equipment_set_guid_generator_like_cpp:
        Option<Arc<EquipmentSetGuidGeneratorLikeCpp>>,
    // Process-wide C++ ObjectMgr generator for character_void_storage.itemId.
    #[cfg(test)]
    pub(in crate::session) void_storage_item_id_generator_like_cpp:
        Option<Arc<VoidStorageItemIdGeneratorLikeCpp>>,

    // Characters confirmed for this account
    pub(in crate::session) legit_characters: Vec<ObjectGuid>,

    // Pending async packets to process
    pub(in crate::session) pending_packets: VecDeque<WorldPacket>,
    pub(in crate::session) character_rename_callbacks: driver::RenameCallbacks,

    // ── ConnectTo flow ──────────────────────────────────────────
    /// GUID of the character being logged in (set during PlayerLogin).
    pub(in crate::session) player_loading: Option<ObjectGuid>,
    /// Strong identity for this session's process-wide live-character claim.
    pub(in crate::session) player_login_claim_like_cpp: Option<(ObjectGuid, Arc<()>)>,
    /// C++ `WorldSession::m_playerLogout`: true only while the logout routine is executing.
    pub(in crate::session) player_logout_like_cpp: bool,
    pub(in crate::session) finalization: Option<crate::finalization::SessionFinalization>,

    /// Session manager for ConnectTo flow (shared with instance listener).
    pub(in crate::session) session_mgr: Option<Arc<SessionManager>>,

    /// Canonical per-session time-sync protocol state.
    pub(in crate::session) time_synchronization: TimeSynchronizationStateLikeCpp,

    // ── Logout ──────────────────────────────────────────────────────
    /// When set, the session is counting down to logout (20s timer).
    /// `None` means no logout is pending.
    pub(crate) logout_time: Option<Instant>,
    /// Timestamp set when the player enters the world (PlayerLogin).
    pub(crate) login_time: Option<Instant>,
    /// C++ `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` in milliseconds.
    pub(in crate::session) player_save_interval_ms_like_cpp: u32,
    /// C++ `Player::m_nextSave` countdown in milliseconds; 0 disables autosave.
    pub(in crate::session) next_player_save_ms_like_cpp: u32,
    /// Set by the sync update loop when the autosave countdown expires.
    pub(in crate::session) pending_periodic_player_save_like_cpp: bool,
    /// Total played time loaded from DB (seconds).
    pub(crate) total_played_time: u32,
    /// Time played at current level loaded from DB (seconds).
    pub(crate) level_played_time: u32,
    /// C++ `CONFIG_MAX_PLAYER_LEVEL`. `RestMgr::SetRestBonus` reads this value
    /// directly; `Player::IsMaxLevel` reads the expansion-bounded active field.
    pub(in crate::session) max_player_level_config_like_cpp: u32,
    /// C++ `CONFIG_MAX_PRIMARY_TRADE_SKILL`, kept independent from talent
    /// `CharacterPoints` and from the two physical profession associations.
    pub(in crate::session) max_primary_trade_skills_like_cpp: u8,
    /// C++ `World::IsPvPRealm()` classification.
    pub(in crate::session) is_pvp_realm_like_cpp: bool,
    /// C++ `World::IsFFAPvPRealm()` classification.
    pub(in crate::session) is_ffa_pvp_realm_like_cpp: bool,
    /// C++ Recruit-A-Friend XP level gates used by `Player::GetsRecruitAFriendBonus(true)`.
    pub(in crate::session) max_recruit_a_friend_bonus_player_level_like_cpp: u32,
    pub(in crate::session) max_recruit_a_friend_bonus_player_level_difference_like_cpp: u32,
    /// Handle-less RestMgr and rate-policy fixture; production state belongs to Player.
    #[cfg(test)]
    pub(in crate::session) rest_mgr_test_fixture_like_cpp: RestMgrTestFixtureLikeCpp,
    /// Detached loaded Player flag values used only by persistence tests.
    #[cfg(test)]
    pub(in crate::session) player_flags_test_fixture_like_cpp: LoadedPlayerFlagsTestFixtureLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    /// Production money lives exclusively in `Player::ActivePlayerData::Coinage`.
    #[cfg(test)]
    pub(in crate::session) player_gold: u64,
    /// Handle-less test fallback for C++ `Player::_specializationInfo.ResetTalentsCost`.
    #[cfg(test)]
    pub(in crate::session) represented_talent_reset_cost_like_cpp: u32,
    /// Handle-less test fallback for C++ `Player::_specializationInfo.ResetTalentsTime`.
    #[cfg(test)]
    pub(in crate::session) represented_talent_reset_time_secs_like_cpp: u64,
    #[cfg(test)]
    pub(in crate::session) player_item_test_fixture_like_cpp: PlayerItemTestFixtureLikeCpp,
    /// C++ `UF::ActivePlayerData::CharacterPoints`, recalculated by InitTalentForLevel/LearnTalent.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_character_points_like_cpp: i32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) represented_player_powers_like_cpp: CharacterPowerSnapshotLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) represented_player_max_powers_like_cpp: CharacterPowerSnapshotLikeCpp,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) represented_player_base_mana_like_cpp: i32,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) represented_bank_bag_slot_flags_like_cpp: [u32; 7],
    #[cfg(test)]
    pub(in crate::session) represented_bank_item_moves_like_cpp:
        Vec<RepresentedBankItemMoveLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_inventory_moves_like_cpp:
        Vec<RepresentedGuildBankInventoryMoveLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_list_requests_like_cpp:
        Vec<RepresentedGuildBankListRequestLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_money_moves_like_cpp:
        Vec<RepresentedGuildBankMoneyMoveLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_bank_tab_actions_like_cpp:
        Vec<RepresentedGuildBankTabActionLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_replicate_requests_like_cpp:
        Vec<RepresentedAuctionReplicateRequestLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_place_bids_like_cpp:
        Vec<RepresentedAuctionPlaceBidLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_remove_items_like_cpp:
        Vec<RepresentedAuctionRemoveItemLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auction_sell_items_like_cpp:
        Vec<RepresentedAuctionSellItemLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_auto_unequip_offhand_requests_like_cpp:
        Vec<RepresentedAutoUnequipOffhandLikeCpp>,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_xp: u32,
    /// XP required to reach next level, cached from player_xp_for_level.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_next_level_xp: u32,
    /// Currently selected target GUID (SetSelection).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) selection_guid: Option<wow_core::ObjectGuid>,

    /// GUID of the character currently logged in (set after login completes).
    pub(in crate::session) player_guid: Option<ObjectGuid>,
    /// C++ `WorldSession::m_GUIDLow`: last logged-in character low GUID kept after logout.
    pub(in crate::session) recent_player_guid_low_like_cpp: u64,
    /// Test fixtures may attach a Player bootstrap before injecting the
    /// production MapManager. Production attachment is represented solely by
    /// the generation-checked PlayerHandle.
    #[cfg(test)]
    pub(in crate::session) player_bootstrap_attached_like_cpp: bool,
    /// C++ `WorldSession::_accountData`, represented in-memory until DB load/save is wired.
    pub(in crate::session) account_data_like_cpp: [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES],
    /// C++ `WorldSession::_tutorials`, account-scoped tutorial completion flags.
    pub(in crate::session) tutorials_like_cpp: [u32; 8],
    pub(in crate::session) tutorials_loaded_from_db_like_cpp: bool,
    pub(in crate::session) tutorials_loaded_coherently_like_cpp: bool,
    pub(in crate::session) tutorials_changed_like_cpp: bool,

    /// Pending creature spawn request (set during login, processed async).
    pub(crate) pending_creature_spawn: Option<PendingCreatureSpawn>,
    /// Creature kills observed from synchronous melee ticks and completed in `process_pending`.
    pub(in crate::session) pending_creature_kill_loot_like_cpp: Vec<ObjectGuid>,
    pub(in crate::session) pending_creature_kill_rewards_like_cpp:
        Vec<PendingCreatureKillRewardLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_creature_kill_events_like_cpp:
        Vec<RepresentedCreatureKillEventLikeCpp>,

    /// Detached guild membership and invitation state used only by tests.
    #[cfg(test)]
    pub(in crate::session) guild_test_fixture_like_cpp: GuildTestFixtureLikeCpp,
    /// Calendar request evidence used only by detached Session tests.
    #[cfg(test)]
    pub(in crate::session) calendar_test_fixture_like_cpp: CalendarTestFixtureLikeCpp,
    #[cfg(test)]
    pub(in crate::session) represented_arena_team_id_invited_like_cpp: u32,
    #[cfg(test)]
    pub(in crate::session) represented_wargame_invite_acceptances_like_cpp:
        Vec<RepresentedWargameInviteAcceptanceLikeCpp>,
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
    pub(in crate::session) represented_guild_repair_bank_state_like_cpp:
        Option<RepresentedGuildRepairBankStateLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_guild_repair_bank_withdraws_like_cpp:
        Vec<RepresentedGuildRepairBankWithdrawLikeCpp>,

    /// Legacy handle-less test fixture for C++ `Player::_currencyStorage`.
    #[cfg(test)]
    pub(in crate::session) player_currencies: HashMap<u32, PlayerCurrency>,
    pub(in crate::session) represented_quest_objective_progress_events_like_cpp:
        VecDeque<RepresentedQuestObjectiveProgressEventLikeCpp>,
    pub(in crate::session) represented_quest_objective_progress_draining_like_cpp: bool,

    /// In-memory item objects keyed by item GUID, mirroring C++ `Player::m_items` ownership.
    #[cfg(test)]
    pub(in crate::session) inventory_item_objects: HashMap<ObjectGuid, Item>,

    /// Current map ID for VALUES update packets.
    pub(in crate::session) current_map_id: u16,

    /// Login-only identity input consumed while the canonical Player is being
    /// constructed. Production reads resolve from that Player after install.
    pub(in crate::session) player_identity_bootstrap_like_cpp:
        Option<PlayerIdentityBootstrapLikeCpp>,

    /// Detached fixture identity; production identity belongs to Player.
    #[cfg(test)]
    pub(in crate::session) player_race: u8,
    #[cfg(test)]
    pub(in crate::session) player_class: u8,
    #[cfg(test)]
    pub(in crate::session) player_level: u8,
    #[cfg(test)]
    pub(in crate::session) player_gender: u8,
    /// C++ `Player::m_createMode`, loaded from `characters.createMode`.
    #[cfg(test)]
    pub(in crate::session) player_create_mode_like_cpp: u8,
    /// Represented C++ `Player::GetShapeshiftForm()` until shapeshift aura state owns it.
    #[cfg(test)]
    pub(in crate::session) represented_shapeshift_form_like_cpp: u32,
    /// C++ ActivePlayerData::LootSpecID represented session state.
    #[cfg(test)]
    pub(in crate::session) loot_specialization_id: u32,
    /// Represented C++ ActivePlayerData::CurrentSpecID / GetPrimarySpecialization.
    #[cfg(test)]
    pub(crate) represented_primary_specialization_id_like_cpp: u32,
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
    /// C++ `Player::m_weaponProficiency`; `Spell::EffectProficiency` ORs into it.
    #[cfg(test)]
    pub(in crate::session) represented_weapon_proficiency_like_cpp: u32,
    /// C++ `Player::m_armorProficiency`; `Spell::EffectProficiency` ORs into it.
    #[cfg(test)]
    pub(in crate::session) represented_armor_proficiency_like_cpp: u32,
    /// C++ `CollectionMgr::_mounts` represented account mount collection.
    #[cfg(test)]
    pub(in crate::session) account_mounts_like_cpp: HashMap<i32, u8>,
    /// Login snapshot of the player's spell history + charge packets. C++ reads these
    /// live from `Player::GetSpellHistory()` in `SendInitialPacketsBeforeAddToMap`; Rust
    /// persists the login snapshot so the before-add helper can re-send it on far teleport
    /// without a DB round trip. #NEXT.R8.ENTITIES.1229.
    #[cfg(test)]
    pub(in crate::session) represented_spell_history_packets_like_cpp:
        (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>),
    /// Handle-less unit-test fallback; production C++ `Player::_CUFProfiles` lives on canonical
    /// `wow_entities::Player`.
    #[cfg(test)]
    pub(in crate::session) cuf_profiles_like_cpp:
        Vec<Option<wow_packet::packets::misc::CufProfile>>,
    #[cfg(test)]
    pub(in crate::session) cuf_profiles_loaded_like_cpp: bool,

    // ── Dual-connection (realm + instance) ───────────────────────
    // After ConnectTo completes, the session uses the instance socket for
    // game packets but MUST keep the realm socket alive — the WoW client
    // disconnects if either connection drops.

    // ── Movement & World position ─────────────────────────────────
    /// Server-side position of the player (updated from CMSG_MOVE_*).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_position: Option<wow_core::Position>,
    /// Last accepted player movement flags, mirroring C++ `Unit::m_movementInfo`.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_movement_flags_like_cpp: MovementFlag,
    /// Represented C++ `MOVEMENTFLAG2_CAN_SWIM_TO_FLY_TRANS` server-controlled state.
    #[cfg(test)]
    pub(in crate::session) represented_can_swim_to_fly_transition_like_cpp: bool,
    /// Represented `m_unitMovedByMe->GetVehicle()->GetVehicleInfo()->Flags & VEHICLE_FLAG_FIXED_POSITION`.
    #[cfg(test)]
    pub(in crate::session) represented_mover_fixed_position_vehicle_like_cpp: bool,

    /// Cached character name for chat messages.
    /// Detached fixture identity; production name belongs to the canonical Player.
    #[cfg(test)]
    pub(in crate::session) player_name: Option<String>,

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
    pub(in crate::session) vendor_buy_item_test_override_like_cpp:
        Option<VendorBuyItemTestOverrideLikeCpp>,

    /// Shared, server-wide map state. When `Some`, creature reads/writes can
    /// route through here so all sessions on the same map see the same world.
    /// `None` until the world server injects the manager (see `set_map_manager`).
    pub(crate) map_manager: Option<crate::map_manager::SharedMapManager>,
    /// Canonical C++-style `wow-map` manager. This is injected separately from
    /// the legacy `wow-world` manager while handlers migrate to `wow-map`.
    pub(crate) canonical_map_manager: Option<SharedCanonicalMapManager>,
    /// Generation-checked identity of the one canonical Player value owned by
    /// MapManager. It remains resolvable while detached for a far teleport.
    pub(in crate::session) player_handle_like_cpp: Option<wow_map::PlayerHandle>,
    /// Set by the first canonical map-phase request (#787). Until then this
    /// session has no coordinator and keeps draining its own queue.
    pub(in crate::session) map_phase_coordinated_like_cpp: bool,
    /// Dedicated Detour owner handle. The underlying `MMapManager` remains on
    /// its worker thread because Detour state is not `Send + Sync`.
    pub(in crate::session) mmap_pathfinder_like_cpp: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    /// Shared C++ `InstanceLockMgr` analogue used by raid-info and instance entry paths.
    pub(crate) instance_lock_mgr: Option<Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>>,

    // ── Combat state ─────────────────────────────────────────────
    /// Current auto-attack target (None if not in combat).
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(crate) combat_target: Option<wow_core::ObjectGuid>,
    /// Last represented player melee tick used to decrement C++ `m_attackTimer`.
    pub(in crate::session) combat_tick_last_at_like_cpp: Instant,
    /// True when the player is engaged in combat.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(crate) in_combat: bool,
    /// Test-only legacy fixture for sessions without an installed Player owner.
    #[cfg(test)]
    pub(in crate::session) player_alive_like_cpp: bool,
    /// Handle-less fixture for C++ `Player::IsGameMaster()`.
    #[cfg(test)]
    pub(in crate::session) player_game_master_like_cpp: bool,
    /// Represented `CHEAT_GOD` movement/fall guard.
    #[cfg(test)]
    pub(in crate::session) player_cheat_god_like_cpp: bool,
    /// Represented `IsImmunedToDamage(SPELL_SCHOOL_MASK_NORMAL)` fall guard.
    #[cfg(test)]
    pub(in crate::session) player_normal_damage_immune_like_cpp: bool,
    /// Represented `IsImmuneToEnvironmentalDamage()` guard inside EnvironmentalDamage.
    #[cfg(test)]
    pub(in crate::session) player_environmental_damage_immune_like_cpp: bool,
    /// Test-only legacy health fixture for sessions without a Player handle.
    #[cfg(test)]
    pub(in crate::session) player_health_like_cpp: u32,
    /// Test-only legacy max-health fixture for sessions without a Player handle.
    #[cfg(test)]
    pub(in crate::session) player_max_health_like_cpp: u32,
    /// High-water mark for map-owned creature-melee presentation commands.
    /// Canonical health/death authority lives on `wow-map`; this suppresses
    /// durable FIFO replay without writing delayed values back to that owner.
    pub(in crate::session) last_presented_creature_melee_health_state_revision_like_cpp: u64,
    /// Represented `Unit::m_movementInfo.time` for client movement ACK side effects.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_movement_time_like_cpp: u32,
    /// Represented `Unit::m_movementInfo.jump`, reset by `Player::TeleportTo`.
    /// Test-only evidence: production consumes the typed movement status and
    /// does not retain a second packet-shaped jump mirror.
    #[cfg(test)]
    pub(in crate::session) player_movement_jump_like_cpp: wow_packet::packets::movement::JumpInfo,
    /// C++ `Player::m_lastFallTime`.
    #[cfg(test)]
    pub(in crate::session) last_fall_time_like_cpp: u32,
    /// C++ `Player::m_lastFallZ`.
    #[cfg(test)]
    pub(in crate::session) last_fall_z_like_cpp: f32,
    /// Recorded fall damage events until combat log/update packet runtime is complete.
    #[cfg(test)]
    pub(in crate::session) fall_damage_events_like_cpp: Vec<MovementFallDamageEvent>,
    /// C++ `PLAYER_FLAGS_IS_OUT_OF_BOUNDS` represented state.
    #[cfg(test)]
    pub(in crate::session) player_out_of_bounds_like_cpp: bool,
    /// Recorded `DAMAGE_FALL_TO_VOID` events until environmental damage packets are complete.
    #[cfg(test)]
    pub(in crate::session) under_map_damage_events_like_cpp: Vec<MovementUnderMapDamageEvent>,
    /// Represented stand state used by movement side effects until UnitData owns it.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_stand_state_like_cpp: UnitStandStateType,
    /// Test-only successful represented->live evidence. Production emits
    /// bounded structured telemetry instead of retaining client-controlled
    /// history for the lifetime of the session.
    #[cfg(test)]
    pub(in crate::session) represented_live_applications_like_cpp:
        Vec<RepresentedLiveApplicationLikeCpp>,
    /// Represented `UnitData::EmoteState`, used to clear stateful emotes on movement like C++.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_emote_state_like_cpp: u32,
    /// Count of C++ temporary pet unsummon side effects requested by movement.
    #[cfg(test)]
    pub(in crate::session) temporary_pet_unsummon_requests_like_cpp: u32,
    /// Count of C++ jump proc side effects requested by movement.
    #[cfg(test)]
    pub(in crate::session) movement_jump_proc_requests_like_cpp: u32,
    /// Represented `ActivePlayerData::LocalFlags`.
    #[cfg(test)]
    pub(in crate::session) active_player_local_flags_like_cpp: u32,
    /// Represented `ActivePlayerData::TransportServerTime`.
    #[cfg(test)]
    pub(in crate::session) active_player_transport_server_time_like_cpp: i32,
    /// Represented `ActivePlayerData::MultiActionBars`.
    #[cfg(test)]
    pub(in crate::session) active_player_multi_action_bars_like_cpp: u8,
    /// Test-only action-button owner for fixtures without a canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_action_buttons_like_cpp:
        [u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS],
    #[cfg(test)]
    pub(in crate::session) represented_action_buttons_loaded_like_cpp: bool,
    /// C++ `Player::_advancedCombatLoggingEnabled`; consumed when combat-log fanout selects full/basic payloads.
    /// C++ `WorldSession::_filterAddonMessages`' sibling for
    /// `SMSG_SPELL_GO`: shared so a producer can commit the combat-log packet
    /// variant per recipient while distributing a cast, the way C++ selects it
    /// synchronously inside `WorldObject::SendCombatLogMessage`.
    pub(in crate::session) advanced_combat_logging_enabled_like_cpp: Arc<AtomicBool>,
    /// C++ `Player::GetUnitBeingMoved()` represented GUID.
    #[cfg(test)]
    pub(in crate::session) player_moved_unit_guid_like_cpp: ObjectGuid,
    /// Count of visibility refreshes requested by movement initialization.
    pub(in crate::session) movement_visibility_refresh_requests_like_cpp: u32,
    /// ACKs accepted by represented movement handling until full Unit movement runtime/broadcasts exist.
    #[cfg(test)]
    pub(in crate::session) movement_ack_events_like_cpp: Vec<MovementAckEventLikeCpp>,
    /// Represented `PlayerTaxi::m_TaxiDestinations` until PlayerTaxi/MotionMaster runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) taxi_destinations_like_cpp: Vec<u32>,
    /// Represented accepted `CMSG_ACTIVATE_TAXI` requests until TaxiPathGraph/MotionMaster are canonical.
    #[cfg(test)]
    pub(in crate::session) represented_activate_taxi_requests_like_cpp:
        Vec<RepresentedActivateTaxiLikeCpp>,
    /// Represented accepted barber-shop requests until ChrCustomization DB2/cost/update runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_alter_appearance_requests_like_cpp:
        Vec<RepresentedAlterAppearanceLikeCpp>,
    /// Represented accepted barber confirmation requests until Player::SetCustomizations is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_confirm_barbers_choice_requests_like_cpp:
        Vec<RepresentedConfirmBarbersChoiceLikeCpp>,
    /// Represented accepted talent-respec wipe requests until Player::ResetTalents is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_confirm_respec_wipe_requests_like_cpp:
        Vec<RepresentedConfirmRespecWipeLikeCpp>,
    /// C++ `Player::m_atLoginFlags`, represented for reset-on-login side effects.
    #[cfg(test)]
    pub(in crate::session) represented_at_login_flags_like_cpp: u16,
    /// Represented `sScriptMgr->OnPlayerTalentsReset` calls until ScriptMgr is live.
    #[cfg(test)]
    pub(in crate::session) represented_talent_reset_script_hooks_like_cpp:
        Vec<RepresentedTalentResetScriptHookLikeCpp>,
    /// Represented persistent `RemoveAtLoginFlag` calls until direct character DB execution is live.
    #[cfg(test)]
    pub(in crate::session) represented_at_login_flag_removals_like_cpp:
        Vec<RepresentedAtLoginFlagRemovalLikeCpp>,
    /// Represented `unit->CastSpell(_player, 14867, true)` after successful talent reset.
    #[cfg(test)]
    pub(in crate::session) represented_talent_respec_visual_spell_casts_like_cpp:
        Vec<RepresentedTalentRespecVisualSpellCastLikeCpp>,
    /// Represented `CriteriaType::MoneySpentOnRespecs` / `TotalRespecs` events.
    #[cfg(test)]
    pub(in crate::session) represented_talent_respec_criteria_events_like_cpp:
        Vec<RepresentedTalentRespecCriteriaEventLikeCpp>,
    /// Handle-less test fallback; production C++ `Player::_equipmentSets` lives on canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_equipment_sets_like_cpp:
        wow_entities::PlayerEquipmentSetsLikeCpp,
    /// Handle-less test fallback; production C++ `Player::_voidStorageItems` lives on canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_void_storage_items_like_cpp:
        [Option<RepresentedVoidStorageItemLikeCpp>;
            wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP],
    #[cfg(test)]
    pub(in crate::session) represented_void_storage_loaded_like_cpp: bool,
    /// Represented accepted Adventure Map quest starts until AddQuestAndCheckCompletion is canonical.
    #[cfg(test)]
    pub(in crate::session) represented_adventure_map_start_quest_requests_like_cpp:
        Vec<RepresentedAdventureMapStartQuestLikeCpp>,
    /// Minimal TaxiNodes.db2 map lookup used by represented `MoveSplineDone` taxi transitions.
    pub(in crate::session) taxi_node_map_ids_like_cpp: HashMap<u32, u16>,
    /// Represented active `FlightPathMovementGenerator`, if any.
    #[cfg(test)]
    pub(in crate::session) taxi_flight_state_like_cpp: Option<RepresentedTaxiFlightStateLikeCpp>,
    /// Represented unit flags touched by `CleanupAfterTaxiFlight`.
    #[cfg(test)]
    pub(in crate::session) taxi_unit_flags_like_cpp: UnitFlags,
    /// Represented mount state touched by `CleanupAfterTaxiFlight`.
    #[cfg(test)]
    pub(in crate::session) taxi_mounted_like_cpp: bool,
    /// Handle-less fixture for C++ `Unit::GetMountDisplayId()`.
    #[cfg(test)]
    pub(in crate::session) player_mount_display_id_like_cpp: i32,
    /// Represented vehicle id selected from mount creature template until VehicleKit exists.
    #[cfg(test)]
    pub(in crate::session) player_mount_vehicle_id_like_cpp: u32,
    /// Legacy handle-less test fixture for C++ `Unit::m_vehicleKit`.
    #[cfg(test)]
    pub(crate) player_mount_vehicle_kit_like_cpp: Option<Vehicle>,
    /// Vehicle accessory rows selected by C++ `Vehicle::InstallAllAccessories(false)`.
    #[cfg(test)]
    pub(in crate::session) player_mount_vehicle_accessories_like_cpp: Vec<VehicleAccessory>,
    /// Represented number of VehicleSeat rows installed by C++ `Vehicle` constructor.
    #[cfg(test)]
    pub(in crate::session) player_mount_vehicle_seat_count_like_cpp: u8,
    /// Represented C++ `Vehicle::UsableSeatNum`.
    #[cfg(test)]
    pub(in crate::session) player_mount_vehicle_usable_seat_count_like_cpp: u8,
    /// Legacy handle-less test fixture for current `VehicleSeatEntry::Flags`.
    #[cfg(test)]
    pub(crate) player_vehicle_seat_flags_like_cpp: Option<i32>,
    /// Legacy handle-less test fixture for current `VehicleSeatEntry::ID`.
    #[cfg(test)]
    pub(crate) player_vehicle_seat_id_like_cpp: Option<u32>,
    /// Represented `Player::ChangeSeat(seatId, next)` requests until live vehicle ownership exists.
    #[cfg(test)]
    pub(in crate::session) represented_vehicle_seat_change_requests_like_cpp:
        Vec<RepresentedVehicleSeatChangeRequestLikeCpp>,
    /// Represented cross-vehicle `HandleSpellClick(player, seatId)` requests from vehicle switching.
    #[cfg(test)]
    pub(in crate::session) represented_vehicle_seat_spell_click_requests_like_cpp:
        Vec<RepresentedVehicleSeatSpellClickRequestLikeCpp>,
    /// Represented `Player::EnterVehicle(targetPlayer)` requests from `CMSG_RIDE_VEHICLE_INTERACT`.
    #[cfg(test)]
    pub(in crate::session) represented_vehicle_enter_requests_like_cpp:
        Vec<RepresentedVehicleEnterRequestLikeCpp>,
    /// Represented `m_movementInfo = MoveDismissVehicle.Status` before live `ExitVehicle`.
    #[cfg(test)]
    pub(in crate::session) represented_vehicle_dismiss_movements_like_cpp:
        Vec<RepresentedVehicleDismissMovementLikeCpp>,
    /// Represented `vehicle_base->m_movementInfo = MoveChangeVehicleSeats.Status`.
    #[cfg(test)]
    pub(in crate::session) represented_vehicle_base_movements_like_cpp:
        Vec<RepresentedVehicleBaseMovementLikeCpp>,
    /// Represented `Player::GetBattleground()->GetTypeID()` for C++ battleground object use.
    #[cfg(test)]
    pub(in crate::session) player_battleground_type_id_like_cpp: Option<u32>,
    /// Represented `Player::GetBattleground()->GetMapId()` until live Battleground ownership exists.
    #[cfg(test)]
    pub(in crate::session) player_battleground_map_id_like_cpp: Option<u32>,
    /// Represented `Battleground::GetStatus()` until live Battleground ownership exists.
    #[cfg(test)]
    pub(in crate::session) represented_battleground_status_like_cpp: Option<u8>,
    /// Count of represented `Player::LeaveBattleground()` requests.
    #[cfg(test)]
    pub(in crate::session) represented_battleground_leave_requests_like_cpp: u32,
    /// Represented `BattlegroundMgr::SendBattlegroundList` intents from battlemaster hello.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_hellos_like_cpp:
        Vec<RepresentedBattlemasterHelloLikeCpp>,
    /// Represented `BattlegroundMgr::SendBattlegroundList` intents from CMSG_BATTLEFIELD_LIST.
    #[cfg(test)]
    pub(in crate::session) represented_battlefield_lists_like_cpp:
        Vec<RepresentedBattlefieldListLikeCpp>,
    /// Represented `BattlegroundQueue::AddGroup` intents from CMSG_BATTLEMASTER_JOIN.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_joins_like_cpp:
        Vec<RepresentedBattlemasterJoinLikeCpp>,
    /// Represented rated arena queue intents from CMSG_BATTLEMASTER_JOIN_ARENA.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_join_arenas_like_cpp:
        Vec<RepresentedBattlemasterJoinArenaLikeCpp>,
    /// Represented arena skirmish queue intents from CMSG_BATTLEMASTER_JOIN_SKIRMISH.
    #[cfg(test)]
    pub(in crate::session) represented_battlemaster_join_skirmishes_like_cpp:
        Vec<RepresentedBattlemasterJoinSkirmishLikeCpp>,
    /// Represented `Player::m_bgBattlegroundQueueID[PLAYER_MAX_BATTLEGROUND_QUEUES]`.
    #[cfg(test)]
    pub(in crate::session) represented_battleground_queue_slots_like_cpp:
        Vec<RepresentedBattlegroundQueueSlotLikeCpp>,
    /// Represented accepted/leave requests from CMSG_BATTLEFIELD_PORT before live BattlegroundMgr.
    #[cfg(test)]
    pub(in crate::session) represented_battlefield_ports_like_cpp:
        Vec<RepresentedBattlefieldPortLikeCpp>,
    /// C++ `Player::_areaSpiritHealerGUID`, represented until battleground/player resurrection owns it.
    #[cfg(test)]
    pub(in crate::session) area_spirit_healer_guid_like_cpp: ObjectGuid,
    /// Legacy handle-less test fixture for the current Player-owned pet GUID.
    #[cfg(test)]
    pub(crate) represented_pet_guid_like_cpp: Option<ObjectGuid>,
    /// C++ `Player::m_temporaryUnsummonedPetNumber`, represented until pet DB load/resummon is live.
    #[cfg(test)]
    pub(in crate::session) represented_temporary_unsummoned_pet_number_like_cpp: u32,
    /// C++ `Player::m_oldpetspell`, used by `RemovePet(nullptr, ..., returnreagent=true)`.
    #[cfg(test)]
    pub(in crate::session) represented_old_pet_spell_like_cpp: u32,
    /// Represented `character_pet`/stable rows until `Pet::LoadPetFromDB` is wired to DB.
    #[cfg(test)]
    pub(in crate::session) represented_pet_stable_like_cpp: PetStable,
    /// True only after the current Player's complete `character_pet` query
    /// returned no rows. Any later pet load or lifetime mutation revokes this
    /// narrow proof instead of attempting to model pet-to-owner aura casts.
    #[cfg(test)]
    pub(in crate::session) represented_character_pet_rows_empty_authority_complete_like_cpp: bool,
    /// Per-character asynchronous C++ `PetLoadQueryHolder` result lifetime.
    pub(in crate::session) pet_load_query_holder_rows_like_cpp:
        lifecycle::PetLoadQueryHolderRowsLikeCpp,
    /// Represented `Pet::m_unitData->CreatedBySpell` for the active pet until UnitData owns it.
    #[cfg(test)]
    pub(in crate::session) represented_pet_created_by_spell_like_cpp: u32,
    /// Represented current pet react state for C++ mount/dismount PetMode side effects.
    #[cfg(test)]
    pub(in crate::session) represented_pet_react_state_like_cpp: u8,
    /// Represented current pet command state for C++ mount/dismount PetMode side effects.
    #[cfg(test)]
    pub(in crate::session) represented_pet_command_state_like_cpp: u8,
    /// C++ `Player::m_temporaryPetReactState` saved by `DisablePetControlsOnMount`.
    #[cfg(test)]
    pub(in crate::session) temporary_mount_pet_react_state_like_cpp: Option<u8>,
    /// Count of C++ `CreateVehicleKit` mount side effects represented until Vehicle runtime sends packets.
    #[cfg(test)]
    pub(in crate::session) mount_vehicle_create_requests_like_cpp: u32,
    /// Count of C++ `RemoveVehicleKit` mount side effects represented until Vehicle runtime sends packets.
    #[cfg(test)]
    pub(in crate::session) mount_vehicle_remove_requests_like_cpp: u32,
    /// Count of C++ `SendOnCancelExpectedVehicleRideAura` packets emitted after vehicle-kit creation.
    #[cfg(test)]
    pub(in crate::session) mount_cancel_expected_vehicle_aura_packets_like_cpp: u32,
    /// Count of C++ `DisablePetControlsOnMount` side effects represented until pet runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) mount_pet_control_disable_requests_like_cpp: u32,
    /// Count of C++ `EnablePetControlsOnDismount` side effects represented until pet runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) mount_pet_control_enable_requests_like_cpp: u32,
    /// Count of C++ mount/dismount pet resummon calls represented until pet runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) mount_pet_resummon_requests_like_cpp: u32,
    /// Count of C++ mount collision-height updates represented until movement packets are canonical.
    #[cfg(test)]
    pub(in crate::session) mount_collision_height_update_requests_like_cpp: u32,
    /// C++ `Unit::m_movementCounter`: one per-player counter shared by ALL movement-control
    /// packets (vehicle-rec, collision height, near-teleport, speed/flag changes) and read
    /// for `SMSG_RESUME_TOKEN` SequenceIndex on far teleport. Reset to 0 in
    /// `send_initial_packets_before_add_to_map` (non-seamless). #NEXT.R8.ENTITIES.1229.
    #[cfg(test)]
    pub(in crate::session) movement_counter_like_cpp: u32,
    /// Represented `Unit::GetCollisionHeight()` until model-display collision data owns it.
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_collision_height_like_cpp: f32,
    /// Handle-less fixture for C++ `Object::GetObjectScale()`.
    #[cfg(test)]
    pub(in crate::session) player_object_scale_like_cpp: f32,
    /// Represented `UnitData::ScaleDuration` for movement collision-height packets.
    #[cfg(test)]
    pub(in crate::session) player_scale_duration_like_cpp: i32,
    /// Handle-less fixture for C++ `UnitData::Flags`.
    #[cfg(test)]
    pub(in crate::session) player_unit_flags_like_cpp: UnitFlags,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    #[cfg(test)]
    pub(in crate::session) player_faction_template_like_cpp: Option<u32>,
    /// Handle-less fixture for C++ `UNIT_FLAG_MOUNT`.
    #[cfg(test)]
    pub(in crate::session) player_mounted_like_cpp: bool,
    /// Represented `pvpInfo.IsHostile` branch for Honorless Target after taxi landing.
    #[cfg(test)]
    pub(in crate::session) player_pvp_hostile_like_cpp: bool,
    /// Represented `Player::IsPvP()` branch for friendly-area near teleport handling.
    #[cfg(test)]
    pub(in crate::session) player_pvp_enabled_like_cpp: bool,
    /// Represented `PLAYER_FLAGS_IN_PVP` branch for friendly-area near teleport handling.
    #[cfg(test)]
    pub(in crate::session) player_in_pvp_flag_like_cpp: bool,
    /// Represented `pvpInfo.EndTimer` consumed by `Player::UpdatePvPFlag`.
    #[cfg(test)]
    pub(in crate::session) player_pvp_end_timer_like_cpp: Option<i64>,
    /// C++ `Player::m_contestedPvPTimer`, reset by `Player::ResetContestedPvP`.
    #[cfg(test)]
    pub(in crate::session) player_contested_pvp_timer_like_cpp: u32,
    /// Current represented zone/area ids until Map/Terrain runtime can calculate them.
    #[cfg(test)]
    pub(in crate::session) player_zone_id_like_cpp: u32,
    #[cfg(test)]
    pub(in crate::session) player_area_id_like_cpp: u32,
    /// True only when the current zone/area came from an extracted C++ terrain
    /// tile, rather than the DB-seeded or map-wide fallback.
    #[cfg(test)]
    pub(in crate::session) player_zone_area_authority_complete_like_cpp: bool,
    /// Permanent fail-closed marker for this C++ Player lifetime after a login
    /// cast closure that Rust did not retain losslessly (currently FIRST).
    #[cfg(test)]
    pub(in crate::session) player_spell_hit_aura_authority_tombstoned_like_cpp: bool,
    /// `MoveSplineDone` taxi decisions recorded until full Taxi/MotionMaster runtime exists.
    #[cfg(test)]
    pub(in crate::session) move_spline_done_taxi_events_like_cpp:
        Vec<MoveSplineDoneTaxiEventLikeCpp>,
    /// C++ `Player::m_bCanDelayTeleport`, represented around update-owned work.
    #[cfg(test)]
    pub(in crate::session) represented_can_delay_teleport_like_cpp: bool,
    /// C++ `Player::m_bHasDelayedTeleport`, represented for same-map near teleports.
    #[cfg(test)]
    pub(in crate::session) represented_has_delayed_teleport_like_cpp: bool,
    /// C++ `Player::mSemaphoreTeleport_Near` represented state.
    #[cfg(test)]
    pub(in crate::session) near_teleport_pending_like_cpp: bool,
    /// C++ `Player::mSemaphoreTeleport_Far` represented state.
    #[cfg(test)]
    pub(in crate::session) represented_far_teleport_pending_like_cpp: bool,
    /// C++ `Player::m_teleport_dest` represented state for near teleports.
    #[cfg(test)]
    pub(in crate::session) near_teleport_destination_like_cpp: Option<(u16, wow_core::Position)>,
    /// Saved `TeleportTo` arguments while `m_bHasDelayedTeleport` is set.
    #[cfg(test)]
    pub(in crate::session) represented_delayed_teleport_like_cpp:
        Option<(u32, wow_core::Position, TeleportToOptionsLikeCpp)>,
    /// Represented zone/area for the pending near-teleport destination.
    #[cfg(test)]
    pub(in crate::session) near_teleport_destination_zone_area_like_cpp: Option<(u32, u32)>,
    /// Handle-less compatibility for older tests. Production C++
    /// `Player::m_homebind` lives on the canonical Player.
    #[cfg(test)]
    pub(in crate::session) represented_homebind_like_cpp: Option<RepresentedHomebindLikeCpp>,
    /// C++ `Player::_resurrectionData`, represented until real Player/Spell
    /// resurrection request ownership exists.
    #[cfg(test)]
    pub(in crate::session) represented_resurrection_request_like_cpp:
        Option<PlayerResurrectionRequestLikeCpp>,
    /// C++ `DELAYED_RESURRECT_PLAYER`, represented for resurrection requests
    /// that initiate teleport and must apply after WorldPortResponse.
    #[cfg(test)]
    pub(in crate::session) represented_delayed_resurrection_after_teleport_like_cpp:
        Option<PlayerResurrectionRequestLikeCpp>,
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
    /// C++ `CONFIG_CAST_UNSTUCK` immutable world policy injected into spell effects.
    pub(in crate::session) represented_cast_unstuck_enabled_like_cpp: bool,
    /// C++ `Player::GetDeathTimer()` represented for `Spell::EffectStuck`.
    #[cfg(test)]
    pub(in crate::session) represented_death_timer_active_like_cpp: bool,
    /// Near teleport ACK side-effect audit events.
    #[cfg(test)]
    pub(in crate::session) move_teleport_ack_events_like_cpp: Vec<MoveTeleportAckEventLikeCpp>,
    /// Count of C++ `ResummonPetTemporaryUnSummonedIfAny` calls after near teleport ACK.
    #[cfg(test)]
    pub(in crate::session) temporary_pet_resummon_requests_like_cpp: u32,
    /// Count of C++ `ProcessDelayedOperations` calls after successful near teleport ACK.
    #[cfg(test)]
    pub(in crate::session) delayed_operations_processed_like_cpp: u32,
    /// C++ `Player::m_forced_speed_changes[MAX_MOVE_TYPE]` represented state.
    #[cfg(test)]
    pub(in crate::session) forced_speed_changes_like_cpp: [u8; UnitMoveTypeLikeCpp::COUNT],
    /// C++ `Unit::m_speed_rate[MAX_MOVE_TYPE]` represented state for player-controlled movers.
    #[cfg(test)]
    pub(in crate::session) movement_speed_rates_like_cpp: [f32; UnitMoveTypeLikeCpp::COUNT],
    /// C++ `Player::GetPet()->SetSpeedRate` propagation represented until pet Unit runtime owns it.
    #[cfg(test)]
    pub(in crate::session) represented_pet_movement_speed_rates_like_cpp:
        [f32; UnitMoveTypeLikeCpp::COUNT],
    /// Count of represented player speed changes propagated to the active pet.
    #[cfg(test)]
    pub(in crate::session) represented_pet_speed_propagations_like_cpp: u32,
    /// Represented transport guard for speed ACK anticheat; C++ skips speed mismatch while on transport.
    #[cfg(test)]
    pub(in crate::session) player_on_transport_like_cpp: bool,
    /// C++ `Player::m_movementForceModMagnitudeChanges` represented state.
    #[cfg(test)]
    pub(in crate::session) movement_force_mod_magnitude_changes_like_cpp: u8,
    /// C++ `MovementForces::GetModMagnitude()` represented value; default is 1.0 when no force container exists.
    #[cfg(test)]
    pub(in crate::session) movement_force_mod_magnitude_like_cpp: f32,
    /// Speed ACK outcomes recorded until full Unit speed runtime owns this state.
    #[cfg(test)]
    pub(in crate::session) movement_speed_ack_events_like_cpp: Vec<MovementSpeedAckEventLikeCpp>,

    // ── Aura system ───────────────────────────────────────────────
    /// Legacy fixture mirror for tests that construct a Session without a
    /// canonical Player owner.
    #[cfg(test)]
    pub(crate) visible_auras: HashMap<u8, AuraApplication>,
    /// True only after both persisted aura tables were read successfully for
    /// the active character. Absence is evidence only while this is complete.
    #[cfg(test)]
    pub(in crate::session) player_aura_authority_complete_like_cpp: bool,
    /// True only when `SEL_CHAR_EQUIPMENT` succeeded and proved that the active
    /// character has no top-level persisted item rows. Empty runtime inventory
    /// alone is not source proof because malformed rows may be rejected.
    #[cfg(test)]
    pub(in crate::session) player_equipment_inventory_authority_complete_like_cpp: bool,
    /// Difficulty-selected C++ `AuraEffect` identity captured when the aura is applied.
    #[cfg(test)]
    pub(in crate::session) canonical_threat_aura_snapshots_like_cpp:
        HashMap<u8, CanonicalThreatAuraSnapshotLikeCpp>,

    pub(crate) spell_acquisition_cast_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCastAuthorityLikeCpp>>,
    pub(crate) spell_acquisition_craft_authority_like_cpp:
        Option<Arc<crate::spell_acquisition::SpellAcquisitionCraftValidityAuthorityLikeCpp>>,
    /// Effective C++ spell-script hooks. These remain optional so a session
    /// constructed without the startup audit fails closed.
    pub(in crate::session) spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) spell_script_all_rank_root_spell_ids_like_cpp:
        Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) spell_linked_rejected_trigger_spell_ids_like_cpp:
        Option<Arc<BTreeSet<u32>>>,
    pub(in crate::session) talent_store: Option<Arc<TalentStore>>,
    pub(in crate::session) num_talents_at_level_store: Option<Arc<NumTalentsAtLevelStore>>,
    pub(in crate::session) power_type_store: Option<Arc<PowerTypeStore>>,
    pub(in crate::session) cinematic_sequences_store: Option<Arc<CinematicSequencesStore>>,
    pub(in crate::session) movie_store: Option<Arc<MovieStore>>,
    #[cfg(test)]
    pub(in crate::session) represented_cinematic_state_like_cpp:
        wow_entities::PlayerCinematicStateLikeCpp,
    #[cfg(test)]
    pub(in crate::session) represented_cinematic_next_camera_events_like_cpp: Vec<u16>,
    #[cfg(test)]
    pub(in crate::session) represented_cinematic_end_events_like_cpp: Vec<u32>,
    #[cfg(test)]
    pub(in crate::session) represented_movie_complete_events_like_cpp: Vec<u32>,
    /// Detached support feature configuration used only by tests.
    #[cfg(test)]
    pub(in crate::session) support_feature_test_fixture_like_cpp: SupportFeatureTestFixtureLikeCpp,
    pub(in crate::session) script_name_interner: Option<Arc<ScriptNameInternerLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) object_mgr_catalogs_like_cpp: Option<Arc<ObjectMgrCatalogsLikeCpp>>,
    pub(in crate::session) gameobject_template_lifecycle_store_like_cpp:
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
    pub(in crate::session) represented_character_spell_cooldowns_like_cpp:
        HashMap<u32, RepresentedCharacterSpellCooldownLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_character_spell_cooldowns_loaded_like_cpp: bool,
    #[cfg(test)]
    pub(in crate::session) represented_character_spell_charges_like_cpp:
        BTreeMap<u32, Vec<RepresentedCharacterSpellChargeLikeCpp>>,
    #[cfg(test)]
    pub(in crate::session) represented_character_spell_charges_loaded_like_cpp: bool,
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
    #[cfg(test)]
    pub(crate) min_discovered_scaled_xp_ratio_like_cpp: u32,
    #[cfg(test)]
    pub(crate) quest_test_fixture_like_cpp: QuestTestFixtureLikeCpp,
    pub(crate) min_quest_scaled_xp_ratio_like_cpp: u32,
    pub(crate) quest_low_level_hide_diff_like_cpp: u32,
    pub(crate) quest_high_level_hide_diff_like_cpp: u32,
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
    /// Handle-less battle-pet state used only by isolated Session tests.
    #[cfg(test)]
    pub(crate) battle_pet_test_fixture_like_cpp: BattlePetTestFixtureLikeCpp,
    pub(in crate::session) battle_pet_account_attachment_like_cpp:
        Option<BattlePetAccountAttachmentLikeCpp>,
    /// C++ `Player::HasAchieved`, represented per-session until character achievements are fully loaded.
    #[cfg(test)]
    pub(crate) represented_completed_achievements_like_cpp: HashSet<u32>,
    /// C++ `Player::_instanceResetTimes`: instance id -> release time.
    #[cfg(test)]
    pub(crate) represented_instance_reset_times_like_cpp: BTreeMap<u32, u64>,
    /// Evidence for represented `Player::CompleteQuest` status-update side effects.
    pub(crate) represented_quest_complete_status_updates_like_cpp:
        Vec<RepresentedQuestCompleteStatusUpdateLikeCpp>,
    /// C++ `ActivePlayerData::ExploredZones`, represented before the canonical Player owns persistence.
    #[cfg(test)]
    pub(in crate::session) represented_explored_zones_like_cpp:
        [u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    /// Represented `CriteriaType::RevealWorldMapOverlay` events from area discovery.
    #[cfg(test)]
    pub(in crate::session) represented_reveal_world_map_overlay_criteria_like_cpp: Vec<u32>,
    /// Represented `Player::UpdateArea` / `Player::UpdateZone` criteria side effects.
    #[cfg(test)]
    pub(in crate::session) represented_area_zone_criteria_like_cpp:
        Vec<RepresentedAreaZoneCriteriaLikeCpp>,

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
    pub(in crate::session) durable_item_loot_persistence_like_cpp:
        DurableItemLootPersistenceTrackerLikeCpp,
    /// Per-character fence published to remote loot sources before they begin
    /// mutating this character's durable balance.
    pub(in crate::session) durable_loot_money_persistence_like_cpp:
        Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    /// Test fixture for the process-owned linked-module registry. Production
    /// borrows the required registry from the session driver.
    #[cfg(test)]
    pub(in crate::session) module_registry_like_cpp: Option<Arc<wow_module_api::ModuleRegistry>>,
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
    /// C++ `CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN` represented switch.
    pub(in crate::session) represented_offhand_check_at_spell_unlearn_like_cpp: bool,
    /// C++ `CONFIG_VMAP_INDOOR_CHECK` represented switch.
    pub(in crate::session) vmap_indoor_check_like_cpp: bool,
    /// Represented C++ `WorldObject::IsOutdoors()` result until VMAP owns it.
    #[cfg(test)]
    pub(in crate::session) represented_is_outdoors_like_cpp: Option<bool>,
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
    /// C++ `CONFIG_ENABLE_AE_LOOT` represented switch.
    pub(in crate::session) enable_ae_loot_like_cpp: bool,
    /// C++ `CONFIG_ADDON_CHANNEL` represented switch.
    #[cfg(test)]
    pub(in crate::session) addon_channel_like_cpp: bool,
    /// C++ `CONFIG_CHAT_FAKE_MESSAGE_PREVENTING` represented switch for chat validation.
    #[cfg(test)]
    pub(in crate::session) chat_fake_message_preventing_like_cpp: bool,
    /// C++ `CONFIG_CHAT_PARTY_RAID_WARNINGS` represented switch.
    #[cfg(test)]
    pub(in crate::session) party_raid_warnings_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_GM_GROUP` represented switch.
    #[cfg(test)]
    pub(in crate::session) allow_gm_group_like_cpp: bool,
    /// C++ `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP` represented switch.
    #[cfg(test)]
    pub(in crate::session) allow_two_side_interaction_group_like_cpp: bool,
    /// C++ `CONFIG_PARTY_LEVEL_REQ` represented gate.
    #[cfg(test)]
    pub(in crate::session) party_level_req_like_cpp: u32,
    /// C++ `CONFIG_CHAT_STRICT_LINK_CHECKING_KICK` represented switch.
    #[cfg(test)]
    pub(in crate::session) chat_strict_link_checking_kick_like_cpp: bool,
    /// C++ `CONFIG_CHAT_*_LEVEL_REQ` represented chat level gates.
    #[cfg(test)]
    pub(in crate::session) chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp,
    /// C++ `CONFIG_LISTEN_RANGE_*` represented nearby-chat ranges.
    #[cfg(test)]
    pub(in crate::session) chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp,
    /// C++ `CONFIG_CHATFLOOD_*` represented chat spam protection.
    #[cfg(test)]
    pub(in crate::session) chat_flood_config_like_cpp: ChatFloodConfigLikeCpp,
    pub(in crate::session) chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp; 2],
    /// C++ `CONFIG_ENABLE_MMAPS` + `DataDir` represented until map lifecycle owns real mmaps.
    pub(in crate::session) mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp,
    /// C++ `sWaypointMgr->GetPath(pathId)` resolver for session-created legacy `WorldCreature`
    /// compatibility objects. The canonical path store is owned by `world-server`.
    pub(in crate::session) waypoint_path_resolver_like_cpp: Option<WaypointPathResolverLikeCpp>,
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
    /// Confirmed pending bind ids observed by the represented `CMSG_INSTANCE_LOCK_RESPONSE`
    /// fixture. C++ keeps the real confirmation on Player/InstanceMap; this is test evidence,
    /// not production gameplay state.
    #[cfg(test)]
    pub(crate) represented_confirmed_pending_binds: Vec<u32>,
    /// Count of represented `Player::RepopAtGraveyard` calls from rejected pending binds.
    #[cfg(test)]
    pub(crate) represented_repop_at_graveyard_count: u32,
    /// Session-local representation of `GameObject::m_tapList` for personal encounter loot.
    pub(crate) represented_gameobject_tap_lists:
        std::collections::HashMap<wow_core::ObjectGuid, Vec<wow_core::ObjectGuid>>,
    /// Handle-less compatibility for represented encounter-loot fixtures.
    /// Production `Player::IsLockedToDungeonEncounter` resolves the immutable
    /// DungeonEncounter catalog and shared InstanceLockMgr.
    #[cfg(test)]
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
    pub(crate) client_visible_transports_like_cpp:
        crate::session::mailbox::SharedClientVisibleTransportsLikeCpp,
    /// Current C++ `m_movementInfo.transport.guid`, used to exclude the
    /// player's own transport from `Map::SendInitTransports`.
    #[cfg(test)]
    pub(in crate::session) player_transport_login_state_like_cpp:
        Option<Box<PlayerTransportLoginStateLikeCpp>>,
    /// Login-start delivery guard for creature movement packets.
    ///
    /// The C++ 3.4.3 login baseline does not deliver `SMSG_ON_MONSTER_MOVE`
    /// during the initial enter-world packet burst, even after
    /// `UpdateVisibilityForPlayer()` has repopulated `m_clientGUIDs`. Rust uses
    /// the cutoff to drop movement commands queued before the burst completes
    /// without blocking movement generated after the player is in world.
    pub(crate) suppress_creature_movement_queued_at_or_before_like_cpp: Option<Instant>,
    /// Detached represented inputs used only by visibility tests.
    #[cfg(test)]
    pub(crate) visibility_test_fixture_like_cpp: VisibilityTestFixtureLikeCpp,
    /// Last canonical FarsightObject value observed by this Session's
    /// publication rail. This is a delivery fence, not gameplay authority:
    /// it lets the session emit the one explicit clear packet required when a
    /// map-owned viewpoint disappears between map ticks.
    pub(in crate::session) last_observed_farsight_object_like_cpp: wow_core::ObjectGuid,
    /// Session-local delivery guard for represented DynamicObject VALUES packets
    /// consumed from the last map-owned `Map::SendObjectUpdates` stable snapshot.
    /// Includes the map update generation so repeated `process_pending()` calls
    /// suppress the same snapshot without blocking later identical bytes.
    pub(in crate::session) represented_dynamic_object_values_updates_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid, u64)>,
    /// Session-local delivery guard for Player/Creature/Pet VALUES packets
    /// consumed from the canonical map's `Map::SendObjectUpdates` snapshot.
    pub(in crate::session) represented_player_unit_values_updates_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid, u64)>,
    /// Session-local delivery guard for represented GameObjectDespawn packets
    /// consumed from the last map-owned `GameObject::Update` summary.
    pub(in crate::session) represented_gameobject_visual_despawns_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid)>,
    /// Session-local delivery guard for represented CapturePointRemoved packets
    /// consumed from the last map-owned `GameObject::Delete` summary.
    pub(in crate::session) represented_capture_point_removed_delivered_like_cpp:
        std::collections::HashSet<(u32, u32, u64, wow_core::ObjectGuid)>,
    /// Represented C++ `GameObject::GetPhaseShift()` for visible DB-spawned
    /// gameobjects until canonical gameobject map ownership lands.
    pub(crate) represented_gameobject_phase_shifts:
        std::collections::HashMap<wow_core::ObjectGuid, PhaseShift>,
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
    pub(in crate::session) player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp,
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
    pub(in crate::session) pending_teleport: Option<(u32, wow_core::Position)>,
}
