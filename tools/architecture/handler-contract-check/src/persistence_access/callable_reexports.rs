// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Callable reexport collection and fixed-point provenance resolution.

use super::*;

#[cfg(test)]
mod tests;

/// Unresolved generic outputs are not evidence that a return is pool-free.
/// Direct input/turbofish substitutions keep their precision; only remaining
/// generic output slots receive conservative argument/result provenance. For
/// callbacks this uses the existing conservative argument information; it does
/// not solve higher-order where-clause chains or prove captures are excluded.
pub(super) fn inferred_return_with_unresolved_fallback(
    info: &VariableInfo,
    params: &[String],
    substitutions: &BTreeMap<String, VariableInfo>,
    fallback: impl FnOnce() -> VariableInfo,
) -> VariableInfo {
    let mut result = info.clone();
    substitute_nominal_params(&mut result, substitutions);
    let mut unresolved = params
        .iter()
        .filter(|param| !substitutions.contains_key(*param))
        .map(|param| (param.clone(), VariableInfo::default()))
        .collect::<BTreeMap<_, _>>();
    // Probe exact generic slots, including nested tuple/field/payload shapes;
    // a concrete String or an already substituted turbofish must stay concrete.
    if !unresolved.is_empty() && substitute_nominal_params(&mut result.clone(), &unresolved) {
        let argument_info = fallback();
        for info in unresolved.values_mut() {
            *info = argument_info.clone();
        }
        substitute_nominal_params(&mut result, &unresolved);
    }
    result
}

pub(super) fn collect_public_callable_reexports(
    items: &[Item],
    parent_symbols: &ModuleSymbols,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut Vec<(String, String)>,
) {
    collect_callable_imports(
        items,
        parent_symbols,
        cfg,
        source_class,
        errors,
        output,
        true,
    );
}

pub(super) fn collect_local_callable_imports(
    items: &[Item],
    parent_symbols: &ModuleSymbols,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut Vec<(String, String)>,
) {
    collect_callable_imports(
        items,
        parent_symbols,
        cfg,
        source_class,
        errors,
        output,
        false,
    );
}

