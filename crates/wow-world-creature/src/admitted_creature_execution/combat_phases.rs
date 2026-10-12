// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3a/D3b: the admitted canonical executor runs the creature phases.
//!
//! For one admitted canonical map tick the executor runs, in the order C++
//! `Map::Update` and the former Rust creature loop established: player melee
//! (`Player::Update` → `DoMeleeAttackIfReady`, phase 0); the lifecycle phase
//! (JUST_DIED respawn-time save, corpse despawn, the canonical map's own
//! respawn queue and the rebuild and admission of each ready respawn); the
//! movement phase (`Unit::Update(p_time)`'s clock step, `UpdateSplineMovement`
//! and `MotionMaster::Update` with the home, random, waypoint and chase
//! generators, and the reached-home values update); aggro/threat/assist/evade
//! with its attack start/stop commit; the creature spell phase; and creature
//! melee. Creatures are enumerated from the admitted canonical selection and
//! mutated in place on the canonical map. A despawned corpse leaves the
//! selection of the later phases and a respawn joins it within the same tick.
//!
//! Every phase is one `run_*_phase_on_manager_like_cpp` function over the
//! locked canonical manager and a selection, so a phase can also be driven on
//! its own (the unit tests do). Every publication is returned as deferred
//! output — the plan events, the committed commands, the respawn DB statements
//! and the visibility refreshes — and delivered by the caller after the guard
//! is released.

use super::*;

/// What the combat phases read beside the admitted canonical map: the inputs
/// the caller collects from the player registry before any map lock.
pub struct AdmittedCreatureCombatPhaseInputsLikeCpp<'a> {
    pub player_melee_attackers: &'a [PlayerMeleeAttackerSnapshotLikeCpp],
    pub player_melee_phase_state: &'a mut PlayerMeleePhaseStateLikeCpp,
    pub aggro_candidates: &'a [CreatureAggroCandidateLikeCpp],
    /// The map environment seam the family-assistance LOS check and the
    /// movement generators' height normalization read.
    pub terrain: Option<&'a wow_world_core::terrain::LiveTerrainHeights>,
    /// The movement phase's pathfinding seam.
    pub mmap_config: &'a MMapRuntimeConfigLikeCpp,
    pub mmap_pathfinder: Option<&'a wow_world_core::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    /// The player chase victims snapshotted from the registry.
    pub chase_targets:
        &'a HashMap<(u16, u32, ObjectGuid), wow_world_core::creature_movement::ChaseTargetSnapshotLikeCpp>,
    pub config: &'a CreatureAggroConfigLikeCpp,
    /// The lifecycle phase's map catalog (which maps persist respawn times) and
    /// the tick deadline its respawn queue drains against.
    pub map_store: &'a wow_data::MapStore,
    pub now: Instant,
}

/// The typed outcome of one admitted combat-phase execution.
#[derive(Debug, Clone)]
pub struct AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    pub admission: CreatureExecutionAdmissionLikeCpp,
    pub owner: CreatureExecutionOwnerLikeCpp,
    /// The one `Unit::Update` clock advancement of the transition.
    pub clock_advanced_ms: u64,
    pub player_melee: PlayerMeleeTickOutcomeLikeCpp,
    /// The lifecycle phase outcome (F6-8D3a-3), with its deferred respawn DB
    /// statements and refresh map keys.
    pub lifecycle: CreatureLifecycleTickOutcomeLikeCpp,
    /// The movement phase outcome (F6-8D3a-2), driven on the canonical
    /// incarnation in place.
    pub movement: CreatureMovementTickOutcomeLikeCpp,
    /// The aggro phase outcome. Its plan is already reduced to the committed
    /// combat events.
    pub aggro: CreatureAggroTickOutcomeLikeCpp,
    pub aggro_committed_starts: Vec<wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand>,
    pub aggro_committed_stops: Vec<wow_world_core::session::mailbox::CreatureAttackStopLikeCppCommand>,
    /// Whether the spell phase ran (F6-8D3a-1b). Its outcome carries the
    /// START/GO plan.
    pub spell_phase_executed: bool,
    pub spell: CreatureSpellTickOutcomeLikeCpp,
    pub melee: CreatureMeleeTickOutcomeLikeCpp,
}

