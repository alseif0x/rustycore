//! F6-7 R1a regressions: one loot authority per creature incarnation.
//!
//! Reviewer signature §5.3.1 R1 and repair order §5.3.2: the canonical map's
//! `wow_entities::Creature` owns the single allocatable `wow_loot` authority for
//! an object incarnation; the legacy `map_manager::WorldCreature` and the
//! session's recorded loot-view authorities are aliases to that same allocation.
//! Admission must select that authority *before* a legacy alias is published,
//! and must never reconcile two independently allocated claimable pools.
//!
//! Incarnation identity is `(MapKey, ObjectGuid, health-timeline allocation)`
//! (`HealthStateRevisionAuthorityLikeCpp::shares_storage_like_cpp`), not the
//! GUID alone: a recreated object gets a fresh allocation, and an old lease or
//! view authority cannot serve it.
//!
//! These tests drive the production registration chain
//! (`WorldSession::register_world_creature` →
//! `WorldEntitiesState::register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp`)
//! and the production admission entry point
//! (`insert_canonical_creature_map_object_on_map_like_cpp`) that the lifecycle
//! tick uses, not a test double.
//!
//! Boundary of the admission decision: `Refused` (the canonical owner exists and
//! the candidate would be a second claimable pool) stops legacy publication. A
//! session that has a canonical manager but no canonical map instance for the
//! registration key has no canonical incarnation to decide against, so its
//! previous legacy publication path is preserved; the registration caller always
//! builds an unused pristine candidate, so the refusal is exercised at the
//! admission entry point (test 3) and the caller's abort is unreachable today.

use std::sync::{Arc, Mutex};

use super::{
    authoritative_test_loot_like_cpp, make_session_with_send_capacity,
    register_test_creature_like_cpp, represented_loot_object_guid_like_cpp, test_creature,
    test_creature_guid,
};
use crate::session::{
    SharedCanonicalMapManager, WorldSession, insert_canonical_creature_map_object_on_map_like_cpp,
};
use wow_core::{ObjectGuid, Position};
use wow_entities::Creature;
use wow_loot::{LootClaimCommitError, OwnedLootAuthority, OwnedLootAuthorityLifecycle};
use wow_map::MapManager;

const TEST_MAP_ID: u16 = 571;

