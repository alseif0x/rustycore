//! Body-analysis methods, part 4 of 5.
//!
//! The inherent `BodyAnalyzer` impl is divided by the analysis phase its
//! methods serve under #634; every method keeps its original body.

use super::*;

impl<'a, 'b> BodyAnalyzer<'a, 'b> {
    pub(super) fn flow_of_call(&self, call: &ExprCall) -> Flow {
        let Expr::Path(path) = call.func.as_ref() else {
            let mut flow = self.info_from_expr(&call.func).flow;
            for argument in &call.args {
                flow.union(self.flow_of_expr(argument));
            }
            return flow;
        };
        let names = path_names(&path.path);
        // The result of an aliased constructor carries the same flow as the
        // constructor it names, or a chained `execute` loses its query stage.
        let canonical = self.canonical_names(names.clone());
        let last = canonical.last().map(String::as_str).unwrap_or_default();
        if is_standard_replacement(&self.canonical_local_path_names(names.clone()))
            && let Some(binding) = call.args.first().and_then(mutable_storage_receiver_name)
        {
            if let Some(info) = self
                .replacement_result_infos
                .borrow()
                .get(&(call as *const ExprCall as usize))
            {
                return info.flow.clone();
            }
            return self
                .lookup(&binding)
                .map(|info| info.flow.clone())
                .unwrap_or_default();
        }
        let rooted_sqlx =
            path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical, self.symbols);
        if rooted_sqlx
            && last == "new"
            && let Some(target) = sqlx_pool_options_target(&canonical)
        {
            return Flow::pools(&BTreeSet::from([target]));
        }
        if (rooted_sqlx && is_query_name(last))
            || (names.len() == 1 && self.symbols.query_callables.contains(last))
            || (names.len() == 1 && self.lookup(last).is_some_and(|info| info.query_callable))
        {
            return Flow::query();
        }
        if rooted_sqlx
            && last == "new"
            && canonical
                .iter()
                .nth_back(1)
                .is_some_and(|owner| owner == "QueryBuilder")
        {
            return Flow::query();
        }
        // UFCS `begin` opens a transaction on its receiver; falling through to
        // the generic path targets would return a pool stage and the later
        // `commit` would no longer be recognized.
        if rooted_sqlx
            && last == "begin"
            && let Some(receiver) = call.args.first()
        {
            let flow = self.flow_of_expr(receiver);
            let opened = flow.map_pool_stage(FlowStage::Transaction);
            if !opened.is_empty() {
                return opened;
            }
            // A connection carries no pool stage, yet `begin` opens a
            // transaction on it just the same.
            let opened_on_targets = flow
                .targets()
                .iter()
                .map(|target| (*target, FlowStage::Transaction))
                .collect::<BTreeSet<_>>();
            if !opened_on_targets.is_empty() {
                return Flow(opened_on_targets);
            }
            return Flow(BTreeSet::from([(
                PersistenceTarget::Sqlx,
                FlowStage::Transaction,
            )]));
        }
        if is_flow_passthrough_call(&names)
            || is_standard_identity(&self.canonical_local_path_names(names.clone()))
        {
            let mut flow = Flow::default();
            for argument in &call.args {
                flow.union(self.flow_of_expr(argument));
            }
            return flow;
        }
        // `use sqlx::mysql::MySqlPoolOptions as Opt` must not erase which
        // provider `Opt::new()` builds, or the pool the chain opens loses its
        // concrete identity.
        let path_targets = targets_for_names(&canonical, self.symbols);
        if !path_targets.is_empty() {
            return Flow::pools(&path_targets);
        }
        let associated = self.associated_return_info(path, Some(&call.args)).flow;
        if !associated.is_empty() {
            return associated;
        }
        if names.len() == 1 {
            let turbofish =
                path.path
                    .segments
                    .last()
                    .and_then(|segment| match &segment.arguments {
                        syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                        _ => None,
                    });
            if let Some(info) = self.lookup(last) {
                if !info.callable_signatures.is_empty() {
                    let mut result = VariableInfo::default();
                    let mut return_info = info.clone();
                    return_info.callable_signatures.clear();
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
                    return result.flow;
                }
                let mut flow = info.flow.clone();
                for argument in &call.args {
                    flow.union(self.flow_of_expr(argument));
                }
                return flow;
            }
            if let Some(info) = self.symbols.function_returns.get(last) {
                let params = self.symbols.function_generic_params.get(last);
                let result = self.apply_turbofish_args(info, params, turbofish);
                return self
                    .preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.function_generic_input_params.get(last),
                            &call.args,
                        ),
                        &call.args,
                    )
                    .flow;
            }
            // Free functions declared in another source module resolve
            // through the package-wide canonical-path registry.
            let key = self.package_function_key(names.clone());
            if let Some(info) = self.symbols.package_function_returns.get(&key) {
                let params = self.symbols.package_function_generic_params.get(&key);
                let result = self.apply_turbofish_args(info, params, turbofish);
                return self
                    .preserve_opaque_argument_info(
                        self.apply_inferred_args(
                            &result,
                            params,
                            self.symbols.package_function_generic_input_params.get(&key),
                            &call.args,
                        ),
                        &call.args,
                    )
                    .flow;
            }
        }
        self.symbols
            .function_returns
            .get(last)
            .map(|info| info.flow.clone())
            .unwrap_or_default()
    }
    pub(super) fn flow_of_method(&self, method: &ExprMethodCall) -> Flow {
        let receiver = self.flow_of_expr(&method.receiver);
        let name = normalized_ident(&method.method);
        if let Some(target) = database_getter_target(&name) {
            return Flow::pools(&BTreeSet::from([target]));
        }
        match name.as_str() {
            "begin" => receiver.map_pool_stage(FlowStage::Transaction),
            // `QueryBuilder::build*` hands back the query it has assembled, so
            // what is chained onto it still executes SQL.
            "build" | "build_query_as" | "build_query_scalar"
                if receiver.targets().contains(&PersistenceTarget::Sqlx) =>
            {
                Flow::query()
            }
            "acquire" | "pool" => receiver.map_pool_stage(FlowStage::DerivedPool),
            "prepare" | "describe" => receiver.map_pool_stage(FlowStage::Query),
            "query" | "direct_query" | "delay_query_holder_like_cpp" => {
                receiver.map_pool_stage(FlowStage::Query)
            }
            "execute" | "direct_execute" => receiver.map_pool_stage(FlowStage::Query),
            "bind" if receiver.has_stage(FlowStage::Query) => receiver,
            // A modifier returns the query it was called on, so the executor
            // chained after it is still executing that query.
            "persistent" | "fetch_last_insert_id" | "try_map" | "map"
                if receiver.has_stage(FlowStage::Query) =>
            {
                receiver
            }
            name if FLOW_TRANSFORMING_METHODS.contains(&name) => {
                let mut flow = receiver;
                for argument in &method.args {
                    // The closure/fallback argument may produce the result
                    // value; union its produced flow, including closure
                    // bodies, instead of treating the combinator as a
                    // receiver-only passthrough.
                    flow.union(self.subtree_flow(argument));
                }
                flow
            }
            name if FLOW_PASSTHROUGH_METHODS.contains(&name) => receiver,
            _ => {
                // An unmodelled (external) method can transform or wrap a
                // capability-carrying receiver without dropping the value,
                // e.g. `Some(database).iter().next()` later unwrapped into
                // `.pool()`. Keep only capability flow: query-stage flow and
                // data containers (row results, fields, holders) reading
                // scalars are ordinary data access already inventoried at the
                // query/execute call sites, and unioning them would drown the
                // ratchet in value_alias noise.
                let mut flow = Flow(
                    receiver
                        .0
                        .iter()
                        .filter(|(target, stage)| {
                            !matches!(stage, FlowStage::Query)
                                && !matches!(
                                    target,
                                    PersistenceTarget::SqlResult
                                        | PersistenceTarget::SqlFields
                                        | PersistenceTarget::SqlQueryHolder
                                        | PersistenceTarget::SqlQueryHolderResult
                                )
                        })
                        .copied()
                        .collect(),
                );
                flow.union(self.method_return_info(method).flow);
                flow
            }
        }
    }
    /// Flow carried by the value an expression produces, excluding unrelated
    /// side effects evaluated before that value. This is narrower than
    /// `subtree_flow` and is used at named-field projection boundaries.
    pub(super) fn produced_value_flow(&self, expression: &Expr) -> Flow {
        let direct = self.flow_of_expr(expression);
        if !direct.is_empty() {
            return direct;
        }
        match expression {
            Expr::Reference(reference) => self.produced_value_flow(&reference.expr),
            Expr::Paren(paren) => self.produced_value_flow(&paren.expr),
            Expr::Group(group) => self.produced_value_flow(&group.expr),
            Expr::Try(try_expression) => self.produced_value_flow(&try_expression.expr),
            Expr::Await(await_expression) => self.produced_value_flow(&await_expression.base),
            Expr::Cast(cast) => self.produced_value_flow(&cast.expr),
            Expr::Unary(unary) => self.produced_value_flow(&unary.expr),
            Expr::MethodCall(method) => {
                let name = normalized_ident(&method.method);
                if matches!(name.as_str(), "try_read" | "read" | "read_string")
                    || FLOW_PASSTHROUGH_METHODS.contains(&name.as_str())
                    || FLOW_TRANSFORMING_METHODS.contains(&name.as_str())
                {
                    let mut flow = self.produced_value_flow(&method.receiver);
                    if FLOW_TRANSFORMING_METHODS.contains(&name.as_str()) {
                        for argument in &method.args {
                            flow.union(self.produced_value_flow(argument));
                        }
                    }
                    flow
                } else {
                    Flow::default()
                }
            }
            Expr::Block(block) => block
                .block
                .stmts
                .last()
                .and_then(|statement| match statement {
                    Stmt::Expr(tail, None) => Some(self.produced_value_flow(tail)),
                    _ => None,
                })
                .unwrap_or_default(),
            Expr::If(if_expression) => {
                let mut flow = if_expression
                    .then_branch
                    .stmts
                    .last()
                    .and_then(|statement| match statement {
                        Stmt::Expr(tail, None) => Some(self.produced_value_flow(tail)),
                        _ => None,
                    })
                    .unwrap_or_default();
                if let Some((_, alternative)) = &if_expression.else_branch {
                    flow.union(self.produced_value_flow(alternative));
                }
                flow
            }
            Expr::Match(match_expression) => {
                let mut flow = Flow::default();
                for arm in &match_expression.arms {
                    flow.union(self.produced_value_flow(&arm.body));
                }
                flow
            }
            Expr::Tuple(tuple) => {
                let mut flow = Flow::default();
                for element in &tuple.elems {
                    flow.union(self.produced_value_flow(element));
                }
                flow
            }
            _ => Flow::default(),
        }
    }
    pub(super) fn flow_of_expr(&self, expression: &Expr) -> Flow {
        let key = (expression as *const Expr as usize, self.context_version);
        if let Some(flow) = self.flow_cache.borrow().get(&key) {
            return flow.clone();
        }
        let flow = self.compute_flow_of_expr(expression);
        self.flow_cache.borrow_mut().insert(key, flow.clone());
        flow
    }
    pub(super) fn subtree_flow(&self, expression: &Expr) -> Flow {
        let key = (expression as *const Expr as usize, self.context_version);
        if let Some(flow) = self.subtree_flow_cache.borrow().get(&key) {
            return flow.clone();
        }
        let mut flow = self.flow_of_expr(expression);
        if let Expr::Field(field) = expression {
            let mut root = field.base.as_ref();
            while let Expr::Field(parent) = root {
                root = parent.base.as_ref();
            }
            if !matches!(root, Expr::Path(_)) {
                flow.union(self.subtree_flow(root));
            }
            self.subtree_flow_cache
                .borrow_mut()
                .insert(key, flow.clone());
            return flow;
        }
        let mut collector = DirectChildFlowCollector {
            analyzer: self,
            flow: Flow::default(),
            at_root: true,
        };
        collector.visit_expr(expression);
        flow.union(collector.flow);
        self.subtree_flow_cache
            .borrow_mut()
            .insert(key, flow.clone());
        flow
    }
    pub(super) fn compute_flow_of_expr(&self, expression: &Expr) -> Flow {
        match expression {
            Expr::Path(path) => self.flow_of_path(path),
            Expr::Field(field) => self.field_flow(field),
            Expr::Reference(reference) => self.flow_of_expr(&reference.expr),
            Expr::Paren(paren) => self.flow_of_expr(&paren.expr),
            Expr::Group(group) => self.flow_of_expr(&group.expr),
            Expr::Try(try_expression) => self.flow_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => self.flow_of_expr(&await_expression.base),
            Expr::Cast(cast) => self.flow_of_expr(&cast.expr),
            Expr::Unary(unary) => self.flow_of_expr(&unary.expr),
            Expr::Call(call) => self.flow_of_call(call),
            Expr::MethodCall(method) => self.flow_of_method(method),
            Expr::If(if_expression) => {
                let mut flow = implicit_tail_flow(&if_expression.then_branch, self);
                if let Some((_, else_expression)) = &if_expression.else_branch {
                    flow.union(self.flow_of_expr(else_expression));
                }
                flow
            }
            Expr::Match(match_expression) => {
                // Conservatively retain the scrutinee flow because match-arm
                // bindings can return an adapter-derived value (for example
                // `Some(db) => Arc::clone(db)`). A future type-aware data-flow
                // pass may narrow decoded scalar arms without losing those
                // bindings.
                let mut flow = self.flow_of_expr(&match_expression.expr);
                for arm in &match_expression.arms {
                    flow.union(self.flow_of_expr(&arm.body));
                }
                flow
            }
            Expr::Block(block) => implicit_tail_flow(&block.block, self),
            Expr::Tuple(tuple) => {
                let mut flow = Flow::default();
                for element in &tuple.elems {
                    flow.union(self.flow_of_expr(element));
                }
                flow
            }
            // These expressions always evaluate to `()`; persistence used by
            // their bodies is inventoried at the actual call/store sites and
            // must not be misreported as their result value.
            Expr::ForLoop(_) | Expr::While(_) => Flow::default(),
            Expr::Macro(expression) => self.flow_of_macro(&expression.mac),
            _ => {
                // `syn::Expr` is non-exhaustive. Conservatively propagate the
                // flow of every direct child for syntax that has no more
                // precise rule above, so present and future wrappers cannot
                // silently launder a concrete persistence value.
                let mut collector = DirectChildFlowCollector {
                    analyzer: self,
                    flow: Flow::default(),
                    at_root: true,
                };
                collector.visit_expr(expression);
                collector.flow
            }
        }
    }
    pub(super) fn flow_of_macro(&self, mac: &syn::Macro) -> Flow {
        let names = path_names(&mac.path);
        // The result of an aliased constructor carries the same flow as the
        // constructor it names, or a chained `execute` loses its query stage.
        let canonical = self.canonical_names(names.clone());
        let last = canonical.last().map(String::as_str).unwrap_or_default();
        let rooted_sqlx =
            path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical, self.symbols);
        if (rooted_sqlx && is_query_name(last))
            || (names.len() == 1 && self.symbols.query_callables.contains(last))
        {
            Flow::query()
        } else if let Some(targets) = (names.len() == 1)
            .then(|| self.symbols.persistence_macros.get(last))
            .flatten()
            .or_else(|| {
                let key = self.package_function_key(names.clone());
                self.symbols
                    .package_persistence_macros
                    .get(&key)
                    .or_else(|| self.symbols.package_persistence_macros.get(last))
            })
        {
            // A registered macro's definition already proves its concrete
            // targets. Preserve that result flow as well as auditing the call
            // site so `database!().pool()` cannot disappear behind the
            // definition's existing baseline row.
            Flow::pools(targets)
        } else if matches!(last, "vec" | "join" | "try_join" | "select") {
            // `vec!` is whitelisted as an opaque macro but it produces a
            // value: its result flow is the union of every in-scope
            // persistence value named in its input. Without this,
            // `let values = vec![database]; values[0].pool()` escapes both
            // ratchets.
            let mut flow = Flow::default();
            for scope in &self.scopes {
                for (local, info) in scope {
                    if !info.flow.is_empty()
                        && tokens_contain_identifier(
                            mac.tokens.clone(),
                            &BTreeSet::from([local.clone()]),
                        )
                    {
                        flow.union(info.flow.clone());
                    }
                }
            }
            for (callable, info) in self.symbols.package_function_returns.iter() {
                let leaf = callable.rsplit("::").next().unwrap_or(callable);
                if !info.flow.is_empty()
                    && tokens_contain_callable_invocation(
                        mac.tokens.clone(),
                        &BTreeSet::from([leaf.to_owned()]),
                    )
                {
                    flow.union(info.flow.clone());
                }
            }
            flow
        } else {
            Flow::default()
        }
    }
    pub(super) fn record_flow(
        &mut self,
        flow: &Flow,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        for target in flow.targets() {
            self.add(target, operation, symbol, cfg, fingerprint.clone());
        }
    }
    pub(super) fn record_pool_escape(
        &mut self,
        flow: &Flow,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        for target in flow.pool_targets() {
            self.add(target, operation, symbol, cfg, fingerprint.clone());
        }
    }
    pub(super) fn record_persistence_escape(
        &mut self,
        flow: &Flow,
        operation: PersistenceOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        for target in flow.targets() {
            self.add(target, operation, symbol, cfg, fingerprint.clone());
        }
    }
    pub(super) fn known_persistence_names(&self) -> BTreeSet<String> {
        let mut names = module_persistence_names(self.symbols);
        for scope in &self.scopes {
            names.extend(
                scope
                    .iter()
                    .filter(|(_, info)| !info.flow.0.is_empty())
                    .map(|(name, _)| name.clone()),
            );
        }
        names
    }
    pub(super) fn audit_macro(&mut self, mac: &syn::Macro, attributes: &[Attribute], owner: &str) {
        if !self.allows_source_class(attributes, owner) {
            return;
        }
        let names = path_names(&mac.path);
        let name = names.last().cloned().unwrap_or_default();
        // An alias — module-level or block-local — hides which macro is being
        // invoked, and every decision below depends on knowing that: whether
        // this is a query macro at all, which argument carries the statement,
        // and whether the referenced SQL lives outside the snapshot.
        let canonical_names = self.canonical_names(names.clone());
        let canonical_name = canonical_names.last().cloned().unwrap_or_default();
        let rooted_sqlx =
            path_is_sqlx(&names, self.symbols) || path_is_sqlx(&canonical_names, self.symbols);
        let imported_query = names.len() == 1
            && (self.symbols.query_callables.contains(&name)
                || self.symbols.query_callables.contains(&canonical_name));
        let cfg = item_cfg(&self.cfg, attributes);
        if canonical_name == "include" && !is_pinned_wow_proto_include(&self.context, mac) {
            self.errors.push(format!(
                "{} contains include! whose Rust source is outside the persistence AST inventory; mount and parse the included source explicitly",
                self.enclosing
            ));
            return;
        }
        if matches!(name.as_str(), "write" | "writeln")
            && let Ok(expressions) =
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.tokens.clone())
            && let Some(place) = expressions.first().and_then(mutable_storage_place)
        {
            let current = self.info_from_expr(place);
            if !current.sql_sources.is_empty() {
                let mut sql_sources = current.sql_sources.clone();
                sql_sources.insert(normalized_tokens(mac));
                let mut sql_expression = current.sql_expression;
                for argument in expressions.iter().skip(1) {
                    sql_expression = sql_expression.max(self.sql_expression_kind(argument));
                    sql_sources.extend(self.sql_sources(argument));
                }
                union_into_assignment_place(
                    self,
                    place,
                    &VariableInfo {
                        sql_expression,
                        sql_sources,
                        ..VariableInfo::default()
                    },
                );
                return;
            }
        }
        if (rooted_sqlx && is_query_name(&canonical_name)) || imported_query {
            // Every `query_file*` variant, `_unchecked` included, reads SQL
            // this inventory cannot see.
            if canonical_name.starts_with("query_file") {
                // Name the macro that is actually invoked: under an alias,
                // reporting `q!` alone leaves the reader to guess which SQLx
                // macro put the SQL outside the snapshot.
                let invoked = (canonical_name != name)
                    .then(|| format!("{name}! ({canonical_name}!)"))
                    .unwrap_or_else(|| format!("{canonical_name}!"));
                self.errors.push(format!(
                    "{} uses {invoked} SQL whose referenced file is outside the persistence snapshot; mount and fingerprint the SQL file explicitly",
                    self.enclosing
                ));
                return;
            }
            let fingerprint = normalized_tokens(mac);
            self.add_generated(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                fingerprint.clone(),
            );
            // Only one argument of a query macro is the statement; the rest are
            // bound values. `query!("SELECT ?", "GET_LOCK('x', 0)")` executes no
            // lock, so classifying the whole invocation would invent one.
            if query_macro_statement(&canonical_name, &mac.tokens)
                .is_some_and(|statement| self.statement_takes_advisory_lock(&statement))
            {
                self.add_generated(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::AdvisoryLock,
                    &name,
                    &cfg,
                    fingerprint,
                );
            }
            return;
        }
        // `migrate!` runs every `.sql` file of a directory this inventory does
        // not read, so a baselined invocation would let their contents — and
        // the statements they execute — change with no row moving.
        if rooted_sqlx && canonical_name == "migrate" {
            self.errors.push(format!(
                "{} runs migrate! SQL whose migration directory is outside the persistence snapshot; mount and fingerprint the migrations explicitly",
                self.enclosing
            ));
            return;
        }
        let direct_targets = targets_for_path(&mac.path, self.symbols);
        if rooted_sqlx || !direct_targets.is_empty() {
            let targets = if direct_targets.is_empty() {
                BTreeSet::from([PersistenceTarget::Sqlx])
            } else {
                direct_targets
            };
            for target in targets {
                self.add_generated(
                    target,
                    PersistenceOperation::MacroReference,
                    &name,
                    &cfg,
                    normalized_tokens(mac),
                );
            }
            return;
        }
        let registered_macro_targets = (names.len() == 1)
            .then(|| self.symbols.persistence_macros.get(&name))
            .flatten()
            .or_else(|| {
                let key = self.package_function_key(names.clone());
                self.symbols
                    .package_persistence_macros
                    .get(&key)
                    .or_else(|| self.symbols.package_persistence_macros.get(&name))
            })
            .cloned();
        if let Some(targets) = registered_macro_targets {
            // Invocation of a registered persistence-generating `macro_rules!`:
            // the definition row covers the body, but each call site must
            // leave its own row so adding another invocation cannot bypass
            // both ratchets without touching the generated artifacts.
            let mut fingerprint = normalized_tokens(mac);
            let mut sql_sources = BTreeSet::new();
            let mut sql_kind = SqlExpressionKind::Static;
            if let Ok(expressions) =
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.tokens.clone())
            {
                for expression in expressions {
                    let sources = self.sql_sources(&expression);
                    if !sources.is_empty() {
                        sql_kind = sql_kind.max(self.sql_expression_kind(&expression));
                        sql_sources.extend(sources);
                    }
                }
            }
            if !sql_sources.is_empty() {
                fingerprint = format!(
                    "{fingerprint}|sql-source:{}",
                    sql_sources.into_iter().collect::<Vec<_>>().join("|")
                );
            }
            for target in targets {
                self.add(
                    target,
                    PersistenceOperation::MacroReference,
                    &name,
                    &cfg,
                    fingerprint.clone(),
                );
                if self.statement_takes_advisory_lock(&fingerprint) {
                    self.add(
                        target,
                        PersistenceOperation::AdvisoryLock,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                }
                if matches!(sql_kind, SqlExpressionKind::Interpolated) {
                    self.add(
                        target,
                        PersistenceOperation::InterpolatedSql,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                }
            }
            return;
        }
        let mut argument_flow = Flow::default();
        if let Ok(expressions) =
            syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
        {
            for expression in expressions {
                argument_flow.union(self.subtree_flow(&expression));
            }
        }
        let known = self.known_persistence_names();
        if !tokens_contain_identifier(mac.tokens.clone(), &known) && argument_flow.is_empty() {
            return;
        }
        if OPAQUE_PERSISTENCE_MACROS.contains(&name.as_str()) {
            let fingerprint = normalized_tokens(mac);
            let mut sqlx_calls = Vec::new();
            sqlx_calls_in_tokens(mac.tokens.clone(), &mut sqlx_calls);
            for (callable, call_fingerprint) in sqlx_calls {
                self.add(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::Query,
                    &callable,
                    &cfg,
                    call_fingerprint.clone(),
                );
                if self.statement_takes_advisory_lock(&call_fingerprint) {
                    self.add(
                        PersistenceTarget::Sqlx,
                        PersistenceOperation::AdvisoryLock,
                        &callable,
                        &cfg,
                        call_fingerprint,
                    );
                }
            }
            let advisory_methods = BTreeSet::from([
                "acquire_like_cpp".to_owned(),
                "release_like_cpp".to_owned(),
                "wait_until_lost_like_cpp".to_owned(),
            ]);
            let mut methods = Vec::new();
            persistence_methods_in_tokens(mac.tokens.clone(), &advisory_methods, &mut methods);
            methods.sort();
            for method in methods {
                let mut targets = TargetSet::new();
                for scope in &self.scopes {
                    for (local, info) in scope {
                        if tokens_contain_identifier(
                            mac.tokens.clone(),
                            &BTreeSet::from([local.clone()]),
                        ) {
                            targets.extend(info.flow.targets());
                        }
                    }
                }
                for target in targets {
                    self.add(
                        target,
                        PersistenceOperation::AdvisoryLock,
                        &method,
                        &cfg,
                        format!("opaque-macro-method:{method}"),
                    );
                }
            }
            let mut targets = targets_in_tokens(mac.tokens.clone(), self.symbols);
            let mut escaped = Flow::default();
            let parsed = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone());
            if let Ok(expressions) = parsed {
                for expression in expressions {
                    escaped.union(self.subtree_flow(&expression));
                }
                targets.extend(escaped.targets());
            } else {
                // Macros with custom grammars (`select!`, pattern arms, etc.)
                // remain fail-closed: any referenced persistent local escapes.
                for scope in &self.scopes {
                    for (local, info) in scope {
                        if tokens_contain_identifier(
                            mac.tokens.clone(),
                            &BTreeSet::from([local.clone()]),
                        ) {
                            targets.extend(info.flow.targets());
                            escaped.union(info.flow.clone());
                        }
                    }
                }
            }
            for target in targets {
                self.add(
                    target,
                    PersistenceOperation::MacroReference,
                    &name,
                    &cfg,
                    fingerprint.clone(),
                );
                if target == PersistenceTarget::Sqlx
                    && self.statement_takes_advisory_lock(&fingerprint)
                {
                    self.add(
                        target,
                        PersistenceOperation::AdvisoryLock,
                        &name,
                        &cfg,
                        fingerprint.clone(),
                    );
                }
            }
            self.record_pool_escape(
                &escaped,
                PersistenceOperation::ArgumentEscape,
                &format!("macro:{name}"),
                &cfg,
                fingerprint,
            );
        } else {
            self.errors.push(format!(
                "{} passes concrete persistence syntax/value through unknown macro {name}!; expose an explicit SQLx query or ordinary Rust call before baselining it",
                self.enclosing
            ));
        }
    }
    pub(super) fn register_parameters(
        &mut self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) {
        for input in inputs {
            let FnArg::Typed(typed) = input else {
                continue;
            };
            if !self.allows_source_class(&typed.attrs, "function parameter") {
                continue;
            }
            let mut info = self.info_from_type(&typed.ty);
            // Parameter values are supplied at runtime even when their type
            // is a source-known `&str`; only literal-producing expressions
            // can retain the default Static classification.
            info.sql_expression = SqlExpressionKind::Nonliteral;
            self.bind_pattern(&typed.pat, &info);
            add_type_records(
                self.accumulator,
                &self.context,
                self.symbols,
                &typed.ty,
                &self.enclosing,
                &normalized_tokens(&typed.pat),
                &self.visibility,
                &item_cfg(&self.cfg, &typed.attrs),
                PersistenceOperation::TypeReference,
            );
        }
    }
}
