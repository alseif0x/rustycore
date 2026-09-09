//! Body-analysis methods, part 5 of 5.
//!
//! The inherent `BodyAnalyzer` impl is divided by the analysis phase its
//! methods serve under #634; every method keeps its original body.

use super::*;

impl<'a, 'b> BodyAnalyzer<'a, 'b> {
    pub(super) fn register_bound_args(
        &mut self,
        name: &str,
        trait_path: &str,
        bound: &syn::TraitBound,
    ) {
        let Some(segment) = bound.path.segments.last() else {
            return;
        };
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return;
        };
        let args: Vec<VariableInfo> = arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                // The argument's full info unions the package-wide named-type
                // registry, so substituting it later brings the concrete
                // type's flow, fields, and payload shapes with it.
                syn::GenericArgument::Type(inner) => {
                    Some(variable_info_in_type(inner, self.symbols))
                }
                _ => None,
            })
            .collect();
        if !args.is_empty() {
            self.generic_trait_bound_args
                .insert((name.to_owned(), trait_path.to_owned()), args);
        }
        let associated = arguments
            .args
            .iter()
            .filter_map(|argument| {
                let syn::GenericArgument::AssocType(binding) = argument else {
                    return None;
                };
                Some((
                    normalized_ident(&binding.ident),
                    variable_info_in_type(&binding.ty, self.symbols),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        if !associated.is_empty() {
            self.generic_trait_bound_associated
                .entry((name.to_owned(), trait_path.to_owned()))
                .or_default()
                .extend(associated);
        }
    }
    pub(super) fn register_generic_bounds(&mut self, generics: &syn::Generics) {
        self.bump_context();
        for parameter in &generics.params {
            let syn::GenericParam::Type(parameter) = parameter else {
                continue;
            };
            let name = normalized_ident(&parameter.ident);
            for bound in &parameter.bounds {
                if let syn::TypeParamBound::Trait(bound) = bound {
                    let trait_path = self
                        .canonical_local_path_names(path_names(&bound.path))
                        .join("::");
                    self.generic_trait_bounds
                        .entry(name.clone())
                        .or_default()
                        .insert(trait_path.clone());
                    self.register_bound_args(&name, &trait_path, bound);
                }
            }
        }
        if let Some(where_clause) = &generics.where_clause {
            for predicate in &where_clause.predicates {
                let syn::WherePredicate::Type(predicate) = predicate else {
                    continue;
                };
                let Type::Path(path) = &predicate.bounded_ty else {
                    continue;
                };
                let Some(name) = last_path_name(&path.path) else {
                    continue;
                };
                for bound in &predicate.bounds {
                    if let syn::TypeParamBound::Trait(bound) = bound {
                        let trait_path = self
                            .canonical_local_path_names(path_names(&bound.path))
                            .join("::");
                        self.generic_trait_bounds
                            .entry(name.clone())
                            .or_default()
                            .insert(trait_path.clone());
                        self.register_bound_args(&name, &trait_path, bound);
                    }
                }
            }
        }
    }
    /// Substitutes a trait's generic parameters with the type arguments the
    /// receiver's bound recorded (`M: Maker<Holder>` makes a `-> T` trait
    /// return surface `Holder`, including `where` predicates).
    pub(super) fn apply_bound_generic_args(
        &self,
        info: &VariableInfo,
        trait_bound: &str,
        receiver_types: &BTreeSet<String>,
    ) -> VariableInfo {
        let mut info = info.clone();
        let params = self
            .symbols
            .trait_generic_params
            .get(trait_bound)
            .cloned()
            .unwrap_or_default();
        for receiver_type in receiver_types {
            if let Some(args) = self
                .generic_trait_bound_args
                .get(&(receiver_type.clone(), trait_bound.to_owned()))
            {
                let map: BTreeMap<String, VariableInfo> =
                    params.iter().cloned().zip(args.iter().cloned()).collect();
                substitute_nominal_params(&mut info, &map);
            }
            if let Some(associated) = self
                .generic_trait_bound_associated
                .get(&(receiver_type.clone(), trait_bound.to_owned()))
            {
                substitute_nominal_params(&mut info, associated);
            }
        }
        info
    }
    /// Applies an explicit method/function turbofish (`make::<T>()`) to a
    /// recorded return. When the callee's generic parameter list is known,
    /// the arguments are substituted positionally into the return. When it is
    /// not recorded (external or unmodelled callee), a persistence-bearing
    /// turbofish argument is unioned into the result instead: the argument
    /// may select the return type, so dropping it would let the call bypass
    /// both ratchets.
    pub(super) fn apply_turbofish_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        turbofish: Option<&syn::AngleBracketedGenericArguments>,
    ) -> VariableInfo {
        let Some(turbofish) = turbofish else {
            return info.clone();
        };
        let args: Vec<VariableInfo> = turbofish
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(inner) => {
                    Some(variable_info_in_type(inner, self.symbols))
                }
                _ => None,
            })
            .collect();
        if args.is_empty() {
            return info.clone();
        }
        let mut info = info.clone();
        let substituted = params.is_some_and(|params| {
            let map: BTreeMap<String, VariableInfo> =
                params.iter().cloned().zip(args.iter().cloned()).collect();
            substitute_nominal_params(&mut info, &map)
        });
        if !substituted {
            for argument in &args {
                if !argument.flow.is_empty() || !argument.trait_bounds.is_empty() {
                    info.union(argument);
                }
            }
        }
        info
    }
    pub(super) fn apply_inferred_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        input_params: Option<&Vec<GenericInputSpec>>,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) -> VariableInfo {
        let (Some(params), Some(input_params)) = (params, input_params) else {
            return info.clone();
        };
        let mut substitutions = BTreeMap::<String, VariableInfo>::new();
        for (argument, formal_input) in args.iter().zip(input_params) {
            if formal_input.params.is_empty() {
                continue;
            }
            let argument_info = self.info_from_expr(argument);
            for param in &formal_input.params {
                substitutions
                    .entry(param.clone())
                    .or_default()
                    .union(&projected_generic_argument(
                        &argument_info,
                        formal_input,
                        param,
                    ));
            }
        }
        substitutions.retain(|param, _| params.contains(param));
        inferred_return_with_unresolved_fallback(info, params, &substitutions, || {
            args.iter()
                .fold(VariableInfo::default(), |mut result, argument| {
                    result.union(&self.info_from_expr(argument));
                    result
                })
        })
    }
    pub(super) fn apply_inferred_method_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        input_params: Option<&Vec<GenericInputSpec>>,
        args: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) -> VariableInfo {
        let input_params = input_params.map(|inputs| {
            if inputs
                .first()
                .is_some_and(|input| input.params.contains(RECEIVER_INPUT_MARKER))
            {
                &inputs[1..]
            } else {
                inputs.as_slice()
            }
        });
        let Some(params) = params else {
            return info.clone();
        };
        let Some(input_params) = input_params else {
            return info.clone();
        };
        let mut substitutions = BTreeMap::<String, VariableInfo>::new();
        for (argument, formal_input) in args.iter().zip(input_params) {
            let argument_info = self.info_from_expr(argument);
            for param in &formal_input.params {
                substitutions
                    .entry(param.clone())
                    .or_default()
                    .union(&projected_generic_argument(
                        &argument_info,
                        formal_input,
                        param,
                    ));
            }
        }
        substitutions.retain(|param, _| params.contains(param));
        inferred_return_with_unresolved_fallback(info, params, &substitutions, || {
            args.iter()
                .fold(VariableInfo::default(), |mut result, argument| {
                    result.union(&self.info_from_expr(argument));
                    result
                })
        })
    }
    pub(super) fn apply_optional_inferred_args(
        &self,
        info: &VariableInfo,
        params: Option<&Vec<String>>,
        input_params: Option<&Vec<GenericInputSpec>>,
        args: Option<&syn::punctuated::Punctuated<Expr, syn::token::Comma>>,
    ) -> VariableInfo {
        args.map(|args| self.apply_inferred_args(info, params, input_params, args))
            .unwrap_or_else(|| info.clone())
    }
    pub(super) fn preserve_opaque_argument_info(
        &self,
        mut result: VariableInfo,
        arguments: &syn::punctuated::Punctuated<Expr, syn::token::Comma>,
    ) -> VariableInfo {
        if result.flow.is_empty() && !result.trait_bounds.is_empty() {
            for argument in arguments {
                result.union(&self.info_from_expr(argument));
            }
        }
        result
    }
    pub(super) fn is_unresolved_opaque_call(&self, expression: &Expr) -> bool {
        let expression = match expression {
            Expr::Await(value) => value.base.as_ref(),
            Expr::Try(value) => value.expr.as_ref(),
            Expr::Paren(value) => value.expr.as_ref(),
            Expr::Group(value) => value.expr.as_ref(),
            _ => expression,
        };
        let Expr::Call(call) = expression else {
            return false;
        };
        let Expr::Path(path) = call.func.as_ref() else {
            return false;
        };
        let names = path_names(&path.path);
        let local = names
            .last()
            .and_then(|name| self.symbols.function_returns.get(name));
        let package = self
            .symbols
            .package_function_returns
            .get(&self.package_function_key(names));
        local.or(package).is_some_and(|info| {
            info.flow.is_empty() && !info.trait_bounds.is_empty() && call.args.is_empty()
        })
    }
    pub(super) fn closure_mutations_for_args(
        &self,
        info: &VariableInfo,
        arguments: &[VariableInfo],
    ) -> BTreeMap<String, VariableInfo> {
        if info.callable_signatures.is_empty() {
            return info.closure_mutations.clone();
        }
        let mut instantiated = BTreeMap::<String, VariableInfo>::new();
        for signature in &info.callable_signatures {
            let mut substitutions = BTreeMap::<String, VariableInfo>::new();
            for (argument, formal_input) in arguments.iter().zip(&signature.generic_inputs) {
                for param in &formal_input.params {
                    substitutions
                        .entry(param.clone())
                        .or_default()
                        .union(&projected_generic_argument(argument, formal_input, param));
                }
            }
            substitutions.retain(|param, _| signature.generic_params.contains(param));
            for (captured, mutation) in &info.closure_mutations {
                let mut mutation = mutation.clone();
                substitute_nominal_params(&mut mutation, &substitutions);
                instantiated
                    .entry(captured.clone())
                    .or_default()
                    .union(&mutation);
            }
        }
        instantiated
    }
}
