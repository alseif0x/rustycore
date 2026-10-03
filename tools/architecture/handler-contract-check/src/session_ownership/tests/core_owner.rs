//! Closed-world SessionCore owner regressions.

use super::super::core_owner::{SESSION_CORE_NAME, WORLD_CORE_SESSION_CORE_MODULE};
use super::scenarios_1::{server_source, synthetic_baseline_with_core, world_source};
use super::*;

const NETWORK: &str =
    "pub enum SessionCommand { Kick(KickCommand) } pub struct KickCommand { pub reason: String }";

fn core_source(extra: &str) -> String {
    format!(
        r#"
            pub mod session {{
                pub mod state {{
                    pub mod session_core {{
                        pub struct SessionCore {{
                            pub account_id: u32,
                            #[cfg(test)] pub test_slot: bool,
                        }}
                        impl SessionCore {{
                            pub fn account_id(&self) -> u32 {{ self.account_id }}
                        }}
                        {extra}
                    }}
                    pub use self::session_core::SessionCore;
                }}
                pub use self::state::SessionCore;
            }}
        "#
    )
}

fn world_with_core_facade() -> String {
    world_with_core_type(
        "state::SessionCore",
        "pub(crate) use wow_world_core::session::SessionCore; \
         pub mod state { pub(crate) use super::SessionCore; }",
    )
}

fn world_with_core_type(core_type: &str, session_items: &str) -> String {
    world_source(&format!("pub core: {core_type},"), "").replacen(
        "pub mod session {",
        &format!("pub mod session {{ {session_items}"),
        1,
    )
}

fn baseline(world: &str, core: &str) -> Result<SessionSyntaxBaseline, String> {
    synthetic_baseline_with_core(world, &server_source("", ""), NETWORK, core)
}

fn baseline_with_world_social(
    world: &str,
    core: &str,
    social: &str,
) -> Result<SessionSyntaxBaseline, String> {
    let unit = |role, path: &str, source: &str| SourceUnit {
        role,
        source_path: PathBuf::from(path),
        repository_relative_path: path.to_owned(),
        logical_module_path: "crate".to_owned(),
        cfg: Vec::new(),
        availability: Availability {
            production: true,
            test: true,
        },
        source: source.to_owned(),
    };
    collect_units(
        vec![
            unit(PackageRole::World, "wow-world/src/lib.rs", world),
            unit(PackageRole::WorldCore, WORLD_CORE_CRATE_ROOT, core),
            unit(
                PackageRole::WorldSocial,
                WORLD_SOCIAL_CRATE_ROOT,
                social,
            ),
            unit(
                PackageRole::Server,
                "world-server/src/main.rs",
                &server_source("", ""),
            ),
            unit(PackageRole::Network, "wow-network/src/lib.rs", NETWORK),
        ],
        PersistenceAccessBaseline {
            schema_version: 3,
            accesses: Vec::new(),
        },
    )
}

fn baseline_with_world_domain(
    role: PackageRole,
    domain: &str,
) -> Result<SessionSyntaxBaseline, String> {
    let unit = |role, path: &str, source: &str| SourceUnit {
        role,
        source_path: PathBuf::from(path),
        repository_relative_path: path.to_owned(),
        logical_module_path: "crate".to_owned(),
        cfg: Vec::new(),
        availability: Availability {
            production: true,
            test: true,
        },
        source: source.to_owned(),
    };
    let domain_path = format!("crates/{}/src/lib.rs", role.package_name());
    collect_units(
        vec![
            unit(PackageRole::World, "crates/wow-world/src/lib.rs", &world_with_core_facade()),
            unit(PackageRole::WorldCore, WORLD_CORE_CRATE_ROOT, &core_source("")),
            unit(role, &domain_path, domain),
            unit(
                PackageRole::Server,
                "crates/world-server/src/lib.rs",
                &server_source("", ""),
            ),
            unit(PackageRole::Network, "crates/wow-network/src/lib.rs", NETWORK),
        ],
        PersistenceAccessBaseline {
            schema_version: 3,
            accesses: Vec::new(),
        },
    )
}

