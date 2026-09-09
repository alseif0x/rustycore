//! Workspace and dependency-scoped resolution caches.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

/// Parse and inventory already-classified production/test source mounts.
/// Source order is irrelevant and duplicate logical mounts fail closed. Each
/// mount is analyzed once with `cfg(test) = false` and once with
/// `cfg(test) = true`; the test pass retains only syntax that cannot exist in
/// production, so shared imports and helpers are not double-counted.
pub(super) fn workspace_named_type_info(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    source_class: PersistenceSourceClass,
    provider_roots: &BTreeSet<String>,
) -> BTreeMap<String, VariableInfo> {
    let mut workspace = BTreeMap::<String, VariableInfo>::new();
    for ((provider, candidate_class), named_types) in registries {
        if *candidate_class != source_class {
            continue;
        }
        let crate_name = provider.replace('-', "_");
        if !provider_roots.contains(&crate_name) {
            continue;
        }
        for (path, info) in named_types {
            workspace
                .entry(format!("{crate_name}::{path}"))
                .or_default()
                .union(info);
        }
    }
    workspace
}

pub(super) fn workspace_named_type_info_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, VariableInfo>>> {
    dependencies
        .production
        .iter()
        .map(|(package, aliases)| {
            let roots = aliases.values().cloned().collect();
            (
                (package.clone(), PersistenceSourceClass::Production),
                std::sync::Arc::new(workspace_named_type_info(
                    registries,
                    PersistenceSourceClass::Production,
                    &roots,
                )),
            )
        })
        .chain(dependencies.test.iter().map(|(package, aliases)| {
            let roots = aliases.values().cloned().collect();
            (
                (package.clone(), PersistenceSourceClass::TestFixture),
                std::sync::Arc::new(workspace_named_type_info(
                    registries,
                    PersistenceSourceClass::TestFixture,
                    &roots,
                )),
            )
        }))
        .collect()
}

