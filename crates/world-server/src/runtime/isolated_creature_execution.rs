// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D2: the world-server adapter for the isolated admitted creature
//! execution.
//!
//! F6-8D2 connects the F6-8D1 phase engine to an **admitted** tick in the
//! isolated path. This module is the world-server half of that connection: it
//! takes the plan the canonical producer admitted (`MapTickPlanLikeCpp`) plus
//! the tick's selected canonical objects, and drives
//! [`wow_world::session::run_admitted_creature_execution_isolated_like_cpp`]
//! through the same two fences the production tick uses:
//!
//! 1. **The session barrier.** `crates/world-server/src/runtime/map_tick.rs:95`
//!    resumes a split tick only while the coordinator still awaits that plan's
//!    sessions. This adapter reads the same
//!    `MapTickCoordinationStateLikeCpp::AwaitingSessions(plan.epoch_like_cpp())`
//!    state first, before it reads a single map incarnation, so a recreated or
//!    already-resumed tick executes nothing.
//! 2. **The persistence-order guard.**
//!    `crates/world-server/src/runtime/map/update_loop.rs:250` takes
//!    [`SharedRespawnDbMutationOrderLikeCpp`] around the half of the tick that
//!    produces persistence statements. This adapter takes the same gate around
//!    the whole creature mutation window, so the statements a canonical swing
//!    would submit keep the mailbox ordering the production fence guarantees.
//!
//! Both guards are released **before** this adapter returns, and the deferred
//! `ApplyCreatureMeleeDamageLikeCppCommand` batch is only handed to
//! [`deliver_isolated_admitted_creature_execution_like_cpp`] afterwards: no
//! packet is published while a map or persistence guard is held.
//!
//! Production does not call either entry point. The production owner stays
//! `RuntimeTickOwner::GlobalLegacy` / `MapCreatureUpdateOwnerLikeCpp::ExternalRuntime`,
//! the legacy scheduler stays and the stores stay; D3 owns the exclusive
//! cutover and E owns the retirement of the legacy store. These entries are
//! exported from the composition library the same way
//! `compose_packet_handlers_like_cpp` is, so the cutover is a call-site change
//! rather than a re-write.

use wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand;

use super::*;

/// Why the isolated adapter executed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolatedCreatureExecutionRefusalLikeCpp {
    /// The canonical manager mutex is poisoned; nothing was read or mutated.
    CanonicalManagerPoisoned,
    /// The coordinator is not awaiting this plan's epoch. Refused before any
    /// map incarnation or map object was read.
    SessionBarrier {
        expected_epoch: u64,
        current: wow_map::MapTickCoordinationStateLikeCpp,
    },
    /// The canonical owner could not be read. An unreadable owner is not proof
    /// of absence, so no admission was captured and nothing executed.
    CanonicalOwnerUnreadable,
    /// The wow-world admission fence refused the frozen admission: a map
    /// incarnation, a record identity or the single-owner claim no longer
    /// matches what the tick admitted.
    Admission(wow_world::session::CreatureExecutionAdmissionLikeCpp),
}

impl IsolatedCreatureExecutionRefusalLikeCpp {
    /// Stable label for refusal reporting and assertions.
    #[must_use]
    pub const fn label_like_cpp(self) -> &'static str {
        match self {
            Self::CanonicalManagerPoisoned => "canonical-manager-poisoned",
            Self::SessionBarrier { .. } => "session-barrier",
            Self::CanonicalOwnerUnreadable => "canonical-owner-unreadable",
            Self::Admission(admission) => match admission.refusal_label_like_cpp() {
                Some(label) => label,
                None => "admission",
            },
        }
    }
}

