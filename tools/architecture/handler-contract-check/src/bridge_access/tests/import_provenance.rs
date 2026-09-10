//! Lexical import regressions for physical module moves (#716).

use super::*;

const TRANSLATE: &str = r#"
    fn translate(old: &wow_world::SharedMapManager) {
        let _ = old;
        let _ = Creature::new(false);
    }
"#;

fn mounted<'a>(module: &'a str, path: &'a str, text: &'a str) -> BridgeSource<'a> {
    BridgeSource {
        package: "fixture",
        module,
        source_path: path,
        inherited_cfg: &[],
        source: text,
    }
}

fn pair(parent: &str, child: &str) -> Result<BridgeAccessBaseline, String> {
    inventory_bridge_accesses(&[
        mounted("crate", "src/lib.rs", parent),
        mounted("crate::child", "src/child.rs", child),
    ])
}

#[test]
fn root_inline_and_external_moves_preserve_exact_bridge_evidence() {
    let root = format!("use wow_entities::Creature; {TRANSLATE}");
    let inline = format!("use wow_entities::Creature; mod child {{ use super::*; {TRANSLATE} }}");
    let external = format!("use super::*; {TRANSLATE}");
    let expected = inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &root)])
        .expect("root provenance");
    let inline = inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &inline)])
        .expect("inline provenance");
    let external =
        pair("use wow_entities::Creature; mod child;", &external).expect("external provenance");
    assert_eq!(expected.bridges.len(), 1);
    for mut moved in [inline, external] {
        assert_eq!(moved.bridges.len(), 1);
        moved.bridges[0].module = expected.bridges[0].module.clone();
        moved.bridges[0].path = expected.bridges[0].path.clone();
        compare_bridge_access_baseline(&expected, &moved)
            .expect("only the physical/logical location may change");
    }
}

#[test]
fn explicit_grouped_renamed_and_generic_alias_imports_keep_provenance() {
    let parent = "use wow_entities::Creature; type ParentCreature = Creature; mod child;";
    for imports in [
        "use super::Creature;",
        "use super::{Creature, ParentCreature as Unused};",
        "use super::ParentCreature as Creature;",
        "use super::Creature as Parent; type Creature = Parent;",
        "use super::Creature as Parent; type Creature = std::sync::Arc<Parent>;",
    ] {
        let child = format!("{imports} {TRANSLATE}");
        let baseline = pair(parent, &child).expect(imports);
        assert_eq!(baseline.bridges.len(), 1, "{imports}");
    }
}

#[test]
fn exact_public_canonical_manager_alias_survives_parent_imports() {
    for path in [
        "wow_world::SharedCanonicalMapManager",
        "wow_world::session::SharedCanonicalMapManager",
    ] {
        let parent = format!("use {path}; mod child;");
        let child = r#"
            use super::*;
            fn synchronize(old: &wow_world::SharedMapManager, new: &SharedCanonicalMapManager) {}
        "#;
        assert_eq!(pair(&parent, child).expect(path).bridges.len(), 1);
    }
    let text = r#"
        use unrelated_data::SharedCanonicalMapManager;
        fn inspect(old: &wow_world::SharedMapManager, dto: &SharedCanonicalMapManager) {}
    "#;
    assert!(inventory(text).unwrap().bridges.is_empty());
}

#[test]
fn qualified_and_two_level_parent_paths_resolve_without_bare_imports() {
    let leaf = r#"
        fn translate(old: &wow_world::SharedMapManager) {
            let _ = old;
            let _ = super::super::Creature::new(false);
        }
    "#;
    let sources = [
        mounted(
            "crate",
            "src/lib.rs",
            "use wow_entities::Creature; mod child;",
        ),
        mounted("crate::child", "src/child.rs", "mod leaf;"),
        mounted("crate::child::leaf", "src/child/leaf.rs", leaf),
    ];
    let expected = inventory_bridge_accesses(&sources).expect("two-level qualified path");
    assert_eq!(expected.bridges.len(), 1);
    let reversed = sources.into_iter().rev().collect::<Vec<_>>();
    let actual = inventory_bridge_accesses(&reversed).expect("source-order independence");
    compare_bridge_access_baseline(&expected, &actual).unwrap();
}

#[test]
fn two_level_glob_chain_preserves_parent_type_aliases() {
    let leaf = format!("use super::*; {TRANSLATE}");
    let sources = [
        mounted(
            "crate",
            "src/lib.rs",
            "type Creature = wow_entities::Creature; mod child;",
        ),
        mounted("crate::child", "src/child.rs", "use super::*; mod leaf;"),
        mounted("crate::child::leaf", "src/child/leaf.rs", &leaf),
    ];
    assert_eq!(
        inventory_bridge_accesses(&sources).unwrap().bridges.len(),
        1
    );
}

