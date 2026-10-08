//! F6-7 R7b-2a regressions: the two guarded creature loot mutators.
//!
//! The slice gates the viewed and unviewed guarded loot mutators with the same
//! mechanism the R7a gate uses (`SessionCore::with_admitted_world_creature_like_cpp`:
//! the R7a admission predicate against the current canonical incarnation,
//! canonical→legacy lock order and the canonical application before any success
//! is exposed). Each mutator adds its own admission — the observed allocation
//! must be the representation's, and the fully-looted/unviewed lifecycle
//! observation must still be current — *before* the mutation is invoked, with
//! the observation and the mutation in one critical section.
//!
//! The tests drive the production roots
//! (`LootReleaseOwnerAccessLikeCpp::finish_looted_creature_like_cpp` and
//! `finish_unviewed_looted_creature_like_cpp`), not a double. A refusal must
//! return no success event, must leave both stores at their complete
//! pre-operation state, and must not invoke the mutation at all: the retained
//! lootable dynamic flag and its unchanged `ObjectChangedFields::DYNAMIC_FLAGS`
//! mark are the evidence that the callback never ran.

use super::r7a_canonical_mutation::{
    advance_canonical_max_health_like_cpp, canonical_creature_like_cpp, legacy_creature_like_cpp,
    mirrored_session_like_cpp, observables_like_cpp,
};
use super::r7b2a_canonical_mutation::*;
use super::*;

struct FullyLootedLifecycleLikeCpp {
    authority: wow_loot::OwnedLootAuthority,
    viewed_object_generation: u64,
    viewed_lifecycle_revision: u64,
    unviewed_object_generation: u64,
    unviewed_lifecycle_revision: u64,
}

/// A fully looted allocation installed on both representations of the
/// incarnation, with the viewed and unviewed lifecycle observations a release
/// captures before calling the guarded roots.
fn install_fully_looted_lifecycle_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    viewer: ObjectGuid,
) -> FullyLootedLifecycleLikeCpp {
    let (authority, viewed_object_generation, viewed_lifecycle_revision) =
        observed_fully_looted_authority_like_cpp(guid, viewer);
    rebind_legacy_authority_like_cpp(manager, guid, authority.clone());
    rebind_canonical_authority_like_cpp(canonical, guid, authority.clone());
    let unviewed = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .expect("the fully looted owner is unviewed after the release");
    FullyLootedLifecycleLikeCpp {
        authority,
        viewed_object_generation,
        viewed_lifecycle_revision,
        unviewed_object_generation: unviewed.object_generation,
        unviewed_lifecycle_revision: unviewed.lifecycle_revision,
    }
}

/// Mark the incarnation lootable on **both** stores so the lifecycle closure's
/// `remove_lootable_dynamic_flag_like_cpp` is observable.
fn mark_incarnation_lootable_like_cpp(
    session: &mut WorldSession,
    manager: &crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
) {
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .set_dynamic_flag(UnitDynFlags::Lootable as u32);
        })
        .expect("the fixture marks one admitted incarnation lootable");
    assert!(
        has_lootable_flag_like_cpp(
            &legacy_creature_like_cpp(manager, guid).expect("legacy representation")
        ),
        "the fixture's mark reached the legacy representation"
    );
}

/// A recreation of the incarnation under the same GUID refuses the destroyed
/// incarnation's guarded release, and that refusal cannot resurrect a legacy
/// representation the destroyed incarnation took with it.
#[test]
fn guarded_release_refuses_a_replacement_incarnation_like_cpp() {
    let guid = test_creature_guid(92_212);
    let viewer = ObjectGuid::create_player(1, 92_218);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    // The canonical incarnation is destroyed and recreated under the same GUID.
    remove_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, guid);
    let _ = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        0,
        0,
        admission_candidate_like_cpp(guid),
    )
    .expect("the recreation is admitted");
    let replacement_before = loot_lifecycle_state_like_cpp(
        &canonical_creature_like_cpp(&canonical, guid).expect("replacement"),
    );
    assert!(
        lifecycle
            .authority
            .with_fully_looted_lifecycle_observation_like_cpp(
                lifecycle.viewed_object_generation,
                lifecycle.viewed_lifecycle_revision,
                || (),
            )
            .is_none(),
        "destroying the incarnation leaves the destroyed incarnation's release observation non-current"
    );

    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a release observation of a destroyed incarnation must not mutate its replacement"
    );
    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "the detached variant refuses the replacement incarnation too"
    );
    assert_eq!(
        loot_lifecycle_state_like_cpp(
            &canonical_creature_like_cpp(&canonical, guid).expect("replacement")
        ),
        replacement_before,
        "the replacement keeps its own state, allocation and health timeline"
    );

    // The representation was destroyed with its incarnation: the refusal must
    // not create a legacy representation for the recreated one either.
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove_creature_any(0, 0, guid)
        .expect("the fixture registered the legacy representation");
    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a destroyed representation with no replacement mirror is unavailable ownership"
    );
    assert!(legacy_creature_like_cpp(&manager, guid).is_none());
}

