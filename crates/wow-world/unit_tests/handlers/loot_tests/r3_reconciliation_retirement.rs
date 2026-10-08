//! F6-7 R3 regressions: the dual-store reconciliation is retired.
//!
//! Reviewer signature §5.3.1 R3 and repair order §5.3.2: "replace dual-store
//! reconciliation and the arbitrary eight-round limit with an operation ordered
//! by the single authority's execution owner, preserving incarnation checks".
//!
//! R2 already made the lookup resolve through one designated owner; R3 retires
//! what was left of the machinery: the reconciliation helper and its re-exports,
//! the eight-round retry loop with its exhaustion exit, the reconciliation
//! compare-and-exchange, the repeated per-phase map-key rechecks and the
//! mirror-conflict tombstone producer.
//!
//! The first test is the behavioural proof: two **distinct** allocations are
//! supplied, the lookup answers with the designated incarnation's own
//! allocation, and neither allocation is rebound, converged or tombstoned. The
//! second test is the source contract: the retired symbols cannot come back
//! unnoticed.

use super::{
    CreaturePublicationObservablesLikeCpp, LootAllocationObservablesLikeCpp,
    SingleIncarnationLootFixtureLikeCpp, allowed_creature_loot_like_cpp, attach_canonical_creature,
    authoritative_test_loot_like_cpp, creature_publication_observables_like_cpp,
    expect_found_like_cpp, loot_allocation_observables_like_cpp,
    make_canonical_creature_for_session, make_session_with_send_capacity,
    register_test_creature_like_cpp, represented_loot_object_guid_like_cpp,
    single_incarnation_loot_fixture_like_cpp, test_creature, test_creature_guid,
};
use crate::session::{
    sync_admitted_creature_representation_on_map_like_cpp,
    sync_canonical_creature_entity_on_map_like_cpp,
};
use wow_core::{ObjectGuid, Position};
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityLifecycle};
use wow_map::MapKey;
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;

const TEST_MAP_ID: u16 = 571;

fn map_key_like_cpp() -> MapKey {
    MapKey::new(u32::from(TEST_MAP_ID), 0)
}

#[test]
fn lookup_does_not_reconcile_distinct_mirrors() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 64_001);
    let owner_guid = test_creature_guid(64_002);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(TEST_MAP_ID, Position::ZERO);

    // The legacy representation is published while no canonical incarnation
    // exists and is given its own claimable pool...
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    let legacy = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
        .expect("the legacy representation owns an allocation");
    let mut legacy_loot = authoritative_test_loot_like_cpp(41, false);
    legacy_loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    legacy_loot.allowed_looters = vec![player_guid];
    assert!(
        legacy.initialize_shared_like_cpp(legacy_loot).installed(),
        "the legacy allocation must hold a live pool of its own"
    );

    // ...and a canonical incarnation appears afterwards with a distinct,
    // independently allocated pool: exactly the state the retired
    // reconciliation would have tried to converge with a compare-and-exchange
    // and, on conflict, an attached retired tombstone.
    let canonical_creature = make_canonical_creature_for_session(&session, owner_guid);
    attach_canonical_creature(&mut session, canonical_creature);
    let canonical = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
        .expect("the canonical incarnation owns its own allocation");
    let mut canonical_loot = authoritative_test_loot_like_cpp(7, true);
    canonical_loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    canonical_loot.allowed_looters = vec![player_guid];
    canonical_loot.items[0].allowed_looters = vec![player_guid];
    assert!(
        canonical
            .initialize_shared_like_cpp(canonical_loot)
            .installed(),
        "the canonical allocation must hold a live pool of its own"
    );
    assert!(
        !canonical.shares_storage_like_cpp(&legacy),
        "the fixture must supply two distinct allocations"
    );
    let canonical_stamp = canonical.stamp_like_cpp();
    let legacy_stamp = legacy.stamp_like_cpp();

    let outcome = session.represented_owned_loot_authority_outcome_like_cpp(owner_guid);
    let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(found) = &outcome else {
        panic!("the canonical incarnation is the designated owner; got {outcome:?}");
    };
    assert!(
        found.shares_storage_like_cpp(&canonical),
        "the designated authority must be the canonical incarnation's own allocation"
    );
    assert!(
        !found.shares_storage_like_cpp(&legacy),
        "the surviving mirror must not confer or receive authority"
    );

    // The lookup is a read: both allocations and both stamps survive it, and
    // the mirror is not rebound into an alias of the designated allocation.
    let canonical_after = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
        .expect("the canonical allocation is still published");
    let legacy_after = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key_like_cpp())
        .expect("the legacy allocation is still published");
    assert!(
        canonical_after.shares_storage_like_cpp(&canonical)
            && legacy_after.shares_storage_like_cpp(&legacy),
        "neither allocation may be replaced by the lookup"
    );
    assert_eq!(
        canonical_after.stamp_like_cpp(),
        canonical_stamp,
        "the designated allocation's stamp must be unchanged"
    );
    assert_eq!(
        legacy_after.stamp_like_cpp(),
        legacy_stamp,
        "the mirror's stamp must be unchanged: no convergence, no tombstone"
    );
    assert!(
        !legacy_after.shares_storage_like_cpp(&canonical_after),
        "the two allocations must still be distinct after the lookup"
    );
    assert_eq!(
        legacy_after
            .shared_snapshot_like_cpp()
            .map(|snapshot| snapshot.loot.coins),
        Some(41),
        "the mirror keeps its own pool untouched"
    );
    assert_eq!(
        canonical_after
            .shared_snapshot_like_cpp()
            .map(|snapshot| snapshot.loot.coins),
        Some(7),
        "the designated allocation keeps its own pool untouched"
    );
}