#[test]
fn parent_reexport_cycle_does_not_hide_an_explicit_authority_binding() {
    let result = pair(
        "use wow_entities::Creature; mod child; pub use child::*;",
        &format!("use super::*; {TRANSLATE}"),
    )
    .expect("ordinary parent reexports and child imports have an explicit type source");
    assert_eq!(result.bridges.len(), 1);
}

#[test]
fn conflicting_glob_authorities_fail_closed_at_the_used_name() {
    let text = format!(
        "mod canonical {{ pub use wow_entities::Creature; }} \
         mod legacy {{ pub use wow_world::WorldCreature as Creature; }} \
         mod child {{ use crate::canonical::*; use crate::legacy::*; {TRANSLATE} }}"
    );
    inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &text)])
        .expect_err("a used ambiguous name cannot silently select one glob");
}

#[test]
fn local_and_explicit_foreign_names_shadow_parent_globs() {
    for declaration in [
        "struct Creature;",
        "use unrelated_data::Creature;",
        "enum Creature { Idle }",
    ] {
        let child = format!("use super::*; {declaration} {TRANSLATE}");
        let result = pair("use wow_entities::Creature; mod child;", &child)
            .expect("a local or explicit imported name shadows the glob");
        assert!(result.bridges.is_empty(), "{declaration}");
    }
}

#[test]
fn inline_children_do_not_implicitly_inherit_unimported_parent_types() {
    let text = format!("use wow_entities::Creature; mod child {{ {TRANSLATE} }}");
    let result = inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &text)])
        .expect("no Rust import connects the child name to the parent type");
    assert!(result.bridges.is_empty());
}

#[test]
fn parent_enum_variants_are_not_canonical_creature_authority() {
    let child = r#"
        use super::*;
        fn inspect(old: &wow_world::SharedMapManager) {
            let _ = SpawnObjectType::Creature;
            assert!(matches!(SpawnObjectType::Creature, SpawnObjectType::Creature));
        }
    "#;
    let result = pair(
        "use wow_entities::Creature; use wow_map::SpawnObjectType; mod child;",
        child,
    )
    .expect("variant and imported type have distinct provenance");
    assert!(result.bridges.is_empty());
}

#[test]
fn matching_import_and_function_cfg_retains_test_and_feature_bridges() {
    for cfg in ["test", "feature = \"bridge\""] {
        let text = format!("#[cfg({cfg})] use wow_entities::Creature; #[cfg({cfg})] {TRANSLATE}");
        let baseline = inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &text)])
            .expect("the import and candidate share the same presence condition");
        assert_eq!(baseline.bridges.len(), 1, "{cfg}");
        assert!(!baseline.bridges[0].cfg.is_empty());
    }
}

#[test]
fn nested_test_cfg_uses_the_matching_import_scope() {
    for body in [
        "#[cfg(test)] { let _ = Creature::new(false); }",
        "#[cfg(test)] let _ = Creature::new(false);",
        "#[cfg(test)] consume(Creature::new(false));",
        "let _ = Holder { old, #[cfg(test)] new: Creature::new(false) };",
    ] {
        let text = format!(
            "#[cfg(test)] use wow_entities::Creature; \
             mod child {{ use super::*; \
               struct Holder<'a> {{ old: &'a wow_world::SharedMapManager, #[cfg(test)] new: Creature }} \
               fn translate(old: &wow_world::SharedMapManager) {{ {body} }} \
             }}"
        );
        let baseline = inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &text)]).expect(
            "nested cfg must constrain the provenance use, not just the enclosing function",
        );
        assert_eq!(
            baseline
                .bridges
                .iter()
                .filter(|record| record.enclosing == "fn::translate")
                .count(),
            1,
            "{body}",
        );
    }
}

#[test]
fn block_local_import_replaces_unresolved_parent_glob_context() {
    let child = r#"
        use super::*;
        fn bridge(new: &wow_entities::Creature) {
            use wow_world::WorldCreature;
            let _ = WorldCreature::from_canonical(new);
        }
    "#;
    let baseline = pair("pub use child::*; mod child;", child)
        .expect("a block-local import overrides the unresolved parent glob binding");
    assert_eq!(baseline.bridges.len(), 1);
}

#[test]
fn nested_cfg_retains_enclosing_block_imports_and_shadows() {
    for (parent, import, expected) in [
        ("", "use wow_entities::Creature as Entity;", 1),
        (
            "use wow_entities::Creature as Entity;",
            "use other_data::Entity;",
            0,
        ),
    ] {
        let source = format!(
            "{parent} fn bridge(old: &wow_world::SharedMapManager) {{ \
               {import} #[cfg(test)] {{ let _ = Entity::new(false); }} \
             }}"
        );
        let baseline =
            inventory(&source).expect("entering nested cfg preserves the enclosing lexical import");
        assert_eq!(baseline.bridges.len(), expected, "{source}");
    }
}

