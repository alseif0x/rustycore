//! Per-item analysis entry points, part 2: item macros and module walks.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn analyze_item_macro(
    item_macro: &syn::ItemMacro,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    // An alias does not change what the macro brings in, so the guard below
    // reads the path it resolves to.
    let path_name = canonical_path_names(path_names(&item_macro.mac.path), symbols)
        .last()
        .cloned()
        .unwrap_or_default();
    let symbol = item_macro
        .ident
        .as_ref()
        .map(normalized_ident)
        .unwrap_or_else(|| path_name.clone());
    let mut targets = targets_for_path(&item_macro.mac.path, symbols);
    targets.extend(targets_in_tokens(item_macro.mac.tokens.clone(), symbols));
    if path_name == "include" && !is_pinned_wow_proto_include(context, &item_macro.mac) {
        errors.push(format!(
            "{} contains include! whose Rust source is outside the persistence AST inventory; mount and parse the included source explicitly",
            context.module
        ));
        return;
    }
    for target in targets {
        accumulator.add(
            context,
            NewAccess {
                enclosing: "module",
                target,
                operation: PersistenceOperation::MacroReference,
                symbol: &symbol,
                visibility: "",
                cfg,
                fingerprint: normalized_tokens(&item_macro.mac),
                generated_input: true,
            },
        );
    }
}

pub(super) fn is_pinned_wow_proto_include(context: &RecordContext<'_>, mac: &syn::Macro) -> bool {
    if context.package != "wow-proto" || context.source != "crates/wow-proto/src/lib.rs" {
        return false;
    }
    let suffix = match context.module {
        "crate::bgs::protocol" => "/bgs.protocol.rs",
        "crate::bgs::protocol::account::v1" => "/bgs.protocol.account.v1.rs",
        "crate::bgs::protocol::authentication::v1" => "/bgs.protocol.authentication.v1.rs",
        "crate::bgs::protocol::challenge::v1" => "/bgs.protocol.challenge.v1.rs",
        "crate::bgs::protocol::connection::v1" => "/bgs.protocol.connection.v1.rs",
        "crate::bgs::protocol::game_utilities::v1" => "/bgs.protocol.game_utilities.v1.rs",
        _ => return false,
    };
    let expected: syn::ItemMacro = syn::parse_str(&format!(
        "include!(concat!(env!(\"OUT_DIR\"), {suffix:?}));"
    ))
    .expect("pinned wow-proto include syntax is valid");
    normalized_tokens(mac) == normalized_tokens(&expected.mac)
}

