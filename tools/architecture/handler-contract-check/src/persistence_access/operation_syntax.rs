//! Per-item persistence-operation and parameter-use scans.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[derive(Default)]
pub(super) struct PersistenceOperationSyntax<'a> {
    pub(super) symbols: BTreeSet<String>,
    pub(super) generic_types: BTreeSet<String>,
    /// Each counted site by syntax-node address, so the generic-function
    /// check can drop exactly the sites the body analyzer typed (#1437).
    pub(super) sites: BTreeMap<usize, String>,
    /// Crate roots of the inventoried workspace packages this module depends
    /// on. When set, a call path that names a free function through one of
    /// them is not counted: see `names_inventoried_free_function` (#1437).
    pub(super) workspace_roots: Option<&'a BTreeMap<String, String>>,
}

impl<'ast> Visit<'ast> for PersistenceOperationSyntax<'_> {
    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        let name = normalized_ident(&method.method);
        if !matches!(name.as_str(), "new" | "open")
            && PersistenceOperation::from_executor_method(&name).is_some()
        {
            self.sites
                .insert(std::ptr::from_ref(method) as usize, name.clone());
            self.symbols.insert(name);
        }
        syn::visit::visit_expr_method_call(self, method);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref()
            && path.path.segments.len() >= 2
            && let Some(name) = last_path_name(&path.path)
            && name != "new"
            && PersistenceOperation::from_executor_method(&name).is_some()
            && !self.workspace_roots.is_some_and(|roots| {
                names_inventoried_free_function(path, roots, &self.generic_types)
            })
        {
            let owner = path
                .path
                .segments
                .iter()
                .nth_back(1)
                .map(|segment| normalized_ident(&segment.ident));
            if name != "open" || owner.is_some_and(|owner| self.generic_types.contains(&owner)) {
                self.sites
                    .insert(std::ptr::from_ref(call) as usize, name.clone());
                self.symbols.insert(name);
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// Whether `path` names a module-level free function of an inventoried
/// workspace package, such as `wow_world_spell::player_cast::prepare` (#1437).
///
/// The path must be rooted at `crate`, `self`, `super` or the crate root of
/// an inventoried dependency, and every segment before the function must be
/// a plain module segment: snake_case, without generic arguments, not a
/// primitive type and not one of the function's own type parameters. Such a
/// path cannot name a type, a trait or an associated item, so the call has
/// no generic receiver. The callee's body is inventoried at its own
/// definition, where a generic callee is held to this same check; the
/// qualified spelling therefore audits exactly like its already-accepted
/// imported single-segment form.
pub(super) fn names_inventoried_free_function(
    path: &syn::ExprPath,
    workspace_roots: &BTreeMap<String, String>,
    generic_types: &BTreeSet<String>,
) -> bool {
    const PRIMITIVE_TYPES: &[&str] = &[
        "bool", "char", "str", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8",
        "u16", "u32", "u64", "u128", "usize",
    ];
    let segments = &path.path.segments;
    if path.qself.is_some() || path.path.leading_colon.is_some() || segments.len() < 2 {
        return false;
    }
    let root = normalized_ident(&segments[0].ident);
    if !matches!(root.as_str(), "crate" | "self" | "super") && !workspace_roots.contains_key(&root)
    {
        return false;
    }
    segments.iter().take(segments.len() - 1).all(|segment| {
        let name = normalized_ident(&segment.ident);
        matches!(segment.arguments, syn::PathArguments::None)
            && name
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
            && !PRIMITIVE_TYPES.contains(&name.as_str())
            && !generic_types.contains(&name)
    })
}

pub(super) fn persistence_operations_in_syntax(item: &Item) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax::default();
    visitor.visit_item(item);
    visitor.symbols
}

/// Persistence-shaped sites in a generic function body, by syntax-node
/// address. Free-function calls through `workspace_roots` are not sites.
pub(super) fn generic_persistence_operation_sites(
    block: &syn::Block,
    generic_types: BTreeSet<String>,
    workspace_roots: &BTreeMap<String, String>,
) -> BTreeMap<usize, String> {
    let mut visitor = PersistenceOperationSyntax {
        generic_types,
        workspace_roots: Some(workspace_roots),
        ..PersistenceOperationSyntax::default()
    };
    visitor.visit_block(block);
    visitor.sites
}

#[derive(Default)]
pub(super) struct CalledParameterInputs {
    pub(super) parameters: BTreeMap<String, usize>,
    pub(super) called: BTreeSet<usize>,
}

impl<'ast> Visit<'ast> for CalledParameterInputs {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref()
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && let Some(name) = last_path_name(&path.path)
            && let Some(index) = self.parameters.get(&name)
        {
            self.called.insert(*index);
        }
        syn::visit::visit_expr_call(self, call);
    }
}