#[test]
fn world_social_is_a_distinct_package_role_without_contract_ownership() {
    let social = r#"
        pub mod state { pub struct SessionSocialLimits; }
        pub use self::state::SessionSocialLimits;
        pub mod group {
            pub enum SessionCommand { SocialOnly }
            pub struct KickCommand;
        }
    "#;
    let baseline = baseline_with_world_social(
        &world_with_core_facade(),
        &core_source(""),
        social,
    )
    .expect("the Social domain source is inventoried without taking network contracts");

    assert_eq!(PackageRole::WorldSocial.package_name(), "wow-world-social");
    assert_eq!(PackageRole::Social.package_name(), "wow-social");
    assert_eq!(
        baseline
            .session_core_owner
            .definition
            .as_ref()
            .expect("the Core owner remains present")
            .package,
        "wow-world-core"
    );
}

#[test]
fn world_social_rejects_a_foreign_session_core_definition() {
    let error = baseline_with_world_social(
        &world_with_core_facade(),
        &core_source(""),
        "pub struct SessionCore;",
    )
    .expect_err("a same-named Social definition cannot replace Core");
    assert!(error.contains("wow-world-social"), "{error}");
    assert!(error.contains("defines SessionCore"), "{error}");
}

#[test]
fn world_social_core_borrow_alias_impl_is_rejected_as_foreign() {
    let social = "use wow_world_core::session::SessionCore as CoreBorrow; impl CoreBorrow {}";
    let error = baseline_with_world_social(
        &world_with_core_facade(),
        &core_source(""),
        social,
    )
    .expect_err("a Core borrow alias does not authorize a Social impl");
    assert!(error.contains("wow-world-social"), "{error}");
    assert!(error.contains("implements SessionCore"), "{error}");
}

#[test]
fn world_social_core_borrow_alias_with_ambiguous_provider_fails_closed() {
    let social = r#"
        pub mod local { pub struct OtherCore; }
        use wow_world_core::session::SessionCore as CoreBorrow;
        use self::local::OtherCore as CoreBorrow;
        impl CoreBorrow {}
    "#;
    let error = baseline_with_world_social(
        &world_with_core_facade(),
        &core_source(""),
        social,
    )
    .expect_err("ambiguous nominal providers must remain a hard error in Social");
    assert!(error.contains("candidate CoreBorrow"), "{error}");
    assert!(error.contains("ambiguous"), "{error}");
}

#[test]
fn world_social_rejects_a_world_session_definition() {
    let error = baseline_with_world_social(
        &world_with_core_facade(),
        &core_source(""),
        "pub struct WorldSession;",
    )
    .expect_err("WorldSession remains owned by wow-world");
    assert!(error.contains("wow-world-social"), "{error}");
    assert!(error.contains("WorldSession struct"), "{error}");
}

#[test]
fn world_core_session_core_definition_fields_and_impls_are_pinned() {
    let baseline = baseline(
        &world_with_core_facade(),
        &core_source(
            r#"
                pub mod hub_impls {
                    impl crate::session::state::session_core::SessionCore {
                        #[cfg(test)] pub fn test_helper(&self) {}
                    }
                }
            "#,
        ),
    )
    .expect("the exact Core SessionCore owner fixture parses");
    let owner = &baseline.session_core_owner;
    let definition = owner.definition.as_ref().expect("one Core definition");

    assert_eq!(definition.package, "wow-world-core");
    assert_eq!(definition.definition.module, WORLD_CORE_SESSION_CORE_MODULE);
    assert_eq!(definition.definition.name, SESSION_CORE_NAME);
    assert!(definition.availability.production && definition.availability.test);
    assert!(owner.fields.iter().any(|field| {
        field.package == "wow-world-core"
            && field.module == WORLD_CORE_SESSION_CORE_MODULE
            && field.field.name == "account_id"
            && field.availability.production
    }));
    assert!(owner.fields.iter().any(|field| {
        field.field.name == "test_slot"
            && !field.availability.production
            && field.availability.test
            && field.field.cfg == vec!["cfg (test)".to_owned()]
    }));
    assert!(owner.impls.iter().any(|item| {
        item.package == "wow-world-core"
            && item.module == WORLD_CORE_SESSION_CORE_MODULE
            && item.self_type.ends_with("SessionCore")
            && item.availability.production
    }));
    assert!(owner.impls.iter().any(|item| {
        item.module == "crate::session::state::session_core::hub_impls"
            && item.cfg.is_empty()
            && item.availability.production
            && item.availability.test
    }));
    assert!(owner.impl_items.iter().any(|item| {
        item.module == "crate::session::state::session_core::hub_impls"
            && item.name == "test_helper"
            && item.availability.test
            && !item.availability.production
    }));
    assert_eq!(
        baseline
            .world_session
            .fields
            .iter()
            .find(|field| field.name == "core")
            .map(|field| field.type_expression.as_str()),
        Some("state :: SessionCore"),
        "the World field resolves through its state facade"
    );
    let binding = owner
        .world_session_core_bindings
        .first()
        .expect("WorldSession.core has a resolved nominal binding");
    assert_eq!(binding.package, "wow-world");
    assert_eq!(binding.module, "crate::session");
    assert_eq!(binding.type_expression, "state :: SessionCore");
    assert_eq!(binding.providers.len(), 1);
    assert_eq!(binding.providers[0].package, "wow-world-core");
    assert_eq!(binding.providers[0].module, WORLD_CORE_SESSION_CORE_MODULE);
    assert_eq!(binding.providers[0].symbol, SESSION_CORE_NAME);
    assert_eq!(binding.providers[0].kind, "struct");
}

