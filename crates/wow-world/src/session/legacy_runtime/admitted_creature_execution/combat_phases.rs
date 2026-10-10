// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3a-1: the admitted canonical executor runs the combat phases.
//!
//! The executor runs, for one admitted canonical map tick, the combat phases
//! of the legacy loop body (`world-server/src/runtime/delivery.rs`) in the
//! legacy order — player melee (phase 0), aggro/threat/assist/evade with its
//! attack start/stop commit, and creature melee — through the **same phase
//! bodies** the legacy bridge calls. Only the store differs: creatures are
//! enumerated from the admitted canonical selection and mutated in place on
//! the canonical map, so there is nothing to mirror afterwards.
//!
//! Between player melee and aggro the legacy loop runs lifecycle and movement.
//! Of those this slice runs only the `Unit::Update(p_time)` clock step the
//! movement phase begins with (`creature_movement_tick.rs`,
//! `advance_runtime_clock_like_cpp`), once per admitted creature; lifecycle,
//! the movement generators and the visibility refresh are F6-8D3a-2.
//!
//! #1263 F6-8D3a-1b: the spell phase runs between aggro and creature melee,
//! where the legacy loop runs it, through the same selection body
//! (`collect_creature_spell_actions_on_store_like_cpp`) and the same deferred
//! drain (`drain_creature_spell_actions_like_cpp`) the legacy bridge calls; the
//! caster is the canonical incarnation itself.
//!
//! Every publication is returned as deferred output: the plan events and the
//! committed commands, with the same packet bytes the legacy bridge would
//! deliver. Production still runs the legacy loop; nothing here is called by it.

use super::*;

/// What the combat phases read beside the admitted canonical map: the inputs
/// the legacy bridge collects from the player registry before any map lock.
pub struct AdmittedCreatureCombatPhaseInputsLikeCpp<'a> {
    pub player_melee_attackers: &'a [PlayerMeleeAttackerSnapshotLikeCpp],
    pub player_melee_phase_state: &'a mut PlayerMeleePhaseStateLikeCpp,
    pub aggro_candidates: &'a [LegacyCreatureAggroCandidateLikeCpp],
    /// The map environment seam the family-assistance LOS check reads.
    pub terrain: Option<&'a crate::map_manager::LiveTerrainHeights>,
    pub config: &'a LegacyCreatureAggroConfigLikeCpp,
}

/// The typed outcome of one admitted combat-phase execution.
#[derive(Debug, Clone)]
pub struct AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    pub admission: CreatureExecutionAdmissionLikeCpp,
    pub owner: CreatureExecutionOwnerLikeCpp,
    /// The one `Unit::Update` clock advancement of the transition.
    pub clock_advanced_ms: u64,
    pub player_melee: LegacyPlayerMeleeTickOutcomeLikeCpp,
    /// The aggro phase outcome. Its plan is already reduced to the committed
    /// combat events, exactly as the legacy bridge reduces it before delivery.
    pub aggro: LegacyCreatureAggroTickOutcomeLikeCpp,
    pub aggro_committed_starts: Vec<crate::session::mailbox::CreatureAttackStartLikeCppCommand>,
    pub aggro_committed_stops: Vec<crate::session::mailbox::CreatureAttackStopLikeCppCommand>,
    /// Whether the spell phase ran (F6-8D3a-1b). Its outcome carries the
    /// START/GO plan, exactly as the legacy bridge delivers it.
    pub spell_phase_executed: bool,
    pub spell: LegacyCreatureSpellTickOutcomeLikeCpp,
    pub melee: LegacyCreatureMeleeTickOutcomeLikeCpp,
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
            player_melee: LegacyPlayerMeleeTickOutcomeLikeCpp::default(),
            aggro: LegacyCreatureAggroTickOutcomeLikeCpp::default(),
            aggro_committed_starts: Vec::new(),
            aggro_committed_stops: Vec::new(),
            spell_phase_executed: false,
            spell: LegacyCreatureSpellTickOutcomeLikeCpp::default(),
            melee: LegacyCreatureMeleeTickOutcomeLikeCpp::default(),
        }
    }

    /// Whether the engine actually ran for this transition.
    #[must_use]
    pub const fn executed_like_cpp(&self) -> bool {
        self.admission.is_admitted_like_cpp()
    }

    /// The deferred plan events, in phase order (aggro, spell, then melee),
    /// exactly as the legacy bridge delivers its per-phase plans.
    #[must_use]
    pub fn deferred_plan_events_like_cpp(&self) -> Vec<RuntimeEvent> {
        self.aggro
            .plan
            .events
            .iter()
            .chain(&self.spell.plan.events)
            .chain(&self.melee.plan.events)
            .cloned()
            .collect()
    }
}