/// The retired reconciliation surface, checked against the sources that owned
/// it. The repo pins source contracts by including the file, so this test does
/// the same for each surviving owner and additionally proves that the
/// `wow-world-loot` per-phase recheck module is deleted rather than merely
/// unused.
#[test]
fn dual_store_reconciliation_surface_is_retired_like_cpp() {
    const RETIRED: &[&str] = &[
        "reconcile_creature_loot_authority_mirrors_like_cpp",
        "loot_reconciliation_map_key_still_valid_like_cpp",
        "new_retired_tombstone_like_cpp",
        "for _ in 0..8",
    ];
    let sources: &[(&str, &str)] = &[
        (
            "core loot authority",
            include_str!(
                "../../../../wow-world-core/src/session/canonical_access/loot_release/authority.rs"
            ),
        ),
        (
            "core creature adapter",
            include_str!("../../../../wow-world-core/src/session/creature_canonical_adapter.rs"),
        ),
        (
            "core session owner",
            include_str!("../../../../wow-world-core/src/session/mod.rs"),
        ),
        (
            "loot authority primitives",
            include_str!("../../../../wow-loot/src/authority/ops_1.rs"),
        ),
        (
            "world session owner",
            include_str!("../../../src/session/mod.rs"),
        ),
        (
            "world creature adapter facade",
            include_str!("../../../src/session/creature_canonical_adapter.rs"),
        ),
        (
            "world loot operations",
            include_str!("../../../src/session/loot/operations.rs"),
        ),
        (
            "world loot authority access",
            include_str!("../../../../wow-world-loot/src/state/authority_access.rs"),
        ),
    ];
    for (name, source) in sources {
        for symbol in RETIRED {
            assert!(
                !source.contains(symbol),
                "{name} must not contain the retired reconciliation symbol `{symbol}`"
            );
        }
    }

    // The reconciliation rebind surface is retired with its delegations: the
    // World-layer loot operations keep no rebind at all, and the wow-world-loot
    // authority access keeps only the single-store reads.
    let world_loot_operations = sources
        .iter()
        .find(|(name, _)| *name == "world loot operations")
        .expect("the world loot operations source is listed")
        .1;
    assert!(
        !world_loot_operations.contains("rebind"),
        "the World loot operations must not keep a reconciliation rebind"
    );
    let world_loot_access = sources
        .iter()
        .find(|(name, _)| *name == "world loot authority access")
        .expect("the wow-world-loot authority access source is listed")
        .1;
    assert!(
        !world_loot_access.contains("fn rebind"),
        "wow-world-loot must keep only the single-store authority reads"
    );

    let reconciliation_module = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../wow-world-loot/src/state/reconciliation.rs");
    assert!(
        !reconciliation_module.exists(),
        "the delegated per-phase map-key recheck module must be deleted, not left unused"
    );
    assert!(
        !include_str!("../../../../wow-world-loot/src/state.rs").contains("mod reconciliation;"),
        "the retired recheck module must not be declared"
    );
}

