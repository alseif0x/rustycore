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
    attach_canonical_creature, authoritative_test_loot_like_cpp,
    make_canonical_creature_for_session, make_session_with_send_capacity,
    register_test_creature_like_cpp, represented_loot_object_guid_like_cpp, test_creature,
    test_creature_guid,
};
use wow_core::{ObjectGuid, Position};
use wow_loot::OwnedLootAuthority;
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
