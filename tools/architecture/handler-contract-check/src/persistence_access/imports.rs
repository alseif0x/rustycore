//! Use-tree flattening and import-driven symbol resolution.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct UseLeaf {
    pub(super) source: Vec<String>,
    pub(super) local: String,
    pub(super) fingerprint: String,
    pub(super) namespace_self: bool,
}

pub(super) fn flatten_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    leaves: &mut Vec<UseLeaf>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            flatten_use_tree(&path.tree, prefix, leaves, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let name = normalized_ident(&name.ident);
            let namespace_self = name == "self";
            let mut source = prefix.clone();
            let local = if namespace_self {
                prefix.last().cloned().unwrap_or_else(|| name.clone())
            } else {
                source.push(name.clone());
                name
            };
            let fingerprint = source.join("::");
            leaves.push(UseLeaf {
                source,
                local,
                fingerprint,
                namespace_self,
            });
        }
        UseTree::Rename(rename) => {
            let source_name = normalized_ident(&rename.ident);
            let local = normalized_ident(&rename.rename);
            let mut source = prefix.clone();
            let namespace_self = source_name == "self";
            if !namespace_self {
                source.push(source_name.clone());
            }
            leaves.push(UseLeaf {
                fingerprint: if namespace_self {
                    format!("{}::self as {local}", source.join("::"))
                } else {
                    format!("{} as {local}", source.join("::"))
                },
                source,
                local,
                namespace_self,
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix, leaves, globs);
            }
        }
        UseTree::Glob(_) => globs.push(prefix.clone()),
    }
}

pub(super) fn use_leaves(item_use: &ItemUse) -> (Vec<UseLeaf>, Vec<Vec<String>>) {
    let mut leaves = Vec::new();
    let mut globs = Vec::new();
    flatten_use_tree(&item_use.tree, &mut Vec::new(), &mut leaves, &mut globs);
    (leaves, globs)
}

pub(super) fn collect_public_named_type_paths(
    items: &[Item],
    parent_symbols: &ModuleSymbols,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            Item::Type(alias)
                if matches!(alias.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &alias.attrs,
                        errors,
                        "public type alias",
                    ) =>
            {
                let mut path = parent_symbols.module_path.clone();
                path.push(normalized_ident(&alias.ident));
                output.insert(path.join("::"));
            }
            Item::Struct(item_struct)
                if matches!(item_struct.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &item_struct.attrs,
                        errors,
                        "public struct",
                    ) =>
            {
                let mut path = parent_symbols.module_path.clone();
                path.push(normalized_ident(&item_struct.ident));
                output.insert(path.join("::"));
            }
            Item::Enum(item_enum)
                if matches!(item_enum.vis, Visibility::Public(_))
                    && source_class_allows(
                        source_class,
                        cfg,
                        &item_enum.attrs,
                        errors,
                        "public enum",
                    ) =>
            {
                let mut path = parent_symbols.module_path.clone();
                path.push(normalized_ident(&item_enum.ident));
                output.insert(path.join("::"));
                for variant in &item_enum.variants {
                    if source_class_allows(
                        source_class,
                        cfg,
                        &variant.attrs,
                        errors,
                        "public enum variant",
                    ) {
                        let mut variant_path = path.clone();
                        variant_path.push(normalized_ident(&variant.ident));
                        output.insert(variant_path.join("::"));
                    }
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
                    let mut nested_symbols = parent_symbols.clone();
                    nested_symbols
                        .module_path
                        .push(normalized_ident(&item_mod.ident));
                    collect_public_named_type_paths(
                        nested,
                        &nested_symbols,
                        &item_cfg(cfg, &item_mod.attrs),
                        source_class,
                        errors,
                        output,
                    );
                }
            }
            _ => {}
        }
    }
}

pub(super) fn source_is_sqlx(source: &[String], symbols: &ModuleSymbols) -> bool {
    path_is_sqlx(source, symbols)
}

pub(super) fn source_is_database(source: &[String], symbols: &ModuleSymbols) -> bool {
    source
        .first()
        .is_some_and(|first| symbols.database_namespaces.contains(first))
}

