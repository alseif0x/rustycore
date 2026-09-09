//! Bridge access scan regression scenarios, part 1 of 1.
//!
//! Moved out of the bridge_access.rs root under #660; every test is unchanged.

use super::*;

#[test]
fn innocent_bridge_shaped_function_name_is_not_evidence() {
    let baseline = inventory(
        r#"
            fn canonical_legacy_mirror_story(value: u32) -> u32 {
                value + 1
            }
        "#,
    )
    .expect("an innocent name parses");
    assert!(baseline.bridges.is_empty());
}

#[test]
fn canonical_dto_in_transparent_macro_does_not_invent_authority() {
    let baseline = inventory(
        r#"
            fn step(creature: &crate::map_manager::WorldCreature) {
                let _ = matches!(
                    creature.state(),
                    wow_entities::CreatureAiState::Idle
                );
            }
        "#,
    )
    .expect("a DTO enum in a transparent macro is inspectable");
    assert!(baseline.bridges.is_empty());
}

#[test]
fn explicit_canonical_creature_to_legacy_runtime_is_a_bridge() {
    let baseline = inventory(
        r#"
            use wow_entities::Creature as CanonicalCreature;

            fn translate(old: &wow_world::SharedMapManager) {
                let _ = old;
                let _ = CanonicalCreature::default();
                debug!(value = ?CanonicalCreature::default());
            }
        "#,
    )
    .expect("an explicitly imported canonical entity remains attributable");
    assert_eq!(baseline.bridges.len(), 1);
    assert_eq!(
        baseline.bridges[0].direction_markers,
        vec![BridgeDirection::UnresolvedDualSide]
    );
    assert!(
        baseline.bridges[0]
            .evidence
            .iter()
            .any(|marker| marker.kind == BridgeEvidenceKind::MacroArgument)
    );
}

#[test]
fn enum_variant_does_not_inherit_an_unrelated_import_with_the_same_name() {
    let baseline = inventory(
        r#"
            use wow_entities::Creature;
            use wow_map::SpawnObjectType;

            fn inspect(old: &wow_world::SharedMapManager) {
                let _ = old;
                let _ = SpawnObjectType::Creature;
                assert!(matches!(
                    SpawnObjectType::Creature,
                    SpawnObjectType::Creature
                ));
            }
        "#,
    )
    .expect("the enum variant and imported type have distinct provenance");
    assert!(baseline.bridges.is_empty());
}

#[test]
fn bridge_capable_globs_and_namespace_aliases_fail_closed() {
    let error = inventory(
        r#"
            use wow_entities::*;
            use crate::map_manager as old_runtime;
        "#,
    )
    .expect_err("namespace indirection can hide exact authority types");
    assert!(error.contains("bridge-capable glob import"), "{error}");
    assert!(error.contains("bridge-capable namespace import"), "{error}");
}

#[test]
fn unnamed_true_bridge_is_resolved_from_types_and_receivers() {
    let baseline = inventory(
        r#"
            use wow_map::MapManager as NewMaps;
            use wow_world::SharedMapManager as OldMaps;

            fn synchronize(old: &OldMaps, new: &mut NewMaps) {
                let old_guard = old.read().unwrap();
                new.create_map(old_guard.map_count());
            }
        "#,
    )
    .expect("structural bridge parses");
    assert_eq!(baseline.bridges.len(), 1);
    let bridge = &baseline.bridges[0];
    assert_eq!(
        bridge.direction_markers,
        vec![BridgeDirection::UnresolvedDualSide]
    );
    let sides: BTreeSet<_> = bridge.evidence.iter().map(|marker| marker.side).collect();
    assert_eq!(
        sides,
        BTreeSet::from([BridgeSide::Canonical, BridgeSide::Legacy])
    );
    assert!(bridge.fingerprint.len() < 1_024);
    assert!(
        bridge
            .evidence
            .iter()
            .all(|marker| marker.fingerprint.len() < 96)
    );
}