/// The result of one isolated wiring attempt over one admitted tick plan.
#[derive(Debug, Clone)]
pub struct IsolatedAdmittedCreatureExecutionOutcomeLikeCpp {
    /// Coordinator identity the transition was claimed under.
    pub coordinator_id: u64,
    /// The plan's tick epoch; the session barrier is evaluated against it.
    pub tick_epoch: u64,
    /// The plan's saved diff — the single clock advancement of the transition.
    pub diff_ms: u32,
    /// Admitted map incarnations the plan carried.
    pub admitted_maps: usize,
    /// Admitted map incarnations whose incarnation still matched at capture.
    pub captured_maps: usize,
    /// Selected objects the caller offered.
    pub selected_objects: usize,
    /// Selected objects the admission could freeze.
    pub captured_objects: usize,
    /// The refusal, when the isolated path executed nothing.
    pub refusal: Option<IsolatedCreatureExecutionRefusalLikeCpp>,
    /// The wow-world execution outcome. A refusal still carries its zeroed
    /// outcome, so the caller can assert that nothing was mutated.
    pub execution: Option<wow_world::session::IsolatedCreatureExecutionOutcomeLikeCpp>,
    /// The persistence-order guard was observed held across the whole mutation
    /// window. `false` when the caller supplied no guard.
    pub mutation_window_guarded: bool,
}

impl Default for IsolatedAdmittedCreatureExecutionOutcomeLikeCpp {
    fn default() -> Self {
        Self {
            coordinator_id: 0,
            tick_epoch: 0,
            diff_ms: 0,
            admitted_maps: 0,
            captured_maps: 0,
            selected_objects: 0,
            captured_objects: 0,
            refusal: None,
            execution: None,
            mutation_window_guarded: false,
        }
    }
}

impl IsolatedAdmittedCreatureExecutionOutcomeLikeCpp {
    /// Whether the isolated path refused and therefore produced no effect.
    #[must_use]
    pub const fn refused_like_cpp(&self) -> bool {
        self.refusal.is_some()
    }

    /// The deferred publication batch. It is still owed to
    /// [`deliver_isolated_admitted_creature_execution_like_cpp`].
    #[must_use]
    pub fn deferred_publication_like_cpp(&self) -> &[ApplyCreatureMeleeDamageLikeCppCommand] {
        self.execution
            .as_ref()
            .map_or(&[], |execution| execution.deferred_publication.as_slice())
    }

    /// Whether the deferred batch was handed to a publication consumer.
    #[must_use]
    pub fn publication_delivered_like_cpp(&self) -> bool {
        self.execution
            .as_ref()
            .is_some_and(|execution| execution.publication_delivered)
    }
}

/// What delivering the deferred batch produced.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IsolatedCreatureExecutionDeliveryOutcomeLikeCpp {
    /// Commands the deferred batch carried.
    pub commands: usize,
    /// Commands enqueued on their victim session's rail.
    pub queued: usize,
    /// Commands that could not be enqueued.
    pub failed: usize,
}

