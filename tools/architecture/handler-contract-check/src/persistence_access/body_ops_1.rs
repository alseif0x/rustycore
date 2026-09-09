//! Body-analysis methods, part 1 of 5.
//!
//! The inherent `BodyAnalyzer` impl is divided by the analysis phase its
//! methods serve under #634; every method keeps its original body.

use super::*;

impl<'a, 'b> BodyAnalyzer<'a, 'b> {
    pub(super) fn new(
        context: RecordContext<'a>,
        accumulator: &'b mut AccessAccumulator,
        errors: &'b mut Vec<String>,
        symbols: &'b ModuleSymbols,
        enclosing: String,
        visibility: String,
        cfg: Vec<String>,
    ) -> Self {
        Self {
            context,
            accumulator,
            errors,
            symbols,
            enclosing,
            visibility,
            cfg,
            scopes: vec![BTreeMap::new()],
            local_path_alias_scopes: vec![BTreeMap::new()],
            anonymous_trait_scopes: vec![BTreeSet::new()],
            active_trait: None,
            generic_trait_bounds: BTreeMap::new(),
            generic_trait_bound_args: BTreeMap::new(),
            generic_trait_bound_associated: BTreeMap::new(),
            flow_cache: std::cell::RefCell::new(BTreeMap::new()),
            subtree_flow_cache: std::cell::RefCell::new(BTreeMap::new()),
            closure_effects: std::cell::RefCell::new(BTreeMap::new()),
            closure_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            block_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            replacement_result_infos: std::cell::RefCell::new(BTreeMap::new()),
            loop_flow_collectors: Vec::new(),
            block_exit_collectors: Vec::new(),
            return_exit_collectors: Vec::new(),
            return_value_collectors: Vec::new(),
            context_version: 0,
            suppress_records: false,
            visible_macro_shadows: BTreeSet::new(),
        }
    }
    pub(super) fn analyze_without_records(&mut self, analyze: impl FnOnce(&mut Self)) {
        let previous = self.suppress_records;
        let error_count = self.errors.len();
        self.suppress_records = true;
        analyze(self);
        self.suppress_records = previous;
        self.errors.truncate(error_count);
    }
    pub(super) fn bump_context(&mut self) {
        self.context_version = self.context_version.wrapping_add(1);
        self.flow_cache.get_mut().clear();
        self.subtree_flow_cache.get_mut().clear();
    }
    pub(super) fn visit_loop_block(
        &mut self,
        block: &syn::Block,
        label: Option<String>,
    ) -> LoopFlowCollector {
        self.loop_flow_collectors.push(LoopFlowCollector {
            label,
            ..LoopFlowCollector::default()
        });
        self.visit_block(block);
        self.loop_flow_collectors
            .pop()
            .expect("loop flow collector was installed")
    }
    pub(super) fn visit_for_loop_body(
        &mut self,
        expression: &syn::ExprForLoop,
        iterator_info: &VariableInfo,
        label: Option<String>,
    ) -> LoopFlowCollector {
        self.loop_flow_collectors.push(LoopFlowCollector {
            label,
            ..LoopFlowCollector::default()
        });
        self.push_scope();
        self.register_local_uses(&expression.body.stmts);
        self.register_local_callables(&expression.body.stmts);
        self.register_local_constants(&expression.body.stmts);
        self.bind_pattern(&expression.pat, iterator_info);
        for statement in &expression.body.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
        self.loop_flow_collectors
            .pop()
            .expect("for-loop flow collector was installed")
    }
    pub(super) fn visit_while_loop_body(
        &mut self,
        expression: &syn::ExprWhile,
        label: Option<String>,
    ) -> LoopFlowCollector {
        self.loop_flow_collectors.push(LoopFlowCollector {
            label,
            ..LoopFlowCollector::default()
        });
        self.push_scope();
        self.register_local_uses(&expression.body.stmts);
        self.register_local_callables(&expression.body.stmts);
        self.register_local_constants(&expression.body.stmts);
        visit_let_chain_condition(self, &expression.cond, true);
        // Evaluating a false condition exits the loop with every mutation
        // performed by that condition, before the body can overwrite it.
        self.capture_loop_control(None, true);
        for statement in &expression.body.stmts {
            self.visit_stmt(statement);
        }
        self.pop_scope();
        self.loop_flow_collectors
            .pop()
            .expect("while-loop flow collector was installed")
    }
    pub(super) fn capture_loop_control(&mut self, label: Option<&syn::Lifetime>, is_exit: bool) {
        let explicit_label = label.map(|label| normalized_ident(&label.ident));
        let target = explicit_label
            .as_ref()
            .and_then(|label| {
                self.loop_flow_collectors
                    .iter()
                    .rposition(|collector| collector.label.as_deref() == Some(label.as_str()))
            })
            .or_else(|| {
                explicit_label
                    .is_none()
                    .then(|| self.loop_flow_collectors.len().checked_sub(1))
                    .flatten()
            });
        if target.is_none()
            && is_exit
            && let Some(label) = explicit_label
            && let Some(collector) = self
                .block_exit_collectors
                .iter_mut()
                .rev()
                .find(|collector| collector.label == label)
        {
            let scopes = self.scopes.clone();
            match &mut collector.exits {
                None => collector.exits = Some(scopes),
                Some(accumulated) => merge_scope_stacks(accumulated, &scopes),
            }
            return;
        }
        let Some(target) = target else {
            return;
        };
        let scopes = self.scopes.clone();
        let collector = &mut self.loop_flow_collectors[target];
        let destination = if is_exit {
            &mut collector.exits
        } else {
            &mut collector.back_edges
        };
        match destination {
            None => *destination = Some(scopes),
            Some(accumulated) => merge_scope_stacks(accumulated, &scopes),
        }
    }
    pub(super) fn capture_return_exit(&mut self) {
        if let Some(exits) = self.return_exit_collectors.last_mut() {
            let scopes = self.scopes.clone();
            match exits {
                None => *exits = Some(scopes),
                Some(accumulated) => merge_scope_stacks(accumulated, &scopes),
            }
        }
    }
    pub(super) fn canonical_local_path_names(&self, names: Vec<String>) -> Vec<String> {
        if let Some(first) = names.first().cloned()
            && let Some(source) = self
                .local_path_alias_scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(&first))
        {
            let mut expanded = source.clone();
            expanded.extend(names.into_iter().skip(1));
            return expanded;
        }
        canonical_path_names(names, self.symbols)
    }
    /// Canonical lookup key into the package-wide free-function registry.
    /// Local block imports expand to their absolute `crate::…` source while
    /// module-level aliases already drop the leading `crate`, so normalize
    /// both forms to the registry's crate-relative shape.
    pub(super) fn package_function_key(&self, names: Vec<String>) -> String {
        let mut names = self.canonical_local_path_names(names);
        if names.first().is_some_and(|first| first == "crate") {
            names.remove(0);
        }
        names.join("::")
    }
    /// Register the block's imports, then let them resolve through each other.
    ///
    /// Rust does not order imports, so `use mount as alias;` may precede
    /// `use std::include as mount;`. Resolving once would leave `alias`
    /// pointing at a name that is itself an alias, and a guard reading the
    /// resolved path would miss what the invocation really is.
    pub(super) fn register_local_uses(&mut self, statements: &[Stmt]) {
        self.register_local_use_items(statements);
        // Chain the block's own aliases through each other. Only local entries
        // are substituted: rewriting them with module resolution would change
        // the written form that other lookups depend on.
        for _ in 0..8 {
            let Some(scope) = self.local_path_alias_scopes.last().cloned() else {
                break;
            };
            let expanded = scope
                .iter()
                .map(|(alias, source)| {
                    let mut names = source.clone();
                    if let Some(first) = names.first().cloned()
                        && first != *alias
                        && let Some(target) = scope.get(&first)
                        && target.first() != Some(&first)
                    {
                        names.splice(0..1, target.clone());
                    }
                    (alias.clone(), names)
                })
                .collect::<BTreeMap<_, _>>();
            if expanded == scope {
                break;
            }
            if let Some(scope) = self.local_path_alias_scopes.last_mut() {
                *scope = expanded;
            }
        }
    }
    pub(super) fn register_local_use_items(&mut self, statements: &[Stmt]) {
        self.bump_context();
        let uses = statements.iter().filter_map(|statement| match statement {
            Stmt::Item(Item::Use(item_use)) => Some(item_use),
            _ => None,
        });
        for item_use in uses {
            if !source_class_allows(
                self.context.source_class,
                &self.cfg,
                &item_use.attrs,
                self.errors,
                "local use declaration",
            ) {
                continue;
            }
            let (leaves, _) = use_leaves(item_use);
            for leaf in leaves {
                let source = self.canonical_local_path_names(leaf.source);
                if leaf.local == "_" {
                    self.anonymous_trait_scopes
                        .last_mut()
                        .expect("body analyzer has a lexical scope")
                        .insert(source.join("::"));
                } else {
                    self.local_path_alias_scopes
                        .last_mut()
                        .expect("body analyzer has a lexical scope")
                        .insert(leaf.local, source);
                }
            }
        }
    }
    /// Bind a block's `const`/`static` items before its statements run.
    ///
    /// They are in scope throughout the block, so a query written above the
    /// declaration still reads the value; installing it only when the visitor
    /// reaches the declaration would record that query against an opaque path
    /// and let the literal change without moving a row.
    pub(super) fn register_local_constants(&mut self, statements: &[Stmt]) {
        let items = statements
            .iter()
            .filter_map(|statement| match statement {
                Stmt::Item(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (index, item) in items.iter().enumerate() {
            if !self.allows_source_class(item_attributes(item), "block-local item") {
                continue;
            }
            let shadows = items[..index]
                .iter()
                .filter_map(|item| match item {
                    Item::Macro(item_macro) => item_macro.ident.as_ref().map(normalized_ident),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let declared = match item {
                Item::Const(item_const) => Some((
                    normalized_ident(&item_const.ident),
                    &item_const.expr,
                    &item_const.ty,
                )),
                Item::Static(item_static) => Some((
                    normalized_ident(&item_static.ident),
                    &item_static.expr,
                    &item_static.ty,
                )),
                _ => None,
            };
            let Some((name, expression, declared_type)) = declared else {
                continue;
            };
            let (kind, sources) = source_sql_info(expression, &|path| {
                standard_string_macro_of(
                    path,
                    &|path| self.canonical_names(path),
                    &self.symbols.module_path,
                    &shadows,
                )
            });
            if kind == SqlExpressionKind::Nonliteral {
                continue;
            }
            let mut info = self.info_from_type(declared_type);
            info.sql_expression = kind;
            info.sql_sources = sources;
            self.bind(name, info);
        }
    }
    pub(super) fn register_local_callables(&mut self, statements: &[Stmt]) {
        for function in statements.iter().filter_map(|statement| match statement {
            Stmt::Item(Item::Fn(function)) => Some(function),
            _ => None,
        }) {
            if !source_class_allows(
                self.context.source_class,
                &self.cfg,
                &function.attrs,
                self.errors,
                "block-local function",
            ) {
                continue;
            }
            let ReturnType::Type(_, ty) = &function.sig.output else {
                continue;
            };
            let mut info = self.info_from_type(ty);
            info.sql_expression = SqlExpressionKind::Nonliteral;
            let generic_params = generic_type_param_names(&function.sig.generics);
            if !generic_params.is_empty() {
                info.callable_signatures.insert(CallableSignature {
                    generic_inputs: generic_params_by_input(&function.sig.inputs, &generic_params),
                    generic_params,
                });
            }
            if !info.flow.is_empty()
                || !info.nominal_types.is_empty()
                || !info.payload_variants.is_empty()
                || !info.tuple_items.is_empty()
                || !info.field_items.is_empty()
                || !info.trait_bounds.is_empty()
                || !info.callable_signatures.is_empty()
            {
                self.bind(normalized_ident(&function.sig.ident), info);
            }
        }
    }
    pub(super) fn push_scope(&mut self) {
        self.bump_context();
        self.scopes.push(BTreeMap::new());
        self.local_path_alias_scopes.push(BTreeMap::new());
        self.anonymous_trait_scopes.push(BTreeSet::new());
    }
    pub(super) fn pop_scope(&mut self) {
        self.bump_context();
        self.scopes.pop();
        self.local_path_alias_scopes.pop();
        self.anonymous_trait_scopes.pop();
    }
    pub(super) fn trait_is_in_scope(&self, trait_name: &str) -> bool {
        if self.symbols.anonymous_traits_in_scope.contains(trait_name)
            || self
                .anonymous_trait_scopes
                .iter()
                .any(|scope| scope.contains(trait_name))
        {
            return true;
        }
        let mut shadowed = BTreeSet::new();
        for scope in self.local_path_alias_scopes.iter().rev() {
            for (local, path) in scope {
                if shadowed.insert(local) && path.join("::") == trait_name {
                    return true;
                }
            }
        }
        self.symbols
            .traits_in_scope
            .iter()
            .any(|(local, path)| !shadowed.contains(local) && path == trait_name)
    }
    pub(super) fn add(
        &mut self,
        target: PersistenceTarget,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        if self.suppress_records {
            return;
        }
        self.accumulator.add(
            &self.context,
            NewAccess {
                enclosing: &self.enclosing,
                target,
                operation,
                symbol,
                visibility: &self.visibility,
                cfg,
                fingerprint,
                generated_input: false,
            },
        );
    }
    pub(super) fn add_generated(
        &mut self,
        target: PersistenceTarget,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        if self.suppress_records {
            return;
        }
        self.accumulator.add(
            &self.context,
            NewAccess {
                enclosing: &self.enclosing,
                target,
                operation,
                symbol,
                visibility: &self.visibility,
                cfg,
                fingerprint,
                generated_input: true,
            },
        );
    }
    pub(super) fn allows_source_class(&mut self, attributes: &[Attribute], owner: &str) -> bool {
        let allowed = source_class_allows(
            self.context.source_class,
            &self.cfg,
            attributes,
            self.errors,
            owner,
        );
        if allowed && !self.suppress_records {
            let cfg = item_cfg(&self.cfg, attributes);
            add_attribute_records(
                self.accumulator,
                &self.context,
                self.symbols,
                attributes,
                AttributeRecordContext {
                    enclosing: &self.enclosing,
                    visibility: &self.visibility,
                    cfg: &cfg,
                },
            );
        }
        allowed
    }
    pub(super) fn lookup(&self, name: &str) -> Option<&VariableInfo> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    pub(super) fn bind(&mut self, name: String, info: VariableInfo) {
        self.bump_context();
        self.scopes
            .last_mut()
            .expect("body analyzer always has a scope")
            .insert(name, info);
    }
    pub(super) fn assign(&mut self, name: &str, info: VariableInfo) {
        self.bump_context();
        if let Some(scope) = self
            .scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(name))
        {
            scope.insert(name.to_owned(), info);
        } else {
            self.bind(name.to_owned(), info);
        }
    }
    pub(super) fn info_from_type(&self, ty: &Type) -> VariableInfo {
        let mut info = variable_info_in_type(ty, self.symbols);
        for name in info.nominal_types.clone() {
            if let Some(bounds) = self.generic_trait_bounds.get(&name) {
                info.trait_bounds.extend(bounds.iter().cloned());
            }
        }
        info
    }
    pub(super) fn method_return_info(&self, method: &ExprMethodCall) -> VariableInfo {
        let method_name = normalized_ident(&method.method);
        let mut result = VariableInfo::default();
        let receiver_types = self.nominal_types_of_expr(&method.receiver);
        let mut trait_bounds = BTreeSet::new();
        for receiver_type in &receiver_types {
            if let Some(bounds) = self.generic_trait_bounds.get(receiver_type) {
                trait_bounds.extend(bounds.iter().cloned());
            }
        }
        // The receiver binding itself may carry trait bounds (e.g. `self` in
        // a default trait body is bound to its own trait), not only generic
        // parameters of the enclosing function.
        trait_bounds.extend(self.shallow_trait_bounds_of_expr(&method.receiver));
        if receiver_types.is_empty() {
            trait_bounds.extend(self.shallow_trait_bounds_of_expr(&method.receiver));
        }
        for receiver_type in &receiver_types {
            if let Some(info) =
                self.symbols
                    .method_returns
                    .get(&(receiver_type.clone(), None, method_name.clone()))
            {
                let key = (receiver_type.clone(), None, method_name.clone());
                let params = self.symbols.method_generic_params.get(&key);
                let info = self.apply_turbofish_args(info, params, method.turbofish.as_ref());
                let info = self.apply_inferred_method_args(
                    &info,
                    params,
                    self.symbols.method_generic_input_params.get(&key),
                    &method.args,
                );
                result.union(&info);
                continue;
            }
            let mut trait_impl_hit = false;
            for ((owner, trait_name, candidate), info) in &self.symbols.method_returns {
                if owner == receiver_type
                    && trait_name
                        .as_ref()
                        .is_some_and(|trait_name| self.trait_is_in_scope(trait_name))
                    && candidate == &method_name
                {
                    let key = (owner.clone(), trait_name.clone(), candidate.clone());
                    let params = self.symbols.method_generic_params.get(&key);
                    let info = self.apply_turbofish_args(info, params, method.turbofish.as_ref());
                    let info = self.apply_inferred_method_args(
                        &info,
                        params,
                        self.symbols.method_generic_input_params.get(&key),
                        &method.args,
                    );
                    result.union(&info);
                    trait_impl_hit = true;
                }
            }
            if !trait_impl_hit {
                // Inherent impl declared in another source module: resolve
                // through the package-wide registry keyed by canonical owner
                // path, the same way free functions already resolve.
                let owner_key = if receiver_type.contains("::") {
                    receiver_type
                        .strip_prefix("crate::")
                        .unwrap_or(receiver_type)
                        .to_owned()
                } else {
                    self.package_function_key(vec![receiver_type.clone()])
                };
                if let Some(info) = self
                    .symbols
                    .package_method_returns
                    .get(&(owner_key.clone(), method_name.clone()))
                {
                    let key = (owner_key.clone(), method_name.clone());
                    let params = self.symbols.package_method_generic_params.get(&key);
                    let info = self.apply_turbofish_args(info, params, method.turbofish.as_ref());
                    let info = self.apply_inferred_method_args(
                        &info,
                        params,
                        self.symbols.package_method_generic_input_params.get(&key),
                        &method.args,
                    );
                    result.union(&info);
                }
            }
        }
        let expanded_trait_bounds = self.expand_trait_bounds(trait_bounds);
        for trait_bound in &expanded_trait_bounds {
            if let Some(info) = self
                .symbols
                .trait_method_returns
                .get(&(trait_bound.clone(), method_name.clone()))
            {
                let info = self.apply_bound_generic_args(info, trait_bound, &receiver_types);
                let key = (trait_bound.clone(), method_name.clone());
                let params = self.symbols.trait_method_generic_params.get(&key);
                let info = self.apply_turbofish_args(&info, params, method.turbofish.as_ref());
                let info = self.apply_inferred_method_args(
                    &info,
                    params,
                    self.symbols.trait_method_generic_input_params.get(&key),
                    &method.args,
                );
                result.union(&info);
            }
        }
        result.substitute_self(&receiver_types, &expanded_trait_bounds);
        if method_name == "recv" {
            result
                .payload_variants
                .extend(self.info_from_expr(&method.receiver).payload_variants);
        }
        if result.flow.is_empty()
            && result.nominal_types.is_empty()
            && result.payload_variants.is_empty()
            && result.tuple_items.is_empty()
            && result.trait_bounds.is_empty()
        {
            // The callee is not recorded anywhere (external or unmodelled).
            // A persistence-bearing turbofish can still select the return
            // type (`factory.make::<CharacterDatabase>()`), so keep the
            // argument visible instead of dropping the call's result.
            result = self.apply_turbofish_args(&result, None, method.turbofish.as_ref());
        }
        result
    }
    pub(super) fn method_mutation_effects(
        &self,
        method: &ExprMethodCall,
    ) -> (VariableInfo, BTreeMap<usize, VariableInfo>) {
        let method_name = normalized_ident(&method.method);
        let receiver_types = self.nominal_types_of_expr(&method.receiver);
        let mut receiver_effect = VariableInfo::default();
        let mut parameter_effects = BTreeMap::<usize, VariableInfo>::new();
        for receiver_type in receiver_types {
            for ((owner, trait_name, candidate), info) in &self.symbols.method_mutable_receivers {
                if owner == &receiver_type
                    && candidate == &method_name
                    && trait_name
                        .as_ref()
                        .is_none_or(|trait_name| self.trait_is_in_scope(trait_name))
                {
                    receiver_effect.union(info);
                }
            }
            for ((owner, trait_name, candidate), effects) in &self.symbols.method_mutable_writes {
                if owner == &receiver_type
                    && candidate == &method_name
                    && trait_name
                        .as_ref()
                        .is_none_or(|trait_name| self.trait_is_in_scope(trait_name))
                {
                    for (index, info) in effects {
                        parameter_effects.entry(*index).or_default().union(info);
                    }
                }
            }
            let owner_key = if receiver_type.contains("::") {
                receiver_type
                    .strip_prefix("crate::")
                    .unwrap_or(&receiver_type)
                    .to_owned()
            } else {
                self.package_function_key(vec![receiver_type])
            };
            if let Some(info) = self
                .symbols
                .package_method_mutable_receivers
                .get(&(owner_key.clone(), method_name.clone()))
            {
                receiver_effect.union(info);
            }
            if let Some(effects) = self
                .symbols
                .package_method_mutable_writes
                .get(&(owner_key, method_name.clone()))
            {
                for (index, info) in effects {
                    parameter_effects.entry(*index).or_default().union(info);
                }
            }
        }
        (receiver_effect, parameter_effects)
    }
    pub(super) fn expand_trait_bounds(&self, trait_bounds: BTreeSet<String>) -> BTreeSet<String> {
        let mut pending = trait_bounds.into_iter().collect::<Vec<_>>();
        let mut expanded_trait_bounds = BTreeSet::new();
        while let Some(trait_bound) = pending.pop() {
            if !expanded_trait_bounds.insert(trait_bound.clone()) {
                continue;
            }
            if let Some(supertraits) = self.symbols.trait_supertraits.get(&trait_bound) {
                pending.extend(supertraits.iter().cloned());
            }
        }
        expanded_trait_bounds
    }
    pub(super) fn shallow_trait_bounds_of_expr(&self, expression: &Expr) -> BTreeSet<String> {
        match expression {
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                last_path_name(&path.path)
                    .and_then(|name| self.lookup(&name))
                    .map(|info| info.trait_bounds.clone())
                    .unwrap_or_default()
            }
            Expr::Reference(reference) => self.shallow_trait_bounds_of_expr(&reference.expr),
            Expr::Paren(paren) => self.shallow_trait_bounds_of_expr(&paren.expr),
            Expr::Group(group) => self.shallow_trait_bounds_of_expr(&group.expr),
            Expr::Try(try_expression) => self.shallow_trait_bounds_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => {
                self.shallow_trait_bounds_of_expr(&await_expression.base)
            }
            Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
                self.shallow_trait_bounds_of_expr(&unary.expr)
            }
            Expr::Call(call) => match call.func.as_ref() {
                Expr::Path(path) if path.path.segments.len() == 1 => last_path_name(&path.path)
                    .and_then(|name| {
                        self.lookup(&name)
                            .or_else(|| self.symbols.function_returns.get(&name))
                    })
                    .map(|info| info.trait_bounds.clone())
                    .unwrap_or_default(),
                Expr::Path(path) => {
                    self.associated_return_info(path, Some(&call.args))
                        .trait_bounds
                }
                _ => BTreeSet::new(),
            },
            Expr::MethodCall(method) => self.method_return_info(method).trait_bounds,
            _ => BTreeSet::new(),
        }
    }
}
