// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3b-1: the `RuntimeTickOwner::CanonicalMap` creature tick owner.
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
//! Exclusivity. There is one owner value, `MapManager::tick_owner()` on the
//! shared legacy manager, decided once at startup (`app.rs`). This owner runs
//! only when that value is `CanonicalMap`, re-read every tick before any guard;
//! every legacy phase body runs only when it is `GlobalLegacy`, and every
//! session creature tick only when it is `Session`. Startup also spawns the
//! legacy creature loop idle and builds this owner only for the selected value
//! (`creature_tick_owner_loops_like_cpp`), and the executor claims each admitted
//! transition through its single-owner lease, so no creature is ticked twice.
//!
//! Delivery. The executor publishes nothing itself; every output is deferred
//! and delivered here after all guards are released, through the legacy
//! bridge's own delivery bodies and in the legacy loop's phase order. The
//! respawn statements are submitted by the loop while it still holds the
//! respawn mutation-order gate, after the tick's canonical respawn statements.

use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// The loops startup spawns for one creature tick owner. Both are derived from
/// the one owner value, so they cannot both be live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureTickOwnerLoopsLikeCpp {
    /// `spawn_legacy_creature_runtime_update_loop_like_cpp` runs its body.
    pub(crate) legacy_creature_loop: bool,
    /// The canonical map loop carries a [`CanonicalCreatureRuntimeLikeCpp`].
    pub(crate) canonical_creature_runtime: bool,
}

#[must_use]
pub(crate) const fn creature_tick_owner_loops_like_cpp(
    owner: wow_world::map_manager::RuntimeTickOwner,
) -> CreatureTickOwnerLoopsLikeCpp {
    use wow_world::map_manager::RuntimeTickOwner;
    CreatureTickOwnerLoopsLikeCpp {
        legacy_creature_loop: matches!(owner, RuntimeTickOwner::GlobalLegacy),
        canonical_creature_runtime: matches!(owner, RuntimeTickOwner::CanonicalMap),
    }
}

/// Counters a test or an operator can read without touching a map.
#[derive(Debug, Default)]
pub(crate) struct CanonicalCreatureRuntimeProbeLikeCpp {
    /// Ticks the executor ran for an admitted transition.
    pub(crate) executed_ticks: AtomicU64,
    /// Ticks the admission fence or the lease refused.
    pub(crate) refused_ticks: AtomicU64,
    /// Ticks skipped because the shared owner value was not `CanonicalMap`.
    pub(crate) owner_skipped_ticks: AtomicU64,
    /// The summed `Unit::Update` clock advancement over every creature.
    pub(crate) clock_advanced_ms: AtomicU64,
    /// R7b-1 deferred registrations adopted from the legacy map queue.
    pub(crate) adopted_legacy_respawns: AtomicU64,
}

/// What the canonical creature owner keeps across ticks: the configuration the
/// legacy creature loop is spawned with, the single-owner lease and the
/// per-map player melee accumulator the loop task owns.
pub(crate) struct CanonicalCreatureRuntimeLikeCpp {
    pub(crate) mmap_config: MMapRuntimeConfigLikeCpp,
    pub(crate) mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    pub(crate) aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    pub(crate) group_registry: Option<Arc<wow_social::group::GroupRegistry>>,
    pub(crate) lease: wow_world::session::SharedCreatureExecutionLeaseLikeCpp,
    pub(crate) player_melee_phase_state: wow_world::session::PlayerMeleePhaseStateLikeCpp,
    pub(crate) probe: Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
}

/// The registry snapshots one tick reads, taken before any map guard exactly
/// as the legacy bridge takes them.
pub(crate) struct CanonicalCreatureTickInputsLikeCpp {
    attackers: Vec<wow_world::session::PlayerMeleeAttackerSnapshotLikeCpp>,
    aggro_candidates: Vec<wow_world::session::LegacyCreatureAggroCandidateLikeCpp>,
    chase_targets:
        std::collections::HashMap<(u16, u32, ObjectGuid), wow_world::ChaseTargetSnapshotLikeCpp>,
    terrain: Option<Arc<wow_world::map_manager::LiveTerrainHeights>>,
}

