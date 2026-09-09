//! Module-wide symbol and named-type collection passes.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn collect_module_symbols(
    items: &[Item],
    parent: Option<&ModuleSymbols>,
    package: &str,
    module: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
) -> ModuleSymbols {
    let is_root_collection = parent.is_none();
    let mut symbols = parent
        .cloned()
        .unwrap_or_else(|| ModuleSymbols::for_package(package));
    // A child module does not see the parent's items under their bare names,
    // so these sets are rebuilt for each module's own scope.
    symbols.local_type_definitions.clear();
    symbols.local_macro_definitions.clear();
    symbols
        .local_type_definitions
        .extend(items.iter().filter_map(|item| match item {
            Item::Struct(item_struct) => Some(normalized_ident(&item_struct.ident)),
            Item::Enum(item_enum) => Some(normalized_ident(&item_enum.ident)),
            Item::Type(item_type) => Some(normalized_ident(&item_type.ident)),
            _ => None,
        }));
    // An import shadows a prelude name as surely as a declaration does, unless
    // it is the standard item itself.
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let (leaves, _) = use_leaves(item_use);
        for leaf in leaves {
            if leaf
                .source
                .first()
                .is_some_and(|root| matches!(root.as_str(), "std" | "alloc" | "core"))
            {
                continue;
            }
            symbols.local_type_definitions.insert(leaf.local);
        }
    }

    symbols.module_path = module
        .split("::")
        .filter(|segment| *segment != "crate")
        .map(str::to_owned)
        .collect();
    symbols.traits_in_scope.clear();
    symbols.anonymous_traits_in_scope.clear();
    for _ in 0..=items.len() {
        let mut changed = false;
        for (item_index, item) in items.iter().enumerate() {
            match item {
                Item::Trait(item_trait)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_trait.attrs,
                        errors,
                        "trait",
                    ) =>
                {
                    let local = normalized_ident(&item_trait.ident);
                    let mut path = symbols.module_path.clone();
                    path.push(local.clone());
                    let canonical = path.join("::");
                    if symbols.traits_in_scope.get(&local) != Some(&canonical) {
                        symbols.traits_in_scope.insert(local, canonical);
                        changed = true;
                    }
                }
                Item::Use(item_use)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_use.attrs,
                        errors,
                        "use declaration",
                    ) =>
                {
                    changed |= apply_import_symbols(item_use, &mut symbols);
                }
                Item::ExternCrate(extern_crate)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &extern_crate.attrs,
                        errors,
                        "extern crate",
                    ) =>
                {
                    let source = normalized_ident(&extern_crate.ident);
                    let local = extern_crate
                        .rename
                        .as_ref()
                        .map(|(_, rename)| normalized_ident(rename))
                        .unwrap_or_else(|| source.clone());
                    if symbols.sqlx_namespaces.contains(&source) {
                        changed |= symbols.sqlx_namespaces.insert(local);
                    } else if source == "wow_database" {
                        changed |= symbols.database_namespaces.insert(local);
                    }
                }
                Item::Type(alias)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &alias.attrs,
                        errors,
                        "type alias",
                    ) =>
                {
                    let targets = targets_in_type(&alias.ty, &symbols);
                    if !targets.is_empty() {
                        let entry = symbols
                            .type_aliases
                            .entry(normalized_ident(&alias.ident))
                            .or_default();
                        let before = entry.len();
                        entry.extend(targets);
                        changed |= entry.len() != before;
                    }
                    let nominal_targets = receiver_nominal_types_in_type(&alias.ty);
                    if !nominal_targets.is_empty() {
                        let resolved_targets = nominal_targets
                            .into_iter()
                            .flat_map(|nominal| {
                                symbols
                                    .nominal_type_aliases
                                    .get(&nominal)
                                    .cloned()
                                    .unwrap_or_else(|| BTreeSet::from([nominal]))
                            })
                            .collect::<BTreeSet<_>>();
                        let entry = symbols
                            .nominal_type_aliases
                            .entry(normalized_ident(&alias.ident))
                            .or_default();
                        let before = entry.len();
                        entry.extend(resolved_targets);
                        changed |= entry.len() != before;
                    }
                    let alias_info = variable_info_in_type(&alias.ty, &symbols);
                    let entry = symbols
                        .type_alias_info
                        .entry(normalized_ident(&alias.ident))
                        .or_default();
                    let before = entry.clone();
                    entry.union(&alias_info);
                    changed |= *entry != before;
                }
                Item::Const(item_const)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_const.attrs,
                        errors,
                        "const",
                    ) =>
                {
                    let mut info = variable_info_in_type(&item_const.ty, &symbols);
                    let (kind, sources) = source_sql_info(&item_const.expr, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, &symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    let entry = symbols
                        .item_values
                        .entry(normalized_ident(&item_const.ident))
                        .or_default();
                    let before = entry.clone();
                    entry.union(&info);
                    changed |= *entry != before;
                }
                Item::Static(item_static)
                    if source_class_allows(
                        source_class,
                        cfg,
                        &item_static.attrs,
                        errors,
                        "static",
                    ) =>
                {
                    let mut info = variable_info_in_type(&item_static.ty, &symbols);
                    let (kind, sources) = source_sql_info(&item_static.expr, &|path| {
                        standard_string_macro_of(
                            path,
                            &|path| canonical_path_names(path, &symbols),
                            &symbols.module_path,
                            &macro_shadows_before(items, item_index),
                        )
                    });
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    let entry = symbols
                        .item_values
                        .entry(normalized_ident(&item_static.ident))
                        .or_default();
                    let before = entry.clone();
                    entry.union(&info);
                    changed |= *entry != before;
                }
                _ => {}
            }
        }
        if !changed {
            break;
        }
    }

    if is_root_collection {
        collect_nested_trait_returns(
            items,
            &symbols.module_path.clone(),
            cfg,
            source_class,
            &mut symbols,
            errors,
        );
    }

    for item in items {
        match item {
            Item::Struct(item_struct)
                if source_class_allows(source_class, cfg, &item_struct.attrs, errors, "struct") =>
            {
                for (index, field) in item_struct.fields.iter().enumerate() {
                    if !source_class_allows(source_class, cfg, &field.attrs, errors, "struct field")
                    {
                        continue;
                    }
                    let targets = targets_in_type(&field.ty, &symbols);
                    let field_name = field
                        .ident
                        .as_ref()
                        .map(normalized_ident)
                        .unwrap_or_else(|| index.to_string());
                    symbols
                        .field_owners
                        .entry(field_name.clone())
                        .or_default()
                        .insert(normalized_ident(&item_struct.ident));
                    let nominal_types = receiver_nominal_types_in_type(&field.ty)
                        .into_iter()
                        .flat_map(|nominal| {
                            symbols
                                .nominal_type_aliases
                                .get(&nominal)
                                .cloned()
                                .unwrap_or_else(|| BTreeSet::from([nominal]))
                        })
                        .collect::<BTreeSet<_>>();
                    if !nominal_types.is_empty() {
                        symbols
                            .field_nominal_types
                            .entry((normalized_ident(&item_struct.ident), field_name))
                            .or_default()
                            .extend(nominal_types);
                    }
                    if targets.is_empty() {
                        continue;
                    }
                    if let Some(ident) = &field.ident {
                        symbols
                            .field_targets
                            .entry((
                                normalized_ident(&item_struct.ident),
                                normalized_ident(ident),
                            ))
                            .or_default()
                            .extend(targets);
                    } else {
                        symbols
                            .tuple_field_targets
                            .entry((normalized_ident(&item_struct.ident), index.to_string()))
                            .or_default()
                            .extend(targets);
                    }
                }
            }
            Item::Enum(item_enum)
                if source_class_allows(source_class, cfg, &item_enum.attrs, errors, "enum") =>
            {
                for variant in &item_enum.variants {
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &variant.attrs,
                        errors,
                        "enum variant",
                    ) {
                        continue;
                    }
                    for (index, field) in variant.fields.iter().enumerate() {
                        if !source_class_allows(
                            source_class,
                            cfg,
                            &field.attrs,
                            errors,
                            "enum field",
                        ) {
                            continue;
                        }
                        let targets = targets_in_type(&field.ty, &symbols);
                        let variant_owner = format!(
                            "{}::{}",
                            normalized_ident(&item_enum.ident),
                            normalized_ident(&variant.ident)
                        );
                        let field_name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| index.to_string());
                        symbols
                            .field_owners
                            .entry(field_name.clone())
                            .or_default()
                            .insert(variant_owner.clone());
                        let nominal_types = receiver_nominal_types_in_type(&field.ty)
                            .into_iter()
                            .flat_map(|nominal| {
                                symbols
                                    .nominal_type_aliases
                                    .get(&nominal)
                                    .cloned()
                                    .unwrap_or_else(|| BTreeSet::from([nominal]))
                            })
                            .collect::<BTreeSet<_>>();
                        if !nominal_types.is_empty() {
                            symbols
                                .field_nominal_types
                                .entry((variant_owner.clone(), field_name))
                                .or_default()
                                .extend(nominal_types);
                        }
                        if targets.is_empty() {
                            continue;
                        }
                        if let Some(ident) = &field.ident {
                            symbols
                                .field_targets
                                .entry((variant_owner, normalized_ident(ident)))
                                .or_default()
                                .extend(targets);
                        } else {
                            symbols
                                .tuple_field_targets
                                .entry((variant_owner, index.to_string()))
                                .or_default()
                                .extend(targets);
                        }
                    }
                }
            }
            Item::Macro(item_macro)
                if source_class_allows(
                    source_class,
                    cfg,
                    &item_macro.attrs,
                    errors,
                    "macro definition",
                ) =>
            {
                // A `macro_rules!` whose body reaches concrete persistence is
                // inventoried at its definition site, but that row covers the
                // definition only: register the macro name so `audit_macro`
                // inventories every later invocation instead of letting a new
                // call site slip past both ratchets unannotated.
                if let Some(name) = &item_macro.ident {
                    symbols
                        .local_macro_definitions
                        .insert(normalized_ident(name));
                    let targets = targets_in_tokens(item_macro.mac.tokens.clone(), &symbols);
                    if !targets.is_empty() {
                        symbols
                            .persistence_macros
                            .insert(normalized_ident(name), targets);
                    }
                }
            }
            Item::Fn(function)
                if source_class_allows(source_class, cfg, &function.attrs, errors, "function") =>
            {
                let called_inputs = called_parameter_inputs(function);
                if !called_inputs.is_empty() {
                    symbols
                        .function_called_inputs
                        .insert(normalized_ident(&function.sig.ident), called_inputs);
                }
                let mutable_writes = mutable_parameter_writes(function)
                    .into_iter()
                    .filter_map(|index| {
                        let FnArg::Typed(typed) = function.sig.inputs.iter().nth(index)? else {
                            return None;
                        };
                        Some((index, variable_info_in_type(&typed.ty, &symbols)))
                    })
                    .collect::<BTreeMap<_, _>>();
                if !mutable_writes.is_empty() {
                    symbols
                        .function_mutable_writes
                        .insert(normalized_ident(&function.sig.ident), mutable_writes);
                }
                let generic_params = generic_type_param_names(&function.sig.generics);
                if !generic_params.is_empty() {
                    symbols.function_generic_input_params.insert(
                        normalized_ident(&function.sig.ident),
                        generic_params_by_input(&function.sig.inputs, &generic_params),
                    );
                    symbols
                        .function_generic_params
                        .insert(normalized_ident(&function.sig.ident), generic_params);
                }
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let mut return_info = variable_info_in_type(ty, &symbols);
                    return_info.sql_expression = SqlExpressionKind::Nonliteral;
                    if !return_info.flow.is_empty()
                        || !return_info.nominal_types.is_empty()
                        || !return_info.payload_variants.is_empty()
                        || !return_info.tuple_items.is_empty()
                        || !return_info.trait_bounds.is_empty()
                    {
                        symbols
                            .function_returns
                            .entry(normalized_ident(&function.sig.ident))
                            .or_default()
                            .union(&return_info);
                    }
                }
            }
            Item::Impl(item_impl)
                if source_class_allows(source_class, cfg, &item_impl.attrs, errors, "impl") =>
            {
                let trait_name = item_impl
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| canonical_trait_path(path, &symbols));
                let mut receiver_types = nominal_types_in_type(&item_impl.self_ty)
                    .into_iter()
                    .flat_map(|nominal| {
                        symbols
                            .nominal_type_aliases
                            .get(&nominal)
                            .cloned()
                            .unwrap_or_else(|| BTreeSet::from([nominal]))
                    })
                    .collect::<BTreeSet<_>>();
                if let Type::Path(path) = item_impl.self_ty.as_ref() {
                    let mut names = path_names(&path.path);
                    if names.first().is_some_and(|name| name == "crate") {
                        names.remove(0);
                    }
                    if names.len() > 1 {
                        receiver_types.insert(names.join("::"));
                    }
                }
                let associated_types = item_impl
                    .items
                    .iter()
                    .filter_map(|item| {
                        let ImplItem::Type(associated) = item else {
                            return None;
                        };
                        source_class_allows(
                            source_class,
                            cfg,
                            &associated.attrs,
                            errors,
                            "impl associated type",
                        )
                        .then(|| {
                            (
                                normalized_ident(&associated.ident),
                                variable_info_in_type(&associated.ty, &symbols),
                            )
                        })
                    })
                    .collect::<BTreeMap<_, _>>();
                if let Some(trait_name) = &trait_name {
                    let trait_arguments = item_impl
                        .trait_
                        .as_ref()
                        .and_then(|(_, path, _)| path.segments.last())
                        .and_then(|segment| match &segment.arguments {
                            syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                            _ => None,
                        });
                    let trait_params = symbols
                        .trait_generic_params
                        .iter()
                        .find(|(candidate, _)| {
                            *candidate == trait_name
                                || candidate.ends_with(&format!("::{trait_name}"))
                                || trait_name.ends_with(&format!("::{candidate}"))
                        })
                        .map(|(_, params)| params.clone())
                        .unwrap_or_default();
                    let trait_substitutions = trait_arguments
                        .map(|arguments| {
                            trait_params
                                .iter()
                                .zip(arguments.args.iter().filter_map(|argument| match argument {
                                    syn::GenericArgument::Type(ty) => {
                                        Some(variable_info_in_type(ty, &symbols))
                                    }
                                    _ => None,
                                }))
                                .map(|(param, info)| (param.clone(), info))
                                .collect::<BTreeMap<_, _>>()
                        })
                        .unwrap_or_default();
                    let inherited = symbols
                        .trait_method_returns
                        .iter()
                        .filter(|((candidate_trait, _), _)| {
                            candidate_trait == trait_name
                                || candidate_trait.ends_with(&format!("::{trait_name}"))
                                || trait_name.ends_with(&format!("::{candidate_trait}"))
                        })
                        .map(|((_, method), info)| (method.clone(), info.clone()))
                        .collect::<Vec<_>>();
                    for (method_name, mut return_info) in inherited {
                        substitute_nominal_params(&mut return_info, &trait_substitutions);
                        substitute_nominal_params(&mut return_info, &associated_types);
                        let generic_params = symbols
                            .trait_method_generic_params
                            .iter()
                            .find(|((candidate_trait, candidate_method), _)| {
                                candidate_method == &method_name
                                    && (candidate_trait == trait_name
                                        || candidate_trait.ends_with(&format!("::{trait_name}"))
                                        || trait_name.ends_with(&format!("::{candidate_trait}")))
                            })
                            .map(|(_, params)| params.clone());
                        let generic_input_params = symbols
                            .trait_method_generic_input_params
                            .iter()
                            .find(|((candidate_trait, candidate_method), _)| {
                                candidate_method == &method_name
                                    && (candidate_trait == trait_name
                                        || candidate_trait.ends_with(&format!("::{trait_name}"))
                                        || trait_name.ends_with(&format!("::{candidate_trait}")))
                            })
                            .map(|(_, params)| params.clone());
                        for receiver_type in &receiver_types {
                            let key = (
                                receiver_type.clone(),
                                Some(trait_name.clone()),
                                method_name.clone(),
                            );
                            symbols
                                .method_returns
                                .entry(key.clone())
                                .or_default()
                                .union(&return_info);
                            if let Some(generic_params) = &generic_params {
                                symbols
                                    .method_generic_params
                                    .insert(key.clone(), generic_params.clone());
                            }
                            if let Some(generic_input_params) = &generic_input_params {
                                symbols
                                    .method_generic_input_params
                                    .insert(key, generic_input_params.clone());
                            }
                        }
                    }
                }
                for item in &item_impl.items {
                    let ImplItem::Fn(method) = item else {
                        continue;
                    };
                    if !source_class_allows(source_class, cfg, &method.attrs, errors, "impl method")
                    {
                        continue;
                    }
                    let method_name = normalized_ident(&method.sig.ident);
                    let (receiver_fields, parameter_writes) = mutable_method_writes(method);
                    for receiver_type in &receiver_types {
                        let key = (
                            receiver_type.clone(),
                            trait_name.clone(),
                            method_name.clone(),
                        );
                        let mut receiver_write_info = VariableInfo::default();
                        for field in &receiver_fields {
                            let mut field_write_info = VariableInfo::default();
                            if let Some(targets) = symbols
                                .field_targets
                                .get(&(receiver_type.clone(), field.clone()))
                            {
                                field_write_info.flow.union(Flow::pools(targets));
                            }
                            if let Some(types) = symbols
                                .field_nominal_types
                                .get(&(receiver_type.clone(), field.clone()))
                            {
                                field_write_info.nominal_types.extend(types.iter().cloned());
                            }
                            if field_write_info != VariableInfo::default() {
                                receiver_write_info
                                    .field_items
                                    .entry(field.clone())
                                    .or_default()
                                    .union(&field_write_info);
                            }
                        }
                        if receiver_write_info != VariableInfo::default() {
                            symbols
                                .method_mutable_receivers
                                .entry(key.clone())
                                .or_default()
                                .union(&receiver_write_info);
                        }
                        for index in &parameter_writes {
                            let Some(FnArg::Typed(typed)) = method.sig.inputs.iter().nth(index + 1)
                            else {
                                continue;
                            };
                            let parameter_info = flatten_reachable_variable_info(
                                &variable_info_in_type(&typed.ty, &symbols),
                            );
                            symbols
                                .method_mutable_writes
                                .entry(key.clone())
                                .or_default()
                                .entry(*index)
                                .or_default()
                                .union(&parameter_info);
                        }
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let mut return_info = variable_info_in_type(ty, &symbols);
                        // `Self::Product` is represented by syn as a nominal
                        // projection. Resolve the implementation's associated
                        // binding before recording this concrete method.
                        substitute_nominal_params(&mut return_info, &associated_types);
                        return_info.sql_expression = SqlExpressionKind::Nonliteral;
                        let generic_params = generic_type_param_names(&method.sig.generics);
                        if !generic_params.is_empty() {
                            let input_params =
                                generic_params_by_input(&method.sig.inputs, &generic_params);
                            for receiver_type in &receiver_types {
                                symbols.method_generic_input_params.insert(
                                    (
                                        receiver_type.clone(),
                                        trait_name.clone(),
                                        method_name.clone(),
                                    ),
                                    input_params.clone(),
                                );
                                symbols.method_generic_params.insert(
                                    (
                                        receiver_type.clone(),
                                        trait_name.clone(),
                                        method_name.clone(),
                                    ),
                                    generic_params.clone(),
                                );
                            }
                        }
                        if !return_info.flow.is_empty()
                            || !return_info.nominal_types.is_empty()
                            || !return_info.payload_variants.is_empty()
                            || !return_info.tuple_items.is_empty()
                            || !return_info.trait_bounds.is_empty()
                        {
                            for receiver_type in &receiver_types {
                                let info = symbols
                                    .method_returns
                                    .entry((
                                        receiver_type.clone(),
                                        trait_name.clone(),
                                        method_name.clone(),
                                    ))
                                    .or_default();
                                info.union(&return_info);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let field_targets = symbols.field_targets.clone();
    let tuple_field_targets = symbols.tuple_field_targets.clone();
    let field_nominal_types = symbols.field_nominal_types.clone();
    for info in std::sync::Arc::make_mut(&mut symbols.trait_method_returns).values_mut() {
        for owner in info.nominal_types.clone() {
            for ((candidate_owner, field), targets) in &field_targets {
                if candidate_owner == &owner {
                    info.field_items
                        .entry(field.clone())
                        .or_default()
                        .flow
                        .union(Flow::pools(targets));
                }
            }
            for ((candidate_owner, field), targets) in &tuple_field_targets {
                if candidate_owner == &owner {
                    info.field_items
                        .entry(field.clone())
                        .or_default()
                        .flow
                        .union(Flow::pools(targets));
                }
            }
            for ((candidate_owner, field), nominal_types) in &field_nominal_types {
                if candidate_owner == &owner {
                    info.field_items
                        .entry(field.clone())
                        .or_default()
                        .nominal_types
                        .extend(nominal_types.iter().cloned());
                }
            }
        }
    }
    symbols
}

pub(super) fn collect_named_type_info(
    items: &[Item],
    parent: &ModuleSymbols,
    package: &str,
    module: &str,
    cfg: &[String],
    source_class: PersistenceSourceClass,
    errors: &mut Vec<String>,
    output: &mut BTreeMap<String, VariableInfo>,
) {
    let symbols = collect_module_symbols(
        items,
        Some(parent),
        package,
        module,
        cfg,
        source_class,
        errors,
    );
    for item in items {
        match item {
            Item::Type(alias)
                if source_class_allows(source_class, cfg, &alias.attrs, errors, "type alias") =>
            {
                let mut path = symbols.module_path.clone();
                path.push(normalized_ident(&alias.ident));
                output
                    .entry(path.join("::"))
                    .or_default()
                    .union(&VariableInfo {
                        type_generic_params: generic_type_param_names(&alias.generics),
                        ..variable_info_in_type(&alias.ty, &symbols)
                    });
            }
            Item::Struct(item_struct)
                if source_class_allows(source_class, cfg, &item_struct.attrs, errors, "struct") =>
            {
                let mut path = symbols.module_path.clone();
                path.push(normalized_ident(&item_struct.ident));
                let entry = output.entry(path.join("::")).or_default();
                entry.type_generic_params = generic_type_param_names(&item_struct.generics);
                for (index, field) in item_struct.fields.iter().enumerate() {
                    if source_class_allows(source_class, cfg, &field.attrs, errors, "struct field")
                    {
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| index.to_string());
                        let field_info = variable_info_in_type(&field.ty, &symbols);
                        // Moving, returning, or passing the whole nominal
                        // value also moves every persistence-bearing field.
                        entry.flow.union(field_info.flow.clone());
                        entry
                            .field_items
                            .entry(name)
                            .or_default()
                            .union(&field_info);
                    }
                }
            }
            Item::Enum(item_enum)
                if source_class_allows(source_class, cfg, &item_enum.attrs, errors, "enum") =>
            {
                let mut path = symbols.module_path.clone();
                path.push(normalized_ident(&item_enum.ident));
                let enum_path = path.join("::");
                let enum_generic_params = generic_type_param_names(&item_enum.generics);
                output
                    .entry(enum_path.clone())
                    .or_default()
                    .type_generic_params = enum_generic_params.clone();
                let mut enum_flow = Flow::default();
                for variant in &item_enum.variants {
                    if !source_class_allows(
                        source_class,
                        cfg,
                        &variant.attrs,
                        errors,
                        "enum variant",
                    ) {
                        continue;
                    }
                    // Each variant is also registered as `Enum::Variant` with
                    // its own fields, so a pattern such as
                    // `Product::Database(database)` can recover the payload
                    // through `declared_field_info` across source files.
                    let mut variant_path = symbols.module_path.clone();
                    variant_path.push(normalized_ident(&item_enum.ident));
                    variant_path.push(normalized_ident(&variant.ident));
                    let variant_entry = output.entry(variant_path.join("::")).or_default();
                    variant_entry.type_generic_params = enum_generic_params.clone();
                    let mut shapes = Vec::new();
                    for (index, field) in variant.fields.iter().enumerate() {
                        if !source_class_allows(
                            source_class,
                            cfg,
                            &field.attrs,
                            errors,
                            "enum variant field",
                        ) {
                            continue;
                        }
                        let field_info = variable_info_in_type(&field.ty, &symbols);
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| index.to_string());
                        variant_entry
                            .field_items
                            .entry(name)
                            .or_default()
                            .union(&field_info);
                        variant_entry.flow.union(field_info.flow.clone());
                        enum_flow.union(field_info.flow.clone());
                        shapes.push(nominal_shape_in_type(&field.ty, &symbols).unwrap_or(
                            NominalShape {
                                nominal_types: BTreeSet::new(),
                                arguments: Vec::new(),
                            },
                        ));
                    }
                    if !shapes.is_empty() {
                        output
                            .entry(enum_path.clone())
                            .or_default()
                            .payload_variants
                            .insert(shapes);
                    }
                }
                output.entry(enum_path).or_default().flow.union(enum_flow);
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
                let child_module = format!("{module}::{}", normalized_ident(&item_mod.ident));
                collect_named_type_info(
                    &item_mod.content.as_ref().expect("checked content").1,
                    &symbols,
                    package,
                    &child_module,
                    &item_cfg(cfg, &item_mod.attrs),
                    source_class,
                    errors,
                    output,
                );
            }
            _ => {}
        }
    }
}
