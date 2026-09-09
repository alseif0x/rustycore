//! Assignment places, loop widening and tail-flow helpers for the body analyzer.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

pub(super) fn implicit_tail_info(
    block: &syn::Block,
    analyzer: &BodyAnalyzer<'_, '_>,
) -> VariableInfo {
    if let Some(info) = analyzer
        .block_result_infos
        .borrow()
        .get(&(block as *const syn::Block as usize))
    {
        return info.clone();
    }
    match block.stmts.last() {
        Some(Stmt::Expr(expression, None)) => analyzer.info_from_expr(expression),
        _ => VariableInfo::default(),
    }
}

pub(super) fn implicit_tail_flow(block: &syn::Block, analyzer: &BodyAnalyzer<'_, '_>) -> Flow {
    implicit_tail_info(block, analyzer).flow
}

/// Unions two scope stacks captured after mutually exclusive match arms. Both
/// stacks descend from the same pre-match snapshot, so zip is exact: every
/// outer local assigned by any arm keeps the union of all arm outcomes.
pub(super) fn merge_scope_stacks(
    accumulated: &mut [BTreeMap<String, VariableInfo>],
    incoming: &[BTreeMap<String, VariableInfo>],
) {
    for (accumulated_scope, incoming_scope) in accumulated.iter_mut().zip(incoming.iter()) {
        for (name, info) in incoming_scope {
            accumulated_scope
                .entry(name.clone())
                .or_default()
                .union(info);
        }
    }
}

pub(super) fn collect_shape_nominals(shape: &NominalShape, output: &mut BTreeSet<String>) {
    output.extend(shape.nominal_types.iter().cloned());
    for argument in &shape.arguments {
        collect_shape_nominals(argument, output);
    }
}

/// Loop-carried values can grow without a finite structural fixed point (for
/// example `node = Node::Branch(Box::new(node))`). Once several exact passes
/// have failed to stabilize, collapse nested projections into the containing
/// value while retaining every reachable flow, nominal type, trait bound and
/// callable effect. This is a conservative widening: later projections may
/// produce extra rows, but persistence can no longer disappear at arbitrary
/// runtime iteration depths and the abstract loop state remains finite.
pub(super) fn widen_loop_variable(info: &mut VariableInfo) {
    let mut nested = VariableInfo::default();
    for item in &mut info.tuple_items {
        widen_loop_variable(item);
        nested.union(item);
    }
    for item in info.field_items.values_mut() {
        widen_loop_variable(item);
        nested.union(item);
    }
    for mutation in info.closure_mutations.values_mut() {
        widen_loop_variable(mutation);
        nested.union(mutation);
    }
    for shapes in &info.payload_variants {
        for shape in shapes {
            collect_shape_nominals(shape, &mut nested.nominal_types);
        }
    }
    info.union(&nested);
    info.payload_variants.clear();
    info.tuple_items.clear();
    info.field_items.clear();
}

pub(super) fn widen_loop_scopes(scopes: &mut [BTreeMap<String, VariableInfo>]) {
    for scope in scopes {
        for info in scope.values_mut() {
            widen_loop_variable(info);
        }
    }
}

