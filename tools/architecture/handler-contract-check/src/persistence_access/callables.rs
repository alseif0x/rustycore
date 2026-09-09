//! Callable signatures, closure models and variable/nominal shapes.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct CallableSignature {
    pub(super) generic_params: Vec<String>,
    pub(super) generic_inputs: Vec<GenericInputSpec>,
}

pub(super) fn closure_pattern_info(
    pattern: &Pat,
    input_index: usize,
    path: &mut Vec<usize>,
    generic_params: &mut Vec<String>,
    input: &mut GenericInputSpec,
) -> VariableInfo {
    match pattern {
        Pat::Ident(_) => {
            let suffix = path
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("_");
            let marker = if suffix.is_empty() {
                format!("$closure_arg_{input_index}")
            } else {
                format!("$closure_arg_{input_index}_{suffix}")
            };
            generic_params.push(marker.clone());
            input.params.insert(marker.clone());
            if !path.is_empty() {
                input
                    .tuple_paths
                    .entry(marker.clone())
                    .or_default()
                    .push(path.clone());
            }
            VariableInfo {
                nominal_types: BTreeSet::from([marker]),
                ..VariableInfo::default()
            }
        }
        Pat::Tuple(tuple) => VariableInfo {
            tuple_items: tuple
                .elems
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    path.push(index);
                    let info =
                        closure_pattern_info(element, input_index, path, generic_params, input);
                    path.pop();
                    info
                })
                .collect(),
            ..VariableInfo::default()
        },
        Pat::Reference(reference) => {
            closure_pattern_info(&reference.pat, input_index, path, generic_params, input)
        }
        Pat::Type(typed) => {
            closure_pattern_info(&typed.pat, input_index, path, generic_params, input)
        }
        Pat::Paren(paren) => {
            closure_pattern_info(&paren.pat, input_index, path, generic_params, input)
        }
        Pat::Wild(_) => VariableInfo::default(),
        _ => {
            // Non-tuple projections do not have a structural path model yet.
            // Retain the complete argument conservatively instead of losing it.
            let marker = format!("$closure_arg_{input_index}");
            if !generic_params.contains(&marker) {
                generic_params.push(marker.clone());
            }
            input.params.insert(marker.clone());
            VariableInfo {
                nominal_types: BTreeSet::from([marker]),
                ..VariableInfo::default()
            }
        }
    }
}

pub(super) fn closure_callable_model(
    closure: &ExprClosure,
) -> (CallableSignature, Vec<VariableInfo>) {
    let mut generic_params = Vec::new();
    let mut generic_inputs = Vec::new();
    let mut parameter_infos = Vec::new();
    for (input_index, pattern) in closure.inputs.iter().enumerate() {
        let mut input = GenericInputSpec::default();
        let info = closure_pattern_info(
            pattern,
            input_index,
            &mut Vec::new(),
            &mut generic_params,
            &mut input,
        );
        generic_inputs.push(input);
        parameter_infos.push(info);
    }
    (
        CallableSignature {
            generic_params,
            generic_inputs,
        },
        parameter_infos,
    )
}