pub(super) fn dependency_sorted_packages(
    sources: &[ClassifiedPersistenceSource<'_>],
    dependencies: &WorkspaceDependencyAliases,
) -> Vec<String> {
    let mut remaining = sources
        .iter()
        .map(|source| source.package.to_owned())
        .collect::<BTreeSet<_>>();
    let package_by_root = remaining
        .iter()
        .map(|package| (package.replace('-', "_"), package.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let ready = remaining.iter().find(|package| {
            dependencies
                .production
                .get(*package)
                .into_iter()
                .chain(dependencies.test.get(*package))
                .flat_map(|aliases| aliases.values())
                .filter_map(|root| package_by_root.get(root))
                .all(|provider| !remaining.contains(provider) || provider == *package)
        });
        // A dependency cycle cannot be topologically ordered. Pick its stable
        // first member; the outer fixed-point loop still converges the cycle.
        let package = ready
            .cloned()
            .unwrap_or_else(|| remaining.first().expect("remaining is non-empty").clone());
        remaining.remove(&package);
        ordered.push(package);
    }
    ordered
}

pub(super) fn package_named_type_info_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, VariableInfo>>> {
    registries
        .iter()
        .map(|(key, info)| (key.clone(), std::sync::Arc::new(info.clone())))
        .collect()
}

/// Refresh only the cache entries a finished package can have changed.
///
/// Both caches are pure functions of the registry, and analyzing one package
/// rewrites only that package's own entries. Rebuilding them from scratch once
/// per package re-cloned and re-merged the entire registry O(packages) times
/// per fixpoint iteration; the merged workspace view additionally has to be
/// rebuilt only for consumers that can actually see the changed provider.
pub(super) fn refresh_named_type_caches(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    dependencies: &WorkspaceDependencyAliases,
    changed_package: &str,
    workspace_cache: &mut BTreeMap<
        (String, PersistenceSourceClass),
        std::sync::Arc<BTreeMap<String, VariableInfo>>,
    >,
    package_cache: &mut BTreeMap<
        (String, PersistenceSourceClass),
        std::sync::Arc<BTreeMap<String, VariableInfo>>,
    >,
) {
    for source_class in [
        PersistenceSourceClass::Production,
        PersistenceSourceClass::TestFixture,
    ] {
        let key = (changed_package.to_owned(), source_class);
        match registries.get(&key) {
            Some(info) => {
                package_cache.insert(key, std::sync::Arc::new(info.clone()));
            }
            None => {
                package_cache.remove(&key);
            }
        }
    }
    let changed_crate = changed_package.replace('-', "_");
    for (aliases_by_package, source_class) in [
        (&dependencies.production, PersistenceSourceClass::Production),
        (&dependencies.test, PersistenceSourceClass::TestFixture),
    ] {
        for (consumer, aliases) in aliases_by_package {
            let roots = aliases.values().cloned().collect::<BTreeSet<_>>();
            if !roots.contains(&changed_crate) {
                continue;
            }
            workspace_cache.insert(
                (consumer.clone(), source_class),
                std::sync::Arc::new(workspace_named_type_info(registries, source_class, &roots)),
            );
        }
    }
}

pub(super) fn dependency_alias_cache(
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, String>>> {
    dependencies
        .production
        .iter()
        .map(|(package, aliases)| {
            (
                (package.clone(), PersistenceSourceClass::Production),
                std::sync::Arc::new(aliases.clone()),
            )
        })
        .chain(dependencies.test.iter().map(|(package, aliases)| {
            (
                (package.clone(), PersistenceSourceClass::TestFixture),
                std::sync::Arc::new(aliases.clone()),
            )
        }))
        .collect()
}

pub(super) fn resolve_public_sqlx_namespace_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
) -> BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>> {
    let mut registries = reexports
        .keys()
        .cloned()
        .map(|key| (key, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        for (consumer_key, aliases) in reexports {
            for (export, source) in aliases {
                let direct = source == "sqlx"
                    || before
                        .get(consumer_key)
                        .is_some_and(|known| known.contains(source));
                let forwarded = source.split_once("::").is_some_and(|(root, remainder)| {
                    before.iter().any(|((provider, source_class), known)| {
                        *source_class == consumer_key.1
                            && provider.replace('-', "_") == root
                            && known.contains(remainder)
                    })
                });
                if direct || forwarded {
                    registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .insert(export.clone());
                }
            }
        }
        if registries == before {
            break;
        }
    }
    registries
}

pub(super) fn workspace_sqlx_namespace_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>>,
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeSet<String>>> {
    dependencies
        .production
        .iter()
        .map(|(package, aliases)| {
            let key = (package.clone(), PersistenceSourceClass::Production);
            let mut namespaces = registries.get(&key).cloned().unwrap_or_default();
            for provider_root in aliases.values() {
                if let Some((_, provider_namespaces)) =
                    registries.iter().find(|((provider, source_class), _)| {
                        *source_class == PersistenceSourceClass::Production
                            && provider.replace('-', "_") == *provider_root
                    })
                {
                    namespaces.extend(
                        provider_namespaces
                            .iter()
                            .map(|path| format!("{provider_root}::{path}")),
                    );
                }
            }
            (key, std::sync::Arc::new(namespaces))
        })
        .chain(dependencies.test.iter().map(|(package, aliases)| {
            let key = (package.clone(), PersistenceSourceClass::TestFixture);
            let mut namespaces = registries.get(&key).cloned().unwrap_or_default();
            for provider_root in aliases.values() {
                if let Some((_, provider_namespaces)) =
                    registries.iter().find(|((provider, source_class), _)| {
                        *source_class == PersistenceSourceClass::TestFixture
                            && provider.replace('-', "_") == *provider_root
                    })
                {
                    namespaces.extend(
                        provider_namespaces
                            .iter()
                            .map(|path| format!("{provider_root}::{path}")),
                    );
                }
            }
            (key, std::sync::Arc::new(namespaces))
        }))
        .collect()
}

pub(super) fn qualify_dependency_shape(
    shape: &mut NominalShape,
    provider_root: &str,
    provider_named_types: &BTreeMap<String, VariableInfo>,
) {
    shape.nominal_types = std::mem::take(&mut shape.nominal_types)
        .into_iter()
        .map(|name| {
            if provider_named_types.contains_key(&name) {
                format!("{provider_root}::{name}")
            } else {
                name
            }
        })
        .collect();
    for argument in &mut shape.arguments {
        qualify_dependency_shape(argument, provider_root, provider_named_types);
    }
}

pub(super) fn qualify_dependency_info(
    provider_root: &str,
    provider_named_types: &BTreeMap<String, VariableInfo>,
    info: &VariableInfo,
) -> VariableInfo {
    let mut qualified = info.clone();
    qualified.nominal_types = std::mem::take(&mut qualified.nominal_types)
        .into_iter()
        .map(|name| {
            if provider_named_types.contains_key(&name) {
                format!("{provider_root}::{name}")
            } else {
                name
            }
        })
        .collect();
    qualified.payload_variants = std::mem::take(&mut qualified.payload_variants)
        .into_iter()
        .map(|mut variant| {
            for shape in &mut variant {
                qualify_dependency_shape(shape, provider_root, provider_named_types);
            }
            variant
        })
        .collect();
    for item in &mut qualified.tuple_items {
        *item = qualify_dependency_info(provider_root, provider_named_types, item);
    }
    for item in qualified.field_items.values_mut() {
        *item = qualify_dependency_info(provider_root, provider_named_types, item);
    }
    qualified
}

pub(super) fn resolve_public_named_type_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    dependencies: &WorkspaceDependencyAliases,
    registries: &mut BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>,
    public_paths: &mut BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>>,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        let snapshot = registries.clone();
        let public_snapshot = public_paths.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, provider_entry, provider_root) =
                    if let Some(provider_root) = dependency_root {
                        let provider = snapshot.keys().find_map(|(candidate, candidate_class)| {
                            (*candidate_class == source_class
                                && candidate.replace('-', "_") == provider_root.as_str())
                            .then_some(candidate.clone())
                        });
                        let Some(provider) = provider else {
                            continue;
                        };
                        (
                            (provider, source_class),
                            source_parts.collect::<Vec<_>>().join("::"),
                            Some(provider_root.as_str()),
                        )
                    } else {
                        (consumer_key.clone(), source.to_owned(), None)
                    };
                let Some(provider_registry) = snapshot.get(&provider_key) else {
                    continue;
                };
                let provider_public = public_snapshot
                    .get(&provider_key)
                    .cloned()
                    .unwrap_or_default();
                let entries = if glob {
                    let prefix = (!provider_entry.is_empty())
                        .then(|| format!("{provider_entry}::"))
                        .unwrap_or_default();
                    provider_registry
                        .iter()
                        .filter_map(|(entry, info)| {
                            if !provider_public.contains(entry) {
                                return None;
                            }
                            entry.strip_prefix(&prefix).map(|suffix| {
                                let exported = if export.is_empty() {
                                    suffix.to_owned()
                                } else {
                                    format!("{export}::{suffix}")
                                };
                                (exported, info.clone())
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut entries = Vec::new();
                    if provider_public.contains(&provider_entry)
                        && let Some(info) = provider_registry.get(&provider_entry)
                    {
                        entries.push((export.to_owned(), info.clone()));
                    }
                    let prefix = format!("{provider_entry}::");
                    entries.extend(provider_registry.iter().filter_map(|(entry, info)| {
                        if !provider_public.contains(entry) {
                            return None;
                        }
                        entry.strip_prefix(&prefix).map(|suffix| {
                            let exported = if export.is_empty() {
                                suffix.to_owned()
                            } else {
                                format!("{export}::{suffix}")
                            };
                            (exported, info.clone())
                        })
                    }));
                    entries
                };
                for (exported, source_info) in entries {
                    let info = if let Some(provider_root) = provider_root {
                        qualify_dependency_info(provider_root, provider_registry, &source_info)
                    } else {
                        source_info
                    };
                    registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .entry(exported.clone())
                        .or_default()
                        .union(&info);
                    public_paths
                        .entry(consumer_key.clone())
                        .or_default()
                        .insert(exported);
                }
            }
        }
        if registries == &before {
            break;
        }
    }
}

pub(super) fn resolve_public_macro_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    dependencies: &WorkspaceDependencyAliases,
    registries: &mut BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, TargetSet>>,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        let snapshot = registries.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, provider_entry) = if let Some(provider_root) = dependency_root {
                    let provider = snapshot.keys().find_map(|(candidate, candidate_class)| {
                        (*candidate_class == source_class
                            && candidate.replace('-', "_") == provider_root.as_str())
                        .then_some(candidate.clone())
                    });
                    let Some(provider) = provider else {
                        continue;
                    };
                    (
                        (provider, source_class),
                        source_parts.collect::<Vec<_>>().join("::"),
                    )
                } else {
                    (consumer_key.clone(), source.to_owned())
                };
                let Some(provider_registry) = snapshot.get(&provider_key) else {
                    continue;
                };
                let entries = if glob {
                    let prefix = (!provider_entry.is_empty())
                        .then(|| format!("{provider_entry}::"))
                        .unwrap_or_default();
                    provider_registry
                        .iter()
                        .filter_map(|(entry, targets)| {
                            entry.strip_prefix(&prefix).map(|suffix| {
                                let exported = if export.is_empty() {
                                    suffix.to_owned()
                                } else {
                                    format!("{export}::{suffix}")
                                };
                                (exported, targets.clone())
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut entries = provider_registry
                        .get(&provider_entry)
                        .cloned()
                        .map(|targets| vec![(export.to_owned(), targets)])
                        .unwrap_or_default();
                    let prefix = if provider_entry.is_empty() {
                        String::new()
                    } else {
                        format!("{provider_entry}::")
                    };
                    entries.extend(provider_registry.iter().filter_map(|(entry, targets)| {
                        entry.strip_prefix(&prefix).map(|suffix| {
                            let exported = if export.is_empty() {
                                suffix.to_owned()
                            } else {
                                format!("{export}::{suffix}")
                            };
                            (exported, targets.clone())
                        })
                    }));
                    entries
                };
                for (exported, targets) in entries {
                    registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .entry(exported)
                        .or_default()
                        .extend(targets);
                }
            }
        }
        if registries == &before {
            break;
        }
    }
}

pub(super) type TraitSignatureRegistry = (
    BTreeMap<(String, String), VariableInfo>,
    BTreeMap<String, BTreeSet<String>>,
    BTreeMap<String, Vec<String>>,
    BTreeMap<(String, String), Vec<String>>,
    BTreeMap<(String, String), Vec<GenericInputSpec>>,
);

pub(super) fn resolve_public_trait_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    named_type_registries: &BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    dependencies: &WorkspaceDependencyAliases,
    registries: &mut BTreeMap<(String, PersistenceSourceClass), TraitSignatureRegistry>,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = registries.clone();
        let snapshot = registries.clone();
        for (consumer_key, aliases) in reexports {
            let (consumer, source_class) = (&consumer_key.0, consumer_key.1);
            let dependency_roots = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(consumer),
                PersistenceSourceClass::TestFixture => dependencies.test.get(consumer),
            };
            for (export, source) in aliases {
                let glob = (source == "*" || source.ends_with("::*"))
                    && (export == "*" || export.ends_with("::*"));
                let source = if source == "*" {
                    ""
                } else {
                    source.strip_suffix("::*").unwrap_or(source)
                };
                let export = if export == "*" {
                    ""
                } else {
                    export.strip_suffix("::*").unwrap_or(export)
                };
                let mut source_parts = source.split("::");
                let source_root = source_parts.next().unwrap_or_default();
                let dependency_root = dependency_roots
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                    .find(|root| root.as_str() == source_root);
                let (provider_key, source_trait, provider_root) =
                    if let Some(provider_root) = dependency_root {
                        let provider = snapshot.keys().find_map(|(candidate, candidate_class)| {
                            (*candidate_class == source_class
                                && candidate.replace('-', "_") == provider_root.as_str())
                            .then_some(candidate.clone())
                        });
                        let Some(provider) = provider else {
                            continue;
                        };
                        (
                            (provider, source_class),
                            source_parts.collect::<Vec<_>>().join("::"),
                            Some(provider_root.as_str()),
                        )
                    } else {
                        (consumer_key.clone(), source.to_owned(), None)
                    };
                let Some(provider) = snapshot.get(&provider_key) else {
                    continue;
                };
                let mut provider_traits = provider
                    .0
                    .keys()
                    .map(|(trait_path, _)| trait_path.clone())
                    .collect::<BTreeSet<_>>();
                provider_traits.extend(provider.1.keys().cloned());
                provider_traits.extend(provider.2.keys().cloned());
                provider_traits.extend(provider.3.keys().map(|(path, _)| path.clone()));
                provider_traits.extend(provider.4.keys().map(|(path, _)| path.clone()));
                let source_prefix = (!source_trait.is_empty())
                    .then(|| format!("{source_trait}::"))
                    .unwrap_or_default();
                let candidates = provider_traits
                    .into_iter()
                    .filter_map(|trait_path| {
                        if glob {
                            let suffix = trait_path.strip_prefix(&source_prefix)?.to_owned();
                            let exported = if export.is_empty() {
                                suffix
                            } else {
                                format!("{export}::{suffix}")
                            };
                            Some((exported, trait_path))
                        } else {
                            (trait_path == source_trait).then(|| (export.to_owned(), trait_path))
                        }
                    })
                    .collect::<Vec<_>>();
                for (exported_trait, source_trait) in candidates {
                    let mut pending = vec![source_trait.clone()];
                    let mut inherited = BTreeSet::new();
                    while let Some(trait_path) = pending.pop() {
                        if !inherited.insert(trait_path.clone()) {
                            continue;
                        }
                        if let Some(supertraits) = provider.1.get(&trait_path) {
                            pending.extend(supertraits.iter().cloned());
                        }
                    }
                    let target = registries.entry(consumer_key.clone()).or_default();
                    for inherited_trait in inherited {
                        for ((trait_path, method), info) in &provider.0 {
                            if trait_path != &inherited_trait {
                                continue;
                            }
                            let info = if let Some(provider_root) = provider_root {
                                let named = named_type_registries
                                    .get(&provider_key)
                                    .cloned()
                                    .unwrap_or_default();
                                qualify_dependency_info(provider_root, &named, info)
                            } else {
                                info.clone()
                            };
                            target
                                .0
                                .entry((exported_trait.clone(), method.clone()))
                                .or_default()
                                .union(&info);
                            if let Some(params) =
                                provider.3.get(&(trait_path.clone(), method.clone()))
                            {
                                target
                                    .3
                                    .entry((exported_trait.clone(), method.clone()))
                                    .or_insert_with(|| params.clone());
                            }
                            if let Some(inputs) =
                                provider.4.get(&(trait_path.clone(), method.clone()))
                            {
                                target
                                    .4
                                    .entry((exported_trait.clone(), method.clone()))
                                    .or_insert_with(|| inputs.clone());
                            }
                        }
                    }
                    if let Some(params) = provider.2.get(&source_trait) {
                        target
                            .2
                            .entry(exported_trait)
                            .or_insert_with(|| params.clone());
                    }
                }
            }
        }
        if registries == &before {
            break;
        }
    }
}