pub(super) fn simple_assignment_name(expression: &Expr) -> Option<String> {
    let Expr::Path(path) = expression else {
        return None;
    };
    (path.qself.is_none() && path.path.segments.len() == 1)
        .then(|| last_path_name(&path.path))
        .flatten()
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum PlaceProjection {
    Field(String),
    Index(Option<usize>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct MutablePlace {
    pub(super) root: String,
    pub(super) projections: Vec<PlaceProjection>,
}

pub(super) fn assignment_place(expression: &Expr) -> Option<(String, Vec<PlaceProjection>)> {
    fn collect(expression: &Expr, projections: &mut Vec<PlaceProjection>) -> Option<String> {
        match expression {
            Expr::Path(_) => simple_assignment_name(expression),
            Expr::Field(field) => {
                let root = collect(&field.base, projections)?;
                let name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                projections.push(PlaceProjection::Field(name));
                Some(root)
            }
            Expr::Index(index) => {
                let root = collect(&index.expr, projections)?;
                let numeric = match index.index.as_ref() {
                    Expr::Lit(literal) => match &literal.lit {
                        syn::Lit::Int(value) => value.base10_parse().ok(),
                        _ => None,
                    },
                    _ => None,
                };
                projections.push(PlaceProjection::Index(numeric));
                Some(root)
            }
            Expr::Paren(paren) => collect(&paren.expr, projections),
            Expr::Group(group) => collect(&group.expr, projections),
            _ => None,
        }
    }

    let mut projections = Vec::new();
    let root = collect(expression, &mut projections)?;
    (!projections.is_empty()).then_some((root, projections))
}

pub(super) fn assign_place_projection(
    aggregate: &mut VariableInfo,
    projections: &[PlaceProjection],
    value: &VariableInfo,
) {
    let Some((projection, remaining)) = projections.split_first() else {
        *aggregate = value.clone();
        return;
    };
    match projection {
        PlaceProjection::Field(name) => {
            assign_place_projection(
                aggregate.field_items.entry(name.clone()).or_default(),
                remaining,
                value,
            );
        }
        PlaceProjection::Index(index) => {
            // An index write makes the collection persistence-bearing even
            // when the exact runtime index is unknown. Retain an exact tuple/
            // array projection as well when a literal index is available.
            aggregate.union(value);
            if let Some(index) = index {
                if aggregate.tuple_items.len() <= *index {
                    aggregate
                        .tuple_items
                        .resize_with(index + 1, VariableInfo::default);
                }
                assign_place_projection(&mut aggregate.tuple_items[*index], remaining, value);
            }
        }
    }
}

pub(super) fn union_into_assignment_place(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    place: &Expr,
    info: &VariableInfo,
) -> bool {
    if let Some(root) = simple_assignment_name(place) {
        let mut aggregate = analyzer.lookup(&root).cloned().unwrap_or_default();
        aggregate.union(info);
        analyzer.assign(&root, aggregate);
        return true;
    }
    let Some((root, projections)) = assignment_place(place) else {
        return false;
    };
    let mut aggregate = analyzer.lookup(&root).cloned().unwrap_or_default();
    let mut projected = VariableInfo::default();
    assign_place_projection(&mut projected, &projections, info);
    aggregate.union(&projected);
    analyzer.assign(&root, aggregate);
    true
}

pub(super) fn is_assignment_binop(operation: &syn::BinOp) -> bool {
    matches!(
        operation,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

pub(super) fn mutable_storage_receiver_name(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Reference(reference) if reference.mutability.is_some() => {
            simple_assignment_name(&reference.expr)
        }
        Expr::Paren(paren) => mutable_storage_receiver_name(&paren.expr),
        Expr::Group(group) => mutable_storage_receiver_name(&group.expr),
        _ => None,
    }
}

pub(super) fn mutable_storage_place(expression: &Expr) -> Option<&Expr> {
    match expression {
        Expr::Reference(reference) if reference.mutability.is_some() => Some(&reference.expr),
        Expr::Paren(paren) => mutable_storage_place(&paren.expr),
        Expr::Group(group) => mutable_storage_place(&group.expr),
        _ => None,
    }
}

pub(super) fn visit_let_chain_condition(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    expression: &Expr,
    method_receiver_fallback: bool,
) {
    match expression {
        Expr::Let(let_expression) => {
            analyzer.visit_expr(&let_expression.expr);
            let mut info = analyzer.info_from_expr(&let_expression.expr);
            if method_receiver_fallback
                && info.flow.is_empty()
                && let Expr::MethodCall(method) = let_expression.expr.as_ref()
            {
                info = analyzer.info_from_expr(&method.receiver);
                analyzer.bind_pattern(&let_expression.pat, &info);
            } else {
                analyzer.bind_pattern_from_expr(&let_expression.pat, &let_expression.expr);
            }
        }
        Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
            visit_let_chain_condition(analyzer, &binary.left, method_receiver_fallback);
            let pre_rhs_scopes = analyzer.scopes.clone();
            visit_let_chain_condition(analyzer, &binary.right, method_receiver_fallback);
            merge_scope_stacks(&mut analyzer.scopes, &pre_rhs_scopes);
            analyzer.bump_context();
        }
        Expr::Paren(paren) => {
            visit_let_chain_condition(analyzer, &paren.expr, method_receiver_fallback);
        }
        Expr::Group(group) => {
            visit_let_chain_condition(analyzer, &group.expr, method_receiver_fallback);
        }
        _ => analyzer.visit_expr(expression),
    }
}

pub(super) fn assign_destructured_expr(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    expression: &Expr,
    info: &VariableInfo,
) -> bool {
    match expression {
        Expr::Path(_) => {
            let Some(name) = simple_assignment_name(expression) else {
                return false;
            };
            analyzer.assign(&name, info.clone());
            true
        }
        Expr::Tuple(tuple) => {
            let mut all_supported = true;
            for (index, element) in tuple.elems.iter().enumerate() {
                let item = info.tuple_items.get(index).unwrap_or(info);
                if !assign_destructured_expr(analyzer, element, item) {
                    all_supported = false;
                }
            }
            all_supported
        }
        Expr::Array(array) => {
            let mut all_supported = true;
            for (index, element) in array.elems.iter().enumerate() {
                if !assign_destructured_expr(
                    analyzer,
                    element,
                    info.tuple_items.get(index).unwrap_or(info),
                ) {
                    all_supported = false;
                }
            }
            all_supported
        }
        Expr::Struct(structure) => {
            let mut all_supported = true;
            for field in &structure.fields {
                let field_name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                let field_info = info.field_items.get(&field_name).unwrap_or(info);
                if !assign_destructured_expr(analyzer, &field.expr, field_info) {
                    all_supported = false;
                }
            }
            all_supported
        }
        Expr::Paren(paren) => assign_destructured_expr(analyzer, &paren.expr, info),
        Expr::Group(group) => assign_destructured_expr(analyzer, &group.expr, info),
        _ => false,
    }
}