fn collect_callable_imports(
    items: &[Item],
    parent_symbols: &ModuleSymbols,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut Vec<(String, String)>,
    public_only: bool,
) {
    let mut symbols = parent_symbols.clone();
    // Resolve sibling aliases before interpreting a public re-export. This
    // mirrors module symbol collection without treating the re-export itself
    // as proof that its source is callable.
    for _ in 0..=items.len() {
        let mut changed = false;
        for item in items {
            if let Item::Use(item_use) = item
                && source_class_allows(
                    source_class,
                    cfg,
                    &item_use.attrs,
                    errors,
                    "use declaration",
                )
            {
                changed |= apply_import_symbols(item_use, &mut symbols);
            }
        }
        if !changed {
            break;
        }
    }
    for item in items {
        match item {
            Item::Fn(function)
                if !public_only
                    && source_class_allows(
                        source_class,
                        cfg,
                        &function.attrs,
                        errors,
                        "function",
                    ) =>
            {
                // Even a non-persistence declaration shadows an imported glob.
                let mut name = symbols.module_path.clone();
                name.push(normalized_ident(&function.sig.ident));
                let name = name.join("::");
                output.push((name.clone(), name));
            }
            Item::Use(item_use)
                if (!public_only || matches!(item_use.vis, Visibility::Public(_)))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &item_use.attrs,
                        errors,
                        "callable use declaration",
                    ) =>
            {
                let (leaves, globs) = use_leaves(item_use);
                for leaf in leaves {
                    let mut export = symbols.module_path.clone();
                    export.push(leaf.local);
                    let mut source = canonical_path_names(leaf.source, &symbols);
                    if source.first().is_some_and(|segment| segment == "crate") {
                        source.remove(0);
                    }
                    output.push((export.join("::"), source.join("::")));
                }
                for glob in globs {
                    let mut export = symbols.module_path.clone();
                    export.push("*".to_owned());
                    let mut source = canonical_path_names(glob, &symbols);
                    if source.first().is_some_and(|segment| segment == "crate") {
                        source.remove(0);
                    }
                    source.push("*".to_owned());
                    output.push((export.join("::"), source.join("::")));
                }
            }
            Item::Mod(item_mod)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_mod.attrs,
                    errors,
                    "inline module",
                ) =>
            {
                if let Some((_, nested)) = &item_mod.content {
                    let mut nested_symbols = symbols.clone();
                    nested_symbols
                        .module_path
                        .push(normalized_ident(&item_mod.ident));
                    let nested_cfg = item_cfg(cfg, &item_mod.attrs);
                    collect_callable_imports(
                        nested,
                        &nested_symbols,
                        &nested_cfg,
                        source_class,
                        errors,
                        output,
                        public_only,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Expand imports only after dependency caches have been assembled. Restricted
/// and private aliases must not become provider exports in another package.
/// This resolves provenance for compiler-valid source, not Rust privacy checking.
pub(super) fn resolve_local_callable_imports<V: Clone + PartialEq>(
    imports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    caches: &mut BTreeMap<(String, PersistenceSourceClass), std::sync::Arc<BTreeMap<String, V>>>,
) {
    for (key, aliases) in imports {
        let Some(cache) = caches.get_mut(key) else {
            continue;
        };
        let registry = std::sync::Arc::make_mut(cache);
        // Declarations and explicit imports shadow glob imports independently
        // of file or declaration order. Never union a shadowed function's flow.
        let mut explicit = registry.keys().cloned().collect::<BTreeSet<_>>();
        explicit.extend(
            aliases
                .iter()
                .filter_map(|(export, _)| (!export.ends_with('*')).then_some(export.clone())),
        );
        for _ in 0..=aliases.len() {
            let before = registry.clone();
            for (export, source) in aliases {
                if let (Some(export), Some(source)) =
                    (export.strip_suffix('*'), source.strip_suffix('*'))
                {
                    for (name, value) in &before {
                        // A glob imports immediate names, not every descendant
                        // function. Recursing through prefixes also invents an
                        // unbounded tests::tests::... tree for `use super::*`.
                        if let Some(suffix) = name.strip_prefix(source)
                            && !suffix.contains("::")
                        {
                            let imported = format!("{export}{suffix}");
                            if !explicit.contains(&imported) {
                                registry.entry(imported).or_insert_with(|| value.clone());
                            }
                        }
                    }
                } else if let Some(value) = before.get(source) {
                    registry
                        .entry(export.clone())
                        .or_insert_with(|| value.clone());
                }
            }
            if registry == &before {
                break;
            }
        }
    }
}

pub(super) fn resolve_public_callable_reexports(
    reexports: &BTreeMap<(String, PersistenceSourceClass), Vec<(String, String)>>,
    named_type_registries: &BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    dependencies: &WorkspaceDependencyAliases,
    function_registries: &mut BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, VariableInfo>,
    >,
    generic_registries: &mut BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, Vec<String>>,
    >,
    generic_input_registries: &mut BTreeMap<
        (String, PersistenceSourceClass),
        BTreeMap<String, Vec<GenericInputSpec>>,
    >,
) {
    let pass_limit = reexports.values().map(Vec::len).sum::<usize>() + 1;
    for _ in 0..pass_limit {
        let before = function_registries.clone();
        let function_snapshot = function_registries.clone();
        let generic_snapshot = generic_registries.clone();
        let generic_input_snapshot = generic_input_registries.clone();
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
                        let provider =
                            function_snapshot
                                .keys()
                                .find_map(|(candidate, candidate_class)| {
                                    (*candidate_class == source_class
                                        && candidate.replace('-', "_") == provider_root.as_str())
                                    .then_some(candidate.clone())
                                });
                        let Some(provider) = provider else {
                            continue;
                        };
                        let entry = source_parts.collect::<Vec<_>>().join("::");
                        (
                            (provider, source_class),
                            entry,
                            Some(provider_root.as_str()),
                        )
                    } else {
                        (consumer_key.clone(), source.to_owned(), None)
                    };
                let Some(provider_registry) = function_snapshot.get(&provider_key) else {
                    continue;
                };
                let entries = if glob {
                    let prefix = (!provider_entry.is_empty())
                        .then(|| format!("{provider_entry}::"))
                        .unwrap_or_default();
                    provider_registry
                        .iter()
                        .filter_map(|(entry, info)| {
                            entry.strip_prefix(&prefix).map(|suffix| {
                                let exported = if export.is_empty() {
                                    suffix.to_owned()
                                } else {
                                    format!("{export}::{suffix}")
                                };
                                (exported, entry.clone(), info.clone())
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut entries = provider_registry
                        .get(&provider_entry)
                        .cloned()
                        .map(|info| vec![(export.to_owned(), provider_entry.clone(), info)])
                        .unwrap_or_default();
                    let prefix = if provider_entry.is_empty() {
                        String::new()
                    } else {
                        format!("{provider_entry}::")
                    };
                    entries.extend(provider_registry.iter().filter_map(|(entry, info)| {
                        entry.strip_prefix(&prefix).map(|suffix| {
                            let exported = if export.is_empty() {
                                suffix.to_owned()
                            } else {
                                format!("{export}::{suffix}")
                            };
                            (exported, entry.clone(), info.clone())
                        })
                    }));
                    entries
                };
                for (exported, source_entry, source_info) in entries {
                    let info = if let Some(provider_root) = provider_root {
                        let named = named_type_registries
                            .get(&provider_key)
                            .cloned()
                            .unwrap_or_default();
                        qualify_dependency_info(provider_root, &named, &source_info)
                    } else {
                        source_info
                    };
                    function_registries
                        .entry(consumer_key.clone())
                        .or_default()
                        .entry(exported.clone())
                        .or_default()
                        .union(&info);
                    if let Some(params) = generic_snapshot
                        .get(&provider_key)
                        .and_then(|registry| registry.get(&source_entry))
                    {
                        generic_registries
                            .entry(consumer_key.clone())
                            .or_default()
                            .entry(exported.clone())
                            .or_insert_with(|| params.clone());
                    }
                    if let Some(inputs) = generic_input_snapshot
                        .get(&provider_key)
                        .and_then(|registry| registry.get(&source_entry))
                    {
                        generic_input_registries
                            .entry(consumer_key.clone())
                            .or_default()
                            .entry(exported)
                            .or_insert_with(|| inputs.clone());
                    }
                }
            }
        }
        if function_registries == &before {
            break;
        }
    }
}
