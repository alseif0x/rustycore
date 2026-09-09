//! Persistence targets and shapes resolved from type syntax.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn targets_for_names(names: &[String], symbols: &ModuleSymbols) -> TargetSet {
    let mut targets = TargetSet::new();
    let Some(first) = names.first() else {
        return targets;
    };
    let last = names.last().expect("non-empty path");
    if path_is_sqlx(names, symbols) {
        targets.insert(
            match names.iter().find_map(|name| match name.as_str() {
                "Transaction" => Some(PersistenceTarget::SqlxTransaction),
                _ => None,
            }) {
                Some(target) => target,
                None => match last.as_str() {
                    "MySqlPool" => PersistenceTarget::MySqlPool,
                    "PgPool" => PersistenceTarget::PgPool,
                    "DatabaseConnection" => PersistenceTarget::DatabaseConnection,
                    _ => PersistenceTarget::Sqlx,
                },
            },
        );
    }
    if symbols.database_namespaces.contains(first) {
        for name in names.iter().skip(1) {
            if let Some(target) = PersistenceTarget::from_name(name) {
                targets.insert(target);
                break;
            }
        }
    }
    // A locally imported type alias can only own the root of a path. Looking
    // at every segment makes an unrelated enum variant such as
    // `DatabaseError::Transaction` inherit an in-scope `sqlx::Transaction`
    // import merely because the leaf names collide.
    if let Some(alias_targets) = symbols.type_aliases.get(first) {
        targets.extend(alias_targets);
    }
    targets
}

pub(super) fn targets_for_path(path: &syn::Path, symbols: &ModuleSymbols) -> TargetSet {
    targets_for_names(&path_names(path), symbols)
}

pub(super) struct TypeTargetCollector<'a> {
    pub(super) symbols: &'a ModuleSymbols,
    pub(super) targets: TargetSet,
}

impl<'ast> Visit<'ast> for TypeTargetCollector<'_> {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        self.targets
            .extend(targets_for_path(&ty.path, self.symbols));
        visit::visit_type_path(self, ty);
    }
}

pub(super) fn targets_in_type(ty: &Type, symbols: &ModuleSymbols) -> TargetSet {
    let mut collector = TypeTargetCollector {
        symbols,
        targets: TargetSet::new(),
    };
    collector.visit_type(ty);
    collector.targets
}

pub(super) fn targets_in_generics(generics: &syn::Generics, symbols: &ModuleSymbols) -> TargetSet {
    let mut collector = TypeTargetCollector {
        symbols,
        targets: TargetSet::new(),
    };
    collector.visit_generics(generics);
    collector.targets
}

pub(super) fn nominal_types_in_type(ty: &Type) -> BTreeSet<String> {
    match ty {
        Type::Path(path) => last_path_name(&path.path)
            .map(|name| BTreeSet::from([name]))
            .unwrap_or_default(),
        Type::Reference(reference) => nominal_types_in_type(&reference.elem),
        Type::Ptr(pointer) => nominal_types_in_type(&pointer.elem),
        Type::Paren(paren) => nominal_types_in_type(&paren.elem),
        Type::Group(group) => nominal_types_in_type(&group.elem),
        _ => BTreeSet::new(),
    }
}

pub(super) fn receiver_nominal_types_in_type(ty: &Type) -> BTreeSet<String> {
    match ty {
        Type::Path(path) => {
            let Some(segment) = path.path.segments.last() else {
                return BTreeSet::new();
            };
            let name = normalized_ident(&segment.ident);
            if matches!(name.as_str(), "Box" | "Arc" | "Rc" | "Pin")
                && let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner)) = arguments.args.first()
            {
                receiver_nominal_types_in_type(inner)
            } else {
                BTreeSet::from([name])
            }
        }
        Type::Reference(reference) => receiver_nominal_types_in_type(&reference.elem),
        Type::Ptr(pointer) => receiver_nominal_types_in_type(&pointer.elem),
        Type::Paren(paren) => receiver_nominal_types_in_type(&paren.elem),
        Type::Group(group) => receiver_nominal_types_in_type(&group.elem),
        _ => BTreeSet::new(),
    }
}

pub(super) fn nominal_shape_in_type(ty: &Type, symbols: &ModuleSymbols) -> Option<NominalShape> {
    match ty {
        Type::Tuple(tuple) => Some(NominalShape {
            nominal_types: BTreeSet::new(),
            arguments: tuple
                .elems
                .iter()
                .map(|element| {
                    nominal_shape_in_type(element, symbols).unwrap_or_else(|| NominalShape {
                        nominal_types: BTreeSet::new(),
                        arguments: Vec::new(),
                    })
                })
                .collect(),
        }),
        Type::Path(path) => path.path.segments.last().map(|segment| NominalShape {
            nominal_types: resolve_nominal_types(
                BTreeSet::from([normalized_ident(&segment.ident)]),
                symbols,
            ),
            arguments: match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => arguments
                    .args
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::GenericArgument::Type(inner) => nominal_shape_in_type(inner, symbols),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
        }),
        Type::Reference(reference) => nominal_shape_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => nominal_shape_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => nominal_shape_in_type(&paren.elem, symbols),
        Type::Group(group) => nominal_shape_in_type(&group.elem, symbols),
        _ => None,
    }
}