/// Run the admitted combat phases once.
///
/// Order, mirroring the legacy loop body: the admission fence and the
/// single-owner claim before any mutation; player melee; the clock step;
/// aggro and its canonical start/stop commit; the creature spell phase;
/// creature melee. The canonical
/// guard is held for the whole transition and released before returning, and
/// nothing is delivered.
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
    let admitted =
        admit_and_claim_admitted_creature_execution_like_cpp(admission, &manager, lease, owner);
    let mut outcome = AdmittedCreatureCombatPhasesOutcomeLikeCpp::refused_like_cpp(admitted, owner);
    if !admitted.is_admitted_like_cpp() {
        return outcome;
    }
    let config = inputs.config;

    // Phase 0: player melee (`Player::Update` → `DoMeleeAttackIfReady`), before
    // the creatures' own update, exactly as the legacy loop orders it.
    outcome.player_melee.attackers_seen = inputs.player_melee_attackers.len();
    let (pending, map_difficulties) = collect_player_melee_swings_on_manager_like_cpp(
        &mut manager,
        inputs.player_melee_attackers,
        admission.diff_ms,
        inputs.player_melee_phase_state,
        &mut outcome.player_melee,
    );
    outcome.player_melee.swings_ready = pending.len();
    for swing in pending {
        execute_player_melee_swing_on_manager_like_cpp(
            &mut manager,
            &mut CanonicalPlayerMeleeCreatureVictimsLikeCpp,
            &swing,
            &map_difficulties,
            admission.diff_ms,
            config,
            &mut outcome.player_melee,
        );
    }

    // `Unit::Update(p_time)`: the creature's one clock advancement, at the
    // place the legacy movement phase takes it (F6-8D3a-2 owns the rest).
    for object in &admission.objects {
        let advanced = manager
            .find_map_mut(object.map_id, object.instance_id)
            .and_then(|map| {
                map.map_mut()
                    .with_creature_mut_like_cpp(object.creature_guid, |creature| {
                        creature.advance_runtime_clock_like_cpp(admission.diff_ms);
                    })
            });
        if advanced.is_some() {
            outcome.clock_advanced_ms = outcome
                .clock_advanced_ms
                .saturating_add(u64::from(admission.diff_ms));
        }
    }

    // Aggro: `UpdateVictim`, assistance, `MoveInLineOfSight`, then the
    // canonical commit of every start/stop and the publication reduction the
    // legacy bridge applies before delivery.
    {
        let mut store =
            AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(&mut manager, &admission.objects);
        outcome.aggro = run_creature_aggro_phase_on_store_like_cpp(
            &mut store,
            inputs.terrain,
            &CanonicalCreatureOwnershipLikeCpp::AdmittedCanonicalStore,
            inputs.aggro_candidates,
            config,
        );
        let manager = store.manager_mut_like_cpp();
        let start_outcomes = apply_creature_attack_start_commands_on_manager_like_cpp(
            manager,
            &outcome.aggro.commands,
        );
        let stop_outcomes = apply_creature_attack_stop_commands_on_manager_like_cpp(
            manager,
            &outcome.aggro.stop_commands,
        );
        retain_committed_creature_combat_events_like_cpp(
            &mut outcome.aggro.plan,
            &outcome.aggro.commands,
            &start_outcomes,
            &outcome.aggro.stop_commands,
            &stop_outcomes,
        );
        outcome.aggro_committed_starts =
            committed_creature_combat_commands_like_cpp(&outcome.aggro.commands, &start_outcomes);
        outcome.aggro_committed_stops = committed_creature_combat_commands_like_cpp(
            &outcome.aggro.stop_commands,
            &stop_outcomes,
        );
    }

    // Spell: `CombatAI::UpdateAI` / `TurretAI::UpdateAI` after `UpdateVictim`
    // (`CombatAI.cpp:91-107, 217-223`), before melee as the legacy loop orders
    // it. Selection over the admitted store, then the deferred `DoCast` /
    // `ScheduleEvent` drain on the canonical manager still held here.
    if let Some(spell_store) = config.spell_store.as_ref() {
        let map_difficulties = creature_spell_map_difficulties_like_cpp(&manager);
        let actions = {
            let mut store = AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(
                &mut manager,
                &admission.objects,
            );
            collect_creature_spell_actions_on_store_like_cpp(
                &mut store,
                &CanonicalCreatureOwnershipLikeCpp::AdmittedCanonicalStore,
                &map_difficulties,
                spell_store,
                config,
                &mut outcome.spell,
            )
        };
        drain_creature_spell_actions_like_cpp(
            &mut CanonicalCreatureSpellActionOwnerLikeCpp {
                manager: &mut manager,
            },
            &CanonicalCreatureOwnershipLikeCpp::AdmittedCanonicalStore,
            actions,
            config,
            &mut outcome.spell,
        );
    }
    outcome.spell_phase_executed = true;

    // Creature melee (`DoMeleeAttackIfReady`) with the real attack table.
    let pending = {
        let mut store =
            AdmittedCanonicalCreatureStoreLikeCpp::new_like_cpp(&mut manager, &admission.objects);
        collect_creature_melee_swings_on_store_like_cpp(
            &mut store,
            &CanonicalCreatureOwnershipLikeCpp::AdmittedCanonicalStore,
            &mut outcome.melee,
        )
    };
    // The canonical victim is the only representation: the compatibility
    // syncs the legacy bridge replays into its mirror have no target here.
    let mut creature_victim_syncs = Vec::new();
    for swing in pending {
        execute_creature_melee_swing_on_manager_like_cpp(
            &mut manager,
            &mut CanonicalCreatureMeleeAttackersLikeCpp,
            swing,
            config,
            &mut outcome.melee,
            &mut creature_victim_syncs,
        );
    }
    outcome
}
