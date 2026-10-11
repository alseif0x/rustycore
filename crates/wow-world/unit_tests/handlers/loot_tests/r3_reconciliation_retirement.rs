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
//! #1263 F6-8E deleted the second store itself, so the behavioural proofs that
//! compared two allocations have no subject any more. The source contract
//! remains, extended with the legacy → canonical synchronization and the
//! legacy loot probes F6-8E deleted.

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
        // #1263 F6-8E: the legacy → canonical synchronization and its admission
        // predicate, and the legacy-store loot probes.
        "sync_canonical_creature_entity_on_map_like_cpp",
        "sync_admitted_creature_representation_on_map_like_cpp",
        "apply_canonical_creature_entity_on_map_like_cpp",
        "creature_representation_admission_decision_like_cpp",
        "read_legacy_creature_loot_authority",
        "rebind_legacy_creature_loot_authority",
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