// ---------------------------------------------------------------------------
// Roots 2 and 3: the guarded viewed and unviewed loot mutators
// ---------------------------------------------------------------------------

fn viewed_release_of_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    authority: &wow_loot::OwnedLootAuthority,
    object_generation: u64,
    lifecycle_revision: u64,
) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
    session
        .core
        .loot_release_owner_access_like_cpp()
        .finish_looted_creature_like_cpp(
            guid,
            false,
            0.5,
            Some((authority, object_generation, lifecycle_revision)),
        )
}

fn viewed_release_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    lifecycle: &FullyLootedLifecycleLikeCpp,
) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
    viewed_release_of_like_cpp(
        session,
        guid,
        &lifecycle.authority,
        lifecycle.viewed_object_generation,
        lifecycle.viewed_lifecycle_revision,
    )
}

fn unviewed_release_of_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    authority: &wow_loot::OwnedLootAuthority,
    object_generation: u64,
    lifecycle_revision: u64,
) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
    session
        .core
        .loot_release_owner_access_like_cpp()
        .finish_unviewed_looted_creature_like_cpp(
            guid,
            false,
            0.5,
            authority,
            object_generation,
            lifecycle_revision,
        )
}

fn unviewed_release_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    lifecycle: &FullyLootedLifecycleLikeCpp,
) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
    unviewed_release_of_like_cpp(
        session,
        guid,
        &lifecycle.authority,
        lifecycle.unviewed_object_generation,
        lifecycle.unviewed_lifecycle_revision,
    )
}

/// One fully looted, lootable, mirrored incarnation with a captured release.
fn release_fixture_like_cpp(
    guid: ObjectGuid,
    viewer: ObjectGuid,
) -> (
    WorldSession,
    crate::map_manager::SharedMapManager,
    SharedCanonicalMapManager,
    FullyLootedLifecycleLikeCpp,
) {
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    mark_incarnation_lootable_like_cpp(&mut session, &manager, guid);
    let lifecycle = install_fully_looted_lifecycle_like_cpp(&manager, &canonical, guid, viewer);
    (session, manager, canonical, lifecycle)
}

#[test]
fn fresh_viewed_release_lifecycle_is_applied_once_to_both_stores_like_cpp() {
    let guid = test_creature_guid(92_200);
    let viewer = ObjectGuid::create_player(1, 92_201);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);

    let result = viewed_release_like_cpp(&mut session, guid, &lifecycle);

    assert!(
        result.is_some(),
        "a fresh viewed release observes one success event"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let owner = canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation");
    assert!(
        !has_lootable_flag_like_cpp(&legacy),
        "the legacy representation runs the corpse lifecycle exactly once"
    );
    assert!(
        !has_lootable_flag_like_cpp(&owner),
        "the canonical incarnation carries the same lifecycle result"
    );
    assert_eq!(
        observables_like_cpp(&legacy),
        observables_like_cpp(&owner),
        "the mutation is canonical, not legacy-only"
    );
    assert_one_incarnation_like_cpp(&manager, &canonical, guid);
}

/// **Negative control (R7b-2a, primary).** With the guarded mutator's pre-slice
/// body restored this test fails: the stale representation still runs the corpse
/// lifecycle and reports success while the canonical incarnation keeps its
/// earlier state.
#[test]
fn stale_viewed_release_is_refused_before_invocation_like_cpp() {
    let guid = test_creature_guid(92_202);
    let viewer = ObjectGuid::create_player(1, 92_203);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    // The canonical incarnation advances without the representation.
    advance_canonical_max_health_like_cpp(&canonical, guid, 140);
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    let result = viewed_release_like_cpp(&mut session, guid, &lifecycle);

    assert!(
        result.is_none(),
        "a stale representation must not report a successful lifecycle mutation"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "ownership is refused before the mutation runs: the lootable flag and its change mark stay"
    );
    assert_eq!(
        lifecycle.authority.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active,
        "the refused release leaves the response authority untouched"
    );
}

#[test]
fn unviewed_release_detached_lifecycle_is_applied_once_to_both_stores_like_cpp() {
    let guid = test_creature_guid(92_204);
    let viewer = ObjectGuid::create_player(1, 92_205);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);

    let result = unviewed_release_like_cpp(&mut session, guid, &lifecycle);

    assert!(
        result.is_some(),
        "a fresh detached release observes one success event"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let owner = canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation");
    assert!(
        !has_lootable_flag_like_cpp(&legacy) && !has_lootable_flag_like_cpp(&owner),
        "both representations carry the detached lifecycle result"
    );
    assert_eq!(observables_like_cpp(&legacy), observables_like_cpp(&owner));
    assert_one_incarnation_like_cpp(&manager, &canonical, guid);
}