#[test]
fn reordering_session_core_fields_is_baseline_drift_for_drop_order() {
    let world = world_with_core_facade();
    let original_core = core_source("");
    let expected = baseline(&world, &original_core).expect("the original Core owner parses");

    let ordinal = |name: &str| {
        expected
            .session_core_owner
            .fields
            .iter()
            .find(|field| field.field.name == name)
            .map(|field| field.declaration_ordinal)
    };
    assert_eq!(ordinal("account_id"), Some(0));
    assert_eq!(ordinal("test_slot"), Some(1));

    let reordered_core = original_core.replace(
        "pub account_id: u32,\n                            #[cfg(test)] pub test_slot: bool,",
        "#[cfg(test)] pub test_slot: bool,\n                            pub account_id: u32,",
    );
    assert_ne!(
        reordered_core, original_core,
        "the fixture must reorder fields"
    );
    let actual = baseline(&world, &reordered_core).expect("the reordered Core owner parses");

    let error = compare_baseline(&expected, &actual)
        .expect_err("field declaration order must remain pinned to preserve drop order");
    assert!(
        error.contains("SessionCore field surface changed"),
        "{error}"
    );
}

#[test]
fn resolved_world_session_core_provider_is_baseline_visible() {
    let world = world_with_core_facade();
    let expected = baseline(&world, &core_source(""))
        .expect("the Core field provider is resolved for the baseline");
    let mut actual = expected.clone();
    actual.session_core_owner.world_session_core_bindings[0].providers[0].source_path =
        "crates/foreign/src/lib.rs".to_owned();

    let error = compare_baseline(&expected, &actual)
        .expect_err("a resolved provider identity change must be baseline drift");
    assert!(
        error.contains("WorldSession.core resolved provider surface changed"),
        "{error}"
    );
}

#[test]
fn removing_the_core_definition_and_impls_is_baseline_drift() {
    let expected = baseline(&world_with_core_facade(), &core_source(""))
        .expect("the expected Core owner parses");
    let actual = baseline(&world_source("", ""), "pub mod session {}")
        .expect("an absent Core owner remains observable as an empty surface");

    let error = compare_baseline(&expected, &actual)
        .expect_err("removing the required Core owner must drift from its pinned surface");
    assert!(error.contains("SessionCore definition changed"), "{error}");
    assert!(
        error.contains("SessionCore impl surface changed"),
        "{error}"
    );
}

#[test]
fn duplicate_core_session_core_definitions_are_rejected() {
    let duplicate = core_source("pub struct SessionCore { pub duplicate: u8 }");
    let error = baseline(&world_with_core_facade(), &duplicate)
        .expect_err("duplicate definitions at the canonical destination must fail closed");
    assert!(error.contains("SessionCore"), "{error}");
    assert!(error.contains("mounted more than once"), "{error}");
    assert!(error.contains("ambiguous"), "{error}");
}

