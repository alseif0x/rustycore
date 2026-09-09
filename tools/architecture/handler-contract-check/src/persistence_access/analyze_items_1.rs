//! Per-item analysis entry points, part 1: imports, aliases, types and impls.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn analyze_trait_default_bodies(
    item_trait: &ItemTrait,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    visible_macro_shadows: BTreeSet<String>,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let trait_name = normalized_ident(&item_trait.ident);
    let trait_canonical = {
        let mut path = symbols.module_path.clone();
        path.push(trait_name.clone());
        path.join("::")
    };
    for trait_item in &item_trait.items {
        let TraitItem::Fn(method) = trait_item else {
            continue;
        };
        let Some(default_body) = &method.default else {
            continue;
        };
        if !source_class_allows(
            context.source_class,
            cfg,
            &method.attrs,
            errors,
            "trait default method",
        ) {
            continue;
        }
        let method_name = normalized_ident(&method.sig.ident);
        let mut analyzer = BodyAnalyzer::new(
            RecordContext {
                classification: context.classification,
                source_class: context.source_class,
                package: context.package,
                module: context.module,
                source: context.source,
            },
            accumulator,
            errors,
            symbols,
            format!("trait {trait_name}::{method_name}"),
            normalized_visibility(&item_trait.vis),
            item_cfg(cfg, &method.attrs),
        );
        analyzer.register_generic_bounds(&item_trait.generics);
        analyzer.register_generic_bounds(&method.sig.generics);
        analyzer.register_parameters(&method.sig.inputs);
        analyzer.bind(
            "self".to_owned(),
            VariableInfo {
                nominal_types: BTreeSet::from([trait_name.clone()]),
                trait_bounds: BTreeSet::from([trait_canonical.clone()]),
                ..VariableInfo::default()
            },
        );
        analyzer.visible_macro_shadows = visible_macro_shadows.clone();
        analyzer.visit_block(default_body);
    }
}

pub(super) fn analyze_import(
    item_use: &ItemUse,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &item_use.attrs,
        AttributeRecordContext {
            enclosing: "module",
            visibility: &normalized_visibility(&item_use.vis),
            cfg,
        },
    );
    let (leaves, globs) = use_leaves(item_use);
    for glob in globs {
        if source_is_sqlx(&glob, symbols) || source_is_database(&glob, symbols) {
            errors.push(format!(
                "glob import {}::* can hide concrete persistence access; import every SQLx/database symbol explicitly",
                glob.join("::")
            ));
        }
    }
    for leaf in leaves {
        for target in targets_for_use_leaf(&leaf, symbols) {
            accumulator.add(
                context,
                NewAccess {
                    enclosing: "module",
                    target,
                    operation: PersistenceOperation::Import,
                    symbol: &leaf.local,
                    visibility: &normalized_visibility(&item_use.vis),
                    cfg,
                    fingerprint: leaf.fingerprint.clone(),
                    generated_input: false,
                },
            );
        }
    }
}

pub(super) fn analyze_type_alias(
    alias: &ItemType,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
) {
    let visibility = normalized_visibility(&alias.vis);
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &alias.attrs,
        AttributeRecordContext {
            enclosing: "module",
            visibility: &visibility,
            cfg,
        },
    );
    add_generics_records(
        accumulator,
        context,
        symbols,
        &alias.generics,
        "module",
        &visibility,
        cfg,
    );
    add_type_records(
        accumulator,
        context,
        symbols,
        &alias.ty,
        "module",
        &normalized_ident(&alias.ident),
        &visibility,
        cfg,
        PersistenceOperation::TypeAlias,
    );
}

pub(super) fn analyze_struct(
    item_struct: &ItemStruct,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enclosing = format!("struct {}", normalized_ident(&item_struct.ident));
    let visibility = normalized_visibility(&item_struct.vis);
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &item_struct.attrs,
        AttributeRecordContext {
            enclosing: &enclosing,
            visibility: &visibility,
            cfg,
        },
    );
    add_generics_records(
        accumulator,
        context,
        symbols,
        &item_struct.generics,
        &enclosing,
        &visibility,
        cfg,
    );
    for (index, field) in item_struct.fields.iter().enumerate() {
        if !source_class_allows(
            context.source_class,
            cfg,
            &field.attrs,
            errors,
            "struct field",
        ) {
            continue;
        }
        let symbol = field
            .ident
            .as_ref()
            .map(normalized_ident)
            .unwrap_or_else(|| index.to_string());
        let field_cfg = item_cfg(cfg, &field.attrs);
        let field_visibility = normalized_visibility(&field.vis);
        add_attribute_records(
            accumulator,
            context,
            symbols,
            &field.attrs,
            AttributeRecordContext {
                enclosing: &enclosing,
                visibility: &field_visibility,
                cfg: &field_cfg,
            },
        );
        add_type_records(
            accumulator,
            context,
            symbols,
            &field.ty,
            &enclosing,
            &symbol,
            &field_visibility,
            &field_cfg,
            PersistenceOperation::TypeReference,
        );
    }
}

