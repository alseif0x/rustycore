// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature canonical adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use std::collections::HashSet;

use crate::map_manager::SharedMapManager;
use crate::session::SharedCanonicalMapManager;
use wow_core::{ObjectGuid, Position};
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootAuthorityStamp};

pub fn relocate_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    position: Position,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().relocate_map_object_like_cpp(guid, position);
}

/// What applying one coexisting creature representation did to the canonical
/// incarnation that currently owns its GUID.
#[derive(Debug)]
pub enum CanonicalCreatureEntityApplicationLikeCpp {
    /// The snapshot replaced the canonical entity state; the returned authority
    /// is the incarnation's authority.
    Applied(OwnedLootAuthority),
    /// The incarnation admitted the representation but refused this exact
    /// snapshot under the health-revision/tuple guard. The canonical entity
    /// state is unchanged, so the caller must not publish the mutation.
    Rejected(OwnedLootAuthority),
    /// The incarnation, its authority or the canonical object refused the
    /// representation outright. Nothing was selected and nothing was written.
    Refused,
}

/// Which clause of [`creature_representation_is_admitted_like_cpp`] decided one
/// admission verdict.
///
/// #1263 C1 capture surface. The predicate itself is unchanged: it stays a
/// boolean and is derived from the same clause sequence reported here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureRepresentationAdmissionClauseLikeCpp {
    /// The representation carries another health-state revision authority: it
    /// belongs to a different incarnation.
    HealthTimeline,
    /// Same incarnation, but the transported revision is behind the canonical
    /// one, or equal to it with a disagreeing health tuple — the stale/ABA case.
    Revision,
    /// Same incarnation at or ahead of the canonical revision, but the carried
    /// loot allocation is neither the incarnation's own nor an unused pristine
    /// candidate.
    LootAllocation,
    /// All three clauses passed.
    Admitted,
    /// Not a predicate clause: the canonical incarnation that owns the GUID is
    /// absent, so the predicate was never evaluated.
    IncarnationAbsent,
    /// Not a predicate clause: no canonical manager, no map instance or the
    /// canonical execution lock was unavailable, so the predicate was never
    /// evaluated.
    OwnerUnavailable,
}

impl CreatureRepresentationAdmissionClauseLikeCpp {
    pub fn as_str_like_cpp(self) -> &'static str {
        match self {
            Self::HealthTimeline => "health_timeline",
            Self::Revision => "revision",
            Self::LootAllocation => "loot_allocation",
            Self::Admitted => "admitted",
            Self::IncarnationAbsent => "incarnation_absent",
            Self::OwnerUnavailable => "owner_unavailable",
        }
    }
}

/// One complete admission decision for a transported representation.
///
/// #1263 C1. Every field is copied while the canonical incarnation guard is
/// held so the event can be emitted after the guards are released.
#[derive(Debug, Clone, Copy)]
pub struct CreatureRepresentationAdmissionDecisionLikeCpp {
    pub admitted: bool,
    pub clause: CreatureRepresentationAdmissionClauseLikeCpp,
    pub owner_guid: ObjectGuid,
    /// Identity of the canonical incarnation's health timeline.
    pub incarnation_timeline_identity: u64,
    /// Identity of the transported representation's health timeline.
    pub representation_timeline_identity: u64,
    pub shares_health_timeline: bool,
    pub canonical_health_revision: u64,
    pub representation_health_revision: u64,
    pub health_tuple_matches: bool,
    pub allocation_is_own: bool,
    pub allocation_is_pristine: bool,
}

impl CreatureRepresentationAdmissionDecisionLikeCpp {
    /// The decision for a case where the predicate could not be evaluated.
    pub fn unevaluated_like_cpp(
        owner_guid: ObjectGuid,
        clause: CreatureRepresentationAdmissionClauseLikeCpp,
    ) -> Self {
        Self {
            admitted: false,
            clause,
            owner_guid,
            incarnation_timeline_identity: 0,
            representation_timeline_identity: 0,
            shares_health_timeline: false,
            canonical_health_revision: 0,
            representation_health_revision: 0,
            health_tuple_matches: false,
            allocation_is_own: false,
            allocation_is_pristine: false,
        }
    }
}