impl AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    fn refused_like_cpp(
        admission: CreatureExecutionAdmissionLikeCpp,
        owner: CreatureExecutionOwnerLikeCpp,
    ) -> Self {
        Self {
            admission,
            owner,
            clock_advanced_ms: 0,
            player_melee: PlayerMeleeTickOutcomeLikeCpp::default(),
            lifecycle: CreatureLifecycleTickOutcomeLikeCpp::default(),
            movement: CreatureMovementTickOutcomeLikeCpp::default(),
            aggro: CreatureAggroTickOutcomeLikeCpp::default(),
            aggro_committed_starts: Vec::new(),
            aggro_committed_stops: Vec::new(),
            spell_phase_executed: false,
            spell: CreatureSpellTickOutcomeLikeCpp::default(),
            melee: CreatureMeleeTickOutcomeLikeCpp::default(),
        }
    }

    /// Whether the engine actually ran for this transition.
    #[must_use]
    pub const fn executed_like_cpp(&self) -> bool {
        self.admission.is_admitted_like_cpp()
    }
}

/// Run the admitted combat phases once.
///
/// Order: the admission fence and the single-owner claim before any mutation;
/// player melee; the lifecycle phase; the movement phase; aggro and its
/// canonical start/stop commit; the creature spell phase; creature melee. The
/// canonical guard is held for the whole transition and released before
/// returning, and nothing is delivered.
#[must_use]
pub fn run_admitted_creature_combat_phases_isolated_like_cpp(
    admission: &AdmittedCreatureExecutionLikeCpp,
    canonical_map_manager: &SharedCanonicalMapManager,
    lease: &SharedCreatureExecutionLeaseLikeCpp,
    inputs: AdmittedCreatureCombatPhaseInputsLikeCpp<'_>,
) -> AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    let owner = CreatureExecutionOwnerLikeCpp::CanonicalAdmitted;
    let Ok(mut manager) = canonical_map_manager.lock() else {
        return AdmittedCreatureCombatPhasesOutcomeLikeCpp::refused_like_cpp(
            CreatureExecutionAdmissionLikeCpp::RefusedSessionBarrier {
                expected_epoch: admission.tick_epoch,
                current: wow_map::MapTickCoordinationStateLikeCpp::Idle,
            },
            owner,
        );
    };
    run_admitted_creature_combat_phases_on_locked_manager_like_cpp(
        admission,
        &mut manager,
        lease,
        inputs,
    )
}