#[test]
fn unviewed_release_refuses_a_stale_representation_before_invocation_like_cpp() {
    let guid = test_creature_guid(92_206);
    let viewer = ObjectGuid::create_player(1, 92_207);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    advance_canonical_max_health_like_cpp(&canonical, guid, 140);
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a stale representation must not report a successful detached mutation"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before
    );
}

#[test]
fn guarded_release_refuses_a_mismatched_authority_before_invocation_like_cpp() {
    let guid = test_creature_guid(92_208);
    let viewer = ObjectGuid::create_player(1, 92_209);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    // A second, independently observed allocation for the same GUID: the
    // representation is admitted by the incarnation and only the observed
    // allocation is the wrong owner.
    let (foreign, foreign_generation, foreign_revision) =
        observed_fully_looted_authority_like_cpp(guid, viewer);
    assert!(
        !foreign.shares_storage_like_cpp(&lifecycle.authority),
        "the foreign allocation is not the incarnation's"
    );
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    assert!(
        viewed_release_of_like_cpp(
            &mut session,
            guid,
            &foreign,
            foreign_generation,
            foreign_revision
        )
        .is_none(),
        "the observation must belong to the representation the mutation would run on"
    );
    let foreign_unviewed = foreign
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .expect("the foreign allocation is fully looted and unviewed");
    assert!(
        unviewed_release_of_like_cpp(
            &mut session,
            guid,
            &foreign,
            foreign_unviewed.object_generation,
            foreign_unviewed.lifecycle_revision
        )
        .is_none(),
        "the detached variant refuses a competing allocation too"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "a mismatched allocation refuses before the mutation is invoked"
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(
                canonical_creature_like_cpp(&canonical, guid)
                    .expect("canonical incarnation")
                    .loot_authority_like_cpp()
            ),
        "the refused release keeps one incarnation"
    );
}

#[test]
fn guarded_release_refuses_a_stale_lifecycle_observation_before_invocation_like_cpp() {
    let guid = test_creature_guid(92_210);
    let viewer = ObjectGuid::create_player(1, 92_211);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    // A replacement pool after the release observation: the owner is still fully
    // looted but its generation and lifecycle revision are no longer the observed
    // ones, so the observation and the mutation cannot be one serialized
    // operation any more.
    assert_ne!(
        lifecycle.authority.replace_like_cpp(
            Some(fully_looted_pool_like_cpp(guid, viewer)),
            HashMap::new()
        ),
        0,
        "a replacement pool starts a new generation"
    );
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a stale fully-looted observation must not mutate the object"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "the callback is refused, not executed: the lootable flag and its change mark are untouched"
    );
}

#[test]
fn guarded_release_refuses_a_reopened_pool_for_the_detached_variant_like_cpp() {
    let guid = test_creature_guid(92_213);
    let viewer = ObjectGuid::create_player(1, 92_214);
    let late = ObjectGuid::create_player(1, 92_215);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    lifecycle
        .authority
        .add_viewer_like_cpp(late)
        .expect("a second viewer opens the pool");
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "detached completion requires every authoritative pool to stay unviewed"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before
    );
}

#[test]
fn guarded_release_refuses_an_aba_replay_like_cpp() {
    let guid = test_creature_guid(92_219);
    let viewer = ObjectGuid::create_player(1, 92_220);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);
    // Health leaves and returns to the same tuple while the revision advances,
    // so the representation's tuple matches and its timeline does not.
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(0, 0).expect("canonical map instance");
        map.map_mut()
            .with_creature_mut_like_cpp(guid, |creature| {
                creature.unit_mut().set_health(60);
                creature.unit_mut().set_health(100);
            })
            .expect("canonical incarnation");
    }
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "an ABA replay is a stale representation, not a fresh one"
    );
    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "the detached variant refuses an ABA replay too"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "both refusals ran before the callback: the lootable flag and its change mark stay"
    );
    assert_eq!(
        lifecycle.authority.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active,
        "the refused releases leave the response authority untouched"
    );
}

