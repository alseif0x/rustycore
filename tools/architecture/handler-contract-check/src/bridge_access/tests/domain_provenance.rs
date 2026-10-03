//! Nominal roots and non-authority imports for extracted World domain packages.

use super::*;

fn package_mount<'a>(
    package: &'a str,
    module: &'a str,
    path: &'a str,
    source: &'a str,
) -> BridgeSource<'a> {
    BridgeSource {
        package,
        module,
        source_path: path,
        inherited_cfg: &[],
        source,
    }
}

fn spell_mount<'a>(module: &'a str, path: &'a str, source: &'a str) -> BridgeSource<'a> {
    package_mount("wow-world-spell", module, path, source)
}

fn interaction_mount<'a>(module: &'a str, path: &'a str, source: &'a str) -> BridgeSource<'a> {
    package_mount("wow-world-interaction", module, path, source)
}

fn instances_mount<'a>(module: &'a str, path: &'a str, source: &'a str) -> BridgeSource<'a> {
    package_mount("wow-world-instances", module, path, source)
}

fn visibility_mount<'a>(module: &'a str, path: &'a str, source: &'a str) -> BridgeSource<'a> {
    package_mount("wow-world-visibility", module, path, source)
}

fn core_mount<'a>(module: &'a str, path: &'a str, source: &'a str) -> BridgeSource<'a> {
    package_mount("wow-world-core", module, path, source)
}

fn nominal_identity(
    sources: &[BridgeSource<'_>],
    package: &str,
    module: &str,
    type_path: &str,
) -> Result<Vec<SuppliedTypeIdentity>, TypeIdentityError> {
    let ty = syn::parse_str::<syn::Type>(type_path).expect("fixture type parses");
    crate::bridge_access::provenance::resolve_supplied_type_identity(
        sources,
        package,
        module,
        &ty,
        &[],
    )
}

#[test]
fn world_spell_and_interaction_roots_resolve_nominal_reexports() {
    let sources = [
        spell_mount(
            "crate",
            "spell/lib.rs",
            "pub mod spell_types; pub use self::spell_types::SpellDataLikeCpp as SpellData;",
        ),
        spell_mount(
            "crate::spell_types",
            "spell/spell_types.rs",
            "pub struct SpellDataLikeCpp;",
        ),
        interaction_mount(
            "crate",
            "interaction/lib.rs",
            "pub mod interaction_types; \
             pub use self::interaction_types::InteractionDataLikeCpp as InteractionData;",
        ),
        interaction_mount(
            "crate::interaction_types",
            "interaction/interaction_types.rs",
            "pub struct InteractionDataLikeCpp;",
        ),
        instances_mount(
            "crate",
            "instances/lib.rs",
            "pub mod instance_types; pub use self::instance_types::InstanceDataLikeCpp as InstanceData;",
        ),
        instances_mount(
            "crate::instance_types",
            "instances/instance_types.rs",
            "pub struct InstanceDataLikeCpp;",
        ),
        visibility_mount(
            "crate",
            "visibility/lib.rs",
            "pub mod visibility_types; pub use self::visibility_types::VisibilityDataLikeCpp as VisibilityData;",
        ),
        visibility_mount(
            "crate::visibility_types",
            "visibility/visibility_types.rs",
            "pub struct VisibilityDataLikeCpp;",
        ),
        package_mount(
            "wow-world",
            "crate::consumer",
            "world/consumer.rs",
            "use wow_world_spell as spell; use wow_world_interaction as interaction; \
             use wow_world_instances as instances; use wow_world_visibility as visibility; \
             type SelectedSpell = spell::SpellData; \
             type SelectedInteraction = interaction::InteractionData; \
             type SelectedInstances = instances::InstanceData; \
             type SelectedVisibility = visibility::VisibilityData;",
        ),
    ];

    let spell = nominal_identity(&sources, "wow-world", "crate::consumer", "SelectedSpell")
        .expect("the WorldSpell root alias resolves through its mounted provider");
    assert_eq!(spell.len(), 1);
    assert_eq!(spell[0].package, "wow-world-spell");
    assert_eq!(spell[0].module, "crate::spell_types");
    assert_eq!(spell[0].symbol, "SpellDataLikeCpp");

    let interaction = nominal_identity(
        &sources,
        "wow-world",
        "crate::consumer",
        "SelectedInteraction",
    )
    .expect("the WorldInteraction root alias resolves through its mounted provider");
    assert_eq!(interaction.len(), 1);
    assert_eq!(interaction[0].package, "wow-world-interaction");
    assert_eq!(interaction[0].module, "crate::interaction_types");
    assert_eq!(interaction[0].symbol, "InteractionDataLikeCpp");

    let instances = nominal_identity(&sources, "wow-world", "crate::consumer", "SelectedInstances")
        .expect("the WorldInstances root alias resolves through its mounted provider");
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].package, "wow-world-instances");
    assert_eq!(instances[0].module, "crate::instance_types");
    assert_eq!(instances[0].symbol, "InstanceDataLikeCpp");

    let visibility = nominal_identity(&sources, "wow-world", "crate::consumer", "SelectedVisibility")
        .expect("the WorldVisibility root alias resolves through its mounted provider");
    assert_eq!(visibility.len(), 1);
    assert_eq!(visibility[0].package, "wow-world-visibility");
    assert_eq!(visibility[0].module, "crate::visibility_types");
    assert_eq!(visibility[0].symbol, "VisibilityDataLikeCpp");
}