#[test]
fn world_and_foreign_session_core_definitions_are_rejected() {
    let core = core_source("");
    let world = world_source("", "").replacen(
        "pub mod session {",
        "pub mod session { pub struct SessionCore { pub wrong: u8 } impl SessionCore {}",
        1,
    );
    let error = baseline(&world, &core)
        .expect_err("a same-named World definition cannot satisfy Core ownership");
    assert!(error.contains("wow-world"), "{error}");
    assert!(error.contains("SessionCore"), "{error}");
    assert!(error.contains("candidate SessionCore"), "{error}");

    let foreign = format!("{NETWORK} pub struct SessionCore {{ pub wrong: u8 }}");
    let error = synthetic_baseline_with_core(
        &world_with_core_facade(),
        &server_source("", ""),
        &foreign,
        &core,
    )
    .expect_err("a foreign package definition cannot satisfy Core ownership");
    assert!(error.contains("wow-network"), "{error}");
    assert!(error.contains("SessionCore"), "{error}");
}

#[test]
fn world_session_core_field_rejects_a_foreign_alias_provider() {
    let world = world_with_core_type(
        "SessionCore",
        "pub mod foreign { pub struct ForeignCore; } \
         use self::foreign::ForeignCore as SessionCore;",
    );
    let error = baseline(&world, &core_source(""))
        .expect_err("a foreign alias cannot satisfy the WorldSession.core provider");
    assert!(error.contains("WorldSession.core"), "{error}");
    assert!(error.contains("wow-world"), "{error}");
    assert!(error.contains("ForeignCore"), "{error}");
}

#[test]
fn world_session_core_field_rejects_a_missing_provider() {
    // No declaration or alias supplies this field type, so the nominal
    // WorldSession.core ownership check observes a genuinely missing provider.
    let world = world_with_core_type("MissingOwnerType", "");
    let error = baseline(&world, &core_source(""))
        .expect_err("a named field without a supplied provider must fail closed");
    assert!(error.contains("WorldSession.core"), "{error}");
    assert!(error.contains("missing nominal type identity"), "{error}");
}

#[test]
fn world_session_core_field_rejects_ambiguous_supplied_providers() {
    let world = world_with_core_type(
        "SelectedCore",
        "use wow_world_core::session::SessionCore as SelectedCore; \
         use wow_world_core::other::OtherCore as SelectedCore;",
    );
    let core = format!(
        "{} pub mod other {{ pub struct OtherCore; }}",
        core_source("")
    );
    let error = baseline(&world, &core)
        .expect_err("compatible providers for the field's imported name are ambiguous");
    assert!(error.contains("ambiguous"), "{error}");
    assert!(error.contains("WorldSession.core"), "{error}");
}

#[test]
fn world_session_core_field_rejects_a_shadowing_wrong_owner() {
    let world = world_with_core_type(
        "state::SessionCore",
        "pub(crate) use wow_world_core::session::SessionCore; \
         pub mod state { pub struct SessionCore; }",
    );
    let error = baseline(&world, &core_source(""))
        .expect_err("a local state type cannot shadow the supplied Core owner");
    assert!(error.contains("WorldSession.core"), "{error}");
    assert!(error.contains("wow-world"), "{error}");
    assert!(error.contains("state"), "{error}");
}

#[test]
fn renamed_core_alias_impl_is_counted_and_changes_are_baseline_drift() {
    let world = world_with_core_facade();
    let alias_impl = r#"
        pub mod alias_owner {
            use super::SessionCore as CoreAlias;
            impl CoreAlias { pub fn alias_only(&self) {} }
        }
        pub mod other_owner {
            pub struct CoreAlias;
            impl CoreAlias { pub fn unrelated(&self) {} }
        }
        pub struct Wrapper<T>(pub T);
        impl Wrapper<SessionCore> { pub fn wrapper_only(&self) {} }
    "#;
    let expected = baseline(&world, &core_source(alias_impl))
        .expect("a renamed alias of the canonical owner resolves nominally");
    assert!(expected.session_core_owner.impls.iter().any(|item| {
        item.self_type == "CoreAlias"
            && item.module == format!("{WORLD_CORE_SESSION_CORE_MODULE}::alias_owner")
            && item.resolved_provider.iter().any(|provider| {
                provider.package == "wow-world-core"
                    && provider.module == WORLD_CORE_SESSION_CORE_MODULE
                    && provider.symbol == SESSION_CORE_NAME
            })
    }));
    assert!(!expected.session_core_owner.impls.iter().any(|item| {
        item.self_type == "CoreAlias"
            && item.module == format!("{WORLD_CORE_SESSION_CORE_MODULE}::other_owner")
    }));
    assert!(
        !expected
            .session_core_owner
            .impls
            .iter()
            .any(|item| item.self_type.starts_with("Wrapper"))
    );

    let actual = baseline(&world, &core_source(""))
        .expect("removing the alias impl remains a comparable source surface");
    let error = compare_baseline(&expected, &actual)
        .expect_err("the baseline must detect removal of a renamed Core impl");
    assert!(
        error.contains("SessionCore impl surface changed"),
        "{error}"
    );
}