pub(super) fn analyze_enum(
    item_enum: &ItemEnum,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enum_enclosing = format!("enum {}", normalized_ident(&item_enum.ident));
    let enum_visibility = normalized_visibility(&item_enum.vis);
    add_attribute_records(
        accumulator,
        context,
        symbols,
        &item_enum.attrs,
        AttributeRecordContext {
            enclosing: &enum_enclosing,
            visibility: &enum_visibility,
            cfg,
        },
    );
    add_generics_records(
        accumulator,
        context,
        symbols,
        &item_enum.generics,
        &enum_enclosing,
        &enum_visibility,
        cfg,
    );
    for variant in &item_enum.variants {
        if !source_class_allows(
            context.source_class,
            cfg,
            &variant.attrs,
            errors,
            "enum variant",
        ) {
            continue;
        }
        let enclosing = format!(
            "enum {}::{}",
            normalized_ident(&item_enum.ident),
            normalized_ident(&variant.ident)
        );
        let variant_cfg = item_cfg(cfg, &variant.attrs);
        add_attribute_records(
            accumulator,
            context,
            symbols,
            &variant.attrs,
            AttributeRecordContext {
                enclosing: &enclosing,
                visibility: &enum_visibility,
                cfg: &variant_cfg,
            },
        );
        for (index, field) in variant.fields.iter().enumerate() {
            if !source_class_allows(
                context.source_class,
                &variant_cfg,
                &field.attrs,
                errors,
                "enum field",
            ) {
                continue;
            }
            let symbol = field
                .ident
                .as_ref()
                .map(normalized_ident)
                .unwrap_or_else(|| index.to_string());
            let field_cfg = item_cfg(&variant_cfg, &field.attrs);
            let field_visibility = normalized_visibility(&field.vis);
            add_attribute_records(
                accumulator,
                context,
                symbols,
                &field.attrs,
                AttributeRecordContext {
                    enclosing: &enclosing,
                    visibility: &field_visibility,
                    cfg: &field_cfg,
                },
            );
            add_type_records(
                accumulator,
                context,
                symbols,
                &field.ty,
                &enclosing,
                &symbol,
                &field_visibility,
                &field_cfg,
                PersistenceOperation::TypeReference,
            );
        }
    }
}

pub(super) fn analyze_function(
    function: &ItemFn,
    context: RecordContext<'_>,
    symbols: &ModuleSymbols,
    visible_macro_shadows: BTreeSet<String>,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enclosing = format!("fn {}", normalized_ident(&function.sig.ident));
    let generic_operations = function
        .sig
        .generics
        .params
        .iter()
        .any(|param| matches!(param, syn::GenericParam::Type(_)))
        .then(|| {
            persistence_operations_in_block(
                &function.block,
                generic_type_param_names(&function.sig.generics)
                    .into_iter()
                    .collect(),
            )
        })
        .unwrap_or_default();
    let visibility = normalized_visibility(&function.vis);
    add_attribute_records(
        accumulator,
        &context,
        symbols,
        &function.attrs,
        AttributeRecordContext {
            enclosing: &enclosing,
            visibility: &visibility,
            cfg: &cfg,
        },
    );
    add_generics_records(
        accumulator,
        &context,
        symbols,
        &function.sig.generics,
        &enclosing,
        &visibility,
        &cfg,
    );
    if let ReturnType::Type(_, ty) = &function.sig.output {
        add_type_records(
            accumulator,
            &context,
            symbols,
            ty,
            &enclosing,
            "return",
            &visibility,
            &cfg,
            PersistenceOperation::TypeReference,
        );
    }
    let mut analyzer = BodyAnalyzer::new(
        context,
        accumulator,
        errors,
        symbols,
        enclosing.clone(),
        visibility,
        cfg.clone(),
    );
    analyzer.visible_macro_shadows = visible_macro_shadows;
    analyzer.register_local_uses(&function.block.stmts);
    analyzer.register_local_callables(&function.block.stmts);
    analyzer.register_local_constants(&function.block.stmts);
    analyzer.register_generic_bounds(&function.sig.generics);
    analyzer.register_parameters(&function.sig.inputs);
    for statement in &function.block.stmts {
        analyzer.visit_stmt(statement);
    }
    let tail = implicit_tail_flow(&function.block, &analyzer);
    if !tail.pool_targets().is_empty() {
        let fingerprint = function
            .block
            .stmts
            .last()
            .map(normalized_tokens)
            .unwrap_or_default();
        analyzer.record_pool_escape(
            &tail,
            PersistenceOperation::ReturnEscape,
            "pool",
            &cfg,
            fingerprint,
        );
    }
    drop(analyzer);
    let unresolved = generic_operations
        .into_iter()
        .filter(|operation| !accumulator.contains_symbol(&enclosing, operation))
        .collect::<Vec<_>>();
    if !unresolved.is_empty() {
        errors.push(format!(
            "{enclosing} is generic and contains persistence-shaped operations ({}) that cannot be instantiated from declared flow; expose a typed module-level adapter instead",
            unresolved.join(", ")
        ));
    }
}

pub(super) fn impl_self_name(item_impl: &ItemImpl) -> String {
    normalized_tokens(&item_impl.self_ty)
}