#[test]
fn local_modules_shadow_world_domain_and_core_package_roots() {
    let sources = [
        spell_mount("crate", "spell/lib.rs", "pub struct ExternalSpell;"),
        interaction_mount(
            "crate",
            "interaction/lib.rs",
            "pub struct ExternalInteraction;",
        ),
        instances_mount("crate", "instances/lib.rs", "pub struct ExternalInstances;"),
        visibility_mount("crate", "visibility/lib.rs", "pub struct ExternalVisibility;"),
        core_mount("crate", "core/lib.rs", "pub mod session;"),
        core_mount("crate::session", "core/session.rs", "pub struct SessionCore;"),
        package_mount(
            "wow-world",
            "crate::consumer",
            "world/consumer.rs",
            "mod wow_world_spell { pub struct LocalSpell; } \
             mod wow_world_interaction { pub struct LocalInteraction; } \
             mod wow_world_instances { pub struct LocalInstances; } \
             mod wow_world_visibility { pub struct LocalVisibility; } \
             mod wow_world_core { pub mod session { pub struct LocalCore; } } \
             type SelectedSpell = wow_world_spell::LocalSpell; \
             type SelectedInteraction = wow_world_interaction::LocalInteraction; \
             type SelectedInstances = wow_world_instances::LocalInstances; \
             type SelectedVisibility = wow_world_visibility::LocalVisibility; \
             type SelectedCore = wow_world_core::session::LocalCore;",
        ),
    ];

    for (selected, local_module, symbol) in [
        (
            "SelectedSpell",
            "crate::consumer::wow_world_spell",
            "LocalSpell",
        ),
        (
            "SelectedInteraction",
            "crate::consumer::wow_world_interaction",
            "LocalInteraction",
        ),
        (
            "SelectedInstances",
            "crate::consumer::wow_world_instances",
            "LocalInstances",
        ),
        (
            "SelectedVisibility",
            "crate::consumer::wow_world_visibility",
            "LocalVisibility",
        ),
        (
            "SelectedCore",
            "crate::consumer::wow_world_core::session",
            "LocalCore",
        ),
    ] {
        let resolved = nominal_identity(&sources, "wow-world", "crate::consumer", selected)
            .expect("a local module binding shadows the identically named external crate root");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].package, "wow-world");
        assert_eq!(resolved[0].module, local_module);
        assert_eq!(resolved[0].symbol, symbol);
    }
}

#[test]
fn ambiguous_domain_and_core_imports_fail_closed_for_both_new_roots() {
    for (domain_package, domain_path) in [
        ("wow_world_spell", "spell/lib.rs"),
        ("wow_world_interaction", "interaction/lib.rs"),
        ("wow_world_instances", "instances/lib.rs"),
        ("wow_world_visibility", "visibility/lib.rs"),
    ] {
        let consumer = format!(
            "use {domain_package}::DomainValue as Selected; \
             use wow_world_core::session::SessionCore as Selected;"
        );
        let domain = package_mount(
            match domain_package {
                "wow_world_spell" => "wow-world-spell",
                "wow_world_interaction" => "wow-world-interaction",
                "wow_world_instances" => "wow-world-instances",
                _ => "wow-world-visibility",
            },
            "crate",
            domain_path,
            "pub struct DomainValue;",
        );
        let sources = [
            domain,
            core_mount(
                "crate",
                "core/lib.rs",
                "pub mod session { pub struct SessionCore; }",
            ),
            package_mount(
                "wow-world",
                "crate::consumer",
                "world/consumer.rs",
                &consumer,
            ),
        ];
        assert!(matches!(
            nominal_identity(&sources, "wow-world", "crate::consumer", "Selected"),
            Err(TypeIdentityError::Ambiguous(_))
        ));
    }
}

#[test]
fn world_spell_and_interaction_imports_do_not_add_bridge_authority() {
    let source = package_mount(
        "world-server",
        "crate::bridge",
        "server/bridge.rs",
        r#"
            use wow_world_spell::SpellDataLikeCpp as SpellData;
            use wow_world_interaction::InteractionDataLikeCpp as InteractionData;
            use wow_world_instances::InstanceDataLikeCpp as InstanceData;
            use wow_world_visibility::VisibilityDataLikeCpp as VisibilityData;

            fn bridge(
                old: &wow_world::SharedMapManager,
                new: &wow_entities::Creature,
                spell: &SpellData,
                interaction: &InteractionData,
                instances: &InstanceData,
                visibility: &VisibilityData,
            ) {}
        "#,
    );
    let baseline = inventory_bridge_accesses(&[source])
        .expect("domain value imports remain non-authority beside bridge sides");
    assert_eq!(baseline.bridges.len(), 1);
    let evidence = &baseline.bridges[0].evidence;
    assert!(evidence
        .iter()
        .any(|item| item.side == BridgeSide::Legacy && item.symbol == "SharedMapManager"));
    assert!(evidence
        .iter()
        .any(|item| item.side == BridgeSide::Canonical && item.symbol == "Creature"));
    assert!(!evidence.iter().any(|item| {
        matches!(
            item.symbol.as_str(),
            "SpellDataLikeCpp"
                | "InteractionDataLikeCpp"
                | "InstanceDataLikeCpp"
                | "VisibilityDataLikeCpp"
        )
    }));
}
