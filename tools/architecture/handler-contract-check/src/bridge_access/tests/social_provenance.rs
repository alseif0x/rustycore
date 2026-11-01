//! Nominal and bridge provenance for the extracted WorldSocial package.

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

fn social_mount<'a>(module: &'a str, path: &'a str, source: &'a str) -> BridgeSource<'a> {
    package_mount("wow-world-social", module, path, source)
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
fn world_social_root_and_core_borrow_aliases_resolve_to_their_real_providers() {
    let sources = [
        social_mount(
            "crate",
            "social/lib.rs",
            "pub mod state; pub use self::state::SessionSocialLimits as SocialLimits; \
             pub use wow_world_core::session::HubRef as CoreBorrow;",
        ),
        social_mount(
            "crate::state",
            "social/state.rs",
            "pub struct SessionSocialLimits;",
        ),
        core_mount("crate", "core/lib.rs", "pub mod session;"),
        core_mount("crate::session", "core/session.rs", "pub struct HubRef;"),
        package_mount("wow-world", "crate", "world/lib.rs", "pub mod consumer;"),
        package_mount(
            "wow-world",
            "crate::consumer",
            "world/consumer.rs",
            "use wow_world_social as social; pub type SelectedSocial = social::SocialLimits;",
        ),
    ];

    let social = nominal_identity(&sources, "wow-world", "crate::consumer", "SelectedSocial")
        .expect("the real Social root and reexport resolve");
    assert_eq!(social.len(), 1);
    assert_eq!(social[0].package, "wow-world-social");
    assert_eq!(social[0].module, "crate::state");
    assert_eq!(social[0].symbol, "SessionSocialLimits");

    let borrowed_core = nominal_identity(&sources, "wow-world-social", "crate", "CoreBorrow")
        .expect("Social may name a borrowed Core type without becoming its provider");
    assert_eq!(borrowed_core.len(), 1);
    assert_eq!(borrowed_core[0].package, "wow-world-core");
    assert_eq!(borrowed_core[0].module, "crate::session");
    assert_eq!(borrowed_core[0].symbol, "HubRef");
}

#[test]
fn local_module_shadowing_precedes_the_world_social_crate_root() {
    let sources = [
        social_mount("crate", "social/lib.rs", "pub struct SessionSocialLimits;"),
        package_mount(
            "wow-world",
            "crate::consumer",
            "world/consumer.rs",
            "mod wow_world_social { pub struct SessionSocialLimits; } \
             pub type Selected = wow_world_social::SessionSocialLimits;",
        ),
    ];

    let found = nominal_identity(&sources, "wow-world", "crate::consumer", "Selected")
        .expect("the local module shadows the external package root");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].package, "wow-world");
    assert_eq!(found[0].module, "crate::consumer::wow_world_social");
    assert_eq!(found[0].symbol, "SessionSocialLimits");
}

#[test]
fn ambiguous_imports_between_world_social_and_core_fail_closed() {
    let sources = [
        social_mount(
            "crate",
            "social/lib.rs",
            "pub struct SessionSocialLimits; \
             pub use self::SessionSocialLimits as SocialLimits;",
        ),
        core_mount(
            "crate",
            "core/lib.rs",
            "pub mod session { pub struct SessionCore; }",
        ),
        package_mount(
            "wow-world",
            "crate::consumer",
            "world/consumer.rs",
            "use wow_world_social::SocialLimits as Selected; \
             use wow_world_core::session::SessionCore as Selected;",
        ),
    ];

    assert!(matches!(
        nominal_identity(&sources, "wow-world", "crate::consumer", "Selected"),
        Err(TypeIdentityError::Ambiguous(_))
    ));
}

#[test]
fn duplicate_world_social_nominal_providers_are_ambiguous() {
    let sources = [
        social_mount(
            "crate",
            "social/lib.rs",
            "pub mod left; pub mod right; pub mod consumer;",
        ),
        social_mount(
            "crate::left",
            "social/left.rs",
            "pub struct SessionSocialLimits;",
        ),
        social_mount(
            "crate::right",
            "social/right.rs",
            "pub struct SessionSocialLimits;",
        ),
        social_mount(
            "crate::consumer",
            "social/consumer.rs",
            "use crate::left::*; use crate::right::*;",
        ),
        package_mount(
            "wow-world",
            "crate::consumer",
            "world/consumer.rs",
            "type Selected = wow_world_social::consumer::SessionSocialLimits;",
        ),
    ];

    assert!(matches!(
        nominal_identity(&sources, "wow-world", "crate::consumer", "Selected"),
        Err(TypeIdentityError::Ambiguous(_))
    ));
}

#[test]
fn world_social_types_do_not_become_core_bridge_authority() {
    let source = package_mount(
        "world-server",
        "crate::bridge",
        "server/bridge.rs",
        r#"
            fn bridge(
                old: &wow_world::SharedMapManager,
                new: &wow_entities::Creature,
                limits: &wow_world_social::SessionSocialLimits,
            ) {}
        "#,
    );
    let baseline = inventory_bridge_accesses(&[source])
        .expect("the extracted domain type is non-authority alongside the actual bridge sides");
    assert_eq!(baseline.bridges.len(), 1);
    let evidence = &baseline.bridges[0].evidence;
    assert!(evidence.iter().any(|marker| {
        marker.side == BridgeSide::Legacy && marker.symbol == "SharedMapManager"
    }));
    assert!(
        evidence
            .iter()
            .any(|marker| marker.side == BridgeSide::Canonical && marker.symbol == "Creature")
    );
    assert!(
        !evidence
            .iter()
            .any(|marker| marker.symbol == "SessionSocialLimits")
    );
}