/// Run the isolated admitted creature execution for one admitted tick plan.
///
/// Every guard this function takes — the session barrier read, the admission
/// capture, the persistence-order gate and the canonical manager guard of the
/// execution itself — is released before it returns. The deferred publication
/// is therefore only deliverable by a later call to
/// [`deliver_isolated_admitted_creature_execution_like_cpp`].
///
/// `objects` is the tick's selected canonical object set, already filtered by
/// the caller to the creature family it owns; the keys must be the plan's
/// admitted keys, exactly as the production tick selects them.
#[must_use]
pub fn run_isolated_admitted_creature_execution_for_tick_like_cpp(
    plan: &wow_map::MapTickPlanLikeCpp,
    canonical_map_manager: &SharedCanonicalMapManager,
    lease: &wow_world::session::SharedCreatureExecutionLeaseLikeCpp,
    coordinator_id: u64,
    respawn_db_mutation_order: Option<&SharedRespawnDbMutationOrderLikeCpp>,
    objects: &[(wow_map::MapKey, ObjectGuid)],
) -> IsolatedAdmittedCreatureExecutionOutcomeLikeCpp {
    let tick_epoch = plan.epoch_like_cpp();
    let mut outcome = IsolatedAdmittedCreatureExecutionOutcomeLikeCpp {
        coordinator_id,
        tick_epoch,
        diff_ms: plan.effective_diff_ms(),
        admitted_maps: plan.updated_maps_like_cpp().len(),
        selected_objects: objects.len(),
        ..Default::default()
    };

    // The session barrier first, before any map incarnation is read. This is
    // the same state `canonical_map_tick_resume_like_cpp` resumes against
    // (`crates/world-server/src/runtime/map_tick.rs:95`); a plan whose epoch the
    // coordinator no longer awaits must not reach a map at all.
    {
        let Ok(manager) = canonical_map_manager.lock() else {
            outcome.refusal =
                Some(IsolatedCreatureExecutionRefusalLikeCpp::CanonicalManagerPoisoned);
            return outcome;
        };
        let current = manager.tick_coordination_like_cpp();
        if current != wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(tick_epoch) {
            outcome.refusal = Some(IsolatedCreatureExecutionRefusalLikeCpp::SessionBarrier {
                expected_epoch: tick_epoch,
                current,
            });
            return outcome;
        }
    }

    // Freeze the admission of exactly this tick. The plan supplies the admitted
    // incarnations (never re-resolved by map id alone) and the caller supplies
    // the selected objects.
    let Some(admission) = wow_world::session::capture_admitted_creature_execution_like_cpp(
        canonical_map_manager,
        coordinator_id,
        tick_epoch,
        plan.effective_diff_ms(),
        plan.updated_maps_like_cpp()
            .iter()
            .map(|participant| (participant.key, participant.incarnation)),
        objects.iter().copied(),
    ) else {
        outcome.refusal = Some(IsolatedCreatureExecutionRefusalLikeCpp::CanonicalOwnerUnreadable);
        return outcome;
    };
    outcome.captured_maps = admission.maps.len();
    outcome.captured_objects = admission.objects.len();

    {
        // The persistence-order gate is taken across the whole mutation window,
        // so a swing this window commits submits its statement in the same
        // order the production tick's mailbox fence guarantees
        // (`crates/world-server/src/runtime/map/update_loop.rs:250`).
        let _respawn_db_mutation_order = respawn_db_mutation_order.map(|order| {
            let guard = order
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // Observed while the window is open: this same thread cannot take
            // the gate again, which is what makes "held across the window" a
            // measured fact rather than a comment.
            outcome.mutation_window_guarded =
                matches!(order.try_lock(), Err(std::sync::TryLockError::WouldBlock));
            guard
        });
        let execution = wow_world::session::run_admitted_creature_execution_isolated_like_cpp(
            &admission,
            canonical_map_manager,
            lease,
        );
        if !execution.executed_like_cpp() {
            outcome.refusal = Some(IsolatedCreatureExecutionRefusalLikeCpp::Admission(
                execution.admission,
            ));
        }
        outcome.execution = Some(execution);
    }
    // Every guard is released here. The deferred batch is still owed.
    outcome
}

/// Deliver the deferred publication of one isolated execution.
///
/// This is a separate entry on purpose: it must be called after
/// [`run_isolated_admitted_creature_execution_for_tick_like_cpp`] returned, so
/// no map or persistence guard is held while a session rail is written. The
/// canonical health is already committed by the execution, so this only routes
/// the already-resolved commands.
pub fn deliver_isolated_admitted_creature_execution_like_cpp(
    outcome: &mut IsolatedAdmittedCreatureExecutionOutcomeLikeCpp,
    registry: &PlayerRegistry,
) -> IsolatedCreatureExecutionDeliveryOutcomeLikeCpp {
    let Some(execution) = outcome.execution.as_mut() else {
        return IsolatedCreatureExecutionDeliveryOutcomeLikeCpp::default();
    };
    if execution.publication_delivered {
        return IsolatedCreatureExecutionDeliveryOutcomeLikeCpp::default();
    }
    let summary =
        deliver_creature_melee_damage_commands_like_cpp(&execution.deferred_publication, registry);
    execution.publication_delivered = true;
    IsolatedCreatureExecutionDeliveryOutcomeLikeCpp {
        commands: summary.commands_seen,
        queued: summary.candidates_queued,
        failed: summary.send_failed,
    }
}