/// One instrumented creature-representation decision, ready to be emitted after
/// the canonical/legacy guards have been released.
#[derive(Debug, Clone, Copy)]
pub struct CreatureRepresentationCaptureLikeCpp {
    /// Production root that made the decision.
    pub root: &'static str,
    pub map_id: u16,
    pub instance_id: u32,
    pub decision: CreatureRepresentationAdmissionDecisionLikeCpp,
    /// Whether the root's mutation callback ran.
    pub mutation_invoked: bool,
    /// `applied` | `admission_refused` | `mutation_root_refused` |
    /// `canonical_application_refused` | `canonical_application_rejected` |
    /// `owner_unavailable` | `incarnation_absent`.
    pub application: &'static str,
    /// Whether the canonical incarnation applied the representation.
    pub applied: bool,
}

/// #1263 C1 capture: the single emission point for creature-representation
/// decisions. Callers copy a [`CreatureRepresentationCaptureLikeCpp`] under the
/// guards and call this **only after** those guards are released, so no I/O ever
/// runs inside a guard.
pub fn emit_creature_representation_capture_like_cpp(
    capture: &CreatureRepresentationCaptureLikeCpp,
) {
    // One event per decision, at the level its content deserves. The two
    // mirroring roots evaluate this predicate for every creature of the player's
    // map on every tick, so the routine case — admitted, applied, no mutation
    // invoked and carrying exactly the revision the incarnation already holds,
    // i.e. the creature did not change since the last mirror — is a DEBUG event.
    // A decision that refuses the representation, that carries a *newer*
    // representation revision (the mirror is publishing a creature that actually
    // changed: damage, death, respawn), or that ran the mutation root's own
    // callback (the admission decision is captured before the mutation, so an
    // applied owner mutation always shows equal revisions here) is an INFO
    // event, which is the level the server runs at. Neither path performs any
    // work when its level is disabled.
    let carries_state_change = capture.decision.admitted
        && (capture.mutation_invoked
            || capture.decision.representation_health_revision
                > capture.decision.canonical_health_revision);
    let refused = !capture.decision.admitted || !capture.applied;
    if carries_state_change || refused {
        tracing::info!(
            target: "rustycore::capture::c1",
            capture_id = "C1",
            root = capture.root,
            owner_guid = %capture.decision.owner_guid,
            map_id = capture.map_id,
            instance_id = capture.instance_id,
            incarnation_timeline_identity = capture.decision.incarnation_timeline_identity,
            representation_timeline_identity = capture.decision.representation_timeline_identity,
            shares_health_timeline = capture.decision.shares_health_timeline,
            canonical_health_revision = capture.decision.canonical_health_revision,
            representation_health_revision = capture.decision.representation_health_revision,
            health_tuple_matches = capture.decision.health_tuple_matches,
            allocation_is_own = capture.decision.allocation_is_own,
            allocation_is_pristine = capture.decision.allocation_is_pristine,
            admission_verdict = capture.decision.admitted,
            admission_clause = capture.decision.clause.as_str_like_cpp(),
            mutation_invoked = capture.mutation_invoked,
            application = capture.application,
            representation_applied = capture.applied,
            "RUSTYCORE_CAPTURE_C1 creature representation decision"
        );
    } else {
        tracing::debug!(
            target: "rustycore::capture::c1",
            capture_id = "C1",
            root = capture.root,
            owner_guid = %capture.decision.owner_guid,
            map_id = capture.map_id,
            instance_id = capture.instance_id,
            incarnation_timeline_identity = capture.decision.incarnation_timeline_identity,
            representation_timeline_identity = capture.decision.representation_timeline_identity,
            shares_health_timeline = capture.decision.shares_health_timeline,
            canonical_health_revision = capture.decision.canonical_health_revision,
            representation_health_revision = capture.decision.representation_health_revision,
            health_tuple_matches = capture.decision.health_tuple_matches,
            allocation_is_own = capture.decision.allocation_is_own,
            allocation_is_pristine = capture.decision.allocation_is_pristine,
            admission_verdict = capture.decision.admitted,
            admission_clause = capture.decision.clause.as_str_like_cpp(),
            mutation_invoked = capture.mutation_invoked,
            application = capture.application,
            representation_applied = capture.applied,
            "RUSTYCORE_CAPTURE_C1 creature representation decision"
        );
    }
}

