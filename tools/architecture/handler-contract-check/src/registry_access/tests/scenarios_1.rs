//! Registry access scan regression scenarios, part 1 of 1.
//!
//! Moved out of the registry_access.rs root under #660; every test is unchanged.

use super::*;

#[test]
fn registry_inventory_tracks_nested_types_aliases_members_methods_clones_and_returns() {
    let baseline = inventory(
        r#"
            use wow_network::{
                GroupRegistry,
                PendingInvites,
                PlayerRegistry as Players,
            };
            use std::sync::Arc;

            type SharedPlayers = Option<Arc<Players>>;

            struct Holder {
                players: SharedPlayers,
                pub(crate) groups: Arc<GroupRegistry>,
            }

            fn expose(
                holder: &Holder,
                invites: &PendingInvites,
            ) -> Arc<Players> {
                holder.groups.get(&7);
                invites.remove(&9);
                let players = Arc::clone(holder.players.as_ref().expect("players"));
                players.iter();
                players
            }
        "#,
    )
    .expect("synthetic registry surface parses");

    let found = operations(&baseline);
    for expected in [
        (
            RegistryKind::Player,
            RegistryOperation::ImportAlias,
            "Players",
        ),
        (
            RegistryKind::Player,
            RegistryOperation::TypeAlias,
            "SharedPlayers",
        ),
        (RegistryKind::Group, RegistryOperation::Member, "groups"),
        (RegistryKind::Group, RegistryOperation::Get, "get"),
        (
            RegistryKind::PendingInvites,
            RegistryOperation::Remove,
            "remove",
        ),
        (RegistryKind::Player, RegistryOperation::Clone, "clone"),
        (
            RegistryKind::Player,
            RegistryOperation::LocalAlias,
            "players",
        ),
        (RegistryKind::Player, RegistryOperation::Iter, "iter"),
        (RegistryKind::Player, RegistryOperation::Return, "registry"),
    ] {
        assert!(
            found.contains(&(expected.0, expected.1, expected.2.to_owned())),
            "missing {expected:?} from {found:#?}"
        );
    }
    assert_eq!(
        serde_json::to_string(&baseline).expect("baseline serializes"),
        serde_json::to_string(
            &inventory(
                r#"
                use wow_network::{GroupRegistry, PendingInvites, PlayerRegistry as Players};
                use std::sync::Arc;
                type SharedPlayers = Option<Arc<Players>>;
                struct Holder { players: SharedPlayers, pub(crate) groups: Arc<GroupRegistry> }
                fn expose(holder: &Holder, invites: &PendingInvites) -> Arc<Players> {
                    holder.groups.get(&7);
                    invites.remove(&9);
                    let players = Arc::clone(holder.players.as_ref().expect("players"));
                    players.iter();
                    players
                }
            "#,
            )
            .unwrap()
        )
        .expect("baseline serializes"),
        "formatting must not perturb the AST inventory"
    );
}

#[test]
fn registry_inventory_resolves_cross_file_alias_reexports() {
    let aliases = ProductionRegistrySource {
        package: "fixture",
        module: "crate::aliases",
        source_path: "src/aliases.rs",
        inherited_cfg: &[],
        source: "pub use wow_world::session::directory::PlayerRegistry as Players;",
    };
    let consumer = ProductionRegistrySource {
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
            use crate::aliases::Players;
            use crate::aliases::Players as Directory;

            fn lookup(players: &Players) {
                players.get(&7);
            }

            fn lookup_renamed(directory: &Directory) {
                directory.get(&8);
            }
        "#,
    };

    let baseline = inventory_registry_accesses(&[consumer, aliases])
        .expect("cross-file alias provenance resolves");
    let consumer_rows: BTreeSet<_> = baseline
        .accesses
        .iter()
        .filter(|record| record.module == "crate::consumer")
        .map(|record| (record.registry, record.operation, record.symbol.clone()))
        .collect();
    for expected in [
        (
            RegistryKind::Player,
            RegistryOperation::ImportAlias,
            "Players",
        ),
        (
            RegistryKind::Player,
            RegistryOperation::TypeReference,
            "players",
        ),
        (
            RegistryKind::Player,
            RegistryOperation::ImportAlias,
            "Directory",
        ),
        (
            RegistryKind::Player,
            RegistryOperation::TypeReference,
            "directory",
        ),
        (RegistryKind::Player, RegistryOperation::Get, "get"),
    ] {
        assert!(
            consumer_rows.contains(&(expected.0, expected.1, expected.2.to_owned())),
            "missing cross-file provenance {expected:?} from {consumer_rows:#?}"
        );
    }

    let reordered = inventory_registry_accesses(&[aliases, consumer])
        .expect("global alias resolution is input-order independent");
    assert_eq!(baseline, reordered);
}