/// The two guarded mutator roots themselves: their callback runs exactly once
/// when both admissions pass and not at all when either refuses.
#[test]
fn guarded_mutator_roots_invoke_their_callback_exactly_once_like_cpp() {
    let guid = test_creature_guid(92_221);
    let viewer = ObjectGuid::create_player(1, 92_222);
    let (mut session, _manager, _canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);

    let mut viewed_runs = 0_usize;
    let viewed = session
        .core
        .mutate_world_creature_if_fully_looted_observation_like_cpp(
            guid,
            &lifecycle.authority,
            lifecycle.viewed_object_generation,
            lifecycle.viewed_lifecycle_revision,
            |creature| {
                viewed_runs += 1;
                has_lootable_flag_like_cpp(&creature.creature)
            },
        );
    assert_eq!(
        viewed,
        Some(true),
        "the admitted mutation exposes the callback's own result"
    );
    assert_eq!(viewed_runs, 1, "the admitted mutation executes once");

    let mut unviewed_runs = 0_usize;
    let unviewed = session
        .core
        .mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
            guid,
            &lifecycle.authority,
            lifecycle.unviewed_object_generation,
            lifecycle.unviewed_lifecycle_revision,
            |creature| {
                unviewed_runs += 1;
                has_lootable_flag_like_cpp(&creature.creature)
            },
        );
    assert_eq!(unviewed, Some(true));
    assert_eq!(
        unviewed_runs, 1,
        "the admitted detached mutation executes once"
    );

    // The same roots after the canonical incarnation advances: refuse with the
    // callback never invoked and neither store written.
    let advance_guid = test_creature_guid(92_223);
    let advance_viewer = ObjectGuid::create_player(1, 92_224);
    let (mut stale_session, stale_manager, stale_canonical, stale_lifecycle) =
        release_fixture_like_cpp(advance_guid, advance_viewer);
    advance_canonical_max_health_like_cpp(&stale_canonical, advance_guid, 140);
    let before =
        session_loot_lifecycle_state_like_cpp(&stale_manager, &stale_canonical, advance_guid);

    let mut refused_runs = 0_usize;
    assert!(
        stale_session
            .core
            .mutate_world_creature_if_fully_looted_observation_like_cpp(
                advance_guid,
                &stale_lifecycle.authority,
                stale_lifecycle.viewed_object_generation,
                stale_lifecycle.viewed_lifecycle_revision,
                |_| {
                    refused_runs += 1;
                },
            )
            .is_none(),
        "the stale viewed mutation is refused"
    );
    assert!(
        stale_session
            .core
            .mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
                advance_guid,
                &stale_lifecycle.authority,
                stale_lifecycle.unviewed_object_generation,
                stale_lifecycle.unviewed_lifecycle_revision,
                |_| {
                    refused_runs += 1;
                },
            )
            .is_none(),
        "the stale detached mutation is refused"
    );
    assert_eq!(
        refused_runs, 0,
        "ownership is refused before either callback is invoked"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&stale_manager, &stale_canonical, advance_guid),
        before,
        "the refused mutations wrote neither the legacy nor the canonical store"
    );
    assert!(
        legacy_creature_like_cpp(&stale_manager, advance_guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(
                canonical_creature_like_cpp(&stale_canonical, advance_guid)
                    .expect("canonical incarnation")
                    .loot_authority_like_cpp()
            ),
        "the refusals keep the incarnation's single loot allocation"
    );
}

#[test]
fn guarded_release_refuses_unavailable_ownership_like_cpp() {
    let guid = test_creature_guid(92_216);
    let viewer = ObjectGuid::create_player(1, 92_217);
    let (mut session, manager, canonical, lifecycle) = release_fixture_like_cpp(guid, viewer);

    // A canonical map instance without the incarnation for this GUID.
    remove_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, guid);
    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a missing canonical object is unavailable ownership"
    );
    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a missing canonical object is unavailable ownership for the detached variant too"
    );
    assert!(
        has_lootable_flag_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        "the refused release left the legacy representation untouched"
    );

    // A session with no legacy representation at all.
    let (mut session, _, _) = make_session();
    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a missing legacy representation is unavailable ownership"
    );
    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a missing legacy representation refuses the detached variant too"
    );

    // A bound canonical manager whose map instance does not exist.
    let guid = test_creature_guid(92_225);
    let viewer = ObjectGuid::create_player(1, 92_226);
    let (mut session, manager, _, lifecycle) = release_fixture_like_cpp(guid, viewer);
    let empty_canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&empty_canonical));
    let before = loot_lifecycle_state_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    assert!(
        viewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a missing canonical map instance is unavailable ownership"
    );
    assert!(
        unviewed_release_like_cpp(&mut session, guid, &lifecycle).is_none(),
        "a missing canonical map instance refuses the detached variant too"
    );
    assert_eq!(
        loot_lifecycle_state_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        before,
        "every refusal above left the legacy representation at its pre-operation state"
    );
    assert_eq!(
        lifecycle.authority.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active,
        "no refusal retired the response authority"
    );
}