/// Is `incoming` a representation of the incarnation `current` belongs to?
///
/// This is the read-only admission predicate evaluated *before* an owner mutates
/// an existing legacy representation: the same-incarnation health timeline, a
/// snapshot at or ahead of the canonical revision, and either the incarnation's
/// own loot allocation or an unused pristine candidate. It is the same guard
/// [`apply_canonical_creature_entity_on_map_like_cpp`] applies, so an admitted
/// representation cannot be refused because of its incarnation.
pub fn creature_representation_is_admitted_like_cpp(
    current: &wow_entities::Creature,
    incoming: &wow_entities::Creature,
) -> bool {
    creature_representation_admission_decision_like_cpp(current, incoming).admitted
}

/// The complete admission decision behind
/// [`creature_representation_is_admitted_like_cpp`], including which clause
/// decided it.
///
/// #1263 C1 capture surface. The clause sequence and short-circuit order are
/// exactly the ones the boolean predicate used, so the verdict is identical for
/// every input; this function only also reports *why*.
pub fn creature_representation_admission_decision_like_cpp(
    current: &wow_entities::Creature,
    incoming: &wow_entities::Creature,
) -> CreatureRepresentationAdmissionDecisionLikeCpp {
    let current_unit = current.unit();
    let incoming_unit = incoming.unit();
    let current_timeline_authority = current_unit.health_state_revision_authority_like_cpp();
    let incoming_timeline_authority = incoming_unit.health_state_revision_authority_like_cpp();
    let canonical_health_revision = current_unit.health_state_revision_like_cpp();
    let representation_health_revision = incoming_unit.health_state_revision_like_cpp();
    let shares_health_timeline =
        incoming_unit.shares_health_state_revision_authority_like_cpp(&current_timeline_authority);
    let health_tuple_matches = incoming_unit.data().health == current_unit.data().health
        && incoming_unit.data().max_health == current_unit.data().max_health
        && incoming_unit.death_state() == current_unit.death_state();
    let mut decision = CreatureRepresentationAdmissionDecisionLikeCpp {
        admitted: false,
        clause: CreatureRepresentationAdmissionClauseLikeCpp::HealthTimeline,
        owner_guid: current_unit.world().object().guid(),
        incarnation_timeline_identity: current_timeline_authority.timeline_identity_like_cpp(),
        representation_timeline_identity: incoming_timeline_authority.timeline_identity_like_cpp(),
        shares_health_timeline,
        canonical_health_revision,
        representation_health_revision,
        health_tuple_matches,
        allocation_is_own: false,
        allocation_is_pristine: false,
    };
    if !shares_health_timeline {
        return decision;
    }
    if !(representation_health_revision > canonical_health_revision
        || (representation_health_revision == canonical_health_revision && health_tuple_matches))
    {
        decision.clause = CreatureRepresentationAdmissionClauseLikeCpp::Revision;
        return decision;
    }
    let incoming_authority = incoming.loot_authority_like_cpp();
    decision.allocation_is_own =
        incoming_authority.shares_storage_like_cpp(current.loot_authority_like_cpp());
    decision.allocation_is_pristine = incoming_authority.is_pristine_like_cpp();
    if !decision.allocation_is_own && !decision.allocation_is_pristine {
        decision.clause = CreatureRepresentationAdmissionClauseLikeCpp::LootAllocation;
        return decision;
    }
    decision.admitted = true;
    decision.clause = CreatureRepresentationAdmissionClauseLikeCpp::Admitted;
    decision
}