#[test]
fn registry_inventory_follows_accessors_combinators_and_tuple_bindings() {
    let baseline = inventory(
        r#"
            struct Session;
            impl Session {
                fn inspect(&self) {
                    let (Some(players), Some(groups)) =
                        (self.player_registry(), self.group_registry()) else { return; };
                    players.get(&1);
                    groups.get_mut(&2);
                    self.pending_invites().and_then(|invites| invites.get(&3));
                }
            }
        "#,
    )
    .expect("accessor provenance is inspectable");
    let found = operations(&baseline);
    assert!(found.contains(&(
        RegistryKind::Player,
        RegistryOperation::Accessor,
        "player_registry".to_owned()
    )));
    assert!(found.contains(&(
        RegistryKind::Group,
        RegistryOperation::GetMut,
        "get_mut".to_owned()
    )));
    assert!(found.contains(&(
        RegistryKind::PendingInvites,
        RegistryOperation::Get,
        "get".to_owned()
    )));
    assert!(
        !baseline.accesses.iter().any(|record| {
            record.registry == RegistryKind::Group
                && record.operation == RegistryOperation::Get
                && record.fingerprint.starts_with("get(")
        }),
        "tuple binding must not give the player get() group provenance"
    );
}

#[test]
fn registry_inventory_distinguishes_production_from_test_only_cfg() {
    let baseline = inventory(
        r#"
            #[cfg(test)]
            fn test_only(registry: &PlayerRegistry) {
                registry.clear();
            }

            #[cfg(any(test, feature = "fixture"))]
            fn production_capable(registry: &GroupRegistry) {
                registry.retain(|_, _| true);
            }
        "#,
    )
    .expect("cfg-aware source parses");
    assert!(!baseline.accesses.iter().any(|record| {
        record.registry == RegistryKind::Player || record.operation == RegistryOperation::Clear
    }));
    assert!(baseline.accesses.iter().any(|record| {
        record.registry == RegistryKind::Group
            && record.operation == RegistryOperation::Retain
            && record
                .cfg
                .iter()
                .any(|cfg| cfg.contains("feature = \"fixture\""))
    }));
}

#[test]
fn registry_inventory_rejects_globs_and_unknown_macro_escape() {
    let glob =
        inventory("use wow_network::*;\n").expect_err("a registry-capable glob must fail closed");
    assert!(glob.contains("can hide a registry alias"), "{glob}");

    let alias_module = ProductionRegistrySource {
        package: "fixture",
        module: "crate::aliases",
        source_path: "src/aliases.rs",
        inherited_cfg: &[],
        source: "pub use wow_world::session::directory::PlayerRegistry as Players;",
    };
    let glob_consumer = ProductionRegistrySource {
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: "use crate::aliases::*; fn read(players: &Players) { players.get(&1); }",
    };
    let cross_module_glob = inventory_registry_accesses(&[alias_module, glob_consumer])
        .expect("a private cross-module glob must inherit indexed aliases exactly");
    assert!(
        cross_module_glob.accesses.iter().any(|record| {
            record.module == "crate::consumer"
                && record.registry == RegistryKind::Player
                && record.operation == RegistryOperation::Get
        }),
        "inherited Players alias must retain PlayerRegistry provenance"
    );

    let macro_escape = inventory(
        r#"
            fn hidden(players: &PlayerRegistry) {
                hide_access!(players);
            }
        "#,
    )
    .expect_err("an unknown macro cannot consume registry provenance");
    assert!(
        macro_escape.contains("unknown macro hide_access!"),
        "{macro_escape}"
    );

    let macro_definition = inventory(
        r#"
            macro_rules! hidden_alias {
                () => { type Hidden = PlayerRegistry; };
            }
        "#,
    )
    .expect_err("a macro-generated alias cannot bypass the inventory");
    assert!(
        macro_definition.contains("hides registry provenance inside item macro"),
        "{macro_definition}"
    );
}

/// Issue #138 moved the player directory to `wow_world::session::directory`.
/// The glob guard must fail closed on the relocated owner exactly as it
/// already does on the `wow-network` mailbox module, otherwise a single
/// `use ...::directory::*;` would silently reintroduce hidden
/// `PlayerRegistry` access after the move.
#[test]
fn registry_inventory_rejects_relocated_directory_glob() {
    for import in [
        "use wow_world::session::directory::*;\n",
        "use crate::session::directory::*;\n",
    ] {
        let error = inventory(import)
            .expect_err("a glob over the relocated directory owner must fail closed");
        assert!(
            error.contains("can hide a registry alias"),
            "{import} -> {error}"
        );
    }

    inventory("use crate::session::admission::*;\n")
        .expect("an unrelated session submodule glob stays allowed");
}