pub(super) fn targets_for_use_leaf(leaf: &UseLeaf, symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = targets_for_names(&leaf.source, symbols);
    if leaf.namespace_self {
        if source_is_sqlx(&leaf.source, symbols) {
            targets.insert(PersistenceTarget::Sqlx);
        }
        if source_is_database(&leaf.source, symbols) {
            targets.insert(PersistenceTarget::Database);
        }
    }
    // `use` paths resolve the imported symbol at the leaf, unlike expression
    // paths where only the root can be an in-scope type alias. The adapter's
    // own `crate`/`self`/`super` re-exports therefore need leaf resolution,
    // while consumer crates must still avoid same-named local imports.
    if symbols.database_namespaces.contains("crate") {
        if let Some(last) = leaf.source.last() {
            if let Some(alias_targets) = symbols.type_aliases.get(last) {
                targets.extend(alias_targets);
            }
        }
    }
    targets
}

pub(super) fn apply_import_symbols(item_use: &ItemUse, symbols: &mut ModuleSymbols) -> bool {
    let (leaves, _) = use_leaves(item_use);
    let mut changed = false;
    for leaf in leaves {
        let canonical_source = match leaf.source.first() {
            // `use foo::foo` binds the leaf, not a recursively expanding
            // alias for its own root. Preserve the external/root path.
            Some(first) if leaf.source.len() > 1 && first == &leaf.local => leaf.source.clone(),
            Some(first) if leaf.source.len() > 1 => symbols
                .path_aliases
                .get(first)
                .map(|mapped| {
                    let mut source = mapped.clone();
                    source.extend(leaf.source.iter().skip(1).cloned());
                    source
                })
                .unwrap_or_else(|| canonical_path_names(leaf.source.clone(), symbols)),
            _ => canonical_path_names(leaf.source.clone(), symbols),
        };
        let alias_source = if leaf.namespace_self {
            let mut absolute = vec!["crate".to_owned()];
            absolute.extend(canonical_source.iter().cloned());
            absolute
        } else {
            canonical_source.clone()
        };
        if leaf.local != "_" && symbols.path_aliases.get(&leaf.local) != Some(&alias_source) {
            symbols
                .path_aliases
                .insert(leaf.local.clone(), alias_source);
            changed = true;
        }
        let canonical_trait = canonical_source.join("::");
        if leaf.namespace_self {
            // A namespace-self import puts the module in scope; it does not
            // import a trait under that local module name.
        } else if leaf.local == "_" {
            changed |= symbols.anonymous_traits_in_scope.insert(canonical_trait);
        } else if symbols.traits_in_scope.get(&leaf.local) != Some(&canonical_trait) {
            symbols
                .traits_in_scope
                .insert(leaf.local.clone(), canonical_trait);
            changed = true;
        }
        let source_is_sqlx = source_is_sqlx(&leaf.source, symbols);
        let source_is_database = source_is_database(&leaf.source, symbols);
        if (leaf.namespace_self || leaf.source.len() == 1) && source_is_sqlx {
            changed |= symbols.sqlx_namespaces.insert(leaf.local.clone());
        }
        if (leaf.namespace_self || leaf.source.len() == 1) && source_is_database {
            changed |= symbols.database_namespaces.insert(leaf.local.clone());
        }
        let imported_targets = targets_for_use_leaf(&leaf, symbols);
        if !imported_targets.is_empty() {
            let entry = symbols.type_aliases.entry(leaf.local.clone()).or_default();
            let before = entry.len();
            entry.extend(imported_targets);
            changed |= entry.len() != before;
        }
        if source_is_sqlx && leaf.source.last().is_some_and(|name| is_query_name(name)) {
            changed |= symbols.query_callables.insert(leaf.local);
        }
    }
    changed
}