/// Apply one transported creature snapshot to the canonical incarnation that
/// currently owns its GUID, with the canonical map already locked.
///
/// This is the in-guard half of [`sync_canonical_creature_entity_on_map_like_cpp`]:
/// the caller owns the canonical execution lock, so this performs no manager
/// lookup, no manager re-entry, no I/O, no await and no delivery.
pub fn apply_canonical_creature_entity_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    mut creature: wow_entities::Creature,
) -> CanonicalCreatureEntityApplicationLikeCpp {
    let guid = creature.unit().world().object().guid();
    if map
        .creature_transform_vitals_snapshot_like_cpp(guid)
        .is_none()
    {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    let Some((shares_health_timeline, accept_incoming_entity_state, current_authority)) = map
        .with_creature_like_cpp(guid, |current| {
            let incoming_unit = creature.unit();
            let current_unit = current.unit();
            let shares_health_timeline = incoming_unit
                .shares_health_state_revision_authority_like_cpp(
                    &current_unit.health_state_revision_authority_like_cpp(),
                );
            let incoming_revision = incoming_unit.health_state_revision_like_cpp();
            let current_revision = current_unit.health_state_revision_like_cpp();
            let health_tuple_matches = incoming_unit.data().health == current_unit.data().health
                && incoming_unit.data().max_health == current_unit.data().max_health
                && incoming_unit.death_state() == current_unit.death_state();

            // Whole-entity replacement is safe only inside the same incarnation
            // timeline. A lower revision is a stale snapshot even when health has
            // completed an ABA cycle; an equal revision is valid only when its full
            // represented health tuple agrees. Reject the entire snapshot instead
            // of copying only health, because death/respawn hooks also mutate AI,
            // combat, loot, aura, timer, flag, and runtime-plan state.
            (
                shares_health_timeline,
                shares_health_timeline
                    && (incoming_revision > current_revision
                        || (incoming_revision == current_revision && health_tuple_matches)),
                current.loot_authority_like_cpp().clone(),
            )
        })
    else {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    };

    let incoming_authority = creature.loot_authority_like_cpp().clone();
    // R1b guard, before any authority is selected: a coexisting representation
    // may only synchronize an incarnation it belongs to and may only carry the
    // incarnation's own allocation or an unused pristine candidate. A snapshot
    // from another health timeline is a different incarnation, and a second
    // independently used allocation is a competing claimable pool, so neither
    // may be applied to this object or republished from it. Refusing here leaves
    // the canonical object and its authority untouched; the legacy owner keeps
    // whatever it had instead of adopting a competing pool.
    if !shares_health_timeline
        || (!incoming_authority.shares_storage_like_cpp(&current_authority)
            && !incoming_authority.is_pristine_like_cpp())
    {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    // F6-7 R3: one execution owner, no dual-store reconciliation. The guard
    // above already refused every representation whose allocation is neither
    // this incarnation's own nor an unused pristine candidate, so there is no
    // second claimable pool left to arbitrate: the canonical incarnation keeps
    // its own allocation, and a quarantined one stays terminal and fail-closed.
    // The single remaining case is an allocation that can never own loot again
    // (a displaced `Detached` one) taking the admitted pristine candidate, which
    // is how an incarnation acquires a fresh allocation after replacement.
    let authority = if current_authority.lifecycle_like_cpp()
        == OwnedLootAuthorityLifecycle::Detached
        && incoming_authority.is_pristine_like_cpp()
    {
        incoming_authority
    } else {
        current_authority.clone()
    };
    let current_stamp = current_authority.stamp_like_cpp();
    // The original `??` contract: a missing canonical object or a failed
    // expected-stamp compare/exchange refuses the snapshot. `Some(false)` is the
    // successful "already this authority" case, not a refusal.
    if map
        .with_creature_mut_like_cpp(guid, |current| {
            current.rebind_loot_authority_if_current_like_cpp(
                &current_authority,
                current_stamp,
                authority.clone(),
            )
        })
        .flatten()
        .is_none()
    {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    if !accept_incoming_entity_state {
        // The actual legacy owner performs its own expected-stamp CAS with the
        // returned authority. Its rejected transport clone must not replace any
        // canonical lifecycle fields.
        return CanonicalCreatureEntityApplicationLikeCpp::Rejected(authority);
    }
    // `creature` is a cloned transport snapshot whose old authority is still
    // owned by the live legacy entity. Do not detach it here; the caller
    // performs the expected-stamp CAS on that actual entity.
    creature.adopt_loot_authority_for_snapshot_like_cpp(authority);
    let old_threat_guids = map
        .with_creature_like_cpp(guid, |current| {
            current.unit().subsystems().combat.sorted_threat_guids()
        })
        .unwrap_or_default();
    let incoming_threat_guids: HashSet<_> = creature
        .unit()
        .subsystems()
        .combat
        .sorted_threat_guids()
        .into_iter()
        .collect();
    let removed_threat_guids: Vec<_> = old_threat_guids
        .into_iter()
        .filter(|threat_guid| !incoming_threat_guids.contains(threat_guid))
        .collect();
    let mirrored_threat_guids: Vec<_> = incoming_threat_guids.iter().copied().collect();

    creature.unit_mut().world_mut().object_mut().add_to_world();
    let Ok(record) = wow_entities::MapObjectRecord::new_creature(creature) else {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    };
    let Some(authority) = record
        .creature()
        .map(|creature| creature.loot_authority_like_cpp().clone())
    else {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    };
    if map.insert_map_object_record(record).is_err() {
        return CanonicalCreatureEntityApplicationLikeCpp::Refused;
    }
    for added_guid in mirrored_threat_guids {
        let threat_ref = map
            .with_creature_like_cpp(guid, |creature| {
                creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_ref(added_guid)
                    .copied()
            })
            .flatten();
        let Some(threat_ref) = threat_ref else {
            continue;
        };
        if let Some(player) = map.get_typed_player_mut(added_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        } else if let Some(creature) = map.get_typed_creature_mut(added_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        }
    }
    for removed_guid in removed_threat_guids {
        if let Some(player) = map.get_typed_player_mut(removed_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        } else if let Some(creature) = map.get_typed_creature_mut(removed_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        }
    }
    CanonicalCreatureEntityApplicationLikeCpp::Applied(authority)
}

/// Synchronize one transported creature snapshot through the shared canonical
/// map manager.
///
/// This is the locking compatibility wrapper around
/// [`apply_canonical_creature_entity_on_map_like_cpp`]: it keeps the historical
/// signature and the historical return contract (the selected authority for an
/// applied *or* rejected snapshot, `None` only for a refused one). Owners that
/// mutate an existing representation go through
/// [`crate::session::SessionCore::mutate_world_creature`], which holds the
/// canonical execution lock across the mutation and refuses to expose a result
/// the canonical incarnation did not apply.
pub fn sync_canonical_creature_entity_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    creature: wow_entities::Creature,
) -> Option<OwnedLootAuthority> {
    let Ok(mut manager) = manager.lock() else {
        return None;
    };
    let map = manager.find_map_mut(map_id, instance_id)?;
    match apply_canonical_creature_entity_on_map_like_cpp(map.map_mut(), creature) {
        CanonicalCreatureEntityApplicationLikeCpp::Applied(authority)
        | CanonicalCreatureEntityApplicationLikeCpp::Rejected(authority) => Some(authority),
        CanonicalCreatureEntityApplicationLikeCpp::Refused => None,
    }
}

/// Synchronize one transported creature representation into the canonical
/// incarnation that owns its GUID **through the R7a/R7b-2a admission gate**,
/// and then — only on an applied snapshot — rebind the coexisting legacy
/// representation's loot alias.
///
/// F6-7 R7b-2b. This is the one shared map-level gate the mirroring roots use.
/// The two production mirror sites (the player melee tick and the creature
/// movement tick) used to call [`sync_canonical_creature_entity_on_map_like_cpp`]
/// directly and then run the legacy expected-authority/expected-stamp
/// compare-and-exchange themselves, so a representation that belonged to
/// another incarnation — or carried a competing used allocation — was still
/// offered to the canonical object and still attempted a legacy rebind. This
/// root evaluates the existing admission predicate
/// ([`creature_representation_is_admitted_like_cpp`]) against the **current**
/// canonical incarnation *before* the application is invoked, so a missing
/// canonical manager, a missing map instance, a missing incarnation, a
/// representation from another health timeline, a stale or ABA-replayed
/// revision and a competing used loot allocation are all refused with nothing
/// written in either store.
///
/// Lock order and scope: the canonical execution lock is held first and the
/// legacy manager second — the established canonical→legacy order. The
/// admission read, the application and the legacy compare-and-exchange all run
/// inside those guards, and none of them performs allocation beyond the
/// application's own record construction, I/O, await, delivery or manager
/// re-entry.
///
/// Return contract: `true` if and only if the canonical incarnation **applied**
/// the representation. A missing legacy representation, or a legacy
/// compare-and-exchange that did not match the expected authority/stamp,
/// changes nothing about that: the canonical application already happened and
/// the caller must observe it as applied. `false` means no canonical
/// application took place (admission refused, or the incarnation refused the
/// snapshot), and therefore neither store was written.
pub fn sync_admitted_creature_representation_on_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    legacy: Option<&SharedMapManager>,
    map_id: u16,
    instance_id: u32,
    creature: wow_entities::Creature,
    expected_authority: &OwnedLootAuthority,
    expected_stamp: OwnedLootAuthorityStamp,
) -> bool {
    // #1263 C1: the decision is copied inside the guards and emitted here, after
    // the inner function — and therefore both guards — has returned.
    let (applied, capture) = sync_admitted_creature_representation_captured_like_cpp(
        canonical,
        legacy,
        map_id,
        instance_id,
        creature,
        expected_authority,
        expected_stamp,
    );
    if let Some(capture) = capture {
        emit_creature_representation_capture_like_cpp(&capture);
    }
    applied
}

/// The guarded body of
/// [`sync_admitted_creature_representation_on_map_like_cpp`].
///
/// It returns the same boolean the public root exposes plus the already-copied
/// capture record, so the public root can emit it **after** the canonical and
/// legacy guards taken here have been released. No I/O, await or manager
/// re-entry happens inside these guards.
#[allow(clippy::too_many_arguments)]
fn sync_admitted_creature_representation_captured_like_cpp(
    canonical: &SharedCanonicalMapManager,
    legacy: Option<&SharedMapManager>,
    map_id: u16,
    instance_id: u32,
    creature: wow_entities::Creature,
    expected_authority: &OwnedLootAuthority,
    expected_stamp: OwnedLootAuthorityStamp,
) -> (bool, Option<CreatureRepresentationCaptureLikeCpp>) {
    const ROOT: &str = "sync_admitted_creature_representation_on_map_like_cpp";
    let unfavourable = |decision: CreatureRepresentationAdmissionDecisionLikeCpp,
                        application: &'static str|
     -> (bool, Option<CreatureRepresentationCaptureLikeCpp>) {
        (
            false,
            Some(CreatureRepresentationCaptureLikeCpp {
                root: ROOT,
                map_id,
                instance_id,
                decision,
                mutation_invoked: false,
                application,
                applied: false,
            }),
        )
    };
    let guid = creature.unit().world().object().guid();
    let Ok(mut manager) = canonical.lock() else {
        return unfavourable(
            CreatureRepresentationAdmissionDecisionLikeCpp::unevaluated_like_cpp(
                guid,
                CreatureRepresentationAdmissionClauseLikeCpp::OwnerUnavailable,
            ),
            "owner_unavailable",
        );
    };
    let Some(map) = manager.find_map_mut(u32::from(map_id), instance_id) else {
        return unfavourable(
            CreatureRepresentationAdmissionDecisionLikeCpp::unevaluated_like_cpp(
                guid,
                CreatureRepresentationAdmissionClauseLikeCpp::OwnerUnavailable,
            ),
            "owner_unavailable",
        );
    };
    // Admission resolves ownership before the application is invoked, exactly
    // as `SessionCore::mutate_world_creature` does: a refusal here cannot be
    // reconciled, quarantined or published by the application below.
    let decision = map.map().with_creature_like_cpp(guid, |current| {
        creature_representation_admission_decision_like_cpp(current, &creature)
    });
    let Some(decision) = decision else {
        return unfavourable(
            CreatureRepresentationAdmissionDecisionLikeCpp::unevaluated_like_cpp(
                guid,
                CreatureRepresentationAdmissionClauseLikeCpp::IncarnationAbsent,
            ),
            "incarnation_absent",
        );
    };
    if !decision.admitted {
        return unfavourable(decision, "admission_refused");
    }
    let authority = match apply_canonical_creature_entity_on_map_like_cpp(map.map_mut(), creature) {
        CanonicalCreatureEntityApplicationLikeCpp::Applied(authority) => authority,
        // The gate above refuses every representation the application could
        // reject, so `Rejected` and `Refused` are unreachable here unless the
        // incarnation changed under the guard; neither applied anything, so
        // neither may be reported as applied and neither may rebind the alias.
        CanonicalCreatureEntityApplicationLikeCpp::Rejected(_) => {
            return unfavourable(decision, "canonical_application_rejected");
        }
        CanonicalCreatureEntityApplicationLikeCpp::Refused => {
            return unfavourable(decision, "canonical_application_refused");
        }
    };
    // Applied. The legacy alias is synchronized afterwards, still under the
    // canonical guard and only if that representation exists. Its
    // compare-and-exchange is a best-effort rebind of the alias: it does not
    // decide whether the canonical incarnation applied the representation.
    if let Some(legacy) = legacy {
        let mut legacy = legacy
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(world_creature) = legacy.find_creature_mut(map_id, instance_id, guid) {
            let _ = world_creature
                .creature
                .rebind_loot_authority_if_current_like_cpp(
                    expected_authority,
                    expected_stamp,
                    authority,
                );
        }
    }
    (
        true,
        Some(CreatureRepresentationCaptureLikeCpp {
            root: ROOT,
            map_id,
            instance_id,
            decision,
            mutation_invoked: false,
            application: "applied",
            applied: true,
        }),
    )
}

pub fn remove_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    remove_canonical_creature_map_object_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        guid,
    );
}

