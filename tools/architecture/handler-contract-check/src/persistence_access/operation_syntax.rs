//! Per-item persistence-operation and parameter-use scans.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

#[derive(Default)]
pub(super) struct PersistenceOperationSyntax {
    pub(super) symbols: BTreeSet<String>,
    pub(super) generic_types: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PersistenceOperationSyntax {
    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        let name = normalized_ident(&method.method);
        if !matches!(name.as_str(), "new" | "open")
            && PersistenceOperation::from_executor_method(&name).is_some()
        {
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
        {
            let owner = path
                .path
                .segments
                .iter()
                .nth_back(1)
                .map(|segment| normalized_ident(&segment.ident));
            if name != "open" || owner.is_some_and(|owner| self.generic_types.contains(&owner)) {
                self.symbols.insert(name);
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

pub(super) fn persistence_operations_in_syntax(item: &Item) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax::default();
    visitor.visit_item(item);
    visitor.symbols
}

pub(super) fn persistence_operations_in_block(
    block: &syn::Block,
    generic_types: BTreeSet<String>,
) -> BTreeSet<String> {
    let mut visitor = PersistenceOperationSyntax {
        generic_types,
        ..PersistenceOperationSyntax::default()
    };
    visitor.visit_block(block);
    visitor.symbols
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