pub(super) fn collect_nested_trait_returns(
    items: &[Item],
    module_path: &[String],
    cfg: &[String],
    source_class: PersistenceSourceClass,
    symbols: &mut ModuleSymbols,
    errors: &mut Vec<String>,
) {
    for item in items {
        match item {
            Item::Trait(item_trait)
                if source_class_allows(source_class, cfg, &item_trait.attrs, errors, "trait") =>
            {
                let mut trait_path = module_path.to_vec();
                trait_path.push(normalized_ident(&item_trait.ident));
                let trait_path = trait_path.join("::");
                let generic_params: Vec<String> = item_trait
                    .generics
                    .params
                    .iter()
                    .filter_map(|parameter| match parameter {
                        syn::GenericParam::Type(parameter) => {
                            Some(normalized_ident(&parameter.ident))
                        }
                        _ => None,
                    })
                    .collect();
                if !generic_params.is_empty() {
                    std::sync::Arc::make_mut(&mut symbols.trait_generic_params)
                        .insert(trait_path.clone(), generic_params);
                }
                record_trait_supertraits(
                    &trait_path,
                    &item_trait.supertraits,
                    module_path,
                    symbols,
                );
                for item in &item_trait.items {
                    let syn::TraitItem::Fn(method) = item else {
                        continue;
                    };
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &method.attrs,
                        errors,
                        "trait method",
                    ) {
                        continue;
                    }
                    let method_generic_params = generic_type_param_names(&method.sig.generics);
                    if !method_generic_params.is_empty() {
                        std::sync::Arc::make_mut(&mut symbols.trait_method_generic_input_params)
                            .insert(
                                (trait_path.clone(), normalized_ident(&method.sig.ident)),
                                generic_params_by_input(&method.sig.inputs, &method_generic_params),
                            );
                        std::sync::Arc::make_mut(&mut symbols.trait_method_generic_params).insert(
                            (trait_path.clone(), normalized_ident(&method.sig.ident)),
                            method_generic_params,
                        );
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let mut info = variable_info_in_type(ty, symbols);
                        info.sql_expression = SqlExpressionKind::Nonliteral;
                        if !info.flow.is_empty()
                            || !info.nominal_types.is_empty()
                            || !info.payload_variants.is_empty()
                            || !info.tuple_items.is_empty()
                            || !info.trait_bounds.is_empty()
                        {
                            std::sync::Arc::make_mut(&mut symbols.trait_method_returns)
                                .entry((trait_path.clone(), normalized_ident(&method.sig.ident)))
                                .or_default()
                                .union(&info);
                        }
                    }
                }
            }
            Item::Mod(item_mod)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_mod.attrs,
                    errors,
                    "inline module",
                ) && item_mod.content.is_some() =>
            {
                let mut child_path = module_path.to_vec();
                child_path.push(normalized_ident(&item_mod.ident));
                let child_cfg = item_cfg(cfg, &item_mod.attrs);
                collect_nested_trait_returns(
                    &item_mod.content.as_ref().expect("checked content").1,
                    &child_path,
                    &child_cfg,
                    source_class,
                    symbols,
                    errors,
                );
            }
            _ => {}
        }
    }
}