/// [`remove_canonical_creature_map_object_on_map_like_cpp`] on a canonical
/// manager the caller already holds (#1263 F6-8D3a-3: the admitted executor
/// runs the lifecycle under its one canonical guard).
pub fn remove_canonical_creature_map_object_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
) {
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().remove_from_map_like_cpp(guid, true);
}

pub fn add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    info: wow_map::RespawnInfoLikeCpp,
) -> (bool, bool) {
    let Ok(mut manager) = manager.lock() else {
        return (false, false);
    };
    add_canonical_creature_respawn_info_and_remove_map_object_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        guid,
        info,
    )
}

pub fn add_canonical_creature_respawn_info_and_remove_map_object_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    info: wow_map::RespawnInfoLikeCpp,
) -> (bool, bool) {
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return (false, false);
    };

    let respawn_added = matches!(
        map.map_mut().add_respawn_info_like_cpp(info),
        wow_map::AddRespawnInfoOutcomeLikeCpp::Inserted
            | wow_map::AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    let object_removed = map.map_mut().remove_from_map_like_cpp(guid, true).is_ok();
    (respawn_added, object_removed)
}

pub fn remove_canonical_respawn_time_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    remove_canonical_respawn_time_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        object_type,
        spawn_id,
    )
}

pub fn remove_canonical_respawn_time_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return false;
    };
    map.map_mut()
        .remove_respawn_time_like_cpp(object_type, spawn_id)
        .is_some()
}
