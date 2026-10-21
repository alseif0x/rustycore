// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3b-1: the canonical map creature runtime.
//!
//! C++ `Map::Update` (`Maps/Map.cpp:669-760`) drives the map's sessions, runs
//! `ProcessRespawns`, then visits the active cells and calls `Creature::Update`
//! for every object they hold, and only then `SendObjectUpdates`. Under the
//! `CanonicalMap` owner the canonical map update loop does the same with the
//! admitted canonical executor: after the session phase and the respawn phase
//! of a split tick, and before its object visitors, it admits the creatures of
//! each admitted map's `NearbyCells` selection and runs every creature phase on
//! the canonical incarnations in place, under the one canonical guard the tick
//! resume already holds.
//!
//! Exclusivity. This is the only creature runtime: #1263 F6-8D3b-2 stopped
//! spawning the legacy global creature loop and F6-8E deleted it, the
//! per-session creature ticks and the owner value that selected between them.
//! The executor's single-owner lease serialises its own transition claims.
//!
//! Delivery. The executor publishes nothing itself; every output is deferred
//! and delivered here after all guards are released, in phase order. The
//! respawn statements are submitted by the loop while it still holds the
//! respawn mutation-order gate, after the tick's canonical respawn statements.

use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// Counters a test or an operator can read without touching a map.
#[derive(Debug, Default)]
pub(crate) struct CanonicalCreatureRuntimeProbeLikeCpp {
    /// Ticks the executor ran for an admitted transition.
    pub(crate) executed_ticks: AtomicU64,
    /// Ticks the admission fence or the lease refused.
    pub(crate) refused_ticks: AtomicU64,
    /// The summed `Unit::Update` clock advancement over every creature.
    pub(crate) clock_advanced_ms: AtomicU64,
}

/// What the canonical creature runtime keeps across ticks: its configuration,
/// the shared terrain, the single-owner lease and the per-map player melee
/// accumulator the loop task owns.
pub(crate) struct CanonicalCreatureRuntimeLikeCpp {
    pub(crate) mmap_config: MMapRuntimeConfigLikeCpp,
    pub(crate) mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    pub(crate) aggro_config: wow_world::session::CreatureAggroConfigLikeCpp,
    pub(crate) group_registry: Option<Arc<wow_social::group::GroupRegistry>>,
    /// The file-backed terrain heights the movement and lifecycle phases read.
    pub(crate) terrain: Option<Arc<wow_world::terrain::LiveTerrainHeights>>,
    pub(crate) lease: wow_world::session::SharedCreatureExecutionLeaseLikeCpp,
    pub(crate) player_melee_phase_state: wow_world::session::PlayerMeleePhaseStateLikeCpp,
    pub(crate) probe: Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
}

/// The registry snapshots one tick reads, taken before any map guard.
pub(crate) struct CanonicalCreatureTickInputsLikeCpp {
    attackers: Vec<wow_world::session::PlayerMeleeAttackerSnapshotLikeCpp>,
    aggro_candidates: Vec<wow_world::session::CreatureAggroCandidateLikeCpp>,
    chase_targets:
        std::collections::HashMap<(u16, u32, ObjectGuid), wow_world::ChaseTargetSnapshotLikeCpp>,
    terrain: Option<Arc<wow_world::terrain::LiveTerrainHeights>>,
    /// Whether a linked module wants the `creature.*` family (#1444 H3b):
    /// asked before the guards, so the lifecycle phase copies no event for a
    /// zero-module server.
    record_creature_module_events: bool,
}

impl CanonicalCreatureRuntimeLikeCpp {
    #[must_use]
    pub(crate) fn new_like_cpp(
        mmap_config: MMapRuntimeConfigLikeCpp,
        mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
        aggro_config: wow_world::session::CreatureAggroConfigLikeCpp,
        group_registry: Option<Arc<wow_social::group::GroupRegistry>>,
        terrain: Option<Arc<wow_world::terrain::LiveTerrainHeights>>,
    ) -> Self {
        Self {
            mmap_config,
            mmap_pathfinder,
            aggro_config,
            group_registry,
            terrain,
            lease: Arc::new(Mutex::new(
                wow_world::session::CreatureExecutionLeaseLikeCpp::new(),
            )),
            player_melee_phase_state: wow_world::session::PlayerMeleePhaseStateLikeCpp::default(),
            probe: Arc::new(CanonicalCreatureRuntimeProbeLikeCpp::default()),
        }
    }

    /// Read the registry snapshots before any guard.
    pub(crate) fn collect_inputs_like_cpp(
        &self,
        canonical_map_manager: &SharedCanonicalMapManager,
        registry: &PlayerRegistry,
        record_creature_module_events: bool,
    ) -> CanonicalCreatureTickInputsLikeCpp {
        let tap_index = build_tap_group_index_like_cpp(self.group_registry.as_ref());
        CanonicalCreatureTickInputsLikeCpp {
            attackers: collect_player_melee_attackers_like_cpp(registry, &tap_index),
            aggro_candidates: collect_creature_aggro_candidates_with_canonical_like_cpp(
                registry,
                Some(canonical_map_manager),
            ),
            chase_targets: collect_chase_target_snapshots_like_cpp(registry),
            terrain: self.terrain.clone(),
            record_creature_module_events,
        }
    }

