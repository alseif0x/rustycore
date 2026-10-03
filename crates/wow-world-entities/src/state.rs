use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use wow_core::ObjectGuid;
use wow_entities::PhaseShift;

use crate::{
    PendingCreatureKillRewardLikeCpp, PendingCreatureSpawn, RepresentedCreatureAuraLikeCpp,
    RepresentedGameObjectUseEffect, RepresentedGameObjectUseState,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::{RepresentedCreatureKillEventLikeCpp, RepresentedGameObjectCriteriaEvent};

/// Session-local represented creature and gameobject runtime: creature auras, kills,
/// spawn and tick, gameobject use and phase state.
///
/// The state has one owner. Its fields remain internal to this crate.
pub struct WorldEntitiesState {
    // Creature auras this session applied, with the wall-clock deadline the
    // represented duration expires at (`Aura::Update`).
    pub(crate) represented_creature_auras_like_cpp: Vec<RepresentedCreatureAuraLikeCpp>,

    /// Pending creature spawn request (set during login, processed async).
    pub(crate) pending_creature_spawn: Option<PendingCreatureSpawn>,
    /// Creature kills observed from synchronous melee ticks and completed in `process_pending`.
    pub(crate) pending_creature_kill_loot_like_cpp: Vec<ObjectGuid>,
    pub(crate) pending_creature_kill_rewards_like_cpp: Vec<PendingCreatureKillRewardLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_creature_kill_events_like_cpp: Vec<RepresentedCreatureKillEventLikeCpp>,

    // ── Creature AI tracking ──────────────────────────────────────
    /// Tick counter for creature movement (throttle to every N ticks).
    pub(crate) creature_tick: u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_gameobject_criteria_events: Vec<RepresentedGameObjectCriteriaEvent>,
    /// Represented C++ `GameEvents::Trigger` and `TriggeringLinkedGameObject` hook points.
    pub(crate) represented_gameobject_use_effects: Vec<RepresentedGameObjectUseEffect>,
    /// Session-local represented GameObject use state until canonical gameobject runtime ownership lands.
    /// Deterministic iteration order by GUID (not a strict C++ ordering guarantee).
    pub(crate) represented_gameobject_use_states: BTreeMap<ObjectGuid, RepresentedGameObjectUseState>,
    /// Login-start delivery guard for creature movement packets.
    ///
    /// The C++ 3.4.3 login baseline does not deliver `SMSG_ON_MONSTER_MOVE`
    /// during the initial enter-world packet burst, even after
    /// `UpdateVisibilityForPlayer` has repopulated `m_clientGUIDs`. Rust uses
    /// the cutoff to drop movement commands queued before the burst completes
    /// without blocking movement generated after the player is in world.
    pub(crate) suppress_creature_movement_queued_at_or_before_like_cpp: Option<Instant>,
    /// Represented C++ `GameObject::GetPhaseShift` for visible DB-spawned
    /// gameobjects until canonical gameobject map ownership lands.
    pub(crate) represented_gameobject_phase_shifts: HashMap<ObjectGuid, PhaseShift>,
}

impl WorldEntitiesState {
    /// Construct the represented state with the original Session field defaults.
    pub fn new_like_cpp() -> Self {
        Self {
            represented_creature_auras_like_cpp: Vec::new(),
            pending_creature_spawn: None,
            pending_creature_kill_loot_like_cpp: Vec::new(),
            pending_creature_kill_rewards_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_creature_kill_events_like_cpp: Vec::new(),
            creature_tick: 0,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_gameobject_criteria_events: Vec::new(),
            represented_gameobject_use_effects: Vec::new(),
            represented_gameobject_use_states: BTreeMap::new(),
            suppress_creature_movement_queued_at_or_before_like_cpp: None,
            represented_gameobject_phase_shifts: HashMap::new(),
        }
    }
}