pub(super) fn payload_variants_in_type(
    ty: &Type,
    symbols: &ModuleSymbols,
) -> BTreeSet<Vec<NominalShape>> {
    nominal_shape_in_type(ty, symbols)
        .and_then(|shape| (!shape.arguments.is_empty()).then_some(shape.arguments))
        .map(|arguments| BTreeSet::from([arguments]))
        .unwrap_or_default()
}

pub(super) fn tuple_items_in_type(ty: &Type, symbols: &ModuleSymbols) -> Vec<VariableInfo> {
    match ty {
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .map(|element| VariableInfo {
                flow: Flow::pools(&targets_in_type(element, symbols)),
                sql_expression: SqlExpressionKind::Static,
                sql_sources: BTreeSet::new(),
                nominal_types: resolve_nominal_types(
                    receiver_nominal_types_in_type(element),
                    symbols,
                ),
                payload_variants: payload_variants_in_type(element, symbols),
                tuple_items: tuple_items_in_type(element, symbols),
                field_items: BTreeMap::new(),
                trait_bounds: trait_bounds_in_type(element, symbols),
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
                mutable_pointees: BTreeSet::new(),
                mutable_places: BTreeSet::new(),
                query_callable: false,
                executor_callable: None,
            })
            .collect(),
        Type::Reference(reference) => tuple_items_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => tuple_items_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => tuple_items_in_type(&paren.elem, symbols),
        Type::Group(group) => tuple_items_in_type(&group.elem, symbols),
        Type::Path(path) => path
            .path
            .segments
            .last()
            .and_then(|segment| {
                symbols
                    .type_alias_info
                    .get(&normalized_ident(&segment.ident))
            })
            .map(|info| info.tuple_items.clone())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub(super) fn instantiate_named_type_info(
    named: &VariableInfo,
    path: &syn::TypePath,
    symbols: &ModuleSymbols,
) -> VariableInfo {
    let mut instantiated = named.clone();
    let arguments = path
        .path
        .segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
            _ => None,
        })
        .into_iter()
        .flat_map(|arguments| &arguments.args)
        .filter_map(|argument| match argument {
            syn::GenericArgument::Type(ty) => Some(variable_info_in_type(ty, symbols)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !arguments.is_empty() {
        let substitutions = instantiated
            .type_generic_params
            .iter()
            .cloned()
            .zip(arguments)
            .collect::<BTreeMap<_, _>>();
        substitute_nominal_params(&mut instantiated, &substitutions);
    }
    instantiated.type_generic_params.clear();
    instantiated
}

pub(super) fn variable_info_in_type(ty: &Type, symbols: &ModuleSymbols) -> VariableInfo {
    let mut info = VariableInfo {
        flow: Flow::pools(&targets_in_type(ty, symbols)),
        sql_expression: SqlExpressionKind::Static,
        sql_sources: BTreeSet::new(),
        nominal_types: resolve_nominal_types(receiver_nominal_types_in_type(ty), symbols),
        payload_variants: payload_variants_in_type(ty, symbols),
        tuple_items: tuple_items_in_type(ty, symbols),
        field_items: BTreeMap::new(),
        trait_bounds: trait_bounds_in_type(ty, symbols),
        type_generic_params: Vec::new(),
        callable_signatures: BTreeSet::new(),
        closure_mutations: BTreeMap::new(),
        mutable_pointees: BTreeSet::new(),
        mutable_places: BTreeSet::new(),
        query_callable: false,
        executor_callable: None,
    };
    if let Type::Path(path) = ty
        && let Some(alias) = path.path.segments.last().and_then(|segment| {
            symbols
                .type_alias_info
                .get(&normalized_ident(&segment.ident))
        })
    {
        info.union(alias);
    }
    if let Type::Path(path) = ty {
        let names = path_names(&path.path);
        if names
            .first()
            .is_some_and(|name| matches!(name.as_str(), "crate" | "self" | "super"))
        {
            let canonical = canonical_path_names(names, symbols).join("::");
            if !canonical.is_empty() {
                info.nominal_types.insert(canonical);
            }
        }
    }
    let canonical = match ty {
        Type::Path(path) => Some(canonical_path_names(path_names(&path.path), symbols).join("::")),
        Type::Reference(reference) => return variable_info_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => return variable_info_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => return variable_info_in_type(&paren.elem, symbols),
        Type::Group(group) => return variable_info_in_type(&group.elem, symbols),
        _ => None,
    };
    if let Some(canonical) = &canonical
        && let Some(named) = symbols.named_type_info.get(canonical)
        && let Type::Path(path) = ty
    {
        info.union(&instantiate_named_type_info(named, path, symbols));
    }
    if let Some(canonical) = &canonical
        && let Some(named) = symbols.workspace_named_type_info.get(canonical)
        && let Type::Path(path) = ty
    {
        info.union(&instantiate_named_type_info(named, path, symbols));
    }
    info
}

pub(super) fn payload_variants_in_path(
    path: &syn::Path,
    symbols: &ModuleSymbols,
) -> BTreeSet<Vec<NominalShape>> {
    path.segments
        .last()
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(arguments) => {
                let arguments = arguments
                    .args
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::GenericArgument::Type(inner) => nominal_shape_in_type(inner, symbols),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                (!arguments.is_empty()).then_some(BTreeSet::from([arguments]))
            }
            _ => None,
        })
        .unwrap_or_default()
}

pub(super) fn resolve_nominal_types(
    types: BTreeSet<String>,
    symbols: &ModuleSymbols,
) -> BTreeSet<String> {
    types
        .into_iter()
        .flat_map(|nominal| {
            symbols
                .nominal_type_aliases
                .get(&nominal)
                .cloned()
                .unwrap_or_else(|| BTreeSet::from([nominal]))
        })
        .collect()
}

pub(super) fn canonical_path_names(mut names: Vec<String>, symbols: &ModuleSymbols) -> Vec<String> {
    let mut absolute = false;
    let mut base = symbols.module_path.clone();
    match names.first().map(String::as_str) {
        Some("crate") => {
            names.remove(0);
            base.clear();
            absolute = true;
        }
        Some("self") => {
            names.remove(0);
            absolute = true;
        }
        Some("super") => {
            while names.first().is_some_and(|name| name == "super") {
                names.remove(0);
                base.pop();
            }
            absolute = true;
        }
        _ => {}
    }
    let mut seen = BTreeSet::new();
    while let Some(first) = names.first().cloned() {
        if !seen.insert(first.clone()) {
            break;
        }
        let Some(source) = symbols.path_aliases.get(&first) else {
            break;
        };
        if source.len() == 1 && source[0] == first {
            break;
        }
        let mut expanded = source.clone();
        expanded.extend(names.into_iter().skip(1));
        names = expanded;
        if names.first().is_some_and(|name| name == "crate") {
            names.remove(0);
        }
        base.clear();
        absolute = true;
    }
    // Only direct Cargo dependencies may introduce an external crate root.
    // Rewrite dependency renames to the provider's canonical registry root;
    // unrelated workspace packages must remain ordinary relative paths.
    if !absolute
        && let Some(first) = names.first_mut()
        && let Some(provider_root) = symbols.dependency_crate_aliases.get(first)
    {
        *first = provider_root.clone();
        base.clear();
        absolute = true;
    }
    if !absolute {
        base.extend(names);
        base
    } else {
        base.extend(names);
        base
    }
}

pub(super) fn path_is_sqlx(names: &[String], symbols: &ModuleSymbols) -> bool {
    if names
        .first()
        .is_some_and(|first| symbols.sqlx_namespaces.contains(first))
    {
        return true;
    }
    if symbols.workspace_sqlx_namespaces.is_empty() {
        return false;
    }
    let canonical = canonical_path_names(names.to_vec(), symbols);
    (1..=canonical.len()).any(|length| {
        symbols
            .workspace_sqlx_namespaces
            .contains(&canonical[..length].join("::"))
    })
}

pub(super) fn canonical_trait_path(path: &syn::Path, symbols: &ModuleSymbols) -> String {
    canonical_path_names(path_names(path), symbols).join("::")
}

pub(super) fn canonical_trait_path_in_module(
    path: &syn::Path,
    module_path: &[String],
    symbols: &ModuleSymbols,
) -> String {
    let mut scoped = symbols.clone();
    scoped.module_path = module_path.to_vec();
    canonical_trait_path(path, &scoped)
}

pub(super) fn record_trait_supertraits(
    trait_path: &str,
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    module_path: &[String],
    symbols: &mut ModuleSymbols,
) {
    let supertraits = bounds
        .iter()
        .filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(bound) => Some(canonical_trait_path_in_module(
                &bound.path,
                module_path,
                symbols,
            )),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if !supertraits.is_empty() {
        std::sync::Arc::make_mut(&mut symbols.trait_supertraits)
            .entry(trait_path.to_owned())
            .or_default()
            .extend(supertraits);
    }
}

pub(super) fn trait_bounds_in_type(ty: &Type, symbols: &ModuleSymbols) -> BTreeSet<String> {
    let bounds = match ty {
        Type::TraitObject(object) => Some(&object.bounds),
        Type::ImplTrait(object) => Some(&object.bounds),
        Type::Reference(reference) => return trait_bounds_in_type(&reference.elem, symbols),
        Type::Ptr(pointer) => return trait_bounds_in_type(&pointer.elem, symbols),
        Type::Paren(paren) => return trait_bounds_in_type(&paren.elem, symbols),
        Type::Group(group) => return trait_bounds_in_type(&group.elem, symbols),
        _ => None,
    };
    bounds
        .into_iter()
        .flatten()
        .filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(bound) => Some(canonical_trait_path(&bound.path, symbols)),
            _ => None,
        })
        .collect()
}