pub(super) fn dependency_scoped_trait_supertrait_cache(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<String, BTreeSet<String>>>,
    trait_names: &BTreeMap<(String, PersistenceSourceClass), BTreeSet<String>>,
    dependencies: &WorkspaceDependencyAliases,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, BTreeSet<String>>>>
{
    let mut consumers = registries.keys().cloned().collect::<BTreeSet<_>>();
    consumers.extend(
        dependencies
            .production
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::Production)),
    );
    consumers.extend(
        dependencies
            .test
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::TestFixture)),
    );
    consumers
        .into_iter()
        .map(|key| {
            let (package, source_class) = (&key.0, key.1);
            let mut scoped = registries.get(&key).cloned().unwrap_or_default();
            let aliases = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(package),
                PersistenceSourceClass::TestFixture => dependencies.test.get(package),
            };
            for provider_root in aliases
                .into_iter()
                .flat_map(|aliases| aliases.values())
                .collect::<BTreeSet<_>>()
            {
                let provider = registries.keys().find_map(|(candidate, candidate_class)| {
                    (*candidate_class == source_class
                        && candidate.replace('-', "_") == provider_root.as_str())
                    .then_some(candidate)
                });
                let Some(provider) = provider else {
                    continue;
                };
                let provider_key = (provider.clone(), source_class);
                let provider_names = trait_names.get(&provider_key);
                if let Some(entries) = registries.get(&provider_key) {
                    for (trait_path, supertraits) in entries {
                        scoped.insert(
                            format!("{provider_root}::{trait_path}"),
                            supertraits
                                .iter()
                                .map(|supertrait| {
                                    if provider_names
                                        .is_some_and(|names| names.contains(supertrait))
                                    {
                                        format!("{provider_root}::{supertrait}")
                                    } else {
                                        supertrait.clone()
                                    }
                                })
                                .collect(),
                        );
                    }
                }
            }
            (key, std::sync::Arc::new(scoped))
        })
        .collect()
}