/// [`run_admitted_creature_combat_phases_isolated_like_cpp`] on a canonical
/// manager the caller already holds.
///
/// #1263 F6-8D3b-1: the production entry. The canonical map update loop calls
/// it under the one canonical guard of its tick resume, after the session
/// phase and before the object visitors, where C++ `Map::Update` runs
/// `Creature::Update`. The fence, the claim and every phase are the isolated
/// path's; only who took the guard differs, and nothing is delivered here.
#[must_use]
pub fn run_admitted_creature_combat_phases_on_locked_manager_like_cpp(
    admission: &AdmittedCreatureExecutionLikeCpp,
    manager: &mut wow_map::MapManager,
    lease: &SharedCreatureExecutionLeaseLikeCpp,
    inputs: AdmittedCreatureCombatPhaseInputsLikeCpp<'_>,
) -> AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    let owner = CreatureExecutionOwnerLikeCpp::CanonicalAdmitted;
    let admitted =
        admit_and_claim_admitted_creature_execution_like_cpp(admission, manager, lease, owner);
    let mut outcome = AdmittedCreatureCombatPhasesOutcomeLikeCpp::refused_like_cpp(admitted, owner);
    if !admitted.is_admitted_like_cpp() {
        return outcome;
    }
    let config = inputs.config;
    let maps = &admission.maps;

    run_player_melee_phase_on_manager_like_cpp(
        manager,
        inputs.player_melee_attackers,
        admission.diff_ms,
        inputs.player_melee_phase_state,
        config,
        &mut outcome.player_melee,
    );
    let mut selection = admission.objects.clone();
    run_creature_lifecycle_phase_on_manager_like_cpp(
        manager,
        maps,
        &mut selection,
        inputs.terrain,
        inputs.map_store,
        inputs.now,
        &mut outcome.lifecycle,
    );
    run_creature_movement_phase_on_manager_like_cpp(
        manager,
        maps,
        &selection,
        inputs.terrain,
        inputs.mmap_config,
        inputs.mmap_pathfinder,
        inputs.chase_targets,
        admission.diff_ms,
        &mut outcome.movement,
    );
    outcome.clock_advanced_ms =
        (outcome.movement.creatures_seen as u64).saturating_mul(u64::from(admission.diff_ms));
    let aggro = run_creature_aggro_phase_on_manager_like_cpp(
        manager,
        maps,
        &selection,
        inputs.terrain,
        inputs.aggro_candidates,
        config,
    );
    outcome.aggro = aggro.outcome;
    outcome.aggro_committed_starts = aggro.committed_starts;
    outcome.aggro_committed_stops = aggro.committed_stops;
    run_creature_spell_phase_on_manager_like_cpp(
        manager,
        maps,
        &selection,
        config,
        &mut outcome.spell,
    );
    outcome.spell_phase_executed = true;
    run_creature_melee_phase_on_manager_like_cpp(
        manager,
        maps,
        &selection,
        config,
        &mut outcome.melee,
    );
    outcome
}

/// Phase 0, player melee (`Player::Update` → `DoMeleeAttackIfReady`), before
/// the creatures' own update.
pub(crate) fn run_player_melee_phase_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    attackers: &[PlayerMeleeAttackerSnapshotLikeCpp],
    diff_ms: u32,
    phase_state: &mut PlayerMeleePhaseStateLikeCpp,
    config: &CreatureAggroConfigLikeCpp,
    outcome: &mut PlayerMeleeTickOutcomeLikeCpp,
) {
    outcome.attackers_seen = attackers.len();
    let (pending, map_difficulties) = collect_player_melee_swings_on_manager_like_cpp(
        manager,
        attackers,
        diff_ms,
        phase_state,
        outcome,
    );
    outcome.swings_ready = pending.len();
    for swing in pending {
        execute_player_melee_swing_on_manager_like_cpp(
            manager,
            &mut CanonicalPlayerMeleeCreatureVictimsLikeCpp,
            &swing,
            &map_difficulties,
            diff_ms,
            config,
            outcome,
        );
    }
}