/// F6-7 R5 closure. R3 retired the mirror-conflict tombstone constructor and its
/// propagation, so this regression guards the **rule** that constructor used to
/// violate rather than the deleted surface: a foreign loot alias refused by the
/// production admission/application path must leave both allocations exactly as
/// they were — backing-`Arc` identity, stamp, lifecycle and complete pool
/// contents — and must leave neither one quarantined, retired, detached,
/// rebound or merged. Each refusal must be reported as a refusal and must
/// publish nothing. The designated owner then keeps its own allocation and loots
/// it exactly once.
#[tokio::test]
async fn foreign_loot_alias_refusal_does_not_quarantine_owner_like_cpp() {
    /// Every refusal must leave both allocations and both representations
    /// exactly as they were.
    fn assert_refusals_left_everything_untouched_like_cpp(
        fixture: &SingleIncarnationLootFixtureLikeCpp,
        owner_before: &LootAllocationObservablesLikeCpp,
        competing: &OwnedLootAuthority,
        competing_before: &LootAllocationObservablesLikeCpp,
        canonical_before: &CreaturePublicationObservablesLikeCpp,
        legacy_before: &CreaturePublicationObservablesLikeCpp,
        packets_before: usize,
        refusal: &str,
    ) {
        assert_eq!(
            loot_allocation_observables_like_cpp(&fixture.owner_allocation),
            *owner_before,
            "{refusal}: the owner's allocation must not be quarantined, retired, detached, rebound or merged"
        );
        assert_eq!(
            loot_allocation_observables_like_cpp(competing),
            *competing_before,
            "{refusal}: the refused allocation must keep its own identity, stamp, lifecycle and pool"
        );
        assert!(
            fixture
                .canonical_allocation_like_cpp()
                .shares_storage_like_cpp(&fixture.owner_allocation),
            "{refusal}: the canonical store must still hold the incarnation's own allocation"
        );
        assert!(
            fixture
                .legacy_allocation_like_cpp()
                .shares_storage_like_cpp(&fixture.owner_allocation),
            "{refusal}: the legacy alias must not be rebound onto the refused allocation"
        );
        assert!(
            !fixture
                .canonical_allocation_like_cpp()
                .shares_storage_like_cpp(competing)
                && !fixture
                    .legacy_allocation_like_cpp()
                    .shares_storage_like_cpp(competing),
            "{refusal}: neither store may adopt or merge the refused allocation"
        );
        assert!(
            !fixture.owner_allocation.shares_storage_like_cpp(competing),
            "{refusal}: the two allocations must still be distinct"
        );
        assert_eq!(
            creature_publication_observables_like_cpp(&fixture.canonical_creature_like_cpp()),
            *canonical_before,
            "{refusal}: the refused operation must publish no canonical entity state"
        );
        assert_eq!(
            creature_publication_observables_like_cpp(
                &fixture.transported_legacy_representation_like_cpp()
            ),
            *legacy_before,
            "{refusal}: the refused operation must publish no legacy entity state"
        );
        assert_eq!(
            fixture.send_rx.len(),
            packets_before,
            "{refusal}: the refused operation must produce no packet"
        );
    }

    // ---- one incarnation, one live pool, in both stores.
    let mut fixture = single_incarnation_loot_fixture_like_cpp(64_100, 57);
    assert_eq!(
        u32::from(TEST_MAP_ID),
        fixture.map_key_like_cpp().map_id,
        "the fixture must be registered at the key this module reads"
    );
    assert_eq!(
        fixture.owner_allocation.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active,
        "the fixture's own pool is live"
    );
    let owner_before = loot_allocation_observables_like_cpp(&fixture.owner_allocation);

    // Another live, already used allocation for the same object: exactly what a
    // foreign alias carries when it competes for the object's loot authority.
    let competing = OwnedLootAuthority::new();
    assert!(
        competing
            .initialize_shared_like_cpp(allowed_creature_loot_like_cpp(
                fixture.owner_guid,
                91,
                fixture.player_guid,
            ))
            .installed(),
        "the competing allocation must hold a live pool of its own"
    );
    assert_eq!(
        competing.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active,
        "a competing used allocation is Active, not pristine"
    );
    assert!(
        !competing.shares_storage_like_cpp(&fixture.owner_allocation),
        "the fixture must supply two independently allocated pools"
    );
    let competing_before = loot_allocation_observables_like_cpp(&competing);
    let canonical_before =
        creature_publication_observables_like_cpp(&fixture.canonical_creature_like_cpp());
    let legacy_before = creature_publication_observables_like_cpp(
        &fixture.transported_legacy_representation_like_cpp(),
    );
    let packets_before = fixture.send_rx.len();

    // ---- refusal 1: a competing used allocation offered through the production
    // admission/application path. The representation is the incarnation's own
    // clone (the shape both production mirror sites transport), so it shares the
    // incarnation's health timeline and revision: only the allocation can decide
    // this refusal.
    let mut transported = fixture.transported_legacy_representation_like_cpp();
    transported.adopt_loot_authority_for_snapshot_like_cpp(competing.clone());
    assert!(
        transported
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &fixture
                    .canonical_creature_like_cpp()
                    .unit()
                    .health_state_revision_authority_like_cpp()
            ),
        "the fixture must offer the incarnation's own timeline"
    );
    assert_eq!(
        transported.unit().health_state_revision_like_cpp(),
        fixture
            .canonical_creature_like_cpp()
            .unit()
            .health_state_revision_like_cpp(),
        "the fixture must offer the incarnation's own revision"
    );
    let expected = transported.loot_authority_like_cpp().clone();
    let expected_stamp = expected.stamp_like_cpp();
    assert!(
        !sync_admitted_creature_representation_on_map_like_cpp(
            &fixture.canonical,
            Some(&fixture.legacy),
            TEST_MAP_ID,
            0,
            transported,
            &expected,
            expected_stamp,
        ),
        "a competing used allocation must be reported as a refusal, not as an applied snapshot"
    );
    assert_refusals_left_everything_untouched_like_cpp(
        &fixture,
        &owner_before,
        &competing,
        &competing_before,
        &canonical_before,
        &legacy_before,
        packets_before,
        "the competing used allocation refusal",
    );

    // ---- refusal 2: the same competing allocation offered directly to the
    // application half of the path, which owns the R1b guard that refuses a
    // second claimable pool before any authority is selected.
    let mut direct = fixture.transported_legacy_representation_like_cpp();
    direct.adopt_loot_authority_for_snapshot_like_cpp(competing.clone());
    assert!(
        sync_canonical_creature_entity_on_map_like_cpp(
            &fixture.canonical,
            u32::from(TEST_MAP_ID),
            0,
            direct,
        )
        .is_none(),
        "the application's own guard must refuse the competing allocation with nothing selected and nothing written"
    );
    assert_refusals_left_everything_untouched_like_cpp(
        &fixture,
        &owner_before,
        &competing,
        &competing_before,
        &canonical_before,
        &legacy_before,
        packets_before,
        "the application-level competing allocation refusal",
    );

    // ---- refusal 3: a stale incarnation/revision representation on the same
    // fixture. The incarnation's represented health tuple leaves and returns to
    // the same values while its revision advances, so the representation the
    // legacy store still holds agrees on the whole tuple and carries the
    // incarnation's own allocation: only the revision can decide this refusal.
    let revision_before_aba = fixture
        .canonical_creature_like_cpp()
        .unit()
        .health_state_revision_like_cpp();
    fixture.with_canonical_incarnation_mut_like_cpp(|creature| {
        creature.unit_mut().set_max_health(60);
        creature.unit_mut().set_max_health(100);
    });
    let current = fixture.canonical_creature_like_cpp();
    assert!(
        revision_before_aba < current.unit().health_state_revision_like_cpp(),
        "the fixture's ABA transition must advance the incarnation's revision"
    );
    let stale_transported = fixture.transported_legacy_representation_like_cpp();
    assert_eq!(
        stale_transported.unit().health_state_revision_like_cpp(),
        revision_before_aba,
        "the transported representation must replay the older revision"
    );
    assert_eq!(
        stale_transported.unit().data().health,
        current.unit().data().health,
        "the ABA replay agrees with the incarnation's current health tuple"
    );
    assert_eq!(
        stale_transported.unit().data().max_health,
        current.unit().data().max_health
    );
    assert_eq!(
        stale_transported.unit().death_state(),
        current.unit().death_state()
    );
    assert!(
        stale_transported
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &current.unit().health_state_revision_authority_like_cpp()
            ),
        "the stale claim belongs to the incarnation's own timeline"
    );
    assert!(
        stale_transported
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&fixture.owner_allocation),
        "the stale claim carries the incarnation's own allocation"
    );
    // The refusal baselines are the state at the moment of the refusal: the ABA
    // transition above is the fixture's own precondition, not the refused
    // operation.
    let stale_canonical_before =
        creature_publication_observables_like_cpp(&fixture.canonical_creature_like_cpp());
    let stale_legacy_before = creature_publication_observables_like_cpp(
        &fixture.transported_legacy_representation_like_cpp(),
    );
    let stale_packets_before = fixture.send_rx.len();
    let stale_expected = stale_transported.loot_authority_like_cpp().clone();
    let stale_expected_stamp = stale_expected.stamp_like_cpp();
    assert!(
        !sync_admitted_creature_representation_on_map_like_cpp(
            &fixture.canonical,
            Some(&fixture.legacy),
            TEST_MAP_ID,
            0,
            stale_transported,
            &stale_expected,
            stale_expected_stamp,
        ),
        "a stale ABA-replayed representation must be reported as a refusal, not as an applied snapshot"
    );
    assert_refusals_left_everything_untouched_like_cpp(
        &fixture,
        &owner_before,
        &competing,
        &competing_before,
        &stale_canonical_before,
        &stale_legacy_before,
        stale_packets_before,
        "the stale ABA replay refusal",
    );

    // ---- legitimate owner looting on the same fixture: the designated lookup
    // selects the owner's ORIGINAL allocation, never the refused one.
    let found = expect_found_like_cpp(
        fixture
            .session
            .represented_owned_loot_authority_outcome_like_cpp(fixture.owner_guid),
    );
    assert!(
        found.shares_storage_like_cpp(&fixture.owner_allocation),
        "the designated lookup must select the owner's original allocation"
    );
    assert!(
        !found.shares_storage_like_cpp(&competing),
        "the previously refused allocation must not be the one the owner's operation uses"
    );
    let lease = found
        .reserve_money_like_cpp(fixture.player_guid)
        .await
        .expect("the owner's live pool is claimable");
    assert!(
        lease.shares_authority_like_cpp(&fixture.owner_allocation),
        "the award must be reserved against the owner's original allocation"
    );
    assert!(
        !lease.shares_authority_like_cpp(&competing),
        "the award must not be reserved against the refused allocation"
    );
    let (first_award, awarded_snapshot) = lease
        .commit_with_snapshot_like_cpp()
        .expect("the owner's award must commit");
    assert!(first_award, "the award succeeds exactly once");
    let awarded_snapshot = awarded_snapshot.expect("the first award publishes the committed pool");
    assert_eq!(
        awarded_snapshot.loot.coins, 0,
        "the award consumes the owner's money exactly once"
    );
    let (second_award, replay_snapshot) = lease
        .commit_with_snapshot_like_cpp()
        .expect("a replayed award is a no-op, not a failure");
    assert!(
        !second_award,
        "replaying the award produces no second award"
    );
    assert!(
        replay_snapshot.is_none(),
        "a replayed award publishes nothing"
    );
    assert_eq!(
        found.shared_snapshot_like_cpp(),
        Some(awarded_snapshot),
        "the replay must leave the awarded pool exactly as the first award left it"
    );
    assert_eq!(
        loot_allocation_observables_like_cpp(&competing),
        competing_before,
        "the previously refused allocation remains untouched by the owner's award"
    );
    assert_eq!(
        fixture
            .canonical_allocation_like_cpp()
            .shared_snapshot_like_cpp()
            .expect("the owner's pool is still openable")
            .loot
            .coins,
        0,
        "the canonical store observes the single award"
    );

    // ---- positive control on the same production path: a same-incarnation
    // representation carrying the incarnation's own allocation and revision is
    // applied, so the refusals above were decided by the allocation and revision
    // clauses rather than by a missing map instance or incarnation.
    let fresh = fixture.canonical_creature_like_cpp();
    let fresh_expected = fresh.loot_authority_like_cpp().clone();
    let fresh_stamp = fresh_expected.stamp_like_cpp();
    assert!(
        sync_admitted_creature_representation_on_map_like_cpp(
            &fixture.canonical,
            Some(&fixture.legacy),
            TEST_MAP_ID,
            0,
            fresh,
            &fresh_expected,
            fresh_stamp,
        ),
        "an admitted same-incarnation representation is applied"
    );
    assert!(
        fixture
            .canonical_allocation_like_cpp()
            .shares_storage_like_cpp(&fixture.owner_allocation)
            && fixture
                .legacy_allocation_like_cpp()
                .shares_storage_like_cpp(&fixture.owner_allocation),
        "the applied representation keeps the owner's original allocation in both stores"
    );
    assert_eq!(
        fixture.owner_allocation.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active,
        "the applied representation leaves the owner's allocation active"
    );
}
