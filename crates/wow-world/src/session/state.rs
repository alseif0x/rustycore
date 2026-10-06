// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! State: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use std::sync::Arc;

use super::AreaTriggerScriptDispatcherLikeCpp;
#[cfg(test)]
use super::AtomicUsize;
#[cfg(test)]
use super::BTreeMap;
use super::BTreeSet;
use super::ClientOpcodes;
use super::HashMap;
use super::Instant;
use super::OwnedLootAuthority;
use super::RepresentedAdventureMapStartQuestLikeCpp;
#[cfg(test)]
use super::RepresentedAreaZoneCriteriaLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedLootRollCriteriaEvent;
use super::{RepresentedLootRollState, RepresentedPendingBind};

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
    cx_quest_state_ref, hub_mut, hub_ref, split_aura_application_mut, split_battle_pet_handler_mut,
    split_battleground_mut, split_character_handler_mut, split_group_handler_states_mut,
    split_guild_bank_mut, split_instances_mut, split_instances_ref, split_interaction,
    split_interaction_ref, split_interaction_world_entities_mut, split_inventory_mut,
    split_inventory_ref, split_lifecycle_mut, split_lifecycle_ref, split_loot_mut, split_loot_ref,
    split_player_handler_states_mut, split_quest_state_lifecycle_mut, split_quest_state_mut,
    split_quest_state_ref, split_social_inventory_mut, split_social_lifecycle_mut,
    split_social_mut, split_social_ref, split_spell_state_mut, split_spell_state_ref,
    split_trade_mut, split_visibility_mut, split_visibility_ref, split_world_entities_mut,
    split_world_entities_ref,
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

pub(crate) use wow_world_application::SessionQuestState;
pub(crate) use wow_world_application::{
    RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestObjectiveProgressEventLikeCpp,
};
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

/// The session's persistence and lifecycle timeline: the login/logout instants
/// and the periodic-save schedule, the player loading and logout claims, the
/// tutorial and account-data state it persists, the persistence ports and the
/// finalization rail it hands work to, and the pet-load and loot trackers.
pub(crate) use wow_world_lifecycle::SessionLifecycleState;

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
    pub(crate) config: SessionWorldConfig,
    /// Test-only fixture groups (#1241 F3-0): the 11 cfg(test) domain groups, nested unchanged so
    /// an F3 context borrows one member instead of eleven.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fixtures: SessionFixtures,
    /// Player items, bank and equipment sets, money and currencies, and the represented bank,
    /// guild-bank and auction request sinks.
    pub(crate) inventory: InventoryState,

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
    /// Immutable opcode registry shared by the session's dispatch readers.
    pub(in crate::session) dispatch_table:
        Arc<crate::session::registry::WorldPacketHandlerRegistry>,
}
