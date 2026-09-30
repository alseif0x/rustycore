// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::world_entities` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Session-local represented creature and gameobject runtime: creature auras, kills, spawn and
/// tick, gameobject use and phase state.
pub(crate) struct WorldEntitiesState {
    // Creature auras this session applied, with the wall-clock deadline the
    // represented duration expires at (`Aura::Update`).
    pub(in crate::session) represented_creature_auras_like_cpp:
        Vec<crate::session::world_entities::RepresentedCreatureAuraLikeCpp>,

    /// Pending creature spawn request (set during login, processed async).
    pub(crate) pending_creature_spawn: Option<PendingCreatureSpawn>,
    /// Creature kills observed from synchronous melee ticks and completed in `process_pending`.
    pub(in crate::session) pending_creature_kill_loot_like_cpp: Vec<ObjectGuid>,
    pub(in crate::session) pending_creature_kill_rewards_like_cpp:
        Vec<PendingCreatureKillRewardLikeCpp>,
    #[cfg(test)]
    pub(in crate::session) represented_creature_kill_events_like_cpp:
        Vec<RepresentedCreatureKillEventLikeCpp>,

    // ── Creature AI tracking ──────────────────────────────────────
    /// Tick counter for creature movement (throttle to every N ticks).
    pub(crate) creature_tick: u32,
    #[cfg(test)]
    pub(crate) represented_gameobject_criteria_events: Vec<RepresentedGameObjectCriteriaEvent>,
    /// Represented C++ `GameEvents::Trigger` and `TriggeringLinkedGameObject` hook points.
    pub(crate) represented_gameobject_use_effects: Vec<RepresentedGameObjectUseEffect>,
    /// Session-local represented `GameObject` use state until canonical GO runtime ownership lands.
    /// Deterministic iteration order by GUID (not a strict C++ ordering guarantee).
    pub(crate) represented_gameobject_use_states:
        std::collections::BTreeMap<wow_core::ObjectGuid, RepresentedGameObjectUseState>,
    /// Login-start delivery guard for creature movement packets.
    ///
    /// The C++ 3.4.3 login baseline does not deliver `SMSG_ON_MONSTER_MOVE`
    /// during the initial enter-world packet burst, even after
    /// `UpdateVisibilityForPlayer()` has repopulated `m_clientGUIDs`. Rust uses
    /// the cutoff to drop movement commands queued before the burst completes
    /// without blocking movement generated after the player is in world.
    pub(crate) suppress_creature_movement_queued_at_or_before_like_cpp: Option<Instant>,
    /// Represented C++ `GameObject::GetPhaseShift()` for visible DB-spawned
    /// gameobjects until canonical gameobject map ownership lands.
    pub(crate) represented_gameobject_phase_shifts:
        std::collections::HashMap<wow_core::ObjectGuid, PhaseShift>,
}