/// Lifecycle: `Creature::setDeathState(JUST_DIED)`'s `SaveRespawnTime`, the
/// `CORPSE` → `RemoveCorpse` despawn, and `Map::ProcessRespawns` →
/// `DoRespawn`, after player melee (so a creature a player just killed is
/// already dead) and before movement. A despawned corpse leaves `selection`
/// and a respawn joins it.
pub(crate) fn run_creature_lifecycle_phase_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    maps: &[AdmittedCreatureExecutionMapLikeCpp],
    selection: &mut Vec<AdmittedCreatureExecutionObjectLikeCpp>,
    terrain: Option<&wow_world_core::terrain::LiveTerrainHeights>,
    map_store: &wow_data::MapStore,
    now: Instant,
    outcome: &mut CreatureLifecycleTickOutcomeLikeCpp,
) {
    let map_keys = maps
        .iter()
        .filter_map(|map| Some((u16::try_from(map.map_id).ok()?, map.instance_id)))
        .collect();
    let mut store = AdmittedCanonicalCreatureLifecycleStoreLikeCpp {
        store: AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(manager, maps, selection),
        terrain,
        taken: Vec::new(),
        respawned: Vec::new(),
    };
    run_creature_lifecycle_phase_on_store_like_cpp(
        &mut store,
        map_keys,
        map_store,
        now,
        (Instant::now(), unix_now()),
        outcome,
    );
    let AdmittedCanonicalCreatureLifecycleStoreLikeCpp {
        taken, respawned, ..
    } = store;
    selection.retain(|object| {
        !taken.iter().any(|&((map_id, instance_id), guid)| {
            u32::from(map_id) == object.map_id
                && instance_id == object.instance_id
                && guid == object.creature_guid
        })
    });
    for ((map_id, instance_id), creature_guid) in respawned {
        let incarnation = maps
            .iter()
            .find(|map| map.map_id == u32::from(map_id) && map.instance_id == instance_id)
            .map_or(0, |map| map.incarnation);
        // Installed by this transition after its fence: it has no frozen
        // record identity to re-check.
        selection.push(AdmittedCreatureExecutionObjectLikeCpp {
            map_id: u32::from(map_id),
            instance_id,
            incarnation,
            creature_guid,
            record_identity: 0,
        });
    }
}

/// Movement: `Unit::Update(p_time)` — the creature's one clock advancement,
/// `UpdateSplineMovement` and `MotionMaster::Update` — on the canonical
/// incarnation in place. Each frame is therefore already applied and is
/// published as is.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_creature_movement_phase_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    maps: &[AdmittedCreatureExecutionMapLikeCpp],
    selection: &[AdmittedCreatureExecutionObjectLikeCpp],
    terrain: Option<&wow_world_core::terrain::LiveTerrainHeights>,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&wow_world_core::pathfinder::WorldMMapPathfinderWorkerLikeCpp>,
    chase_targets: &HashMap<
        (u16, u32, ObjectGuid),
        wow_world_core::creature_movement::ChaseTargetSnapshotLikeCpp,
    >,
    diff_ms: u32,
    outcome: &mut CreatureMovementTickOutcomeLikeCpp,
) {
    // #1263 F6-8D3b-3: the generators write the new position in place; C++
    // `Unit::UpdatePosition` then runs `Map::CreatureRelocation`
    // (`Map.cpp:1042-1072`), so a creature that moved also moves cells.
    let origins: Vec<_> = selection
        .iter()
        .filter_map(|object| {
            let position = manager
                .find_map(object.map_id, object.instance_id)?
                .map()
                .get_typed_creature(object.creature_guid)?
                .position();
            Some((
                object.map_id,
                object.instance_id,
                object.creature_guid,
                position,
            ))
        })
        .collect();
    let mut store = AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(manager, maps, selection);
    run_creature_movement_phase_on_store_like_cpp(
        &mut store,
        terrain,
        mmap_config,
        mmap_pathfinder,
        chase_targets,
        diff_ms,
        outcome,
    );
    for (map_id, instance_id, guid, from) in origins {
        if let Some(managed) = manager.find_map_mut(map_id, instance_id) {
            let _ = managed
                .map_mut()
                .relocate_creature_moved_in_place_like_cpp(guid, from);
        }
    }
}

/// The aggro phase's outcome with its committed start/stop commands.
pub(crate) struct CreatureAggroPhaseOutcomeLikeCpp {
    pub(crate) outcome: CreatureAggroTickOutcomeLikeCpp,
    pub(crate) committed_starts:
        Vec<wow_world_core::session::mailbox::CreatureAttackStartLikeCppCommand>,
    pub(crate) committed_stops:
        Vec<wow_world_core::session::mailbox::CreatureAttackStopLikeCppCommand>,
}