#[test]
fn self_imported_core_alias_impls_are_counted_and_changes_are_baseline_drift() {
    let world = world_with_core_facade();
    let self_alias_impl = r#"
        pub mod self_alias_owner {
            use super::SessionCore::{self as CoreAlias};
            impl CoreAlias { pub fn self_alias_only(&self) {} }
        }
        pub mod self_alias_chain_owner {
            use super::SessionCore as SessionCoreAlias;
            use SessionCoreAlias::{self as ChainedCoreAlias};
            impl ChainedCoreAlias { pub fn chained_self_alias_only(&self) {} }
        }
    "#;
    let expected = baseline(&world, &core_source(self_alias_impl))
        .expect("self imports of the canonical owner resolve through alias chains");
    for (self_type, module) in [
        (
            "CoreAlias",
            format!("{WORLD_CORE_SESSION_CORE_MODULE}::self_alias_owner"),
        ),
        (
            "ChainedCoreAlias",
            format!("{WORLD_CORE_SESSION_CORE_MODULE}::self_alias_chain_owner"),
        ),
    ] {
        assert!(expected.session_core_owner.impls.iter().any(|item| {
            item.self_type == self_type
                && item.module == module
                && item.resolved_provider.iter().any(|provider| {
                    provider.package == "wow-world-core"
                        && provider.module == WORLD_CORE_SESSION_CORE_MODULE
                        && provider.symbol == SESSION_CORE_NAME
                })
        }));
    }

    let actual = baseline(&world, &core_source(""))
        .expect("removing the self-imported alias impls remains comparable");
    let error = compare_baseline(&expected, &actual)
        .expect_err("the baseline must detect removal of self-imported Core impls");
    assert!(
        error.contains("SessionCore impl surface changed"),
        "{error}"
    );
}

#[test]
fn unresolved_or_ambiguous_core_alias_impl_candidates_fail_closed() {
    let world = world_with_core_facade();
    let missing = core_source("use crate::absent::SessionCore as CoreAlias; impl CoreAlias {}");
    let error = baseline(&world, &missing)
        .expect_err("an alias candidate with no supplied provider is not ignored");
    assert!(error.contains("candidate CoreAlias"), "{error}");
    assert!(error.contains("missing nominal type identity"), "{error}");

    let ambiguous = core_source(
        "pub mod other { pub struct OtherCore; } \
         pub mod ambiguous { \
           use super::SessionCore as CoreAlias; \
           use super::other::OtherCore as CoreAlias; \
           impl CoreAlias {} \
         }",
    );
    let error = baseline(&world, &ambiguous)
        .expect_err("an ambiguous alias candidate is not silently dropped");
    assert!(error.contains("candidate CoreAlias"), "{error}");
    assert!(error.contains("ambiguous"), "{error}");
}

#[test]
fn world_spell_and_interaction_roles_do_not_take_session_contract_ownership() {
    for (role, package) in [
        (PackageRole::WorldSpell, "wow-world-spell"),
        (PackageRole::WorldInteraction, "wow-world-interaction"),
        (PackageRole::WorldInstances, "wow-world-instances"),
        (PackageRole::WorldVisibility, "wow-world-visibility"),
        (PackageRole::WorldLoot, "wow-world-loot"),
        (PackageRole::WorldEntities, "wow-world-entities"),
        (PackageRole::WorldInventory, "wow-world-inventory"),
        (PackageRole::WorldLifecycle, "wow-world-lifecycle"),
    ] {
        let baseline = baseline_with_world_domain(
            role,
            "pub enum SessionCommand { DomainOnly } pub struct KickCommand; \
             pub struct DomainValue; \
             fn domain_bridge(old: &wow_world::SharedMapManager, \
                              new: &wow_entities::Creature) {}",
        )
        .expect("a mounted domain package does not become a Session contract owner");
        assert_eq!(role.package_name(), package);
        assert_eq!(
            baseline
                .session_core_owner
                .definition
                .as_ref()
                .expect("Core remains the SessionCore owner")
                .package,
            "wow-world-core"
        );
        assert!(baseline
            .bridge_accesses
            .bridges
            .iter()
            .any(|bridge| bridge.package == package));
    }
}

