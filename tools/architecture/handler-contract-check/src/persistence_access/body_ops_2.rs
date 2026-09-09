//! Body-analysis methods, part 2 of 5.
//!
//! The inherent `BodyAnalyzer` impl is divided by the analysis phase its
//! methods serve under #634; every method keeps its original body.

use super::*;

impl<'a, 'b> BodyAnalyzer<'a, 'b> {
    pub(super) fn associated_return_info(
        &self,
        expression: &syn::ExprPath,
        call_args: Option<&syn::punctuated::Punctuated<Expr, syn::token::Comma>>,
    ) -> VariableInfo {
        let path = &expression.path;
        if expression.qself.is_none() && path.segments.len() < 2 {
            return VariableInfo::default();
        }
        let method_name = normalized_ident(&path.segments.last().expect("path has a method").ident);
        // An explicit turbofish on the callee (`Factory::make::<T>()`)
        // selects generic returns; it must be substituted into the recorded
        // return below instead of being dropped.
        let turbofish = path
            .segments
            .last()
            .and_then(|segment| match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                _ => None,
            });
        let trait_name = expression.qself.as_ref().and_then(|qself| {
            (qself.position > 0).then(|| {
                self.canonical_local_path_names(
                    path.segments
                        .iter()
                        .take(qself.position)
                        .map(|segment| normalized_ident(&segment.ident))
                        .collect(),
                )
                .join("::")
            })
        });
        let explicit_owner_key = expression.qself.is_none().then(|| {
            let names = path_names(path);
            self.package_function_key(names[..names.len().saturating_sub(1)].to_vec())
        });
        let receiver_types = if let Some(qself) = &expression.qself {
            receiver_nominal_types_in_type(&qself.ty)
                .into_iter()
                .flat_map(|owner| {
                    self.symbols
                        .nominal_type_aliases
                        .get(&owner)
                        .cloned()
                        .unwrap_or_else(|| BTreeSet::from([owner]))
                })
                .collect()
        } else {
            // The owner is the type the path resolves to, not the name it is
            // written with: `use sqlx::mysql::MySqlPoolOptions as Opt` still
            // builds a MySQL pool.
            let written = path_names(path);
            let owner = self
                .canonical_names(written[..written.len().saturating_sub(1)].to_vec())
                .last()
                .cloned()
                .unwrap_or_default();
            if owner == "Self" {
                self.lookup("Self")
                    .map(|info| info.nominal_types.clone())
                    .unwrap_or_default()
            } else {
                let mut owners = self
                    .symbols
                    .nominal_type_aliases
                    .get(&owner)
                    .cloned()
                    .unwrap_or_else(|| BTreeSet::from([owner]));
                if let Some(owner_key) = &explicit_owner_key {
                    owners.insert(owner_key.clone());
                }
                owners
            }
        };
        let mut result = VariableInfo::default();
        for receiver_type in &receiver_types {
            let key = (
                receiver_type.clone(),
                trait_name.clone(),
                method_name.clone(),
            );
            if let Some(info) = self.symbols.method_returns.get(&key) {
                let params = self.symbols.method_generic_params.get(&key);
                let info = self.apply_turbofish_args(info, params, turbofish);
                let info = self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.method_generic_input_params.get(&key),
                    call_args,
                );
                result.union(&info);
            }
        }
        let mut bounds = BTreeSet::new();
        if let Some(trait_name) = trait_name {
            bounds.insert(trait_name);
        }
        for receiver_type in &receiver_types {
            if let Some(generic_bounds) = self.generic_trait_bounds.get(receiver_type) {
                bounds.extend(generic_bounds.iter().cloned());
            }
        }
        let expanded = self.expand_trait_bounds(bounds);
        for trait_bound in &expanded {
            if let Some(info) = self
                .symbols
                .trait_method_returns
                .get(&(trait_bound.clone(), method_name.clone()))
            {
                let info = self.apply_bound_generic_args(info, trait_bound, &receiver_types);
                let key = (trait_bound.clone(), method_name.clone());
                let params = self.symbols.trait_method_generic_params.get(&key);
                let info = self.apply_turbofish_args(&info, params, turbofish);
                let info = self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.trait_method_generic_input_params.get(&key),
                    call_args,
                );
                result.union(&info);
            }
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // Inherent impl declared in another module
            // (`crate::dto::Factory::make()`): the owner path is the callee
            // path minus its method segment, resolved package-wide. No early
            // return: `-> Self` returns still need the substitute below, and
            // a persistence-typed owner path contributes its flow like the
            // ordinary call fallback does.
            if let Some(owner_key) = &explicit_owner_key
                && let Some(info) = self
                    .symbols
                    .package_method_returns
                    .get(&(owner_key.clone(), method_name.clone()))
            {
                let key = (owner_key.clone(), method_name.clone());
                let params = self.symbols.package_method_generic_params.get(&key);
                let mut info = self.apply_turbofish_args(info, params, turbofish);
                info = self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.package_method_generic_input_params.get(&key),
                    call_args,
                );
                info.flow
                    .union(Flow::pools(&targets_for_path(path, self.symbols)));
                result.union(&info);
            }
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // Qualified free-function calls (`crate::factory::database()`)
            // resolve through the package-wide canonical-path registry, the
            // same way trait returns already do.
            let key = self.package_function_key(path_names(path));
            if let Some(info) = self.symbols.package_function_returns.get(&key) {
                let params = self.symbols.package_function_generic_params.get(&key);
                let info = self.apply_turbofish_args(info, params, turbofish);
                return self.apply_optional_inferred_args(
                    &info,
                    params,
                    self.symbols.package_function_generic_input_params.get(&key),
                    call_args,
                );
            }
        }
        if result.substitute_self(&receiver_types, &expanded) {
            // A dependency method may declare `-> Self`. Substitution must
            // recover that exact owner's registered fields (for example
            // `wow_database::DbUpdater::new(...).pool`) without broadly
            // expanding ordinary short return names such as `Holder`, which
            // can legitimately occur in many modules and dependencies.
            for receiver_type in &receiver_types {
                let dependency_owner = receiver_type.split("::").next().is_some_and(|root| {
                    self.symbols
                        .dependency_crate_aliases
                        .values()
                        .any(|provider_root| provider_root == root)
                });
                if dependency_owner
                    && let Some(named) = self
                        .symbols
                        .workspace_named_type_info
                        .get(receiver_type)
                        .cloned()
                {
                    result.union(&named);
                }
            }
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // Unrecorded associated callee: a persistence-bearing turbofish
            // can still select the return type, so keep the argument visible.
            result = self.apply_turbofish_args(&result, None, turbofish);
        }
        result
    }
    pub(super) fn info_from_expr(&self, expression: &Expr) -> VariableInfo {
        match expression {
            Expr::Reference(reference) => {
                let mut info = self.info_from_expr(&reference.expr);
                if reference.mutability.is_some()
                    && let Some(name) = simple_assignment_name(&reference.expr)
                {
                    info.mutable_pointees.insert(name);
                } else if reference.mutability.is_some()
                    && let Some((root, projections)) = assignment_place(&reference.expr)
                {
                    info.mutable_places
                        .insert(MutablePlace { root, projections });
                }
                return info;
            }
            Expr::Paren(paren) => return self.info_from_expr(&paren.expr),
            Expr::Group(group) => return self.info_from_expr(&group.expr),
            Expr::Try(try_expression) => return self.info_from_expr(&try_expression.expr),
            Expr::Await(await_expression) => return self.info_from_expr(&await_expression.base),
            Expr::Block(block) => {
                return self
                    .block_result_infos
                    .borrow()
                    .get(&(&block.block as *const syn::Block as usize))
                    .cloned()
                    .unwrap_or_else(|| implicit_tail_info(&block.block, self));
            }
            // A closure value carries what its body would produce when later
            // called through the binding (e.g. `let factory = || database;`).
            Expr::Closure(closure) => {
                let mut info = self
                    .closure_result_infos
                    .borrow()
                    .get(&(closure as *const ExprClosure as usize))
                    .cloned()
                    .unwrap_or_else(|| self.info_from_expr(&closure.body));
                info.callable_signatures
                    .insert(closure_callable_signature(closure));
                info.closure_mutations = self
                    .closure_effects
                    .borrow()
                    .get(&(closure as *const ExprClosure as usize))
                    .cloned()
                    .unwrap_or_default();
                return info;
            }
            _ => {}
        }
        if let Expr::Path(path) = expression
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && let Some(name) = last_path_name(&path.path)
        {
            if let Some(info) = self.lookup(&name) {
                if let Some(syn::PathArguments::AngleBracketed(turbofish)) =
                    path.path.segments.last().map(|segment| &segment.arguments)
                    && !info.callable_signatures.is_empty()
                {
                    let mut result = VariableInfo::default();
                    for signature in &info.callable_signatures {
                        result.union(&self.apply_turbofish_args(
                            info,
                            Some(&signature.generic_params),
                            Some(turbofish),
                        ));
                    }
                    return result;
                }
                return info.clone();
            }
            if let Some(info) = self.symbols.item_values.get(&name) {
                return info.clone();
            }
        }
        if let Expr::Path(path) = expression {
            let names = path_names(&path.path);
            if path_is_sqlx(&names, self.symbols)
                && names.last().is_some_and(|name| is_query_name(name))
            {
                return VariableInfo {
                    flow: Flow::query(),
                    query_callable: true,
                    ..VariableInfo::default()
                };
            }
            // `let run = sqlx::Executor::execute;` names an executor; the call
            // through `run` sends whatever SQL it is handed.
            // The trait may be imported inside this block, so the path has to
            // be resolved before it can be recognized.
            let canonical = self.canonical_names(names.clone());
            if (path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical, self.symbols))
                && canonical
                    .iter()
                    .nth_back(1)
                    .is_some_and(|owner| owner == "Executor")
                && let Some(method) = canonical.last()
                && PersistenceOperation::from_executor_method(method).is_some()
            {
                return VariableInfo {
                    executor_callable: Some(method.clone()),
                    ..VariableInfo::default()
                };
            }
        }
        if let Expr::Path(path) = expression {
            let key = self.package_function_key(path_names(&path.path));
            if let Some(info) = self.symbols.package_item_values.get(&key) {
                return info.clone();
            }
            // Function items carry their declared return information when
            // stored in a local (`let factory = crate::make; factory()`).
            // Resolve the same local/package/associated registries used by a
            // direct call before the lexical binding hides the declaration.
            if path.path.segments.len() == 1
                && let Some(name) = last_path_name(&path.path)
                && let Some(info) = self.symbols.function_returns.get(&name)
            {
                let params = self.symbols.function_generic_params.get(&name);
                let turbofish = path.path.segments.last().and_then(|segment| {
                    if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        Some(arguments)
                    } else {
                        None
                    }
                });
                let mut info = self.apply_turbofish_args(info, params, turbofish);
                if let Some(params) = params {
                    info.callable_signatures.insert(CallableSignature {
                        generic_params: params.clone(),
                        generic_inputs: self
                            .symbols
                            .function_generic_input_params
                            .get(&name)
                            .cloned()
                            .unwrap_or_default(),
                    });
                }
                return info;
            }
            if let Some(info) = self.symbols.package_function_returns.get(&key) {
                let params = self.symbols.package_function_generic_params.get(&key);
                let turbofish = path.path.segments.last().and_then(|segment| {
                    if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        Some(arguments)
                    } else {
                        None
                    }
                });
                let mut info = self.apply_turbofish_args(info, params, turbofish);
                if let Some(params) = params {
                    info.callable_signatures.insert(CallableSignature {
                        generic_params: params.clone(),
                        generic_inputs: self
                            .symbols
                            .package_function_generic_input_params
                            .get(&key)
                            .cloned()
                            .unwrap_or_default(),
                    });
                }
                return info;
            }
            let associated = self.associated_return_info(path, None);
            if !associated.flow.is_empty()
                || !associated.nominal_types.is_empty()
                || !associated.payload_variants.is_empty()
                || !associated.tuple_items.is_empty()
                || !associated.field_items.is_empty()
                || !associated.trait_bounds.is_empty()
            {
                return associated;
            }
        }
        if let Expr::Call(call) = expression
            && let Expr::Path(path) = call.func.as_ref()
        {
            if let Some(info) = self
                .replacement_result_infos
                .borrow()
                .get(&(call as *const ExprCall as usize))
                .cloned()
            {
                return info;
            }
            if is_standard_identity(&self.canonical_local_path_names(path_names(&path.path)))
                && let Some(argument) = call.args.first()
            {
                // `std::convert::identity` is shape-preserving as well as a
                // flow passthrough. Returning the argument's complete value
                // information keeps tuple/field projections sound.
                return self.info_from_expr(argument);
            }
            if path.path.segments.len() == 1
                && let Some(name) = last_path_name(&path.path)
            {
                let turbofish =
                    path.path
                        .segments
                        .last()
                        .and_then(|segment| match &segment.arguments {
                            syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                            _ => None,
                        });
                // Resolve single-segment callees through the current lexical
                // scope before falling back to module-level `function_returns`:
                // a local closure or function-valued binding may carry the
                // persistence return flow.
                if let Some(info) = self.lookup(&name) {
                    if info.callable_signatures.is_empty() {
                        let mut result = info.clone();
                        result.closure_mutations.clear();
                        for argument in &call.args {
                            result.union(&self.info_from_expr(argument));
                        }
                        return result;
                    }
                    let mut result = VariableInfo::default();
                    let mut return_info = info.clone();
                    return_info.callable_signatures.clear();
                    return_info.closure_mutations.clear();
                    for signature in &info.callable_signatures {
                        let explicit = self.apply_turbofish_args(
                            &return_info,
                            Some(&signature.generic_params),
                            turbofish,
                        );
                        result.union(&self.apply_inferred_args(
                            &explicit,
                            Some(&signature.generic_params),
                            Some(&signature.generic_inputs),
                            &call.args,
                        ));
                    }
                    return result;
                }
                if let Some(info) = self.symbols.function_returns.get(&name) {
                    let params = self.symbols.function_generic_params.get(&name);
                    let result = self.apply_turbofish_args(info, params, turbofish);
                    return self.preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.function_generic_input_params.get(&name),
                            &call.args,
                        ),
                        &call.args,
                    );
                }
                // Free functions declared in another source module resolve
                // through the package-wide canonical-path registry.
                let key = self.package_function_key(vec![name]);
                if let Some(info) = self.symbols.package_function_returns.get(&key) {
                    let params = self.symbols.package_function_generic_params.get(&key);
                    let result = self.apply_turbofish_args(info, params, turbofish);
                    return self.preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.package_function_generic_input_params.get(&key),
                            &call.args,
                        ),
                        &call.args,
                    );
                }
            }
            let return_info = self.preserve_opaque_argument_info(
                self.associated_return_info(path, Some(&call.args)),
                &call.args,
            );
            if !return_info.flow.is_empty()
                || !return_info.nominal_types.is_empty()
                || !return_info.payload_variants.is_empty()
                || !return_info.tuple_items.is_empty()
                || !return_info.trait_bounds.is_empty()
            {
                return return_info;
            }
            if let Some(variant) = last_path_name(&path.path)
                && matches!(variant.as_str(), "Some" | "Ok" | "Err")
            {
                let arguments = call
                    .args
                    .iter()
                    .map(|argument| self.nominal_shape_from_expr(argument))
                    .collect::<Vec<_>>();
                let mut info = VariableInfo {
                    flow: self.flow_of_expr(expression),
                    sql_expression: self.sql_expression_kind(expression),
                    nominal_types: BTreeSet::from([variant]),
                    ..VariableInfo::default()
                };
                if arguments.iter().any(Option::is_some) {
                    info.payload_variants.insert(
                        arguments
                            .into_iter()
                            .map(|shape| {
                                shape.unwrap_or_else(|| NominalShape {
                                    nominal_types: BTreeSet::new(),
                                    arguments: Vec::new(),
                                })
                            })
                            .collect(),
                    );
                }
                return info;
            }
        }
        if let Expr::Call(call) = expression
            && !matches!(call.func.as_ref(), Expr::Path(_))
        {
            let mut callable = self.info_from_expr(&call.func);
            for argument in &call.args {
                callable.union(&self.info_from_expr(argument));
            }
            if !callable.flow.is_empty()
                || !callable.nominal_types.is_empty()
                || !callable.payload_variants.is_empty()
                || !callable.tuple_items.is_empty()
                || !callable.field_items.is_empty()
                || !callable.trait_bounds.is_empty()
            {
                return callable;
            }
        }
        if let Expr::MethodCall(method) = expression {
            let return_info = self.method_return_info(method);
            return VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                sql_sources: self.sql_sources(expression),
                nominal_types: return_info.nominal_types,
                payload_variants: return_info.payload_variants,
                tuple_items: return_info.tuple_items,
                field_items: return_info.field_items,
                trait_bounds: return_info.trait_bounds,
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
                mutable_pointees: BTreeSet::new(),
                mutable_places: BTreeSet::new(),
                query_callable: false,
                executor_callable: None,
            };
        }
        if let Expr::Field(field) = expression {
            let field_name = match &field.member {
                Member::Named(ident) => normalized_ident(ident),
                Member::Unnamed(index) => index.index.to_string(),
            };
            let base_info = self.info_from_expr(&field.base);
            if let Some(info) = base_info.field_items.get(&field_name) {
                return info.clone();
            }
        }
        if let Expr::Struct(structure) = expression {
            let key = self.package_function_key(path_names(&structure.path));
            let mut info = VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                sql_sources: self.sql_sources(expression),
                nominal_types: BTreeSet::from([key.clone()]),
                ..VariableInfo::default()
            };
            if let Some(named) = self.symbols.named_type_info.get(&key) {
                info.union(named);
            }
            if let Some(named) = self.symbols.workspace_named_type_info.get(&key) {
                info.union(named);
            }
            // Explicit initializers are authoritative for this value. In
            // particular, a clean boolean field must not inherit aggregate
            // flow from a persistence-bearing sibling.
            for field in &structure.fields {
                let name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                let mut field_info = self.info_from_expr(&field.expr);
                // Scalar-reading methods intentionally discard aggregate
                // SqlResult flow in ordinary expressions. A struct field is
                // a durable projection boundary, however, so retain the
                // initializer subtree on that exact field without tainting
                // its siblings.
                field_info.flow.union(self.produced_value_flow(&field.expr));
                info.field_items.insert(name, field_info);
            }
            if let Some(rest) = &structure.rest {
                let rest = self.info_from_expr(rest);
                for (name, field) in rest.field_items {
                    info.field_items.entry(name).or_insert(field);
                }
                info.flow.union(rest.flow);
            }
            return info;
        }
        if let Expr::Call(call) = expression
            && let Expr::Path(path) = call.func.as_ref()
            && let Some(owner) = last_path_name(&path.path)
        {
            let is_constructor_method = path.qself.is_none()
                && path.path.segments.len() >= 2
                && matches!(owner.as_str(), "default" | "new");
            let nominal_types = if is_constructor_method {
                path.path
                    .segments
                    .iter()
                    .nth_back(1)
                    .map(|segment| BTreeSet::from([normalized_ident(&segment.ident)]))
                    .unwrap_or_default()
            } else {
                BTreeSet::from([owner])
            };
            let mut info = VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                sql_sources: self.sql_sources(expression),
                nominal_types,
                payload_variants: payload_variants_in_path(&path.path, self.symbols),
                tuple_items: Vec::new(),
                field_items: BTreeMap::new(),
                trait_bounds: BTreeSet::new(),
                type_generic_params: Vec::new(),
                callable_signatures: BTreeSet::new(),
                closure_mutations: BTreeMap::new(),
                mutable_pointees: BTreeSet::new(),
                mutable_places: BTreeSet::new(),
                query_callable: false,
                executor_callable: None,
            };
            let mut names = path_names(&path.path);
            if is_constructor_method {
                names.pop();
            }
            let key = self.package_function_key(names);
            if let Some(named) = self.symbols.named_type_info.get(&key) {
                info.union(named);
            }
            if let Some(named) = self.symbols.workspace_named_type_info.get(&key) {
                info.union(named);
            }
            for argument in &call.args {
                info.union(&self.info_from_expr(argument));
            }
            return info;
        }
        if let Expr::Tuple(tuple) = expression {
            return VariableInfo {
                flow: self.flow_of_expr(expression),
                sql_expression: self.sql_expression_kind(expression),
                tuple_items: tuple
                    .elems
                    .iter()
                    .map(|element| self.info_from_expr(element))
                    .collect(),
                ..VariableInfo::default()
            };
        }
        VariableInfo {
            flow: self.flow_of_expr(expression),
            sql_expression: self.sql_expression_kind(expression),
            sql_sources: self.sql_sources(expression),
            nominal_types: BTreeSet::new(),
            payload_variants: BTreeSet::new(),
            tuple_items: Vec::new(),
            field_items: BTreeMap::new(),
            trait_bounds: BTreeSet::new(),
            type_generic_params: Vec::new(),
            callable_signatures: BTreeSet::new(),
            closure_mutations: BTreeMap::new(),
            mutable_pointees: BTreeSet::new(),
            mutable_places: BTreeSet::new(),
            query_callable: false,
            executor_callable: None,
        }
    }
    pub(super) fn nominal_shape_from_expr(&self, expression: &Expr) -> Option<NominalShape> {
        if let Expr::Tuple(tuple) = expression {
            return Some(NominalShape {
                nominal_types: BTreeSet::new(),
                arguments: tuple
                    .elems
                    .iter()
                    .map(|element| {
                        self.nominal_shape_from_expr(element)
                            .unwrap_or_else(|| NominalShape {
                                nominal_types: BTreeSet::new(),
                                arguments: Vec::new(),
                            })
                    })
                    .collect(),
            });
        }
        let info = self.info_from_expr(expression);
        (!info.nominal_types.is_empty()).then(|| NominalShape {
            nominal_types: info.nominal_types,
            arguments: info.payload_variants.into_iter().next().unwrap_or_default(),
        })
    }
    pub(super) fn nominal_types_of_expr(&self, expression: &Expr) -> BTreeSet<String> {
        match expression {
            Expr::Path(path) if path.qself.is_none() => last_path_name(&path.path)
                .map(|name| {
                    self.lookup(&name)
                        .map(|info| info.nominal_types.clone())
                        .or_else(|| self.symbols.nominal_type_aliases.get(&name).cloned())
                        .unwrap_or_else(|| {
                            let known_owner = self
                                .symbols
                                .method_returns
                                .keys()
                                .any(|(owner, _, _)| owner == &name)
                                || self
                                    .symbols
                                    .tuple_field_targets
                                    .keys()
                                    .any(|(owner, _)| owner == &name);
                            known_owner
                                .then(|| BTreeSet::from([name]))
                                .unwrap_or_default()
                        })
                })
                .unwrap_or_default(),
            Expr::Reference(reference) => self.nominal_types_of_expr(&reference.expr),
            Expr::Paren(paren) => self.nominal_types_of_expr(&paren.expr),
            Expr::Group(group) => self.nominal_types_of_expr(&group.expr),
            Expr::Try(try_expression) => self.nominal_types_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => self.nominal_types_of_expr(&await_expression.base),
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                self.nominal_types_of_expr(&unary.expr)
            }
            Expr::Field(field) => {
                let info = self.info_from_expr(expression);
                if !info.nominal_types.is_empty() {
                    return info.nominal_types;
                }
                let field_name = match &field.member {
                    Member::Named(ident) => normalized_ident(ident),
                    Member::Unnamed(index) => index.index.to_string(),
                };
                let mut nominal_types = BTreeSet::new();
                for owner in self.nominal_types_of_expr(&field.base) {
                    if let Some(field_types) = self
                        .symbols
                        .field_nominal_types
                        .get(&(owner, field_name.clone()))
                    {
                        nominal_types.extend(field_types.iter().cloned());
                    }
                }
                nominal_types
            }
            Expr::Call(call) => {
                let info = self.info_from_expr(expression);
                if !info.nominal_types.is_empty() {
                    return info.nominal_types;
                }
                match call.func.as_ref() {
                    Expr::Path(path) if path.path.segments.len() == 1 => last_path_name(&path.path)
                        .map(|name| {
                            self.symbols
                                .function_returns
                                .get(&name)
                                .map(|info| info.nominal_types.clone())
                                .unwrap_or_else(|| BTreeSet::from([name]))
                        })
                        .unwrap_or_default(),
                    Expr::Path(path) => {
                        self.associated_return_info(path, Some(&call.args))
                            .nominal_types
                    }
                    _ => BTreeSet::new(),
                }
            }
            Expr::MethodCall(method) => self.method_return_info(method).nominal_types,
            Expr::Struct(expression) => last_path_name(&expression.path)
                .map(|name| {
                    self.symbols
                        .nominal_type_aliases
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| BTreeSet::from([name]))
                })
                .unwrap_or_default(),
            _ => BTreeSet::new(),
        }
    }
    pub(super) fn flow_in_block(&self, block: &syn::Block) -> Flow {
        let mut collector = DirectChildFlowCollector {
            analyzer: self,
            flow: Flow::default(),
            at_root: false,
        };
        collector.visit_block(block);
        collector.flow
    }
    pub(super) fn declared_field_info(
        &self,
        owner: &str,
        field: &str,
        named: bool,
    ) -> VariableInfo {
        let owners = self
            .symbols
            .nominal_type_aliases
            .get(owner)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([owner.to_owned()]));
        let mut info = VariableInfo::default();
        for owner in owners {
            let targets = if named {
                self.symbols
                    .field_targets
                    .get(&(owner.clone(), field.to_owned()))
            } else {
                self.symbols
                    .tuple_field_targets
                    .get(&(owner.clone(), field.to_owned()))
            };
            if let Some(targets) = targets {
                info.flow.union(Flow::pools(targets));
            }
            if let Some(types) = self
                .symbols
                .field_nominal_types
                .get(&(owner, field.to_owned()))
            {
                info.nominal_types.extend(types.iter().cloned());
            }
        }
        // Enum variants and structs declared in another source file register
        // their fields in the package-wide named-type registry under
        // `crate::dto::Product::Database`; a match pattern may name them by
        // the full path or by an imported short owner, so accept the exact
        // owner key or any canonical key with that owner suffix.
        info.union(&self.named_field_info(owner, field));
        info
    }
    pub(super) fn named_field_info(&self, owner: &str, field: &str) -> VariableInfo {
        let mut info = VariableInfo::default();
        let suffix = format!("::{owner}");
        for (key, named_info) in self.symbols.named_type_info.iter() {
            if (key == &owner || key.ends_with(&suffix))
                && let Some(field_info) = named_info.field_items.get(field)
            {
                info.union(field_info);
            }
        }
        let mut workspace_keys = BTreeSet::from([owner.to_owned()]);
        workspace_keys.insert(
            canonical_path_names(owner.split("::").map(str::to_owned).collect(), self.symbols)
                .join("::"),
        );
        for key in workspace_keys {
            if let Some(field_info) = self
                .symbols
                .workspace_named_type_info
                .get(&key)
                .and_then(|named_info| named_info.field_items.get(field))
            {
                info.union(field_info);
            }
        }
        info
    }
    pub(super) fn named_field_is_declared(&self, owner: &str, field: &str) -> bool {
        let suffix = format!("::{owner}");
        if self.symbols.named_type_info.iter().any(|(key, info)| {
            (key == owner || key.ends_with(&suffix)) && info.field_items.contains_key(field)
        }) {
            return true;
        }
        let canonical =
            canonical_path_names(owner.split("::").map(str::to_owned).collect(), self.symbols)
                .join("::");
        [owner, canonical.as_str()].into_iter().any(|key| {
            self.symbols
                .workspace_named_type_info
                .get(key)
                .is_some_and(|info| info.field_items.contains_key(field))
        })
    }
}