pub(super) fn analyze_impl(
    item_impl: &ItemImpl,
    context: RecordContext<'_>,
    symbols: &ModuleSymbols,
    visible_macro_shadows: BTreeSet<String>,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let self_name = impl_self_name(item_impl);
    let trait_name = item_impl
        .trait_
        .as_ref()
        .map(|(_, trait_path, _)| canonical_path_names(path_names(trait_path), symbols).join("::"));
    let impl_enclosing = trait_name
        .as_ref()
        .map(|trait_name| format!("impl {trait_name} for {self_name}"))
        .unwrap_or_else(|| format!("impl {self_name}"));
    add_attribute_records(
        accumulator,
        &context,
        symbols,
        &item_impl.attrs,
        AttributeRecordContext {
            enclosing: &impl_enclosing,
            visibility: "",
            cfg: &cfg,
        },
    );
    add_generics_records(
        accumulator,
        &context,
        symbols,
        &item_impl.generics,
        &impl_enclosing,
        "",
        &cfg,
    );
    add_type_records(
        accumulator,
        &context,
        symbols,
        &item_impl.self_ty,
        &impl_enclosing,
        "self",
        "",
        &cfg,
        PersistenceOperation::TypeReference,
    );
    if let Some((_, trait_path, _)) = &item_impl.trait_ {
        for target in targets_for_path(trait_path, symbols) {
            accumulator.add(
                &context,
                NewAccess {
                    enclosing: &impl_enclosing,
                    target,
                    operation: PersistenceOperation::TypeReference,
                    symbol: "trait",
                    visibility: "",
                    cfg: &cfg,
                    fingerprint: normalized_tokens(trait_path),
                    generated_input: false,
                },
            );
        }
    }
    for item in &item_impl.items {
        let ImplItem::Fn(method) = item else {
            if syntax_mentions_persistence(item, symbols) {
                errors.push(format!(
                    "{impl_enclosing} contains unsupported associated persistence syntax; use an ordinary method/type surface before baselining: {}",
                    normalized_tokens(item)
                ));
            }
            continue;
        };
        if !source_class_allows(
            context.source_class,
            &cfg,
            &method.attrs,
            errors,
            "impl method",
        ) {
            continue;
        }
        let method_cfg = item_cfg(&cfg, &method.attrs);
        let enclosing = trait_name
            .as_ref()
            .map(|trait_name| {
                format!(
                    "impl {trait_name} for {self_name}::{}",
                    normalized_ident(&method.sig.ident)
                )
            })
            .unwrap_or_else(|| {
                format!("impl {self_name}::{}", normalized_ident(&method.sig.ident))
            });
        let visibility = normalized_visibility(&method.vis);
        add_attribute_records(
            accumulator,
            &context,
            symbols,
            &method.attrs,
            AttributeRecordContext {
                enclosing: &enclosing,
                visibility: &visibility,
                cfg: &method_cfg,
            },
        );
        add_generics_records(
            accumulator,
            &context,
            symbols,
            &method.sig.generics,
            &enclosing,
            &visibility,
            &method_cfg,
        );
        if let ReturnType::Type(_, ty) = &method.sig.output {
            add_type_records(
                accumulator,
                &context,
                symbols,
                ty,
                &enclosing,
                "return",
                &visibility,
                &method_cfg,
                PersistenceOperation::TypeReference,
            );
        }
        let mut analyzer = BodyAnalyzer::new(
            RecordContext {
                classification: context.classification,
                source_class: context.source_class,
                package: context.package,
                module: context.module,
                source: context.source,
            },
            accumulator,
            errors,
            symbols,
            enclosing,
            visibility,
            method_cfg.clone(),
        );
        analyzer.visible_macro_shadows = visible_macro_shadows.clone();
        analyzer.register_local_uses(&method.block.stmts);
        analyzer.register_local_callables(&method.block.stmts);
        analyzer.register_local_constants(&method.block.stmts);
        analyzer.register_generic_bounds(&item_impl.generics);
        analyzer.register_generic_bounds(&method.sig.generics);
        analyzer.active_trait = item_impl
            .trait_
            .as_ref()
            .map(|(_, path, _)| canonical_path_names(path_names(path), symbols).join("::"));
        let mut self_info = analyzer.info_from_type(&item_impl.self_ty);
        self_info.flow = Flow::default();
        analyzer.bind("Self".to_owned(), self_info.clone());
        if method.sig.receiver().is_some() {
            analyzer.bind("self".to_owned(), self_info);
        }
        analyzer.register_parameters(&method.sig.inputs);
        for statement in &method.block.stmts {
            analyzer.visit_stmt(statement);
        }
        let tail = implicit_tail_flow(&method.block, &analyzer);
        if !tail.pool_targets().is_empty() {
            let fingerprint = method
                .block
                .stmts
                .last()
                .map(normalized_tokens)
                .unwrap_or_default();
            analyzer.record_pool_escape(
                &tail,
                PersistenceOperation::ReturnEscape,
                "pool",
                &method_cfg,
                fingerprint,
            );
        }
    }
}
