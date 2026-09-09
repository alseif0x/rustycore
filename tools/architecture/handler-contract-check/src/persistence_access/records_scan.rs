//! Record emission for types, generics and attributes.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn add_type_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    ty: &Type,
    enclosing: &str,
    symbol: &str,
    visibility: &str,
    cfg: &[String],
    operation: PersistenceOperation,
) {
    for target in targets_in_type(ty, symbols) {
        accumulator.add(
            context,
            NewAccess {
                enclosing,
                target,
                operation,
                symbol,
                visibility,
                cfg,
                fingerprint: normalized_tokens(ty),
                generated_input: false,
            },
        );
    }
}

pub(super) fn add_generics_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    generics: &syn::Generics,
    enclosing: &str,
    visibility: &str,
    cfg: &[String],
) {
    for target in targets_in_generics(generics, symbols) {
        accumulator.add(
            context,
            NewAccess {
                enclosing,
                target,
                operation: PersistenceOperation::TypeReference,
                symbol: "generics",
                visibility,
                cfg,
                fingerprint: normalized_tokens(generics),
                generated_input: false,
            },
        );
    }
}

pub(super) fn pattern_identifiers(pattern: &Pat, output: &mut Vec<String>) {
    match pattern {
        Pat::Ident(ident) => {
            output.push(normalized_ident(&ident.ident));
            if let Some((_, subpat)) = &ident.subpat {
                pattern_identifiers(subpat, output);
            }
        }
        Pat::Reference(reference) => pattern_identifiers(&reference.pat, output),
        Pat::Type(typed) => pattern_identifiers(&typed.pat, output),
        Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                pattern_identifiers(element, output);
            }
        }
        Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                pattern_identifiers(element, output);
            }
        }
        Pat::Struct(structure) => {
            for field in &structure.fields {
                pattern_identifiers(&field.pat, output);
            }
        }
        Pat::Slice(slice) => {
            for element in &slice.elems {
                pattern_identifiers(element, output);
            }
        }
        Pat::Paren(paren) => pattern_identifiers(&paren.pat, output),
        Pat::Or(or_pattern) => {
            for case in &or_pattern.cases {
                pattern_identifiers(case, output);
            }
        }
        _ => {}
    }
}

pub(super) fn tokens_contain_identifier(tokens: TokenStream, names: &BTreeSet<String>) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => names.contains(&normalized_ident(&ident)),
        TokenTree::Group(group) => tokens_contain_identifier(group.stream(), names),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

pub(super) fn control_flow_sql_kind(tokens: TokenStream) -> SqlExpressionKind {
    tokens
        .into_iter()
        .fold(SqlExpressionKind::Nonliteral, |kind, token| {
            let candidate = match token {
                TokenTree::Ident(ident) => match normalized_ident(&ident).as_str() {
                    "include_str" => SqlExpressionKind::Included,
                    "env" => SqlExpressionKind::Environment,
                    "format" | "format_args" => SqlExpressionKind::Interpolated,
                    _ => SqlExpressionKind::Nonliteral,
                },
                TokenTree::Group(group) => control_flow_sql_kind(group.stream()),
                TokenTree::Punct(_) | TokenTree::Literal(_) => SqlExpressionKind::Nonliteral,
            };
            kind.max(candidate)
        })
}

pub(super) fn tokens_contain_callable_invocation(
    tokens: TokenStream,
    names: &BTreeSet<String>,
) -> bool {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token
            && tokens_contain_callable_invocation(group.stream(), names)
        {
            return true;
        }
        if let TokenTree::Ident(ident) = token
            && names.contains(&normalized_ident(ident))
            && matches!(
                tokens.get(index + 1),
                Some(TokenTree::Group(group))
                    if group.delimiter() == proc_macro2::Delimiter::Parenthesis
            )
        {
            return true;
        }
    }
    false
}

pub(super) fn tokens_contain_path_root(tokens: TokenStream, names: &BTreeSet<String>) -> bool {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for window in tokens.windows(3) {
        if let [
            TokenTree::Ident(ident),
            TokenTree::Punct(first),
            TokenTree::Punct(second),
        ] = window
            && names.contains(&normalized_ident(ident))
            && first.as_char() == ':'
            && second.as_char() == ':'
        {
            return true;
        }
    }
    tokens.into_iter().any(|token| match token {
        TokenTree::Group(group) => tokens_contain_path_root(group.stream(), names),
        _ => false,
    })
}

pub(super) fn module_persistence_names(symbols: &ModuleSymbols) -> BTreeSet<String> {
    let mut names = BTreeSet::from([
        "sqlx".to_owned(),
        "MySqlPool".to_owned(),
        "PgPool".to_owned(),
        "DatabaseConnection".to_owned(),
    ]);
    names.extend(symbols.sqlx_namespaces.iter().cloned());
    names.extend(symbols.database_namespaces.iter().cloned());
    names.extend(symbols.type_aliases.keys().cloned());
    names.extend(symbols.query_callables.iter().cloned());
    names
}

pub(super) fn syntax_mentions_persistence(value: &impl ToTokens, symbols: &ModuleSymbols) -> bool {
    tokens_contain_identifier(value.to_token_stream(), &module_persistence_names(symbols))
}