#[test]
fn world_spell_and_interaction_cannot_define_or_impl_the_core_owner() {
    for role in [
        PackageRole::WorldSpell,
        PackageRole::WorldInteraction,
        PackageRole::WorldInstances,
        PackageRole::WorldVisibility,
        PackageRole::WorldLoot,
        PackageRole::WorldEntities,
        PackageRole::WorldInventory,
        PackageRole::WorldLifecycle,
    ] {
        let error = baseline_with_world_domain(role, "pub struct SessionCore;")
            .expect_err("domain packages cannot define SessionCore");
        assert!(error.contains(role.package_name()), "{error}");
        assert!(error.contains("defines SessionCore"), "{error}");

        let error = baseline_with_world_domain(
            role,
            "use wow_world_core::session::SessionCore as CoreBorrow; impl CoreBorrow {}",
        )
        .expect_err("a borrowed Core alias does not authorize a domain impl");
        assert!(error.contains(role.package_name()), "{error}");
        assert!(error.contains("implements SessionCore"), "{error}");
    }
}

#[test]
fn world_session_remains_owned_by_world_across_all_extracted_packages() {
    for role in [
        PackageRole::WorldSocial,
        PackageRole::WorldSpell,
        PackageRole::WorldInteraction,
        PackageRole::WorldInstances,
        PackageRole::WorldVisibility,
        PackageRole::WorldLoot,
        PackageRole::WorldEntities,
        PackageRole::WorldInventory,
        PackageRole::WorldLifecycle,
    ] {
        let error = baseline_with_world_domain(role, "pub struct WorldSession;")
            .expect_err("an extracted package cannot define WorldSession");
        assert!(error.contains(role.package_name()), "{error}");
        assert!(error.contains("WorldSession struct"), "{error}");

        let error = baseline_with_world_domain(
            role,
            "impl crate::session::WorldSession { pub fn foreign_owner(&self) {} }",
        )
        .expect_err("an extracted package cannot implement WorldSession");
        assert!(error.contains(role.package_name()), "{error}");
        assert!(error.contains("WorldSession impl"), "{error}");
    }

    for (source, surface) in [
        ("pub struct WorldSession;", "WorldSession struct"),
        (
            "impl crate::session::WorldSession { pub fn foreign_owner(&self) {} }",
            "WorldSession impl",
        ),
    ] {
        let error = baseline(&world_with_core_facade(), &core_source(source))
            .expect_err("wow-world-core cannot define or implement WorldSession");
        assert!(error.contains("wow-world-core"), "{error}");
        assert!(error.contains(surface), "{error}");
    }
}