pub(super) fn called_parameter_inputs(function: &ItemFn) -> BTreeSet<usize> {
    let parameters = function
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Pat::Ident(ident) = typed.pat.as_ref() else {
                return None;
            };
            Some((normalized_ident(&ident.ident), index))
        })
        .collect();
    let mut visitor = CalledParameterInputs {
        parameters,
        ..CalledParameterInputs::default()
    };
    visitor.visit_block(&function.block);
    visitor.called
}

#[derive(Default)]
pub(super) struct MutableParameterWrites {
    pub(super) parameters: BTreeMap<String, usize>,
    pub(super) written: BTreeSet<usize>,
    pub(super) receiver_fields: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for MutableParameterWrites {
    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        if let Some((root, projections)) = assignment_place(&assignment.left)
            && root == "self"
            && let Some(PlaceProjection::Field(field)) = projections.first()
        {
            self.receiver_fields.insert(field.clone());
        }
        if let Some((root, _)) = assignment_place(&assignment.left)
            && let Some(index) = self.parameters.get(&root)
        {
            self.written.insert(*index);
        }
        if let Expr::Unary(unary) = assignment.left.as_ref()
            && matches!(unary.op, syn::UnOp::Deref(_))
            && let Some(name) = simple_assignment_name(&unary.expr)
            && let Some(index) = self.parameters.get(&name)
        {
            self.written.insert(*index);
        }
        syn::visit::visit_expr_assign(self, assignment);
    }
}

pub(super) fn mutable_method_writes(
    method: &syn::ImplItemFn,
) -> (BTreeSet<String>, BTreeSet<usize>) {
    let receiver_is_mutable = method.sig.inputs.first().is_some_and(
        |input| matches!(input, FnArg::Receiver(receiver) if receiver.mutability.is_some()),
    );
    let parameters = method
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Type::Reference(reference) = typed.ty.as_ref() else {
                return None;
            };
            if reference.mutability.is_none() {
                return None;
            }
            let Pat::Ident(ident) = typed.pat.as_ref() else {
                return None;
            };
            Some((normalized_ident(&ident.ident), index.saturating_sub(1)))
        })
        .collect();
    let mut visitor = MutableParameterWrites {
        parameters,
        ..MutableParameterWrites::default()
    };
    visitor.visit_block(&method.block);
    let receiver_fields = receiver_is_mutable
        .then_some(visitor.receiver_fields)
        .unwrap_or_default();
    (receiver_fields, visitor.written)
}

pub(super) fn mutable_parameter_writes(function: &ItemFn) -> BTreeSet<usize> {
    let parameters = function
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let FnArg::Typed(typed) = input else {
                return None;
            };
            let Type::Reference(reference) = typed.ty.as_ref() else {
                return None;
            };
            if reference.mutability.is_none() {
                return None;
            }
            let Pat::Ident(ident) = typed.pat.as_ref() else {
                return None;
            };
            Some((normalized_ident(&ident.ident), index))
        })
        .collect();
    let mut visitor = MutableParameterWrites {
        parameters,
        ..MutableParameterWrites::default()
    };
    visitor.visit_block(&function.block);
    visitor.written
}

pub(super) fn item_cfg(parent: &[String], attributes: &[Attribute]) -> Vec<String> {
    extend_cfg_context(parent, attributes)
}

pub(super) fn source_class_allows(
    source_class: PersistenceSourceClass,
    parent: &[String],
    attributes: &[Attribute],
    errors: &mut Vec<String>,
    owner: &str,
) -> bool {
    let production = cfg_context_allows_production(parent, attributes);
    let test = cfg_context_allows_test(parent, attributes);
    match (production, test) {
        (Ok(production), Ok(test)) => match source_class {
            PersistenceSourceClass::Production => production,
            PersistenceSourceClass::TestFixture => test,
        },
        (production, test) => {
            if let Err(error) = production {
                errors.push(format!("invalid cfg (production) on {owner}: {error}"));
            }
            if let Err(error) = test {
                errors.push(format!("invalid cfg (test) on {owner}: {error}"));
            }
            false
        }
    }
}