/// Extracts concrete persistence targets from fully qualified adapter paths
/// (`wow_database::CharacterDatabase::open(...)`) inside an opaque token
/// stream, reusing the same name mapping as ordinary paths. Without this an
/// allowed opaque macro such as `assert!(wow_database::...)` would emit
/// neither a database row nor a fail-closed error.
pub(super) fn database_targets_in_tokens(
    tokens: TokenStream,
    symbols: &ModuleSymbols,
) -> TargetSet {
    let mut targets = TargetSet::new();
    let trees = tokens.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < trees.len() {
        if let TokenTree::Group(group) = &trees[index] {
            targets.extend(database_targets_in_tokens(group.stream(), symbols));
        }
        if let TokenTree::Ident(ident) = &trees[index] {
            let root = normalized_ident(ident);
            if symbols.database_namespaces.contains(&root) {
                let mut names = vec![root];
                let mut cursor = index + 1;
                while cursor + 2 < trees.len()
                    && matches!(&trees[cursor], TokenTree::Punct(punct) if punct.as_char() == ':')
                    && matches!(&trees[cursor + 1], TokenTree::Punct(punct) if punct.as_char() == ':')
                {
                    let TokenTree::Ident(segment) = &trees[cursor + 2] else {
                        break;
                    };
                    names.push(normalized_ident(segment));
                    cursor += 3;
                }
                if names.len() > 1 {
                    targets.extend(targets_for_names(&names, symbols));
                }
            }
        }
        index += 1;
    }
    targets
}

pub(super) fn targets_in_tokens(tokens: TokenStream, symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    if tokens_contain_path_root(tokens.clone(), &symbols.sqlx_namespaces) {
        targets.insert(PersistenceTarget::Sqlx);
    }
    targets.extend(database_targets_in_tokens(tokens.clone(), symbols));
    for (name, alias_targets) in &symbols.type_aliases {
        if tokens_contain_identifier(tokens.clone(), &BTreeSet::from([name.clone()])) {
            targets.extend(alias_targets);
        }
    }
    targets
}

pub(super) fn cfg_predicate_allows_source(
    predicate: &syn::Meta,
    source_class: PersistenceSourceClass,
) -> bool {
    let attribute: Attribute = syn::parse_quote!(#[cfg(#predicate)]);
    match source_class {
        PersistenceSourceClass::Production => {
            cfg_context_allows_production(&[], &[attribute]).unwrap_or(false)
        }
        PersistenceSourceClass::TestFixture => {
            cfg_context_allows_test(&[], &[attribute]).unwrap_or(false)
        }
    }
}

pub(super) fn targets_in_attribute_meta(
    meta: &syn::Meta,
    symbols: &ModuleSymbols,
    source_class: PersistenceSourceClass,
    inherited_cfg: &[String],
) -> Vec<(PersistenceTarget, Vec<String>)> {
    if meta.path().is_ident("cfg") {
        return Vec::new();
    }
    if meta.path().is_ident("cfg_attr") {
        let syn::Meta::List(list) = meta else {
            return Vec::new();
        };
        let Ok(items) = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
        else {
            return Vec::new();
        };
        let mut items = items.iter();
        let Some(predicate) = items.next() else {
            return Vec::new();
        };
        if !cfg_predicate_allows_source(predicate, source_class) {
            return Vec::new();
        }
        let conditional: Attribute = syn::parse_quote!(#[cfg(#predicate)]);
        let mut nested_cfg = inherited_cfg.to_vec();
        nested_cfg.push(conditional.meta.to_token_stream().to_string());
        nested_cfg.sort();
        nested_cfg.dedup();
        let mut targets = Vec::new();
        for nested in items {
            targets.extend(targets_in_attribute_meta(
                nested,
                symbols,
                source_class,
                &nested_cfg,
            ));
        }
        return targets;
    }
    let mut targets = targets_for_path(meta.path(), symbols);
    targets.extend(targets_in_tokens(meta.to_token_stream(), symbols));
    targets
        .into_iter()
        .map(|target| (target, inherited_cfg.to_vec()))
        .collect()
}

pub(super) fn targets_in_attributes(
    attribute: &Attribute,
    symbols: &ModuleSymbols,
    source_class: PersistenceSourceClass,
) -> Vec<(PersistenceTarget, Vec<String>)> {
    targets_in_attribute_meta(&attribute.meta, symbols, source_class, &[])
}

pub(super) struct AttributeRecordContext<'a> {
    pub(super) enclosing: &'a str,
    pub(super) visibility: &'a str,
    pub(super) cfg: &'a [String],
}

pub(super) fn add_attribute_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    attributes: &[Attribute],
    record: AttributeRecordContext<'_>,
) {
    for attribute in attributes {
        let symbol = last_path_name(attribute.path()).unwrap_or_else(|| "attribute".to_owned());
        for (target, conditional_cfg) in
            targets_in_attributes(attribute, symbols, context.source_class)
        {
            let mut cfg = record.cfg.to_vec();
            cfg.extend(conditional_cfg);
            cfg.sort();
            cfg.dedup();
            accumulator.add(
                context,
                NewAccess {
                    enclosing: record.enclosing,
                    target,
                    operation: PersistenceOperation::MacroReference,
                    symbol: &symbol,
                    visibility: record.visibility,
                    cfg: &cfg,
                    fingerprint: normalized_tokens(attribute),
                    generated_input: true,
                },
            );
        }
    }
}