#[test]
fn cfg_test_local_shadow_does_not_hide_production_parent_authority() {
    let child = r#"
        use super::*;
        #[cfg(test)] struct Creature;
        #[cfg(not(test))]
        fn production(old: &wow_world::SharedMapManager) { let _ = Creature::new(false); }
        #[cfg(test)]
        fn test_fixture(old: &wow_world::SharedMapManager) { let _ = Creature; }
    "#;
    let result = pair("use wow_entities::Creature; mod child;", child)
        .expect("the local test shadow and production import never overlap");
    assert_eq!(result.bridges.len(), 1);
    assert_eq!(result.bridges[0].enclosing, "fn::production");
}

#[test]
fn impossible_import_does_not_supply_authority() {
    let text = format!("#[cfg(any())] use wow_entities::Creature; {TRANSLATE}");
    let result = inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &text)])
        .expect("an impossible declaration is absent");
    assert!(result.bridges.is_empty());
}

#[test]
fn relevant_conditional_authority_ambiguity_fails_closed() {
    let text = format!(
        "#[cfg(test)] use wow_entities::Creature; \
         #[cfg(not(test))] use other_data::Creature; {TRANSLATE}"
    );
    inventory_bridge_accesses(&[mounted("crate", "src/lib.rs", &text)])
        .expect_err("one unconditional candidate cannot silently select an authority branch");
}

#[test]
fn missing_relative_context_is_reported_only_when_provenance_is_used() {
    let used = format!("use super::Creature; {TRANSLATE}");
    inventory_bridge_accesses(&[mounted("crate::child", "src/child.rs", &used)])
        .expect_err("a used relative authority import needs its parent context");
    let unused = "use super::Creature; fn innocent(value: u32) -> u32 { value }";
    let result = inventory_bridge_accesses(&[mounted("crate::child", "src/child.rs", unused)])
        .expect("an unrelated function does not need missing authority context");
    assert!(result.bridges.is_empty());
}

#[test]
fn renamed_missing_relative_authority_cannot_hide_behind_an_arbitrary_alias() {
    for import in [
        "use super::Entity;",
        "type Entity = super::Entity;",
        "use super::*;",
    ] {
        let text =
            format!("{import} fn translate(old: &wow_world::SharedMapManager, new: &Entity) {{}}");
        inventory_bridge_accesses(&[mounted("crate::child", "src/child.rs", &text)])
            .expect_err("missing parent provenance is independent of the alias spelling");
    }
    let unused = "use super::Entity; fn innocent(value: u32) -> u32 { value }";
    assert!(
        inventory_bridge_accesses(&[mounted("crate::child", "src/child.rs", unused)])
            .unwrap()
            .bridges
            .is_empty()
    );
}

#[test]
fn relative_import_and_type_alias_cycles_fail_closed() {
    let child = format!("use super::Creature; {TRANSLATE}");
    pair("use crate::child::Creature; mod child;", &child)
        .expect_err("a relative import cycle is not proof of absent authority");
    let child = format!("type Creature = super::Creature; {TRANSLATE}");
    pair("type Creature = crate::child::Creature; mod child;", &child)
        .expect_err("aliases must retain the cycle diagnostic");
}

#[test]
fn same_named_modules_in_other_packages_cannot_supply_provenance() {
    let text = r#"
        mod child;
        fn inspect(old: &wow_world::SharedMapManager) { let _ = child::Creature; }
    "#;
    let foreign = BridgeSource {
        package: "another-package",
        module: "crate::child",
        source_path: "other/src/child.rs",
        inherited_cfg: &[],
        source: "pub use wow_entities::Creature;",
    };
    let result = inventory_bridge_accesses(&[
        mounted("crate", "src/lib.rs", text),
        mounted("crate::child", "src/child.rs", "pub struct Creature;"),
        foreign,
    ])
    .expect("provenance stays within the source package");
    assert!(result.bridges.is_empty());
}

#[test]
fn unresolved_external_authority_glob_remains_rejected_after_parent_import() {
    pair(
        "use wow_entities::*; mod child;",
        &format!("use super::*; {TRANSLATE}"),
    )
    .expect_err("a parent glob cannot make an opaque external authority glob inspectable");
}

#[test]
fn moved_pending_respawn_bridge_matches_its_exact_reviewed_record() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let baseline = crate::session_ownership::repository_syntax_for_tests()
        .expect("the moved function retains complete parent import provenance");
    let actual = baseline
        .bridge_accesses
        .bridges
        .iter()
        .filter(|record| record.enclosing == "fn::world_creature_from_pending_respawn_like_cpp")
        .collect::<Vec<_>>();
    assert_eq!(actual.len(), 1);
    let policy: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repository.join("tools/architecture/session-ownership-policy.json"))
            .unwrap(),
    )
    .unwrap();
    let expected = policy["syntax_baseline"]["bridge_accesses"]["bridges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["enclosing"] == "fn::world_creature_from_pending_respawn_like_cpp")
        .expect("reviewed bridge remains in the exact inventory");
    assert_eq!(&serde_json::to_value(actual[0]).unwrap(), expected);
}