#[test]
fn same_count_body_swap_with_identical_symbols_fails_the_exact_comparator() {
    let expected = inventory(
        r#"
            fn reconcile(
                old: &wow_world::SharedMapManager,
                new: &mut wow_map::MapManager,
            ) {
                old.read().unwrap();
                new.find_map(1, 0);
                new.find_map(2, 0);
            }
        "#,
    )
    .unwrap();
    let actual = inventory(
        r#"
            fn reconcile(
                old: &wow_world::SharedMapManager,
                new: &mut wow_map::MapManager,
            ) {
                old.read().unwrap();
                new.find_map(2, 0);
                new.find_map(1, 0);
            }
        "#,
    )
    .unwrap();
    assert_eq!(expected.bridges.len(), actual.bridges.len());
    let error = compare_bridge_access_baseline(&expected, &actual)
        .expect_err("same-count semantic swap must fail");
    assert!(
        error.contains("untracked legacy/canonical bridge"),
        "{error}"
    );
    assert!(
        error.contains("obsolete legacy/canonical bridge baseline row"),
        "{error}"
    );
}

#[test]
fn cfg_identity_is_exact_and_test_only_bridges_are_not_dropped() {
    let expected = inventory(
        r#"
            #[cfg(feature = "old-bridge")]
            fn reconcile(
                old: &wow_world::SharedMapManager,
                new: &wow_map::MapManager,
            ) {}
        "#,
    )
    .unwrap();
    assert!(
        expected.bridges[0]
            .cfg
            .iter()
            .any(|cfg| cfg.contains("old-bridge"))
    );
    let actual = inventory(
        r#"
            #[cfg(test)]
            fn reconcile(
                old: &wow_world::SharedMapManager,
                new: &wow_map::MapManager,
            ) {}
        "#,
    )
    .unwrap();
    assert_eq!(actual.bridges.len(), 1, "cfg(test) is inventoried");
    assert!(compare_bridge_access_baseline(&expected, &actual).is_err());
}

#[test]
fn distinct_cfg_mounts_are_preserved_and_exact_duplicate_mounts_fail() {
    let text = r#"
        fn reconcile(
            old: &wow_world::SharedMapManager,
            new: &wow_map::MapManager,
        ) {}
    "#;
    let production_cfg = vec!["cfg(feature = \"production-mount\")".to_owned()];
    let test_cfg = vec!["cfg(test)".to_owned()];
    let production = BridgeSource {
        package: "fixture",
        module: "crate::fixture",
        source_path: "src/shared.rs",
        inherited_cfg: &production_cfg,
        source: text,
    };
    let test = BridgeSource {
        inherited_cfg: &test_cfg,
        ..production
    };
    let baseline = inventory_bridge_accesses(&[test, production])
        .expect("distinct cfg mounts are separate syntax surfaces");
    assert_eq!(baseline.bridges.len(), 2);
    assert_ne!(baseline.bridges[0].cfg, baseline.bridges[1].cfg);

    let error = inventory_bridge_accesses(&[production, production])
        .expect_err("an exact duplicate source mount must fail");
    assert!(error.contains("duplicate bridge source mount"), "{error}");
}

#[test]
fn bridge_hiding_macros_fail_closed_but_transparent_arguments_are_visible() {
    let invocation = inventory(
        r#"
            fn hidden(
                old: &wow_world::SharedMapManager,
                new: &wow_map::MapManager,
            ) {
                apply_bridge!(old, new);
            }
        "#,
    )
    .expect_err("unknown macro cannot hide both authorities");
    assert!(
        invocation.contains("can hide a legacy/canonical bridge"),
        "{invocation}"
    );

    let split_invocation = inventory(
        r#"
            fn hidden(old: &wow_world::SharedMapManager) {
                consume!(wow_map::MapManager::default());
            }
        "#,
    )
    .expect_err("an unknown macro cannot hide one half of an item-level bridge");
    assert!(
        split_invocation.contains("can hide part of a legacy/canonical bridge"),
        "{split_invocation}"
    );

    let definition = inventory(
        r#"
            macro_rules! generated_bridge {
                () => {
                    fn hidden(
                        old: &wow_world::SharedMapManager,
                        new: &wow_map::MapManager,
                    ) {}
                };
            }
        "#,
    )
    .expect_err("item-generating macro cannot conceal a bridge");
    assert!(definition.contains("can generate or hide"), "{definition}");

    let transparent = inventory(
        r#"
            fn visible(
                old: &wow_world::SharedMapManager,
                new: &wow_map::MapManager,
            ) {
                debug!(?old, ?new, "bridge diagnostics");
            }
        "#,
    )
    .expect("transparent macro arguments remain inventory evidence");
    assert!(
        transparent.bridges[0]
            .evidence
            .iter()
            .any(|marker| marker.kind == BridgeEvidenceKind::MacroArgument)
    );
}