/// A session whose canonical map manager is configured, with (or without) the
/// canonical map instance created. `register_test_creature_like_cpp` supplies
/// the legacy map manager, so both stores are configured.
fn canonical_admission_session_like_cpp(
    create_map: bool,
) -> (WorldSession, SharedCanonicalMapManager) {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let canonical: SharedCanonicalMapManager = Arc::new(Mutex::new(MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_map_position_like_cpp(TEST_MAP_ID, Position::ZERO);
    if create_map {
        canonical
            .lock()
            .expect("canonical map manager")
            .create_world_map(u32::from(TEST_MAP_ID), 0);
    }
    (session, canonical)
}

fn canonical_authority_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> Option<OwnedLootAuthority> {
    let manager = canonical.lock().ok()?;
    manager
        .find_map(u32::from(TEST_MAP_ID), 0)?
        .map()
        .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
}

fn legacy_authority_like_cpp(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<OwnedLootAuthority> {
    let manager = session.core.map_manager.as_ref()?.read().ok()?;
    manager
        .find_creature(TEST_MAP_ID, 0, guid)
        .map(|world_creature| world_creature.creature.loot_authority_like_cpp().clone())
}

/// The minimum an independently constructed admission candidate needs, matching
/// what the production registration sets before admission. `instance_id` must
/// match the canonical map instance the candidate is admitted to
/// (`Map::validate_map_object`).
fn admission_candidate_like_cpp(guid: ObjectGuid, instance_id: u32) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    let _ = creature
        .unit_mut()
        .world_mut()
        .set_map(u32::from(TEST_MAP_ID), instance_id);
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().set_level(1);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature
}

/// An independently allocated, already claimable candidate for the same GUID —
/// the shape the lifecycle tick's legacy store produces when a second pool was
/// built before the canonical object existed.
fn competing_active_candidate_like_cpp(guid: ObjectGuid, player_guid: ObjectGuid) -> Creature {
    let mut candidate = admission_candidate_like_cpp(guid, 0);
    let mut loot = authoritative_test_loot_like_cpp(30, false);
    loot.loot_guid = represented_loot_object_guid_like_cpp(guid);
    loot.allowed_looters = vec![player_guid];
    candidate.initialize_shared_loot_authority_like_cpp(loot);
    candidate
}

fn money_loot_like_cpp(
    guid: ObjectGuid,
    coins: u32,
    player_guid: ObjectGuid,
) -> wow_packet::packets::loot::CreatureLoot {
    let mut loot = authoritative_test_loot_like_cpp(coins, false);
    loot.loot_guid = represented_loot_object_guid_like_cpp(guid);
    loot.allowed_looters = vec![player_guid];
    loot
}

fn player_guid_like_cpp(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

#[test]
fn fresh_admission_shares_one_loot_authority_like_cpp() {
    let (mut session, canonical) = canonical_admission_session_like_cpp(true);
    let guid = test_creature_guid(62_101);
    let player = player_guid_like_cpp(62_101);
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));

    let canonical_authority = canonical_authority_like_cpp(&canonical, guid)
        .expect("fresh admission installs the object");
    let legacy_authority =
        legacy_authority_like_cpp(&session, guid).expect("the legacy alias is published");
    assert!(
        canonical_authority.shares_storage_like_cpp(&legacy_authority),
        "the legacy store must alias the canonical allocation, not hold a clone of it"
    );
    assert_eq!(
        canonical_authority.stamp_like_cpp(),
        legacy_authority.stamp_like_cpp(),
        "both stores must describe the same loot lifetime"
    );

    // Identity, not merely equal contents: a loot generation opened through the
    // canonical allocation is the one the legacy alias reads.
    canonical_authority.initialize_shared_like_cpp(money_loot_like_cpp(guid, 25, player));
    let observed = legacy_authority
        .shared_snapshot_like_cpp()
        .expect("the shared pool is openable through the alias");
    assert_eq!(
        observed.loot.coins, 25,
        "the alias must read the canonical pool, which a separate allocation could not show"
    );
}

#[test]
fn duplicate_pristine_admission_preserves_canonical_identity_like_cpp() {
    let (mut session, canonical) = canonical_admission_session_like_cpp(true);
    let guid = test_creature_guid(62_102);
    let player = player_guid_like_cpp(62_102);
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));
    let first = canonical_authority_like_cpp(&canonical, guid).expect("fresh admission");
    first.initialize_shared_like_cpp(money_loot_like_cpp(guid, 41, player));
    let stamp_before = first.stamp_like_cpp();
    let generation_before = first.generation_like_cpp();

    // The production registration path builds an unused pristine candidate for
    // the same incarnation; admission must return the existing authority and
    // mutate neither store.
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));

    let after =
        canonical_authority_like_cpp(&canonical, guid).expect("the object is still admitted");
    assert!(
        after.shares_storage_like_cpp(&first),
        "a duplicate pristine admission must keep the canonical allocation"
    );
    assert_eq!(
        after.stamp_like_cpp(),
        stamp_before,
        "the canonical lifetime must not be rebound or quarantined"
    );
    assert_eq!(after.generation_like_cpp(), generation_before);
    assert_eq!(
        after.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert_eq!(
        after
            .shared_snapshot_like_cpp()
            .expect("the live pool survives")
            .loot
            .coins,
        41,
        "the live canonical pool is untouched by the duplicate admission"
    );
    let legacy = legacy_authority_like_cpp(&session, guid).expect("the legacy alias still exists");
    assert!(legacy.shares_storage_like_cpp(&after));
}

#[test]
fn duplicate_active_admission_changes_neither_pool_like_cpp() {
    let (mut session, canonical) = canonical_admission_session_like_cpp(true);
    let guid = test_creature_guid(62_103);
    let player = player_guid_like_cpp(62_103);
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));
    let canonical_authority =
        canonical_authority_like_cpp(&canonical, guid).expect("fresh admission");
    canonical_authority.initialize_shared_like_cpp(money_loot_like_cpp(guid, 50, player));
    let canonical_stamp_before = canonical_authority.stamp_like_cpp();

    // A second, independently allocated pool for the same GUID: admitting it
    // must be refused outright instead of reconciling the two live pools.
    let candidate = competing_active_candidate_like_cpp(guid, player);
    let candidate_authority = candidate.loot_authority_like_cpp().clone();
    let candidate_stamp_before = candidate_authority.stamp_like_cpp();
    assert_eq!(
        candidate_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert!(!candidate_authority.shares_storage_like_cpp(&canonical_authority));

    let admitted = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        u32::from(TEST_MAP_ID),
        0,
        candidate,
    );
    assert!(
        admitted.is_none(),
        "a competing active pool must be refused, never reconciled into the canonical object"
    );

    let canonical_after =
        canonical_authority_like_cpp(&canonical, guid).expect("the canonical object is untouched");
    assert!(canonical_after.shares_storage_like_cpp(&canonical_authority));
    assert_eq!(
        canonical_after.stamp_like_cpp(),
        canonical_stamp_before,
        "the canonical pool must not be quarantined, rebound or merged"
    );
    assert_eq!(
        canonical_after
            .shared_snapshot_like_cpp()
            .expect("the canonical pool is still openable")
            .loot
            .coins,
        50
    );
    assert_eq!(
        candidate_authority.stamp_like_cpp(),
        candidate_stamp_before,
        "the refused candidate keeps its own untouched allocation"
    );
    assert_eq!(
        candidate_authority
            .shared_snapshot_like_cpp()
            .expect("the refused candidate still holds its own pool")
            .loot
            .coins,
        30
    );
}