    /// Admit and run every creature phase of one admitted map tick, on the
    /// canonical manager the tick resume holds. Nothing is delivered here.
    pub(crate) fn run_on_locked_manager_like_cpp(
        &mut self,
        manager: &mut wow_map::MapManager,
        plan: &wow_map::MapTickPlanLikeCpp,
        coordinator_id: u64,
        inputs: &CanonicalCreatureTickInputsLikeCpp,
        map_store: &wow_data::MapStore,
        now: Instant,
    ) -> wow_world::session::AdmittedCreatureCombatPhasesOutcomeLikeCpp {
        let diff_ms = plan.effective_diff_ms();
        // C++ `Map::Update` visits the cells around players and active objects
        // (`VisitNearbyCellsOf`); the selection is the canonical map's own
        // `NearbyCells` plan, the one its object visitors use, kept to creatures.
        let mut objects = Vec::new();
        for participant in plan.updated_maps_like_cpp() {
            let key = participant.key;
            let Some(map) = manager.find_map(key.map_id, key.instance_id) else {
                continue;
            };
            let map = map.map();
            objects.extend(
                map.object_update_plan_for_current_tick_like_cpp(diff_ms)
                    .update_guids
                    .into_iter()
                    .filter(|guid| map.get_typed_creature(*guid).is_some())
                    .map(|guid| (key, guid)),
            );
        }
        let admission = wow_world::session::capture_admitted_creature_execution_on_manager_like_cpp(
            manager,
            coordinator_id,
            plan.epoch_like_cpp(),
            diff_ms,
            plan.updated_maps_like_cpp()
                .iter()
                .map(|participant| (participant.key, participant.incarnation)),
            objects,
        );
        let mmap_pathfinder = self.mmap_pathfinder.clone();
        let outcome =
            wow_world::session::run_admitted_creature_combat_phases_on_locked_manager_like_cpp(
                &admission,
                manager,
                &self.lease,
                wow_world::session::AdmittedCreatureCombatPhaseInputsLikeCpp {
                    player_melee_attackers: &inputs.attackers,
                    player_melee_phase_state: &mut self.player_melee_phase_state,
                    aggro_candidates: &inputs.aggro_candidates,
                    terrain: inputs.terrain.as_deref(),
                    mmap_config: &self.mmap_config,
                    mmap_pathfinder: mmap_pathfinder.as_deref(),
                    chase_targets: &inputs.chase_targets,
                    config: &self.aggro_config,
                    map_store,
                    now,
                    record_creature_module_events: inputs.record_creature_module_events,
                },
            );
        if outcome.executed_like_cpp() {
            self.probe.executed_ticks.fetch_add(1, Ordering::Relaxed);
            self.probe
                .clock_advanced_ms
                .fetch_add(outcome.clock_advanced_ms, Ordering::Relaxed);
        } else {
            self.probe.refused_ticks.fetch_add(1, Ordering::Relaxed);
        }
        outcome
    }
}

/// What the deferred delivery of one executor tick reached.
#[derive(Debug, Default)]
pub(crate) struct CanonicalCreatureDeliverySummaryLikeCpp {
    pub(crate) player_melee: RuntimePlayerMeleeDeliverySummaryLikeCpp,
    pub(crate) lifecycle: RuntimeVisibilityRefreshDeliverySummaryLikeCpp,
    pub(crate) movement: RuntimeDeliverySummaryLikeCpp,
    pub(crate) aggro: RuntimeCreatureAttackStartDeliverySummaryLikeCpp,
    pub(crate) aggro_plan: RuntimeDeliverySummaryLikeCpp,
    pub(crate) spell_plan: RuntimeDeliverySummaryLikeCpp,
    pub(crate) melee: RuntimeCreatureMeleeDeliverySummaryLikeCpp,
    pub(crate) melee_plan: RuntimeDeliverySummaryLikeCpp,
}

/// Deliver one executor tick's deferred output, after every guard is released,
/// in phase order: player melee results, the lifecycle visibility refresh, the
/// movement plan, the committed attack starts and stops, the aggro plan, the
/// spell plan, the creature melee results and the melee plan. The respawn statements are not here: the loop submits them under
/// the mutation-order gate before releasing it.
pub(crate) fn deliver_canonical_creature_phases_like_cpp(
    outcome: &wow_world::session::AdmittedCreatureCombatPhasesOutcomeLikeCpp,
    registry: &PlayerRegistry,
) -> CanonicalCreatureDeliverySummaryLikeCpp {
    CanonicalCreatureDeliverySummaryLikeCpp {
        player_melee: deliver_player_melee_results_like_cpp(
            &outcome.player_melee.commands,
            registry,
        ),
        lifecycle: deliver_creature_lifecycle_refreshes_like_cpp(
            &outcome.lifecycle.refresh_map_keys,
            registry,
        ),
        movement: deliver_runtime_plan_like_cpp(&outcome.movement.plan, registry),
        aggro: deliver_committed_creature_combat_commands_like_cpp(
            &outcome.aggro_committed_starts,
            &outcome.aggro_committed_stops,
            registry,
        ),
        aggro_plan: deliver_runtime_plan_like_cpp(&outcome.aggro.plan, registry),
        spell_plan: deliver_runtime_plan_like_cpp(&outcome.spell.plan, registry),
        melee: deliver_creature_melee_damage_commands_like_cpp(&outcome.melee.commands, registry),
        melee_plan: deliver_runtime_plan_like_cpp(&outcome.melee.plan, registry),
    }
}