pub(super) fn closure_callable_signature(closure: &ExprClosure) -> CallableSignature {
    closure_callable_model(closure).0
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct VariableInfo {
    pub(super) flow: Flow,
    pub(super) sql_expression: SqlExpressionKind,
    pub(super) sql_sources: BTreeSet<String>,
    pub(super) nominal_types: BTreeSet<String>,
    pub(super) payload_variants: BTreeSet<Vec<NominalShape>>,
    pub(super) tuple_items: Vec<VariableInfo>,
    pub(super) field_items: BTreeMap<String, VariableInfo>,
    pub(super) trait_bounds: BTreeSet<String>,
    pub(super) type_generic_params: Vec<String>,
    pub(super) callable_signatures: BTreeSet<CallableSignature>,
    pub(super) closure_mutations: BTreeMap<String, VariableInfo>,
    pub(super) mutable_pointees: BTreeSet<String>,
    pub(super) mutable_places: BTreeSet<MutablePlace>,
    pub(super) query_callable: bool,
    /// The SQLx executor method this value names, when it is one. A stored
    /// `sqlx::Executor::execute` still executes the SQL it is handed.
    pub(super) executor_callable: Option<String>,
}

impl VariableInfo {
    pub(super) fn union(&mut self, other: &Self) {
        self.flow.union(other.flow.clone());
        self.nominal_types
            .extend(other.nominal_types.iter().cloned());
        self.payload_variants
            .extend(other.payload_variants.iter().cloned());
        self.trait_bounds.extend(other.trait_bounds.iter().cloned());
        if self.type_generic_params.is_empty() {
            self.type_generic_params = other.type_generic_params.clone();
        }
        self.callable_signatures
            .extend(other.callable_signatures.iter().cloned());
        for (name, mutation) in &other.closure_mutations {
            self.closure_mutations
                .entry(name.clone())
                .or_default()
                .union(mutation);
        }
        for (field, other_info) in &other.field_items {
            self.field_items
                .entry(field.clone())
                .or_default()
                .union(other_info);
        }
        self.sql_expression = self.sql_expression.max(other.sql_expression);
        self.sql_sources.extend(other.sql_sources.iter().cloned());
        self.mutable_pointees
            .extend(other.mutable_pointees.iter().cloned());
        self.mutable_places
            .extend(other.mutable_places.iter().cloned());
        self.query_callable |= other.query_callable;
        if self.executor_callable.is_none() {
            self.executor_callable = other.executor_callable.clone();
        }
        if self.tuple_items.len() < other.tuple_items.len() {
            self.tuple_items
                .resize_with(other.tuple_items.len(), VariableInfo::default);
        }
        for (item, other_item) in self.tuple_items.iter_mut().zip(&other.tuple_items) {
            item.union(other_item);
        }
    }

    pub(super) fn substitute_self(
        &mut self,
        receiver_types: &BTreeSet<String>,
        trait_bounds: &BTreeSet<String>,
    ) -> bool {
        let mut replaced = self.nominal_types.remove("Self");
        if replaced {
            self.nominal_types.extend(receiver_types.iter().cloned());
        }
        let mut variants = BTreeSet::new();
        for variant in std::mem::take(&mut self.payload_variants) {
            let mut shapes = variant;
            let mut variant_replaced = false;
            for shape in &mut shapes {
                variant_replaced |= shape.substitute_self(receiver_types);
            }
            if variant_replaced {
                replaced = true;
            }
            variants.insert(shapes);
        }
        self.payload_variants = variants;
        for item in &mut self.tuple_items {
            replaced |= item.substitute_self(receiver_types, trait_bounds);
        }
        for item in self.field_items.values_mut() {
            replaced |= item.substitute_self(receiver_types, trait_bounds);
        }
        if replaced {
            self.trait_bounds.extend(trait_bounds.iter().cloned());
        }
        replaced
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct NominalShape {
    pub(super) nominal_types: BTreeSet<String>,
    pub(super) arguments: Vec<NominalShape>,
}

impl NominalShape {
    pub(super) fn substitute_self(&mut self, receiver_types: &BTreeSet<String>) -> bool {
        let mut replaced = self.nominal_types.remove("Self");
        if replaced {
            self.nominal_types.extend(receiver_types.iter().cloned());
        }
        for argument in &mut self.arguments {
            replaced |= argument.substitute_self(receiver_types);
        }
        replaced
    }
}

/// Collapse an arbitrarily deep value shape into its reachable persistence
/// facts without recursively cloning that shape. Generic recursive types can
/// otherwise grow one structural layer per workspace fixed-point pass and
/// eventually overflow the stack before the registry converges.
pub(super) fn flatten_reachable_variable_info(root: &VariableInfo) -> VariableInfo {
    let mut flattened = VariableInfo::default();
    let mut pending = vec![root];
    while let Some(info) = pending.pop() {
        flattened.flow.union(info.flow.clone());
        flattened
            .nominal_types
            .extend(info.nominal_types.iter().cloned());
        flattened
            .trait_bounds
            .extend(info.trait_bounds.iter().cloned());
        if flattened.type_generic_params.is_empty() {
            flattened.type_generic_params = info.type_generic_params.clone();
        }
        flattened
            .callable_signatures
            .extend(info.callable_signatures.iter().cloned());
        flattened.query_callable |= info.query_callable;
        if flattened.executor_callable.is_none() {
            flattened.executor_callable = info.executor_callable.clone();
        }
        flattened.sql_expression = flattened.sql_expression.max(info.sql_expression);
        pending.extend(info.tuple_items.iter());
        pending.extend(info.field_items.values());
        pending.extend(info.closure_mutations.values());

        let mut shapes = info
            .payload_variants
            .iter()
            .flat_map(|variant| variant.iter())
            .collect::<Vec<_>>();
        while let Some(shape) = shapes.pop() {
            flattened
                .nominal_types
                .extend(shape.nominal_types.iter().cloned());
            shapes.extend(shape.arguments.iter());
        }
    }
    flattened
}

/// Replaces generic parameter names with the bound's recorded type
/// arguments, recursively (e.g. a `Maker<T>` return resolved through
/// `M: Maker<Holder>` must surface `Holder`, not the nominal `T`). The
/// recorded argument carries its full `VariableInfo`, so the named-type
/// registry data (flow, fields, payloads) of the concrete type arrives with
/// the substitution.
pub(super) fn substitute_shape_params(
    shape: &mut NominalShape,
    map: &BTreeMap<String, VariableInfo>,
    merged: &mut VariableInfo,
) -> bool {
    let mut replaced = false;
    let original = std::mem::take(&mut shape.nominal_types);
    shape.nominal_types = original
        .into_iter()
        .flat_map(|name| match map.get(&name) {
            Some(replacement) => {
                replaced = true;
                // A concrete generic argument may itself contain this nominal
                // payload recursively. Flatten its reachable information
                // before merging so workspace named-type discovery has a
                // finite abstract state while remaining conservative.
                let widened = flatten_reachable_variable_info(replacement);
                merged.union(&widened);
                replacement
                    .nominal_types
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
            }
            None => vec![name],
        })
        .collect();
    for argument in &mut shape.arguments {
        replaced |= substitute_shape_params(argument, map, merged);
    }
    replaced
}

pub(super) fn substitute_nominal_params(
    info: &mut VariableInfo,
    map: &BTreeMap<String, VariableInfo>,
) -> bool {
    let mut replaced = false;
    let original = std::mem::take(&mut info.nominal_types);
    let mut resolved_names = BTreeSet::new();
    let mut merged = VariableInfo::default();
    for name in original {
        match map.get(&name) {
            Some(replacement) => {
                replaced = true;
                resolved_names.extend(replacement.nominal_types.iter().cloned());
                merged.union(replacement);
            }
            None => {
                resolved_names.insert(name);
            }
        }
    }
    info.nominal_types = resolved_names;
    info.union(&merged);
    for item in &mut info.tuple_items {
        replaced |= substitute_nominal_params(item, map);
    }
    for item in info.field_items.values_mut() {
        replaced |= substitute_nominal_params(item, map);
    }
    for mutation in info.closure_mutations.values_mut() {
        replaced |= substitute_nominal_params(mutation, map);
    }
    let variants = std::mem::take(&mut info.payload_variants);
    let mut payload_replacements = VariableInfo::default();
    info.payload_variants = variants
        .into_iter()
        .map(|shapes| {
            shapes
                .into_iter()
                .map(|mut shape| {
                    replaced |= substitute_shape_params(&mut shape, map, &mut payload_replacements);
                    shape
                })
                .collect()
        })
        .collect();
    info.union(&payload_replacements);
    replaced
}
