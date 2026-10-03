// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! State: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::AccountDataLikeCpp;
use super::Arc;
use super::AreaTriggerScriptDispatcherLikeCpp;
#[cfg(test)]
use super::AtomicUsize;
#[cfg(test)]
use super::BTreeMap;
use super::BTreeSet;
use super::BattlePetAccountAttachmentLikeCpp;
use super::ClientOpcodes;
use super::DurableItemLootPersistenceTrackerLikeCpp;
use super::DurableLootMoneyPersistenceTrackerLikeCpp;
use super::HashMap;
use super::NUM_ACCOUNT_DATA_TYPES;
use super::ObjectGuid;
use super::OwnedLootAuthority;
use super::PacketHandlerEntry;
use super::RepresentedAdventureMapStartQuestLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedLootRollCriteriaEvent;
use super::RepresentedQuestCompleteStatusUpdateLikeCpp;
use super::RepresentedQuestObjectiveProgressEventLikeCpp;
use super::SessionPersistencePortsLikeCpp;
use super::VecDeque;
#[cfg(test)]
use super::persistence::test_fixtures::LoadedPlayerFlagsTestFixtureLikeCpp;
#[cfg(test)]
use super::quest::test_fixtures::QuestTestFixtureLikeCpp;
use super::{HomebindPersistenceJobLikeCpp, Instant, Item};
#[cfg(test)]
use super::{RepresentedAreaZoneCriteriaLikeCpp, RepresentedAtLoginFlagRemovalLikeCpp};
use super::{RepresentedLootRollState, RepresentedPendingBind};
use super::{driver, lifecycle};

pub(crate) use wow_world_spell::SessionSpellState;

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
mod inventory_state;
pub(crate) use inventory_state::InventoryState;
mod instances;
pub(crate) use instances::InstanceState;
mod world_entities;
pub(crate) use world_entities::WorldEntitiesState;
mod visibility;
pub(crate) use visibility::VisibilityState;
mod interaction;
pub(crate) use interaction::InteractionState;

pub(crate) use wow_world_social::SessionSocialLimits;

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

/// The canonical producer's phase rail for this session (#787), separate from
/// the command mailbox because a phase pass drains that mailbox.
pub(in crate::session) struct SessionPhaseRail {
    pub(in crate::session) tx: flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    pub(in crate::session) rx: flume::Receiver<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
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