#[tokio::test]
async fn separate_incarnations_stay_isolated_like_cpp() {
    let (mut session, canonical) = canonical_admission_session_like_cpp(true);
    let first_guid = test_creature_guid(62_104);
    let second_guid = test_creature_guid(62_105);
    let player = player_guid_like_cpp(62_104);
    register_test_creature_like_cpp(&mut session, test_creature(first_guid, false));
    register_test_creature_like_cpp(&mut session, test_creature(second_guid, false));

    let first = canonical_authority_like_cpp(&canonical, first_guid).expect("first incarnation");
    let second = canonical_authority_like_cpp(&canonical, second_guid).expect("second incarnation");
    assert!(
        !first.shares_storage_like_cpp(&second),
        "two objects must never share one claimable pool"
    );
    first.initialize_shared_like_cpp(money_loot_like_cpp(first_guid, 10, player));
    second.initialize_shared_like_cpp(money_loot_like_cpp(second_guid, 20, player));

    let first_lease = first
        .reserve_money_like_cpp(player)
        .await
        .expect("the first object's money is claimable");
    let second_lease = second
        .reserve_money_like_cpp(player)
        .await
        .expect("a claim on the first object must not reserve the second object's money");
    assert!(first_lease.commit_like_cpp().expect("first commit"));
    assert!(second_lease.commit_like_cpp().expect("second commit"));
    assert_eq!(
        first.shared_snapshot_like_cpp().expect("first").loot.coins,
        0
    );
    assert_eq!(
        second
            .shared_snapshot_like_cpp()
            .expect("second")
            .loot
            .coins,
        0
    );

    // The same GUID on another map instance is a different incarnation, so it
    // gets its own allocation rather than the instance-0 authority.
    canonical
        .lock()
        .expect("canonical map manager")
        .create_world_map(u32::from(TEST_MAP_ID), 1);
    let instance_outcome = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        u32::from(TEST_MAP_ID),
        1,
        admission_candidate_like_cpp(first_guid, 1),
    )
    .expect("a distinct map key is a distinct incarnation");
    assert!(
        !instance_outcome
            .loot_authority
            .shares_storage_like_cpp(&first),
        "the same GUID on another instance must not reuse the instance-0 allocation"
    );
}

#[tokio::test]
async fn recreation_cannot_reuse_old_views_or_leases_like_cpp() {
    let (mut session, canonical) = canonical_admission_session_like_cpp(true);
    let guid = test_creature_guid(62_106);
    let player = player_guid_like_cpp(62_106);
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));
    let first = canonical_authority_like_cpp(&canonical, guid).expect("fresh admission");
    first.initialize_shared_like_cpp(money_loot_like_cpp(guid, 33, player));
    session
        .loot
        .insert_active_loot_view_authority_for_test_like_cpp(guid, first.clone());
    let old_view = session
        .loot
        .active_loot_view_authority_like_cpp(guid)
        .cloned()
        .expect("the fixture records the live view authority");
    let old_lease = first
        .reserve_money_like_cpp(player)
        .await
        .expect("the live object is claimable");

    // C++ destroys the Creature and its Loot with the object.
    assert!(
        session.remove_world_creature(guid).is_some(),
        "the fixture removes the object from the legacy store"
    );
    assert!(
        canonical_authority_like_cpp(&canonical, guid).is_none(),
        "the canonical object is destroyed with it"
    );
    assert_ne!(
        first.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active,
        "destruction ends the first incarnation's loot lifetime"
    );

    register_test_creature_like_cpp(&mut session, test_creature(guid, false));
    let second =
        canonical_authority_like_cpp(&canonical, guid).expect("the recreation is admitted");
    assert!(
        !second.shares_storage_like_cpp(&first),
        "recreation must allocate a new authority, not reuse the destroyed one"
    );
    assert!(
        !old_view.shares_storage_like_cpp(&second),
        "the recorded view authority cannot serve the recreated incarnation"
    );
    assert!(
        old_view.snapshot_for_player_like_cpp(player).is_none(),
        "the old view authority is closed, so no stale view can reopen it"
    );
    assert!(
        second.snapshot_for_player_like_cpp(player).is_none(),
        "the recreated object has no loot lifetime of its own yet"
    );
    assert_eq!(
        old_lease.commit_with_snapshot_like_cpp().unwrap_err(),
        LootClaimCommitError::StaleGeneration,
        "a lease taken on the destroyed incarnation cannot commit against the recreation"
    );
}