impl CanonicalCreatureRuntimeLikeCpp {
    #[must_use]
    pub(crate) fn new_like_cpp(
        mmap_config: MMapRuntimeConfigLikeCpp,
        mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
        aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
        group_registry: Option<Arc<wow_social::group::GroupRegistry>>,
    ) -> Self {
        Self {
            mmap_config,
            mmap_pathfinder,
            aggro_config,
            group_registry,
            lease: Arc::new(Mutex::new(
                wow_world::session::CreatureExecutionLeaseLikeCpp::new(),
            )),
            player_melee_phase_state: wow_world::session::PlayerMeleePhaseStateLikeCpp::default(),
            probe: Arc::new(CanonicalCreatureRuntimeProbeLikeCpp::default()),
        }
    }

    /// Read the owner and the registry snapshots before any guard. `None` when
    /// the shared owner value is not `CanonicalMap`: then this tick runs no
    /// creature phase here.
    pub(crate) fn collect_inputs_like_cpp(
        &self,
        legacy_map_manager: &SharedMapManager,
        canonical_map_manager: &SharedCanonicalMapManager,
        registry: &PlayerRegistry,
    ) -> Option<CanonicalCreatureTickInputsLikeCpp> {
        if wow_world::map_manager::shared_runtime_tick_owner_like_cpp(legacy_map_manager)
            != wow_world::map_manager::RuntimeTickOwner::CanonicalMap
        {
            self.probe
                .owner_skipped_ticks
                .fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let terrain = legacy_map_manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .terrain();
        let tap_index = build_tap_group_index_like_cpp(self.group_registry.as_ref());
        Some(CanonicalCreatureTickInputsLikeCpp {
            attackers: collect_player_melee_attackers_like_cpp(registry, &tap_index),
            aggro_candidates: collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
                registry,
                Some(canonical_map_manager),
            ),
            chase_targets: collect_legacy_chase_target_snapshots_like_cpp(registry),
            terrain,
        })
    }

    /// R7b-1 under this owner. A registration deferred while the canonical
    /// manager had no instance for its key can only join the legacy map's
    /// queue, and no legacy lifecycle drains it under `CanonicalMap`. Each
    /// admitted map adopts those entries into its own queue, unchanged and in
    /// order, before its lifecycle phase drains it, so the deferred creature
    /// is admitted canonically once its instance exists. Canonical → legacy is
    /// the established lock order (the mutation root, the respawn-phase mirror).
    fn adopt_legacy_respawn_queue_like_cpp(
        &self,
        manager: &mut wow_map::MapManager,
        plan: &wow_map::MapTickPlanLikeCpp,
        legacy_map_manager: &SharedMapManager,
    ) {
        // `drain_ready_respawns` keeps an entry due after its argument; the
        // adoption takes every entry, and each keeps its own due time.
        let Some(all) = Instant::now().checked_add(Duration::from_secs(1 << 32)) else {
            return;
        };
        for participant in plan.updated_maps_like_cpp() {
            let key = participant.key;
            let Ok(map_id) = u16::try_from(key.map_id) else {
                continue;
            };
            let Some(map) = manager.find_map_mut(key.map_id, key.instance_id) else {
                continue;
            };
            let adopted = legacy_map_manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .drain_ready_respawns(map_id, key.instance_id, all);
            let queue = map.map_mut().creature_respawn_queue_like_cpp_mut();
            for respawn in adopted {
                queue.push_respawn(respawn);
                self.probe
                    .adopted_legacy_respawns
                    .fetch_add(1, Ordering::Relaxed);
            }
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
        legacy_map_manager: &SharedMapManager,
        map_store: &wow_data::MapStore,
        now: Instant,
    ) -> wow_world::session::AdmittedCreatureCombatPhasesOutcomeLikeCpp {
        let diff_ms = plan.effective_diff_ms();
        self.adopt_legacy_respawn_queue_like_cpp(manager, plan, legacy_map_manager);
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
/// in the legacy loop's order: player melee results, the lifecycle visibility
/// refresh, the movement plan, the committed attack starts and stops, the aggro
/// plan, the spell plan, the creature melee results and the melee plan. Each
/// step is the legacy bridge's own delivery body, so recipients and bytes are
/// the same. The respawn statements are not here: the loop submits them under
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
