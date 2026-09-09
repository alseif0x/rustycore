//! The persistence-access inventory entry points.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[cfg(test)]
pub(crate) fn inventory_persistence_accesses(
    sources: &[ClassifiedPersistenceSource<'_>],
) -> Result<PersistenceAccessBaseline, String> {
    inventory_persistence_accesses_with_dependencies(
        sources,
        &WorkspaceDependencyAliases::default(),
    )
}

pub(crate) fn inventory_persistence_accesses_with_dependencies(
    sources: &[ClassifiedPersistenceSource<'_>],
    dependencies: &WorkspaceDependencyAliases,
) -> Result<PersistenceAccessBaseline, String> {
    let mut ordered = sources.to_vec();
    let package_order = dependency_sorted_packages(sources, dependencies);
    let package_order = package_order
        .into_iter()
        .enumerate()
        .map(|(index, package)| (package, index))
        .collect::<BTreeMap<_, _>>();
    ordered.sort_by(|left, right| {
        (
            package_order.get(left.package),
            left.classification,
            left.package,
            left.module,
            left.source_path,
            left.inherited_cfg,
        )
            .cmp(&(
                package_order.get(right.package),
                right.classification,
                right.package,
                right.module,
                right.source_path,
                right.inherited_cfg,
            ))
    });
    let mut seen_mounts = BTreeSet::new();
    let mut accumulator = AccessAccumulator::default();
    let mut errors = Vec::new();
    let dependency_aliases = dependency_alias_cache(dependencies);
    let mut named_type_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>::new();
    for _ in 0..=ordered.len() {
        let mut next = named_type_registries.clone();
        let mut workspace_cache = workspace_named_type_info_cache(&next, dependencies);
        let mut package_cache = package_named_type_info_cache(&next);
        let mut package_start = 0;
        while package_start < ordered.len() {
            let package = ordered[package_start].package;
            let package_end = ordered[package_start..]
                .iter()
                .position(|source| source.package != package)
                .map_or(ordered.len(), |offset| package_start + offset);
            for source in &ordered[package_start..package_end] {
                let Ok(syntax) = syn::parse_file(source.source) else {
                    continue;
                };
                let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
                for source_class in [
                    PersistenceSourceClass::Production,
                    PersistenceSourceClass::TestFixture,
                ] {
                    if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                        continue;
                    }
                    let key = (source.package.to_owned(), source_class);
                    let mut base = ModuleSymbols::for_package(source.package);
                    base.named_type_info = package_cache
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
                    base.workspace_named_type_info = workspace_cache
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
                    base.dependency_crate_aliases = dependency_aliases
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
                    let output = next.entry(key).or_default();
                    collect_named_type_info(
                        &syntax.items,
                        &base,
                        source.package,
                        source.module,
                        &cfg,
                        source_class,
                        &mut errors,
                        output,
                    );
                }
            }
            package_start = package_end;
            refresh_named_type_caches(
                &next,
                dependencies,
                package,
                &mut workspace_cache,
                &mut package_cache,
            );
        }
        let converged = next == named_type_registries;
        named_type_registries = next;
        if converged {
            break;
        }
    }
    let mut callable_reexports =
        BTreeMap::<(String, PersistenceSourceClass), Vec<(String, String)>>::new();
    let mut local_callable_imports = BTreeMap::new();
    let mut public_named_type_paths =
        BTreeMap::<(String, PersistenceSourceClass), BTreeSet<String>>::new();
    let initial_named_type_workspace_cache =
        workspace_named_type_info_cache(&named_type_registries, dependencies);
    let initial_named_type_package_cache = package_named_type_info_cache(&named_type_registries);
    for source in &ordered {
        let Ok(syntax) = syn::parse_file(source.source) else {
            continue;
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                continue;
            }
            let key = (source.package.to_owned(), source_class);
            let mut base = ModuleSymbols::for_package(source.package);
            base.named_type_info = initial_named_type_package_cache
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_named_type_info = initial_named_type_workspace_cache
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.dependency_crate_aliases = dependency_aliases
                .get(&key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let symbols = collect_module_symbols(
                &syntax.items,
                Some(&base),
                source.package,
                source.module,
                &cfg,
                source_class,
                &mut errors,
            );
            collect_public_callable_reexports(
                &syntax.items,
                &symbols,
                &cfg,
                source_class,
                &mut errors,
                callable_reexports.entry(key.clone()).or_default(),
            );
            collect_local_callable_imports(
                &syntax.items,
                &symbols,
                &cfg,
                source_class,
                &mut errors,
                local_callable_imports.entry(key.clone()).or_default(),
            );
            collect_public_named_type_paths(
                &syntax.items,
                &symbols,
                &cfg,
                source_class,
                &mut errors,
                public_named_type_paths.entry(key).or_default(),
            );
        }
    }
    let sqlx_namespace_registries = resolve_public_sqlx_namespace_reexports(&callable_reexports);
    let sqlx_namespace_cache =
        workspace_sqlx_namespace_cache(&sqlx_namespace_registries, dependencies);
    resolve_public_named_type_reexports(
        &callable_reexports,
        dependencies,
        &mut named_type_registries,
        &mut public_named_type_paths,
    );
    let named_type_workspace_cache =
        workspace_named_type_info_cache(&named_type_registries, dependencies);
    let named_type_package_cache = package_named_type_info_cache(&named_type_registries);
    let mut trait_registries =
        BTreeMap::<(String, PersistenceSourceClass), TraitSignatureRegistry>::new();
    let mut function_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>::new();
    let mut function_mutable_write_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<String, BTreeMap<usize, VariableInfo>>,
    >::new();
    let mut function_generic_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, Vec<String>>>::new();
    let mut function_generic_input_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, Vec<GenericInputSpec>>>::new(
        );
    let mut method_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), VariableInfo>,
    >::new();
    let mut method_mutable_receiver_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), VariableInfo>,
    >::new();
    let mut method_mutable_write_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), BTreeMap<usize, VariableInfo>>,
    >::new();
    let mut method_generic_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<(String, String), Vec<String>>>::new(
        );
    let mut method_generic_input_registries = BTreeMap::<
        (String, PersistenceSourceClass),
        BTreeMap<(String, String), Vec<GenericInputSpec>>,
    >::new();
    let mut item_value_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, VariableInfo>>::new();
    let mut macro_registries =
        BTreeMap::<(String, PersistenceSourceClass), BTreeMap<String, TargetSet>>::new();
    // Trait declarations and their consumers may live in different physical
    // files (`mod maker;`). Build a package-wide, cfg-aware signature registry
    // before analyzing any body so source order cannot hide bounded returns.
    // Free functions get the same canonical-path treatment: a qualified call
    // such as `crate::factory::database()` must resolve its return flow no
    // matter which file declares the function.
    for source in &ordered {
        let Ok(syntax) = syn::parse_file(source.source) else {
            continue;
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                continue;
            }
            let mut base = ModuleSymbols::for_package(source.package);
            base.named_type_info = named_type_package_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_named_type_info = named_type_workspace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.dependency_crate_aliases = dependency_aliases
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_sqlx_namespaces = sqlx_namespace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeSet::new()));
            let mut symbols = collect_module_symbols(
                &syntax.items,
                Some(&base),
                source.package,
                source.module,
                &cfg,
                source_class,
                &mut errors,
            );
            collect_nested_trait_returns(
                &syntax.items,
                &symbols.module_path.clone(),
                &cfg,
                source_class,
                &mut symbols,
                &mut errors,
            );
            let function_registry = function_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            let module_prefix = symbols.module_path.join("::");
            let item_value_registry = item_value_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            collect_nested_item_values(
                &syntax.items,
                &symbols.module_path,
                source.package,
                &cfg,
                source_class,
                &symbols,
                item_value_registry,
                &mut errors,
            );
            let macro_registry = macro_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, targets) in &symbols.persistence_macros {
                macro_registry
                    .entry(name.clone())
                    .or_default()
                    .extend(targets.iter().copied());
            }
            for (name, info) in symbols.function_returns.iter() {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                function_registry.entry(canonical).or_default().union(info);
            }
            let function_mutable_write_registry = function_mutable_write_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, effects) in &symbols.function_mutable_writes {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                let target = function_mutable_write_registry
                    .entry(canonical)
                    .or_default();
                for (index, info) in effects {
                    target.entry(*index).or_default().union(info);
                }
            }
            let function_generic_registry = function_generic_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, generic_params) in symbols.function_generic_params.iter() {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                function_generic_registry
                    .entry(canonical)
                    .or_insert_with(|| generic_params.clone());
            }
            let function_generic_input_registry = function_generic_input_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (name, input_params) in symbols.function_generic_input_params.iter() {
                let canonical = if module_prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{module_prefix}::{name}")
                };
                function_generic_input_registry
                    .entry(canonical)
                    .or_insert_with(|| input_params.clone());
            }
            let registry = trait_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for (key, info) in symbols.trait_method_returns.iter() {
                registry.0.entry(key.clone()).or_default().union(info);
            }
            for (trait_path, supertraits) in symbols.trait_supertraits.iter() {
                registry
                    .1
                    .entry(trait_path.clone())
                    .or_default()
                    .extend(supertraits.iter().cloned());
            }
            for (trait_path, generic_params) in symbols.trait_generic_params.iter() {
                registry
                    .2
                    .entry(trait_path.clone())
                    .or_insert_with(|| generic_params.clone());
            }
            for (key, generic_params) in symbols.trait_method_generic_params.iter() {
                registry
                    .3
                    .entry(key.clone())
                    .or_insert_with(|| generic_params.clone());
            }
            for (key, input_params) in symbols.trait_method_generic_input_params.iter() {
                registry
                    .4
                    .entry(key.clone())
                    .or_insert_with(|| input_params.clone());
            }
            // Inherent (non-trait) impl methods also get a package-wide
            // registry keyed by canonical owner path, so a call like
            // `factory.make()` resolves when the impl lives in another module.
            let method_registry = method_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), info) in symbols.method_returns.iter() {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default()
                    .union(info);
            }
            let mutable_receiver_registry = method_mutable_receiver_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), info) in &symbols.method_mutable_receivers {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                mutable_receiver_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default()
                    .union(info);
            }
            let mutable_write_registry = method_mutable_write_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), effects) in &symbols.method_mutable_writes {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                let target = mutable_write_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default();
                for (index, info) in effects {
                    target.entry(*index).or_default().union(info);
                }
            }
            let method_generic_registry = method_generic_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), generic_params) in
                symbols.method_generic_params.iter()
            {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_generic_registry
                    .entry((canonical_owner, method.clone()))
                    .or_insert_with(|| generic_params.clone());
            }
            let method_generic_input_registry = method_generic_input_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, trait_name, method), input_params) in
                symbols.method_generic_input_params.iter()
            {
                if trait_name.is_some() {
                    continue;
                }
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_generic_input_registry
                    .entry((canonical_owner, method.clone()))
                    .or_insert_with(|| input_params.clone());
            }
        }
    }
    resolve_public_callable_reexports(
        &callable_reexports,
        &named_type_registries,
        dependencies,
        &mut function_registries,
        &mut function_generic_registries,
        &mut function_generic_input_registries,
    );
    let mut item_value_generic_registries = BTreeMap::new();
    let mut item_value_generic_input_registries = BTreeMap::new();
    resolve_public_callable_reexports(
        &callable_reexports,
        &named_type_registries,
        dependencies,
        &mut item_value_registries,
        &mut item_value_generic_registries,
        &mut item_value_generic_input_registries,
    );
    resolve_public_macro_reexports(&callable_reexports, dependencies, &mut macro_registries);
    resolve_public_trait_reexports(
        &callable_reexports,
        &named_type_registries,
        dependencies,
        &mut trait_registries,
    );
    let trait_method_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.0.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_supertrait_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.1.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_generic_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.2.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_method_generic_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.3.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_method_generic_input_registries = trait_registries
        .iter()
        .map(|(key, registry)| (key.clone(), registry.4.clone()))
        .collect::<BTreeMap<_, _>>();
    let trait_names = trait_registries
        .iter()
        .map(|(key, registry)| {
            let mut names = registry
                .0
                .keys()
                .map(|(trait_path, _)| trait_path.clone())
                .collect::<BTreeSet<_>>();
            names.extend(registry.1.keys().cloned());
            names.extend(registry.2.keys().cloned());
            names.extend(registry.3.keys().map(|(trait_path, _)| trait_path.clone()));
            names.extend(registry.4.keys().map(|(trait_path, _)| trait_path.clone()));
            (key.clone(), names)
        })
        .collect::<BTreeMap<_, _>>();
    let trait_method_registry_cache = dependency_scoped_registry_cache(
        &trait_method_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (trait_path, method)| {
            (format!("{provider_root}::{trait_path}"), method.clone())
        },
        qualify_dependency_info,
    );
    let trait_supertrait_registry_cache = dependency_scoped_trait_supertrait_cache(
        &trait_supertrait_registries,
        &trait_names,
        dependencies,
    );
    let trait_generic_registry_cache = dependency_scoped_registry_cache(
        &trait_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, trait_path| format!("{provider_root}::{trait_path}"),
        |_, _, value| value.clone(),
    );
    let trait_method_generic_registry_cache = dependency_scoped_registry_cache(
        &trait_method_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (trait_path, method)| {
            (format!("{provider_root}::{trait_path}"), method.clone())
        },
        |_, _, value| value.clone(),
    );
    let trait_method_generic_input_registry_cache = dependency_scoped_registry_cache(
        &trait_method_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (trait_path, method)| {
            (format!("{provider_root}::{trait_path}"), method.clone())
        },
        |_, _, value| value.clone(),
    );
    // Trait declarations and implementations can live in different files.
    // Once the first pass has the complete trait-return registry, revisit
    // impls so associated bindings can instantiate inherited default method
    // returns and publish them under the concrete receiver package-wide.
    for source in &ordered {
        let Ok(syntax) = syn::parse_file(source.source) else {
            continue;
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            if !source_class_allows(source_class, &cfg, &[], &mut errors, "source file") {
                continue;
            }
            let mut base = ModuleSymbols::for_package(source.package);
            base.named_type_info = named_type_package_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_named_type_info = named_type_workspace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.dependency_crate_aliases = dependency_aliases
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.workspace_sqlx_namespaces = sqlx_namespace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeSet::new()));
            let registry_key = (source.package.to_owned(), source_class);
            base.trait_method_returns = trait_method_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_supertraits = trait_supertrait_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_generic_params = trait_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_method_generic_params = trait_method_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            base.trait_method_generic_input_params = trait_method_generic_input_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            let symbols = collect_module_symbols(
                &syntax.items,
                Some(&base),
                source.package,
                source.module,
                &cfg,
                source_class,
                &mut errors,
            );
            let module_prefix = symbols.module_path.join("::");
            let method_registry = method_registries
                .entry((source.package.to_owned(), source_class))
                .or_default();
            for ((owner, _, method), info) in &symbols.method_returns {
                let canonical_owner = if module_prefix.is_empty() || owner.contains("::") {
                    owner.clone()
                } else {
                    format!("{module_prefix}::{owner}")
                };
                method_registry
                    .entry((canonical_owner, method.clone()))
                    .or_default()
                    .union(info);
            }
        }
    }
    let mut function_registry_cache = dependency_scoped_registry_cache(
        &function_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        qualify_dependency_info,
    );
    let mut function_mutable_write_registry_cache = dependency_scoped_registry_cache(
        &function_mutable_write_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |provider_root, named, effects| {
            effects
                .iter()
                .map(|(index, info)| (*index, qualify_dependency_info(provider_root, named, info)))
                .collect()
        },
    );
    let mut function_generic_registry_cache = dependency_scoped_registry_cache(
        &function_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    let mut function_generic_input_registry_cache = dependency_scoped_registry_cache(
        &function_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    resolve_local_callable_imports(&local_callable_imports, &mut function_registry_cache);
    resolve_local_callable_imports(
        &local_callable_imports,
        &mut function_mutable_write_registry_cache,
    );
    resolve_local_callable_imports(
        &local_callable_imports,
        &mut function_generic_registry_cache,
    );
    resolve_local_callable_imports(
        &local_callable_imports,
        &mut function_generic_input_registry_cache,
    );
    let method_registry_cache = dependency_scoped_registry_cache(
        &method_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        qualify_dependency_info,
    );
    let method_mutable_receiver_registry_cache = dependency_scoped_registry_cache(
        &method_mutable_receiver_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        qualify_dependency_info,
    );
    let method_mutable_write_registry_cache = dependency_scoped_registry_cache(
        &method_mutable_write_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        |provider_root, named, effects| {
            effects
                .iter()
                .map(|(index, info)| (*index, qualify_dependency_info(provider_root, named, info)))
                .collect()
        },
    );
    let method_generic_registry_cache = dependency_scoped_registry_cache(
        &method_generic_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        |_, _, value| value.clone(),
    );
    let method_generic_input_registry_cache = dependency_scoped_registry_cache(
        &method_generic_input_registries,
        &named_type_registries,
        dependencies,
        |provider_root, (owner, method)| (format!("{provider_root}::{owner}"), method.clone()),
        |_, _, value| value.clone(),
    );
    let item_value_registry_cache = dependency_scoped_registry_cache(
        &item_value_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        qualify_dependency_info,
    );
    let macro_registry_cache = dependency_scoped_registry_cache(
        &macro_registries,
        &named_type_registries,
        dependencies,
        |provider_root, key| format!("{provider_root}::{key}"),
        |_, _, value| value.clone(),
    );
    for source in ordered {
        if source.classification.is_empty()
            || source.package.is_empty()
            || source.module.is_empty()
            || source.source_path.is_empty()
        {
            errors.push(
                "persistence source classification/package/module/path must be non-empty"
                    .to_owned(),
            );
            continue;
        }
        if !seen_mounts.insert((
            source.package,
            source.module,
            source.source_path,
            source.inherited_cfg,
        )) {
            errors.push(format!(
                "duplicate classified persistence source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        let syntax = match syn::parse_file(source.source) {
            Ok(syntax) => syntax,
            Err(error) => {
                errors.push(format!(
                    "cannot parse persistence source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        let production = cfg_context_allows_production(&cfg, &[]);
        let test = cfg_context_allows_test(&cfg, &[]);
        let (production, test) = match (production, test) {
            (Ok(production), Ok(test)) => (production, test),
            (production, test) => {
                if let Err(error) = production {
                    errors.push(format!(
                        "invalid file cfg (production) in persistence source {}: {error}",
                        source.source_path
                    ));
                }
                if let Err(error) = test {
                    errors.push(format!(
                        "invalid file cfg (test) in persistence source {}: {error}",
                        source.source_path
                    ));
                }
                continue;
            }
        };
        if !production && !test {
            errors.push(format!(
                "persistence source {} is unreachable in both production and test cfg",
                source.source_path
            ));
            continue;
        }
        for source_class in [
            PersistenceSourceClass::Production,
            PersistenceSourceClass::TestFixture,
        ] {
            let enabled = match source_class {
                PersistenceSourceClass::Production => production,
                PersistenceSourceClass::TestFixture => test,
            };
            if !enabled {
                continue;
            }
            let mut package_symbols = ModuleSymbols::for_package(source.package);
            package_symbols.named_type_info = named_type_package_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.workspace_named_type_info = named_type_workspace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.dependency_crate_aliases = dependency_aliases
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.workspace_sqlx_namespaces = sqlx_namespace_cache
                .get(&(source.package.to_owned(), source_class))
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeSet::new()));
            let registry_key = (source.package.to_owned(), source_class);
            package_symbols.trait_method_returns = trait_method_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_supertraits = trait_supertrait_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_generic_params = trait_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_method_generic_params = trait_method_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.trait_method_generic_input_params =
                trait_method_generic_input_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_returns = function_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_mutable_writes = function_mutable_write_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_generic_params = function_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_function_generic_input_params =
                function_generic_input_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_returns = method_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_mutable_receivers =
                method_mutable_receiver_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_mutable_writes = method_mutable_write_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_generic_params = method_generic_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_method_generic_input_params =
                method_generic_input_registry_cache
                    .get(&registry_key)
                    .cloned()
                    .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_item_values = item_value_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            package_symbols.package_persistence_macros = macro_registry_cache
                .get(&registry_key)
                .cloned()
                .unwrap_or_else(|| std::sync::Arc::new(BTreeMap::new()));
            analyze_module_items(
                &syntax.items,
                RecordContext {
                    classification: source.classification,
                    source_class,
                    package: source.package,
                    module: source.module,
                    source: source.source_path,
                },
                Some(&package_symbols),
                cfg.clone(),
                &mut accumulator,
                &mut errors,
            );
        }
    }
    if errors.is_empty() {
        Ok(accumulator.finish())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors.join("\n"))
    }
}