/// Aggro: `UpdateVictim`, assistance, `MoveInLineOfSight`, then the canonical
/// commit of every start/stop and the reduction of the plan to the committed
/// events.
pub(crate) fn run_creature_aggro_phase_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    maps: &[AdmittedCreatureExecutionMapLikeCpp],
    selection: &[AdmittedCreatureExecutionObjectLikeCpp],
    terrain: Option<&wow_world_core::terrain::LiveTerrainHeights>,
    candidates: &[CreatureAggroCandidateLikeCpp],
    config: &CreatureAggroConfigLikeCpp,
) -> CreatureAggroPhaseOutcomeLikeCpp {
    let mut store = AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(manager, maps, selection);
    let mut outcome =
        run_creature_aggro_phase_on_store_like_cpp(&mut store, terrain, candidates, config);
    let manager = store.manager_mut_like_cpp();
    let start_outcomes =
        apply_creature_attack_start_commands_on_manager_like_cpp(manager, &outcome.commands);
    let stop_outcomes =
        apply_creature_attack_stop_commands_on_manager_like_cpp(manager, &outcome.stop_commands);
    retain_committed_creature_combat_events_like_cpp(
        &mut outcome.plan,
        &outcome.commands,
        &start_outcomes,
        &outcome.stop_commands,
        &stop_outcomes,
    );
    let committed_starts =
        committed_creature_combat_commands_like_cpp(&outcome.commands, &start_outcomes);
    let committed_stops =
        committed_creature_combat_commands_like_cpp(&outcome.stop_commands, &stop_outcomes);
    CreatureAggroPhaseOutcomeLikeCpp {
        outcome,
        committed_starts,
        committed_stops,
    }
}

/// Spell: `CombatAI::UpdateAI` / `TurretAI::UpdateAI` after `UpdateVictim`
/// (`CombatAI.cpp:91-107, 217-223`), before melee. Selection over the admitted
/// store, then the deferred `DoCast` / `ScheduleEvent` drain on the canonical
/// manager still held here. Without a spell store the phase selects nothing.
pub(crate) fn run_creature_spell_phase_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    maps: &[AdmittedCreatureExecutionMapLikeCpp],
    selection: &[AdmittedCreatureExecutionObjectLikeCpp],
    config: &CreatureAggroConfigLikeCpp,
    outcome: &mut CreatureSpellTickOutcomeLikeCpp,
) {
    let Some(spell_store) = config.spell_store.as_ref() else {
        return;
    };
    let map_difficulties = creature_spell_map_difficulties_like_cpp(manager);
    let actions = {
        let mut store =
            AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(manager, maps, selection);
        collect_creature_spell_actions_on_store_like_cpp(
            &mut store,
            &map_difficulties,
            spell_store,
            config,
            outcome,
        )
    };
    drain_creature_spell_actions_like_cpp(
        &mut CanonicalCreatureSpellActionOwnerLikeCpp { manager },
        actions,
        config,
        outcome,
    );
}

/// Creature melee (`DoMeleeAttackIfReady`) with the real attack table.
pub(crate) fn run_creature_melee_phase_on_manager_like_cpp(
    manager: &mut wow_map::MapManager,
    maps: &[AdmittedCreatureExecutionMapLikeCpp],
    selection: &[AdmittedCreatureExecutionObjectLikeCpp],
    config: &CreatureAggroConfigLikeCpp,
    outcome: &mut CreatureMeleeTickOutcomeLikeCpp,
) {
    let pending = {
        let mut store =
            AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(manager, maps, selection);
        collect_creature_melee_swings_on_store_like_cpp(&mut store, outcome)
    };
    for swing in pending {
        execute_creature_melee_swing_on_manager_like_cpp(
            manager,
            &mut CanonicalCreatureMeleeAttackersLikeCpp,
            swing,
            config,
            outcome,
        );
    }
}

#[cfg(test)]
#[path = "../../unit_tests/admitted_creature_execution/combat_phases/test_only.rs"]
mod test_only;