pub(super) fn dependency_scoped_registry_cache<K, V, QualifyKey, QualifyValue>(
    registries: &BTreeMap<(String, PersistenceSourceClass), BTreeMap<K, V>>,
    named_type_registries: &BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    dependencies: &WorkspaceDependencyAliases,
    qualify_key: QualifyKey,
    qualify_value: QualifyValue,
) -> BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<K, V>>>
where
    K: Clone + Ord,
    V: Clone,
    QualifyKey: Fn(&str, &K) -> K,
    QualifyValue: Fn(&str, &BTreeMap<String, VariableInfo>, &V) -> V,
{
    let mut consumers = registries.keys().cloned().collect::<BTreeSet<_>>();
    consumers.extend(
        dependencies
            .production
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::Production)),
    );
    consumers.extend(
        dependencies
            .test
            .keys()
            .cloned()
            .map(|package| (package, PersistenceSourceClass::TestFixture)),
    );
    consumers
        .into_iter()
        .map(|key| {
            let (package, source_class) = (&key.0, key.1);
            let mut scoped = registries.get(&key).cloned().unwrap_or_default();
            let aliases = match source_class {
                PersistenceSourceClass::Production => dependencies.production.get(package),
                PersistenceSourceClass::TestFixture => dependencies.test.get(package),
            };
            for provider_root in aliases
                .into_iter()
                .flat_map(|aliases| aliases.values())
                .collect::<BTreeSet<_>>()
            {
                let provider = registries.keys().find_map(|(candidate, candidate_class)| {
                    (*candidate_class == source_class
                        && candidate.replace('-', "_") == provider_root.as_str())
                    .then_some(candidate)
                });
                let Some(provider) = provider else {
                    continue;
                };
                let provider_key = (provider.clone(), source_class);
                let provider_named_types = named_type_registries
                    .get(&provider_key)
                    .cloned()
                    .unwrap_or_default();
                if let Some(entries) = registries.get(&provider_key) {
                    for (entry_key, value) in entries {
                        scoped.insert(
                            qualify_key(provider_root, entry_key),
                            qualify_value(provider_root, &provider_named_types, value),
                        );
                    }
                }
            }
            (key, std::sync::Arc::new(scoped))
        })
        .collect()
}