pub(super) fn collect_nested_item_values(
    items: &[Item],
    module_path: &[String],
    package: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    symbols: &ModuleSymbols,
    output: &mut BTreeMap<String, VariableInfo>,
    errors: &mut Vec<String>,
) {
    for (item_index, item) in items.iter().enumerate() {
        match item {
            // A trait's default associated constant is the value every impl
            // inherits unless it overrides it, so it is pinned under the trait.
            Item::Trait(item_trait)
                if source_class_allows(source_class, cfg, &item_trait.attrs, errors, "trait") =>
            {
                for associated in &item_trait.items {
                    let TraitItem::Const(associated) = associated else {
                        continue;
                    };
                    // Each default carries its own cfg. Without this, mutually
                    // exclusive `cfg(test)` and `cfg(not(test))` values are both
                    // registered in both views, and a test-only statement — an
                    // advisory lock included — enters the production inventory.
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &associated.attrs,
                        errors,
                        "trait associated const",
                    ) {
                        continue;
                    }
                    let Some((_, default)) = &associated.default else {
                        continue;
                    };
                    let (kind, sources) = source_sql_info(default, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    if sources.is_empty() {
                        continue;
                    }
                    let mut info = variable_info_in_type(&associated.ty, symbols);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    let mut path = module_path.to_vec();
                    path.push(normalized_ident(&item_trait.ident));
                    path.push(normalized_ident(&associated.ident));
                    output.entry(path.join("::")).or_default().union(&info);
                }
            }
            // `impl Statements { const SQL: &str = "…" }` pins a statement
            // exactly like a module constant, and the call site can only see
            // `Statements::SQL` unless it is registered under that name.
            Item::Impl(item_impl)
                if source_class_allows(source_class, cfg, &item_impl.attrs, errors, "impl") =>
            {
                for associated in &item_impl.items {
                    let ImplItem::Const(associated) = associated else {
                        continue;
                    };
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &associated.attrs,
                        errors,
                        "impl associated const",
                    ) {
                        continue;
                    }
                    let (kind, sources) = source_sql_info(&associated.expr, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    if sources.is_empty() {
                        continue;
                    }
                    let mut info = variable_info_in_type(&associated.ty, symbols);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    // Two traits may each define `SQL` for the same type with
                    // different values, so the key records which impl the value
                    // belongs to. An inherent impl keeps the bare name.
                    let qualifier = item_impl.trait_.as_ref().map(|(_, path, _)| {
                        canonical_path_names(path_names(path), symbols).join("::")
                    });
                    // The impl may target a type of another module, so the key
                    // comes from the self type's own path rather than from
                    // where the impl happens to be written.
                    let owners: Vec<Vec<String>> = match item_impl.self_ty.as_ref() {
                        Type::Path(path) => {
                            vec![canonical_path_names(path_names(&path.path), symbols)]
                        }
                        other => nominal_types_in_type(other)
                            .into_iter()
                            .map(|owner| {
                                let mut path = module_path.to_vec();
                                path.push(owner);
                                path
                            })
                            .collect(),
                    };
                    for owner in owners {
                        let member = normalized_ident(&associated.ident);
                        let key = match &qualifier {
                            Some(trait_path) => {
                                format!("<{} as {trait_path}>::{member}", owner.join("::"))
                            }
                            None => {
                                let mut path = owner.clone();
                                path.push(member.clone());
                                path.join("::")
                            }
                        };
                        output.entry(key).or_default().union(&info);
                    }
                }
            }
            Item::Const(item_const)
                if source_class_allows(source_class, cfg, &item_const.attrs, errors, "const") =>
            {
                let mut path = module_path.to_vec();
                path.push(normalized_ident(&item_const.ident));
                let mut info = variable_info_in_type(&item_const.ty, symbols);
                let (kind, sources) = source_sql_info(&item_const.expr, &|path| {
                    standard_string_macro_of(
                        path,
                        &|path| canonical_path_names(path, symbols),
                        &symbols.module_path,
                        &macro_shadows_before(items, item_index),
                    )
                });
                info.sql_expression = kind;
                info.sql_sources = sources;
                output.entry(path.join("::")).or_default().union(&info);
            }
            Item::Static(item_static)
                if source_class_allows(source_class, cfg, &item_static.attrs, errors, "static") =>
            {
                let mut path = module_path.to_vec();
                path.push(normalized_ident(&item_static.ident));
                let mut info = variable_info_in_type(&item_static.ty, symbols);
                let (kind, sources) = source_sql_info(&item_static.expr, &|path| {
                    standard_string_macro_of(
                        path,
                        &|path| canonical_path_names(path, symbols),
                        &symbols.module_path,
                        &macro_shadows_before(items, item_index),
                    )
                });
                info.sql_expression = kind;
                info.sql_sources = sources;
                output.entry(path.join("::")).or_default().union(&info);
            }
            Item::Mod(item_mod)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_mod.attrs,
                    errors,
                    "inline module",
                ) && item_mod.content.is_some() =>
            {
                let mut child_path = module_path.to_vec();
                child_path.push(normalized_ident(&item_mod.ident));
                let child_cfg = item_cfg(cfg, &item_mod.attrs);
                let child_symbols = collect_module_symbols(
                    &item_mod.content.as_ref().expect("checked content").1,
                    Some(symbols),
                    package,
                    &child_path.join("::"),
                    &child_cfg,
                    source_class,
                    errors,
                );
                collect_nested_item_values(
                    &item_mod.content.as_ref().expect("checked content").1,
                    &child_path,
                    package,
                    &child_cfg,
                    source_class,
                    &child_symbols,
                    output,
                    errors,
                );
            }
            _ => {}
        }
    }
}