/// Issue #137 moved the Group owner to `wow_social::group`. The glob guard
/// must fail closed on the relocated owner exactly as it already does on
/// `wow_network`, otherwise one `use ...::group::*;` would silently
/// reintroduce hidden `GroupRegistry`/`PendingInvites` access.
#[test]
fn registry_inventory_rejects_relocated_group_owner_glob() {
    for import in [
        "use wow_social::group::*;\n",
        "use wow_social::*;\n",
        "use crate::group::invites::*;\n",
    ] {
        let error =
            inventory(import).expect_err("a glob over the relocated Group owner must fail closed");
        assert!(
            error.contains("can hide a registry alias"),
            "{import} -> {error}"
        );
    }

    inventory("use crate::handlers::party_ui::*;\n")
        .expect("an unrelated social-adjacent module glob stays allowed");
}

/// Issue #140 moved the Session mailbox to `wow_world::session::mailbox`.
/// Its payloads name registry types, so a glob over the relocated mailbox
/// must fail closed like every other relocated owner.
#[test]
fn registry_inventory_rejects_relocated_mailbox_glob() {
    for import in [
        "use wow_world::session::mailbox::*;\n",
        "use crate::session::mailbox::protocol::*;\n",
    ] {
        let error = inventory(import)
            .expect_err("a glob over the relocated mailbox owner must fail closed");
        assert!(
            error.contains("can hide a registry alias"),
            "{import} -> {error}"
        );
    }
}

#[test]
fn registry_inventory_records_argument_escape_and_index_access() {
    let baseline = inventory(
        r#"
            fn generic<T>(_value: T) {}
            fn escape(players: &PlayerRegistry) {
                generic(players.clone());
                let _ = &players[&7];
            }
        "#,
    )
    .expect("ordinary generic escape remains visible");
    let found = operations(&baseline);
    assert!(found.contains(&(
        RegistryKind::Player,
        RegistryOperation::Clone,
        "clone".to_owned()
    )));
    assert!(found.contains(&(
        RegistryKind::Player,
        RegistryOperation::ArgumentEscape,
        "generic".to_owned()
    )));
    assert!(found.contains(&(
        RegistryKind::Player,
        RegistryOperation::Index,
        "index".to_owned()
    )));
}

#[test]
fn registry_baseline_is_input_order_independent_and_exact() {
    let source_a = ProductionRegistrySource {
        package: "a",
        module: "crate::a",
        source_path: "src/a.rs",
        inherited_cfg: &[],
        source: "fn a(registry: &PlayerRegistry) { registry.get(&1); }",
    };
    let source_b = ProductionRegistrySource {
        package: "b",
        module: "crate::b",
        source_path: "src/b.rs",
        inherited_cfg: &[],
        source: "fn b(registry: &GroupRegistry) { registry.insert(1, value); }",
    };
    let expected = inventory_registry_accesses(&[source_a, source_b]).unwrap();
    let reordered = inventory_registry_accesses(&[source_b, source_a]).unwrap();
    assert_eq!(expected, reordered);
    compare_registry_access_baseline(&expected, &reordered)
        .expect("identical exact baseline passes");

    let changed = inventory_registry_accesses(&[
        source_a,
        ProductionRegistrySource {
            source: "fn b(registry: &GroupRegistry) { registry.remove(&1); }",
            ..source_b
        },
    ])
    .unwrap();
    let error = compare_registry_access_baseline(&expected, &changed)
        .expect_err("same-count operation swap must fail");
    assert!(
        error.contains("untracked direct registry access"),
        "{error}"
    );
    assert!(
        error.contains("obsolete direct registry baseline row"),
        "{error}"
    );
}

#[test]
fn registry_baseline_rejects_multiplicity_and_noncanonical_rows() {
    let expected =
        inventory("fn f(registry: &PlayerRegistry) { registry.get(&1); registry.get(&1); }")
            .unwrap();
    let actual = inventory("fn f(registry: &PlayerRegistry) { registry.get(&1); }").unwrap();
    let error = compare_registry_access_baseline(&expected, &actual)
        .expect_err("multiplicity reduction needs an explicit baseline cleanup");
    assert!(error.contains("multiplicity changed"), "{error}");

    let mut invalid = actual.clone();
    invalid.accesses.reverse();
    let error = compare_registry_access_baseline(&invalid, &actual)
        .expect_err("checked-in rows must remain deterministic");
    assert!(error.contains("strict canonical order"), "{error}");

    let mut zero = actual.clone();
    zero.accesses[0].count = 0;
    let error = compare_registry_access_baseline(&zero, &actual)
        .expect_err("zero-count rows are meaningless");
    assert!(error.contains("zero-count row"), "{error}");
}