pub(super) fn analyze_module_items(
    items: &[Item],
    context: RecordContext<'_>,
    parent_symbols: Option<&ModuleSymbols>,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let symbols = collect_module_symbols(
        items,
        parent_symbols,
        context.package,
        context.module,
        &cfg,
        context.source_class,
        errors,
    );
    for (item_index, item) in items.iter().enumerate() {
        match item {
            Item::Use(item_use) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_use.attrs,
                    errors,
                    "use declaration",
                ) {
                    continue;
                }
                analyze_import(
                    item_use,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_use.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::ExternCrate(extern_crate) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &extern_crate.attrs,
                    errors,
                    "extern crate",
                ) {
                    continue;
                }
                let source = normalized_ident(&extern_crate.ident);
                let extern_cfg = item_cfg(&cfg, &extern_crate.attrs);
                let extern_visibility = normalized_visibility(&extern_crate.vis);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &extern_crate.attrs,
                    AttributeRecordContext {
                        enclosing: "module",
                        visibility: &extern_visibility,
                        cfg: &extern_cfg,
                    },
                );
                let local = extern_crate
                    .rename
                    .as_ref()
                    .map(|(_, rename)| normalized_ident(rename))
                    .unwrap_or_else(|| source.clone());
                let import_target = match source.as_str() {
                    "sqlx" => Some(PersistenceTarget::Sqlx),
                    "wow_database" => Some(PersistenceTarget::Database),
                    _ => None,
                };
                if let Some(target) = import_target {
                    accumulator.add(
                        &context,
                        NewAccess {
                            enclosing: "module",
                            target,
                            operation: PersistenceOperation::Import,
                            symbol: &local,
                            visibility: &extern_visibility,
                            cfg: &extern_cfg,
                            fingerprint: format!("extern crate {source} as {local}"),
                            generated_input: false,
                        },
                    );
                }
            }
            Item::Type(alias) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &alias.attrs,
                    errors,
                    "type alias",
                ) {
                    continue;
                }
                analyze_type_alias(
                    alias,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &alias.attrs),
                    accumulator,
                );
            }
            Item::Struct(item_struct) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_struct.attrs,
                    errors,
                    "struct",
                ) {
                    continue;
                }
                analyze_struct(
                    item_struct,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_struct.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Enum(item_enum) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_enum.attrs,
                    errors,
                    "enum",
                ) {
                    continue;
                }
                analyze_enum(
                    item_enum,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_enum.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Fn(function) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &function.attrs,
                    errors,
                    "function",
                ) {
                    continue;
                }
                analyze_function(
                    function,
                    RecordContext {
                        classification: context.classification,
                        source_class: context.source_class,
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    &symbols,
                    macro_shadows_before(items, item_index),
                    item_cfg(&cfg, &function.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Impl(item_impl) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_impl.attrs,
                    errors,
                    "impl",
                ) {
                    continue;
                }
                analyze_impl(
                    item_impl,
                    RecordContext {
                        classification: context.classification,
                        source_class: context.source_class,
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    &symbols,
                    macro_shadows_before(items, item_index),
                    item_cfg(&cfg, &item_impl.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Const(item_const) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_const.attrs,
                    errors,
                    "const",
                ) {
                    continue;
                }
                let item_cfg = item_cfg(&cfg, &item_const.attrs);
                let enclosing = format!("const {}", normalized_ident(&item_const.ident));
                let visibility = normalized_visibility(&item_const.vis);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.attrs,
                    AttributeRecordContext {
                        enclosing: &enclosing,
                        visibility: &visibility,
                        cfg: &item_cfg,
                    },
                );
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.ty,
                    &enclosing,
                    &normalized_ident(&item_const.ident),
                    &visibility,
                    &item_cfg,
                    PersistenceOperation::TypeReference,
                );
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
                    &symbols,
                    enclosing,
                    visibility,
                    item_cfg,
                );
                analyzer.visit_expr(&item_const.expr);
            }
            Item::Static(item_static) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_static.attrs,
                    errors,
                    "static",
                ) {
                    continue;
                }
                let item_cfg = item_cfg(&cfg, &item_static.attrs);
                let enclosing = format!("static {}", normalized_ident(&item_static.ident));
                let visibility = normalized_visibility(&item_static.vis);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_static.attrs,
                    AttributeRecordContext {
                        enclosing: &enclosing,
                        visibility: &visibility,
                        cfg: &item_cfg,
                    },
                );
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_static.ty,
                    &enclosing,
                    &normalized_ident(&item_static.ident),
                    &visibility,
                    &item_cfg,
                    PersistenceOperation::TypeReference,
                );
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
                    &symbols,
                    enclosing,
                    visibility,
                    item_cfg,
                );
                analyzer.visit_expr(&item_static.expr);
            }
            Item::Trait(item_trait) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_trait.attrs,
                    errors,
                    "trait",
                ) {
                    continue;
                }
                let name = normalized_ident(&item_trait.ident);
                let targets = symbols.type_aliases.get(&name).cloned().unwrap_or_default();
                if context.package != "wow-database" || targets.is_empty() {
                    // A default trait body is ordinary executable code:
                    // analyze it before anything else, because it can reach
                    // persistence through a nominal wrapper
                    // (`self.holder().0.pool()`) even when the trait
                    // signature never names a concrete database type.
                    analyze_trait_default_bodies(
                        item_trait,
                        &context,
                        &symbols,
                        macro_shadows_before(items, item_index),
                        &cfg,
                        accumulator,
                        errors,
                    );
                    if syntax_mentions_persistence(item_trait, &symbols) {
                        errors.push(format!(
                            "{} contains persistence syntax in an unsupported item grammar; expose an explicit import/type/function/impl before baselining: {}",
                            context.module,
                            normalized_tokens(item_trait)
                        ));
                    }
                    continue;
                }
                let trait_cfg = item_cfg(&cfg, &item_trait.attrs);
                for target in targets {
                    accumulator.add(
                        &context,
                        NewAccess {
                            enclosing: &format!("trait {name}"),
                            target,
                            operation: PersistenceOperation::TypeReference,
                            symbol: &name,
                            visibility: &normalized_visibility(&item_trait.vis),
                            cfg: &trait_cfg,
                            fingerprint: normalized_tokens(item_trait),
                            generated_input: false,
                        },
                    );
                }
            }
            Item::Macro(item_macro) => {
                if !source_class_allows(
                    context.source_class,
                    &cfg,
                    &item_macro.attrs,
                    errors,
                    "item macro",
                ) {
                    continue;
                }
                let macro_cfg = item_cfg(&cfg, &item_macro.attrs);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_macro.attrs,
                    AttributeRecordContext {
                        enclosing: "module",
                        visibility: "",
                        cfg: &macro_cfg,
                    },
                );
                analyze_item_macro(
                    item_macro,
                    &context,
                    &symbols,
                    &macro_cfg,
                    accumulator,
                    errors,
                );
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) => {
                if !source_class_allows(context.source_class, &cfg, attrs, errors, "inline module")
                {
                    continue;
                }
                let child_module = format!("{}::{}", context.module, normalized_ident(ident));
                let module_cfg = item_cfg(&cfg, attrs);
                add_attribute_records(
                    accumulator,
                    &context,
                    &symbols,
                    attrs,
                    AttributeRecordContext {
                        enclosing: &child_module,
                        visibility: "",
                        cfg: &module_cfg,
                    },
                );
                analyze_module_items(
                    child_items,
                    RecordContext {
                        classification: context.classification,
                        source_class: context.source_class,
                        package: context.package,
                        module: &child_module,
                        source: context.source,
                    },
                    Some(&symbols),
                    module_cfg,
                    accumulator,
                    errors,
                );
            }
            unsupported => {
                if syntax_mentions_persistence(unsupported, &symbols) {
                    errors.push(format!(
                        "{} contains persistence syntax in an unsupported item grammar; expose an explicit import/type/function/impl before baselining: {}",
                        context.module,
                        normalized_tokens(unsupported)
                    ));
                }
            }
        }
    }
}