#[test]
fn curated_anchor_records_its_declared_direction() {
    let baseline = inventory_bridge_accesses(&[BridgeSource {
        package: "world-server",
        module: "crate::runtime::game_events",
        source_path: "src/session.rs",
        inherited_cfg: &[],
        source: r#"
            fn mirror_loaded_grid_creature_to_legacy_like_cpp(
                canonical: &wow_map::MapManager,
                legacy: &wow_world::MapManager,
            ) {}
        "#,
    }])
    .expect("curated method parses");
    assert_eq!(baseline.bridges.len(), 1);
    assert_eq!(
        baseline.bridges[0].direction_markers,
        vec![BridgeDirection::CanonicalToLegacy]
    );
    let sides: BTreeSet<_> = baseline.bridges[0]
        .evidence
        .iter()
        .map(|marker| marker.side)
        .collect();
    assert!(sides.contains(&BridgeSide::Legacy));
    assert!(sides.contains(&BridgeSide::Canonical));
}

#[test]
fn comparator_rejects_multiplicity_and_noncanonical_rows() {
    let baseline = inventory(
        r#"
            fn reconcile(
                old: &wow_world::SharedMapManager,
                new: &wow_map::MapManager,
            ) {}
        "#,
    )
    .unwrap();
    let mut multiplicity = baseline.clone();
    multiplicity.bridges[0].multiplicity = 2;
    let error = compare_bridge_access_baseline(&baseline, &multiplicity)
        .expect_err("multiplicity drift must fail");
    assert!(error.contains("multiplicity changed"), "{error}");

    let mut zero = baseline.clone();
    zero.bridges[0].multiplicity = 0;
    let error =
        compare_bridge_access_baseline(&zero, &baseline).expect_err("zero multiplicity is invalid");
    assert!(error.contains("zero-multiplicity"), "{error}");

    let mut bad_evidence = baseline.clone();
    bad_evidence.bridges[0].evidence.reverse();
    let error = compare_bridge_access_baseline(&bad_evidence, &baseline)
        .expect_err("evidence must remain canonical");
    assert!(error.contains("strict canonical order"), "{error}");
}

#[test]
fn real_runtime_ledger_anchor_definitions_are_present_once() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let delivery_path = repository.join("crates/world-server/src/runtime/delivery.rs");
    let game_events_path = repository.join("crates/world-server/src/runtime/game_events.rs");
    let session_path = repository.join("crates/wow-world/src/session/mod.rs");
    let delivery = fs::read_to_string(&delivery_path).expect("world-server delivery source");
    let game_events =
        fs::read_to_string(&game_events_path).expect("world-server game-events source");
    let session = fs::read_to_string(&session_path).expect("wow-world session source");
    let baseline = inventory_bridge_accesses(&[
        BridgeSource {
            package: "world-server",
            module: "crate::runtime::delivery",
            source_path: "crates/world-server/src/runtime/delivery.rs",
            inherited_cfg: &[],
            source: &delivery,
        },
        BridgeSource {
            package: "world-server",
            module: "crate::runtime::game_events",
            source_path: "crates/world-server/src/runtime/game_events.rs",
            inherited_cfg: &[],
            source: &game_events,
        },
        BridgeSource {
            package: "wow-world",
            module: "crate::session",
            source_path: "crates/wow-world/src/session/mod.rs",
            inherited_cfg: &[],
            source: &session,
        },
    ])
    .expect("real bridge anchor sources must remain inspectable");
    validate_curated_bridge_anchors(&baseline)
        .expect("every curated runtime-ledger anchor is defined exactly once");
}