#[test]
fn world_spell_and_interaction_source_units_follow_their_real_root_mounts() {
    let repository_root = crate::repository_root().expect("repository root");
    let spell = repository_units(
        &repository_root,
        PackageRole::WorldSpell,
        WORLD_SPELL_PACKAGE_ROOT,
        WORLD_SPELL_CRATE_ROOT,
    )
    .expect("the actual WorldSpell root and its declared modules are loadable");
    let interaction = repository_units(
        &repository_root,
        PackageRole::WorldInteraction,
        WORLD_INTERACTION_PACKAGE_ROOT,
        WORLD_INTERACTION_CRATE_ROOT,
    )
    .expect("the actual WorldInteraction root and its declared modules are loadable");
    let instances = repository_units(
        &repository_root,
        PackageRole::WorldInstances,
        WORLD_INSTANCES_PACKAGE_ROOT,
        WORLD_INSTANCES_CRATE_ROOT,
    )
    .expect("the actual WorldInstances root and its declared modules are loadable");
    let visibility = repository_units(
        &repository_root,
        PackageRole::WorldVisibility,
        WORLD_VISIBILITY_PACKAGE_ROOT,
        WORLD_VISIBILITY_CRATE_ROOT,
    )
    .expect("the actual WorldVisibility root and its declared modules are loadable");
    let loot = repository_units(
        &repository_root,
        PackageRole::WorldLoot,
        WORLD_LOOT_PACKAGE_ROOT,
        WORLD_LOOT_CRATE_ROOT,
    )
    .expect("the actual WorldLoot root and its declared modules are loadable");
    let entities = repository_units(
        &repository_root,
        PackageRole::WorldEntities,
        WORLD_ENTITIES_PACKAGE_ROOT,
        WORLD_ENTITIES_CRATE_ROOT,
    )
    .expect("the actual WorldEntities root and its declared modules are loadable");
    let inventory = repository_units(
        &repository_root,
        PackageRole::WorldInventory,
        WORLD_INVENTORY_PACKAGE_ROOT,
        WORLD_INVENTORY_CRATE_ROOT,
    )
    .expect("the actual WorldInventory root and its declared modules are loadable");
    let lifecycle = repository_units(
        &repository_root,
        PackageRole::WorldLifecycle,
        WORLD_LIFECYCLE_PACKAGE_ROOT,
        WORLD_LIFECYCLE_CRATE_ROOT,
    )
    .expect("the actual WorldLifecycle root and its declared modules are loadable");

    assert!(spell.iter().all(|unit| unit.role == PackageRole::WorldSpell));
    assert!(spell.iter().any(|unit| unit.logical_module_path == "crate"));
    assert!(spell
        .iter()
        .any(|unit| unit.logical_module_path == "crate::player_cast"));
    assert!(spell
        .iter()
        .any(|unit| unit.logical_module_path == "crate::session"));
    assert!(interaction
        .iter()
        .all(|unit| unit.role == PackageRole::WorldInteraction));
    assert!(interaction
        .iter()
        .any(|unit| unit.logical_module_path == "crate::session"));
    assert!(interaction
        .iter()
        .any(|unit| unit.logical_module_path == "crate::state"));
    assert!(instances
        .iter()
        .all(|unit| unit.role == PackageRole::WorldInstances));
    assert!(instances.iter().any(|unit| unit.logical_module_path == "crate"));
    assert!(instances
        .iter()
        .any(|unit| unit.logical_module_path == "crate::map_key"));
    assert!(instances
        .iter()
        .any(|unit| unit.logical_module_path == "crate::state"));
    assert!(visibility
        .iter()
        .all(|unit| unit.role == PackageRole::WorldVisibility));
    assert!(visibility
        .iter()
        .any(|unit| unit.logical_module_path == "crate"));
    assert!(visibility
        .iter()
        .any(|unit| unit.logical_module_path == "crate::object_updates"));
    assert!(visibility
        .iter()
        .any(|unit| unit.logical_module_path == "crate::state"));
    assert!(loot.iter().all(|unit| unit.role == PackageRole::WorldLoot));
    assert!(loot.iter().any(|unit| unit.logical_module_path == "crate"));
    assert!(loot
        .iter()
        .any(|unit| unit.logical_module_path == "crate::state"));
    assert!(entities
        .iter()
        .all(|unit| unit.role == PackageRole::WorldEntities));
    assert!(entities
        .iter()
        .any(|unit| unit.logical_module_path == "crate::creature_publication"));
    assert!(entities
        .iter()
        .any(|unit| unit.logical_module_path == "crate::state"));
    assert!(inventory
        .iter()
        .all(|unit| unit.role == PackageRole::WorldInventory));
    assert!(inventory
        .iter()
        .any(|unit| unit.logical_module_path == "crate::inventory_request_contracts"));
    assert!(inventory
        .iter()
        .any(|unit| unit.logical_module_path == "crate::state"));
    assert!(lifecycle
        .iter()
        .all(|unit| unit.role == PackageRole::WorldLifecycle));
    assert!(lifecycle.iter().any(|unit| unit.logical_module_path == "crate"));
    assert!(lifecycle
        .iter()
        .any(|unit| unit.logical_module_path == "crate::finalization"));
}