#[tokio::test]
async fn alias_admission_keeps_a_live_canonical_claim_lease_like_cpp() {
    let (mut session, canonical) = canonical_admission_session_like_cpp(true);
    let guid = test_creature_guid(62_107);
    let player = player_guid_like_cpp(62_107);
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));
    let canonical_authority =
        canonical_authority_like_cpp(&canonical, guid).expect("fresh admission");
    canonical_authority.initialize_shared_like_cpp(money_loot_like_cpp(guid, 60, player));
    let lease = canonical_authority
        .reserve_money_like_cpp(player)
        .await
        .expect("the canonical pool is claimable");
    assert!(lease.shares_authority_like_cpp(&canonical_authority));

    // Admit the duplicate/alias candidate while that lease is live. A rebind
    // that detached the live canonical allocation would strand the lease.
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));

    let after =
        canonical_authority_like_cpp(&canonical, guid).expect("the object is still admitted");
    assert!(
        after.shares_storage_like_cpp(&canonical_authority),
        "alias admission must not detach or replace the live canonical allocation"
    );
    assert_eq!(
        after.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    let legacy = legacy_authority_like_cpp(&session, guid).expect("the legacy alias still exists");
    assert!(legacy.shares_storage_like_cpp(&after));

    let (first_commit, snapshot) = lease
        .commit_with_snapshot_like_cpp()
        .expect("the live canonical claim must still complete");
    assert!(first_commit, "the claim completes exactly once");
    assert!(snapshot.is_some(), "the commit reports the committed pool");
    let (second_commit, _) = lease
        .commit_with_snapshot_like_cpp()
        .expect("a repeated commit is a no-op, not a failure");
    assert!(!second_commit, "the same lease must not commit twice");
    assert_eq!(
        after
            .shared_snapshot_like_cpp()
            .expect("the committed pool")
            .loot
            .coins,
        0
    );
}

#[test]
fn canonical_map_absence_keeps_the_previous_legacy_publication_like_cpp() {
    // Canonical manager configured, but no canonical map instance exists for the
    // registration key yet. That is the deliberate shape of the session fixtures
    // and of grid loading racing legacy registration: there is no canonical
    // incarnation to decide against, so no competing pool can be created and the
    // previous legacy publication path is preserved.
    let (mut session, canonical) = canonical_admission_session_like_cpp(false);
    let guid = test_creature_guid(62_108);
    register_test_creature_like_cpp(&mut session, test_creature(guid, false));
    assert!(
        legacy_authority_like_cpp(&session, guid).is_some(),
        "a session whose canonical map instance does not exist yet keeps the legacy path"
    );
    assert!(
        canonical_authority_like_cpp(&canonical, guid).is_none(),
        "no canonical incarnation was admitted"
    );

    // The legacy-only configuration keeps its previous behaviour too: with no
    // canonical manager at all, registration still publishes the legacy object.
    let (mut legacy_only, _send_rx) = make_session_with_send_capacity(4);
    legacy_only.set_player_map_position_like_cpp(TEST_MAP_ID, Position::ZERO);
    let legacy_guid = test_creature_guid(62_109);
    register_test_creature_like_cpp(&mut legacy_only, test_creature(legacy_guid, false));
    assert!(
        legacy_authority_like_cpp(&legacy_only, legacy_guid).is_some(),
        "legacy-only configuration still publishes its object"
    );
}
