//! The syn visitor that drives body analysis; a trait impl cannot be divided.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

impl<'ast> Visit<'ast> for BodyAnalyzer<'_, '_> {
    fn visit_expr_binary(&mut self, expression: &'ast syn::ExprBinary) {
        if !self.allows_source_class(&expression.attrs, "binary expression") {
            return;
        }
        self.visit_expr(&expression.left);
        if matches!(expression.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            // The right side of `&&`/`||` may not execute. Audit it, but
            // retain both the pre-RHS and post-RHS binding states so a
            // conditional assignment cannot erase persistence flow that is
            // still reachable through the short-circuit path.
            let pre_rhs_scopes = self.scopes.clone();
            self.visit_expr(&expression.right);
            merge_scope_stacks(&mut self.scopes, &pre_rhs_scopes);
            self.bump_context();
        } else {
            self.visit_expr(&expression.right);
        }
        if is_assignment_binop(&expression.op) {
            let info = self.info_from_expr(&expression.right);
            union_into_assignment_place(self, &expression.left, &info);
            let root = simple_assignment_name(&expression.left)
                .or_else(|| assignment_place(&expression.left).map(|(root, _)| root));
            if let Some(root) = root {
                let mut aggregate = self.lookup(&root).cloned().unwrap_or_default();
                self.union_into_declared_projections(&mut aggregate, &info);
                self.assign(&root, aggregate);
            }
            let cfg = item_cfg(&self.cfg, &expression.attrs);
            self.record_pool_escape(
                &info.flow,
                PersistenceOperation::StoreEscape,
                "compound_assignment",
                &cfg,
                normalized_tokens(expression),
            );
        }
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        if !self.allows_source_class(&expression.attrs, "for-loop expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        self.visit_expr(&expression.expr);
        let iterator_info = self.info_from_expr(&expression.expr);
        self.record_pool_escape(
            &iterator_info.flow,
            PersistenceOperation::ArgumentEscape,
            "for_iter",
            &cfg,
            normalized_tokens(&expression.expr),
        );
        // The body may run zero times (empty iterator), so its assignments
        // to outer locals cannot be applied unconditionally: conservatively
        // union the pre-loop state with the post-body state.
        let mut accumulated_scopes = self.scopes.clone();
        let label = expression
            .label
            .as_ref()
            .map(|label| normalized_ident(&label.name.ident));
        let initial_error_count = self.errors.len();
        self.accumulator.begin_transaction();
        let first_flow = self.visit_for_loop_body(expression, &iterator_info, label.clone());
        let first_post_body_scopes = self.scopes.clone();
        let mut first_next = accumulated_scopes.clone();
        merge_scope_stacks(&mut first_next, &first_post_body_scopes);
        if let Some(back_edges) = &first_flow.back_edges {
            merge_scope_stacks(&mut first_next, back_edges);
        }
        if first_next == accumulated_scopes {
            self.accumulator.commit_transaction();
            self.scopes = accumulated_scopes;
            if let Some(exits) = first_flow.exits {
                merge_scope_stacks(&mut self.scopes, &exits);
            }
            self.bump_context();
            return;
        }
        self.accumulator.rollback_transaction();
        self.errors.truncate(initial_error_count);
        accumulated_scopes = first_next;
        let mut stabilized = false;
        for iteration in 0..32 {
            self.scopes = accumulated_scopes.clone();
            let mut flow = None;
            self.analyze_without_records(|analyzer| {
                flow =
                    Some(analyzer.visit_for_loop_body(expression, &iterator_info, label.clone()));
            });
            let flow = flow.expect("for-loop analysis produced flow state");
            let post_body_scopes = self.scopes.clone();
            let mut next = accumulated_scopes.clone();
            merge_scope_stacks(&mut next, &post_body_scopes);
            if let Some(back_edges) = &flow.back_edges {
                merge_scope_stacks(&mut next, back_edges);
            }
            if iteration >= 7 {
                widen_loop_scopes(&mut next);
            }
            if next == accumulated_scopes {
                stabilized = true;
                break;
            }
            accumulated_scopes = next;
        }
        if !stabilized {
            self.errors.push(format!(
                "{} for-loop persistence flow did not reach a fixed point",
                self.enclosing
            ));
        }
        self.scopes = accumulated_scopes.clone();
        let flow = self.visit_for_loop_body(expression, &iterator_info, label);
        self.scopes = accumulated_scopes;
        if let Some(exits) = flow.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
        }
        self.bump_context();
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        if !self.allows_source_class(&expression.attrs, "while expression") {
            return;
        }
        // The body may run zero times (false condition), so its assignments
        // to outer locals cannot be applied unconditionally: conservatively
        // union the pre-loop state with the post-body state.
        let mut accumulated_scopes = self.scopes.clone();
        let label = expression
            .label
            .as_ref()
            .map(|label| normalized_ident(&label.name.ident));
        let initial_error_count = self.errors.len();
        self.accumulator.begin_transaction();
        let first_flow = self.visit_while_loop_body(expression, label.clone());
        let first_post_body_scopes = self.scopes.clone();
        let mut first_next = accumulated_scopes.clone();
        merge_scope_stacks(&mut first_next, &first_post_body_scopes);
        if let Some(back_edges) = &first_flow.back_edges {
            merge_scope_stacks(&mut first_next, back_edges);
        }
        if first_next == accumulated_scopes {
            self.accumulator.commit_transaction();
            self.scopes = accumulated_scopes;
            if let Some(exits) = first_flow.exits {
                merge_scope_stacks(&mut self.scopes, &exits);
            }
            self.bump_context();
            return;
        }
        self.accumulator.rollback_transaction();
        self.errors.truncate(initial_error_count);
        accumulated_scopes = first_next;
        let mut stabilized = false;
        for iteration in 0..32 {
            self.scopes = accumulated_scopes.clone();
            let mut flow = None;
            self.analyze_without_records(|analyzer| {
                flow = Some(analyzer.visit_while_loop_body(expression, label.clone()));
            });
            let flow = flow.expect("while-loop analysis produced flow state");
            let post_body_scopes = self.scopes.clone();
            let mut next = accumulated_scopes.clone();
            merge_scope_stacks(&mut next, &post_body_scopes);
            if let Some(back_edges) = &flow.back_edges {
                merge_scope_stacks(&mut next, back_edges);
            }
            if iteration >= 7 {
                widen_loop_scopes(&mut next);
            }
            if next == accumulated_scopes {
                stabilized = true;
                break;
            }
            accumulated_scopes = next;
        }
        if !stabilized {
            self.errors.push(format!(
                "{} while-loop persistence flow did not reach a fixed point",
                self.enclosing
            ));
        }
        self.scopes = accumulated_scopes.clone();
        let flow = self.visit_while_loop_body(expression, label);
        self.scopes = accumulated_scopes;
        if let Some(exits) = flow.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
        }
        self.bump_context();
    }

    fn visit_expr_async(&mut self, expression: &'ast syn::ExprAsync) {
        if !self.allows_source_class(&expression.attrs, "async expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let flow = self.flow_in_block(&expression.block);
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ArgumentEscape,
            "async_capture",
            &cfg,
            normalized_tokens(expression),
        );
        // Constructing a future does not execute its body. Audit the body for
        // captures and accesses, but keep both the declaration-time bindings
        // and the bindings that would result if the future were later polled.
        let declaration_scopes = self.scopes.clone();
        self.push_scope();
        self.return_exit_collectors.push(None);
        self.return_value_collectors.push(VariableInfo::default());
        self.visit_block(&expression.block);
        self.return_value_collectors
            .pop()
            .expect("async return value collector was installed");
        if let Some(exits) = self
            .return_exit_collectors
            .pop()
            .expect("async return collector was installed")
        {
            merge_scope_stacks(&mut self.scopes, &exits);
            self.bump_context();
        }
        self.pop_scope();
        merge_scope_stacks(&mut self.scopes, &declaration_scopes);
        self.bump_context();
    }

    fn visit_expr_array(&mut self, expression: &'ast syn::ExprArray) {
        if !self.allows_source_class(&expression.attrs, "array expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let mut flow = Flow::default();
        for element in &expression.elems {
            flow.union(self.subtree_flow(element));
        }
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ArgumentEscape,
            "array_value",
            &cfg,
            normalized_tokens(expression),
        );
        for element in &expression.elems {
            self.visit_expr(element);
        }
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        if !self.allows_source_class(&expression.attrs, "loop expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        let flow = self.flow_in_block(&expression.body);
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ArgumentEscape,
            "loop_value",
            &cfg,
            normalized_tokens(expression),
        );
        let mut accumulated_scopes = self.scopes.clone();
        let label = expression
            .label
            .as_ref()
            .map(|label| normalized_ident(&label.name.ident));
        let initial_error_count = self.errors.len();
        self.accumulator.begin_transaction();
        let first_flow = self.visit_loop_block(&expression.body, label.clone());
        let first_post_body_scopes = self.scopes.clone();
        let mut first_next = accumulated_scopes.clone();
        merge_scope_stacks(&mut first_next, &first_post_body_scopes);
        if let Some(back_edges) = &first_flow.back_edges {
            merge_scope_stacks(&mut first_next, back_edges);
        }
        if first_next == accumulated_scopes {
            self.accumulator.commit_transaction();
            self.scopes = accumulated_scopes;
            if let Some(exits) = first_flow.exits {
                merge_scope_stacks(&mut self.scopes, &exits);
            }
            self.bump_context();
            return;
        }
        self.accumulator.rollback_transaction();
        self.errors.truncate(initial_error_count);
        accumulated_scopes = first_next;
        let mut stabilized = false;
        for iteration in 0..32 {
            self.scopes = accumulated_scopes.clone();
            self.loop_flow_collectors.push(LoopFlowCollector {
                label: label.clone(),
                ..LoopFlowCollector::default()
            });
            self.analyze_without_records(|analyzer| analyzer.visit_block(&expression.body));
            let flow = self
                .loop_flow_collectors
                .pop()
                .expect("loop flow collector was installed");
            let post_body_scopes = self.scopes.clone();
            let mut next = accumulated_scopes.clone();
            merge_scope_stacks(&mut next, &post_body_scopes);
            if let Some(back_edges) = &flow.back_edges {
                merge_scope_stacks(&mut next, back_edges);
            }
            if iteration >= 7 {
                widen_loop_scopes(&mut next);
            }
            if next == accumulated_scopes {
                stabilized = true;
                break;
            }
            accumulated_scopes = next;
        }
        if !stabilized {
            self.errors.push(format!(
                "{} loop persistence flow did not reach a fixed point",
                self.enclosing
            ));
        }
        self.scopes = accumulated_scopes.clone();
        let flow = self.visit_loop_block(&expression.body, label);
        self.scopes = accumulated_scopes;
        if let Some(exits) = flow.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
        }
        self.bump_context();
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if let syn::Stmt::Item(item) = statement {
            // Block-local `use` declarations are already modelled by
            // `register_local_uses`, which imports their aliases into the
            // lexical path scopes before any statement runs, so they neither
            // hide persistence nor need new inventory rows. Block-local item
            // macros keep the ordinary macro analysis.
            if matches!(item, Item::Use(_) | Item::Macro(_)) {
                syn::visit::visit_stmt(self, statement);
                return;
            }
            // Other block-local item declarations never reach
            // `collect_module_symbols`, so a local alias such as
            // `type Alias = sqlx::MySqlPool;` would let the body reach
            // concrete persistence without any inventory row. Fail closed on
            // any block-local item that mentions persistence instead of
            // silently delegating to the default traversal.
            if !self.allows_source_class(item_attributes(item), "block-local item") {
                return;
            }
            if let Item::Const(item_const) = item {
                let (kind, sources) =
                    source_sql_info(&item_const.expr, &|path| self.standard_string_macro(path));
                if kind != SqlExpressionKind::Nonliteral {
                    let mut info = self.info_from_type(&item_const.ty);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    self.bind(normalized_ident(&item_const.ident), info);
                    return;
                }
            }
            if let Item::Static(item_static) = item {
                let (kind, sources) =
                    source_sql_info(&item_static.expr, &|path| self.standard_string_macro(path));
                if kind != SqlExpressionKind::Nonliteral {
                    let mut info = self.info_from_type(&item_static.ty);
                    info.sql_expression = kind;
                    info.sql_sources = sources;
                    self.bind(normalized_ident(&item_static.ident), info);
                    return;
                }
            }
            if !targets_in_tokens(item.to_token_stream(), self.symbols).is_empty()
                || syntax_mentions_persistence(item, self.symbols)
            {
                self.errors.push(format!(
                    "{} contains a block-local item that mentions concrete persistence; hoist the declaration to module scope so the symbol collector can model it: {}",
                    self.context.module,
                    normalized_tokens(item)
                ));
            } else if matches!(item, Item::Fn(_)) {
                let operations = persistence_operations_in_syntax(item);
                if !operations.is_empty() {
                    self.errors.push(format!(
                        "{} contains a block-local function with persistence-shaped operations ({}) whose generic receiver cannot be audited at call sites; hoist it to module scope",
                        self.context.module,
                        operations.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            return;
        }
        syn::visit::visit_stmt(self, statement);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_scope();
        self.register_local_uses(&block.stmts);
        self.register_local_callables(&block.stmts);
        self.register_local_constants(&block.stmts);
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        if let Some(Stmt::Expr(expression, None)) = block.stmts.last() {
            let info = self.info_from_expr(expression);
            self.block_result_infos
                .borrow_mut()
                .entry(block as *const syn::Block as usize)
                .or_default()
                .union(&info);
        }
        self.pop_scope();
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if !self.allows_source_class(&local.attrs, "local binding") {
            return;
        }
        let previous_cfg = self.cfg.clone();
        self.cfg = item_cfg(&self.cfg, &local.attrs);
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                let continuing_scopes = self.scopes.clone();
                self.visit_expr(diverge);
                self.scopes = continuing_scopes;
                self.bump_context();
            }
            let info = self.info_from_expr(&init.expr);
            let mut names = Vec::new();
            pattern_identifiers(&local.pat, &mut names);
            let cfg = self.cfg.clone();
            for name in names {
                self.record_flow(
                    &info.flow,
                    PersistenceOperation::ValueAlias,
                    &name,
                    &cfg,
                    format!("{}={}", name, normalized_tokens(&init.expr)),
                );
            }
            self.bind_pattern_from_expr(&local.pat, &init.expr);
        } else {
            let info = match &local.pat {
                Pat::Type(typed) => self.info_from_type(&typed.ty),
                _ => VariableInfo::default(),
            };
            self.bind_pattern(&local.pat, &info);
        }
        if let Pat::Type(typed) = &local.pat {
            let cfg = self.cfg.clone();
            add_type_records(
                self.accumulator,
                &self.context,
                self.symbols,
                &typed.ty,
                &self.enclosing,
                &normalized_tokens(&typed.pat),
                &self.visibility,
                &cfg,
                PersistenceOperation::TypeReference,
            );
        }
        self.cfg = previous_cfg;
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if !self.allows_source_class(&path.attrs, "expression path") {
            return;
        }
        if path.qself.is_none() && path.path.segments.len() == 1 {
            if let Some(name) = last_path_name(&path.path) {
                if self.lookup(&name).is_some() {
                    return;
                }
            }
        }
        let cfg = item_cfg(&self.cfg, &path.attrs);
        let symbol = last_path_name(&path.path).unwrap_or_default();
        for target in targets_for_path(&path.path, self.symbols) {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &symbol,
                &cfg,
                canonical_path(&path.path),
            );
            if matches!(
                target,
                PersistenceTarget::LoginStatements
                    | PersistenceTarget::WorldStatements
                    | PersistenceTarget::CharStatements
                    | PersistenceTarget::HotfixStatements
            ) && is_generated_id_read_statement(&symbol)
            {
                self.add(
                    target,
                    PersistenceOperation::GeneratedIdRead,
                    &symbol,
                    &cfg,
                    canonical_path(&path.path),
                );
            }
        }
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        if !self.allows_source_class(&field.attrs, "field expression") {
            return;
        }
        // A nested projection is an implementation detail of the complete
        // field path. Auditing every prefix made `value.inner.clean` record
        // `value.inner` as a persistence reference merely because a sibling
        // field in `inner` was persistent. Still visit the non-field root so
        // calls, indices, and other side-effecting bases remain visible.
        let mut root = field.base.as_ref();
        while let Expr::Field(parent) = root {
            root = parent.base.as_ref();
        }
        self.visit_expr(root);
        let cfg = item_cfg(&self.cfg, &field.attrs);
        let name = match &field.member {
            Member::Named(ident) => normalized_ident(ident),
            Member::Unnamed(index) => index.index.to_string(),
        };
        let flow = self.field_flow(field);
        for target in flow.pool_targets() {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &name,
                &cfg,
                normalized_tokens(field),
            );
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if !self.allows_source_class(&call.attrs, "function call") {
            return;
        }
        for argument in &call.args {
            if matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
        let cfg = item_cfg(&self.cfg, &call.attrs);
        let (
            name,
            canonical_name,
            executor_owner,
            rooted_sqlx,
            imported_query,
            path_targets,
            flow_passthrough,
        ) = match call.func.as_ref() {
            Expr::Path(path) => {
                let names = path_names(&path.path);
                // A block-local `use sqlx::query as q` renames the callee
                // without changing what it constructs, so the decision has
                // to be made on the path it resolves to.
                let canonical = self.canonical_names(names.clone());
                let canonical_name = canonical.last().cloned().unwrap_or_default();
                // The row keeps the name the source writes; only the
                // dispatch below follows the alias to what it constructs.
                let name = names.last().cloned().unwrap_or_default();
                // A local binding can name an executor, and the call
                // through it is that executor.
                let stored_executor = (names.len() == 1)
                    .then(|| {
                        self.lookup(&names[0])
                            .and_then(|info| info.executor_callable.clone())
                    })
                    .flatten();
                let canonical_name = stored_executor.clone().unwrap_or(canonical_name);
                let canonical = match &stored_executor {
                    Some(method) => {
                        vec!["sqlx".to_owned(), "Executor".to_owned(), method.clone()]
                    }
                    None => canonical,
                };
                let rooted_sqlx = path_is_sqlx(&names, self.symbols)
                    || path_is_sqlx(&canonical, self.symbols)
                    || stored_executor.is_some();
                let imported_query = names.len() == 1
                    && (self.symbols.query_callables.contains(&name)
                        || self.symbols.query_callables.contains(&canonical_name)
                        || self.lookup(&name).is_some_and(|info| info.query_callable));
                (
                    name,
                    canonical_name,
                    canonical
                        .iter()
                        .nth_back(1)
                        .is_some_and(|segment| segment == "Executor"),
                    rooted_sqlx,
                    imported_query,
                    targets_for_names(&canonical, self.symbols),
                    is_flow_passthrough_call(&names),
                )
            }
            // `(sqlx::query)(SQL)` names the same callable as `sqlx::query`;
            // the parentheses do not change what it constructs.
            callee => {
                let info = self.info_from_expr(callee);
                let executor = info.executor_callable.clone();
                let name = executor.clone().unwrap_or_else(|| {
                    info.query_callable
                        .then(|| "query".to_owned())
                        .unwrap_or_default()
                });
                (
                    name.clone(),
                    name,
                    executor.is_some(),
                    executor.is_some() || info.query_callable,
                    info.query_callable,
                    TargetSet::new(),
                    false,
                )
            }
        };
        let query_builder_constructor = rooted_sqlx
            && canonical_name == "new"
            && matches!(
                call.func.as_ref(),
                Expr::Path(path)
                    if self
                        .canonical_names(path_names(&path.path))
                        .iter()
                        .nth_back(1)
                        .is_some_and(|segment| segment == "QueryBuilder")
            );
        let query = (rooted_sqlx && (is_query_name(&name) || is_query_name(&canonical_name)))
            || imported_query
            || query_builder_constructor;
        let has_path_targets = !path_targets.is_empty();
        if query {
            let fingerprint = self.fingerprint_with_sql_source(
                canonical_call(call),
                call.args.first(),
            );
            self.add(
                PersistenceTarget::Sqlx,
                PersistenceOperation::Query,
                &name,
                &cfg,
                fingerprint.clone(),
            );
            if canonical_name == "raw_sql" {
                self.add(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::RawSql,
                    &name,
                    &cfg,
                    fingerprint.clone(),
                );
            }
            if self.statement_takes_advisory_lock(&fingerprint) {
                self.add(
                    PersistenceTarget::Sqlx,
                    PersistenceOperation::AdvisoryLock,
                    &name,
                    &cfg,
                    fingerprint,
                );
            }
            if let Some(argument) = call.args.first() {
                match self.sql_expression_kind(argument) {
                    SqlExpressionKind::Static => {}
                    SqlExpressionKind::Included => self.errors.push(format!(
                        "{} passes include_str! SQL whose content is outside the persistence snapshot; mount and fingerprint the included SQL source explicitly",
                        self.enclosing
                    )),
                    SqlExpressionKind::Environment => self.errors.push(format!(
                        "{} passes env! SQL whose expanded content is outside the persistence snapshot; pin the SQL in reviewed source",
                        self.enclosing
                    )),
                    kind @ (SqlExpressionKind::Nonliteral | SqlExpressionKind::Interpolated) => {
                        self.add(
                            PersistenceTarget::Sqlx,
                            match kind {
                                SqlExpressionKind::Interpolated => {
                                    PersistenceOperation::InterpolatedSql
                                }
                                SqlExpressionKind::Nonliteral => {
                                    PersistenceOperation::NonliteralSql
                                }
                                SqlExpressionKind::Static
                                | SqlExpressionKind::Included
                                | SqlExpressionKind::Environment => {
                                    unreachable!()
                                }
                            },
                            &name,
                            &cfg,
                            normalized_tokens(argument),
                        );
                    }
                }
            }
        } else if let Some(operation) = PersistenceOperation::from_executor_method(&canonical_name)
            .filter(
            |_| {
                rooted_sqlx
                    || has_path_targets
                    || (matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
                        && call
                            .args
                            .first()
                            .is_some_and(|receiver| !self.flow_of_expr(receiver).is_empty()))
            },
        )
        {
            // The trait can be imported under another name, and `prepare`
            // receives its statement the same way the executors do.
            let ufcs_executor_sql = matches!(
                operation,
                PersistenceOperation::Execute
                    | PersistenceOperation::Fetch
                    | PersistenceOperation::FetchAll
                    | PersistenceOperation::FetchMany
                    | PersistenceOperation::FetchOne
                    | PersistenceOperation::FetchOptional
                    | PersistenceOperation::PrepareStatement
            ) && rooted_sqlx
                && executor_owner
                && call.args.first().is_some_and(|receiver| {
                    let flow = self.flow_of_expr(receiver);
                    !flow.is_empty() && !flow.has_stage(FlowStage::Query)
                })
                // The second argument is the statement only when it is not an
                // already built query, exactly as in the method form.
                && call.args.get(1).is_some_and(|argument| {
                    !self.flow_of_expr(argument).has_stage(FlowStage::Query)
                });
            let mut targets = path_targets;
            for argument in &call.args {
                targets.extend(self.flow_of_expr(argument).targets());
            }
            if targets.is_empty() {
                targets.insert(PersistenceTarget::Sqlx);
            }
            for target in targets {
                let prepared_statement_sql = name == "new"
                    && target == PersistenceTarget::PreparedStatement
                    && call.args.first().is_some();
                let raw_sql_argument = if prepared_statement_sql {
                    call.args.first()
                } else if ufcs_executor_sql {
                    call.args.get(1)
                } else {
                    None
                };
                let operation = match (name.as_str(), target) {
                    ("new", PersistenceTarget::PreparedStatement)
                    | ("new", PersistenceTarget::SqlQueryHolder)
                    | ("with_capacity_like_cpp", PersistenceTarget::PreparedStatement) => {
                        PersistenceOperation::StatementBuilder
                    }
                    ("new", PersistenceTarget::SqlTransaction) => {
                        PersistenceOperation::TransactionConstruct
                    }
                    ("new", _) => PersistenceOperation::PathReference,
                    _ => operation,
                };
                let fingerprint = if raw_sql_argument.is_some() {
                    self.fingerprint_with_sql_source(canonical_call(call), raw_sql_argument)
                } else {
                    canonical_call(call)
                };
                self.add(target, operation, &name, &cfg, fingerprint.clone());
                if let Some(argument) = raw_sql_argument {
                    self.add(
                        target,
                        PersistenceOperation::RawSql,
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
                    match self.sql_expression_kind(argument) {
                        SqlExpressionKind::Static => {}
                        SqlExpressionKind::Included => self.errors.push(format!(
                            "{} passes include_str! SQL whose content is outside the persistence snapshot; mount and fingerprint the included SQL source explicitly",
                            self.enclosing
                        )),
                        SqlExpressionKind::Environment => self.errors.push(format!(
                            "{} passes env! SQL whose expanded content is outside the persistence snapshot; pin the SQL in reviewed source",
                            self.enclosing
                        )),
                        SqlExpressionKind::Nonliteral => self.add(
                            target,
                            PersistenceOperation::NonliteralSql,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        ),
                        SqlExpressionKind::Interpolated => self.add(
                            target,
                            PersistenceOperation::InterpolatedSql,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        ),
                    }
                }
            }
        }

        let known_persistence_call = query
            || (PersistenceOperation::from_executor_method(&name).is_some()
                && (rooted_sqlx
                    || has_path_targets
                    || (matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
                        && call
                            .args
                            .first()
                            .is_some_and(|receiver| !self.flow_of_expr(receiver).is_empty()))));
        if let Expr::Path(path) = call.func.as_ref()
            && is_standard_replacement(&self.canonical_local_path_names(path_names(&path.path)))
            && let Some(binding) = call.args.first().and_then(mutable_storage_receiver_name)
            && let Some(previous) = self.lookup(&binding).cloned()
        {
            self.replacement_result_infos
                .borrow_mut()
                .entry(call as *const ExprCall as usize)
                .or_default()
                .union(&previous);
        }
        if !flow_passthrough && !known_persistence_call {
            for argument in &call.args {
                let flow = self.flow_of_expr(argument);
                self.record_persistence_escape(
                    &flow,
                    PersistenceOperation::ArgumentEscape,
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
        }
        if matches!(
            name.as_str(),
            "push" | "push_back" | "push_front" | "insert" | "extend" | "append"
        ) && matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
            && let Some(receiver_name) = call.args.first().and_then(mutable_storage_receiver_name)
        {
            let mut stored = self.lookup(&receiver_name).cloned().unwrap_or_default();
            let before = stored.clone();
            for argument in call.args.iter().skip(1) {
                stored.union(&self.info_from_expr(argument));
            }
            if stored != before {
                self.assign(&receiver_name, stored);
            }
        }
        if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
            && matches!(name.as_str(), "replace" | "write")
            && let Some(receiver_name) = call.args.first().and_then(mutable_storage_receiver_name)
            && let Some(replacement) = call.args.iter().nth(1)
        {
            self.assign(&receiver_name, self.info_from_expr(replacement));
        }
        if let Expr::Path(path) = call.func.as_ref()
            && is_standard_replacement(&self.canonical_local_path_names(path_names(&path.path)))
            && name == "take"
            && let Some(receiver_name) = call.args.first().and_then(mutable_storage_receiver_name)
        {
            self.assign(&receiver_name, VariableInfo::default());
        }
        if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() >= 2)
            && name == "swap"
            && let (Some(left), Some(right)) = (
                call.args.first().and_then(mutable_storage_receiver_name),
                call.args.get(1).and_then(mutable_storage_receiver_name),
            )
        {
            let mut merged = self.lookup(&left).cloned().unwrap_or_default();
            merged.union(&self.lookup(&right).cloned().unwrap_or_default());
            self.assign(&left, merged.clone());
            self.assign(&right, merged);
        }
        let argument_infos = call
            .args
            .iter()
            .map(|argument| self.info_from_expr(argument))
            .collect::<Vec<_>>();
        for (index, argument) in call.args.iter().enumerate() {
            let Some(binding) = mutable_storage_receiver_name(argument) else {
                continue;
            };
            let mut updated = self.lookup(&binding).cloned().unwrap_or_default();
            for (other_index, info) in argument_infos.iter().enumerate() {
                if other_index != index {
                    updated.union(info);
                }
            }
            self.assign(&binding, updated);
        }
        if let Expr::Path(path) = call.func.as_ref() {
            let names = path_names(&path.path);
            let callee = names.last().cloned().unwrap_or_default();
            let package_key = self.package_function_key(names.clone());
            let effects = (names.len() == 1)
                .then(|| self.symbols.function_mutable_writes.get(&callee))
                .flatten()
                .or_else(|| {
                    self.symbols
                        .package_function_mutable_writes
                        .get(&package_key)
                })
                .cloned()
                .unwrap_or_default();
            for (index, effect) in effects {
                let Some(place) = call.args.get(index).and_then(mutable_storage_place) else {
                    continue;
                };
                let effect = self.apply_inferred_args(
                    &effect,
                    self.symbols
                        .function_generic_params
                        .get(&callee)
                        .or_else(|| {
                            self.symbols
                                .package_function_generic_params
                                .get(&package_key)
                        }),
                    self.symbols
                        .function_generic_input_params
                        .get(&callee)
                        .or_else(|| {
                            self.symbols
                                .package_function_generic_input_params
                                .get(&package_key)
                        }),
                    &call.args,
                );
                union_into_assignment_place(self, place, &effect);
            }
        }
        let callee_info = self.info_from_expr(&call.func);
        for (captured, info) in self.closure_mutations_for_args(&callee_info, &argument_infos) {
            self.assign(&captured, info);
        }
        if !known_persistence_call && !flow_passthrough {
            let before_callback = self.scopes.clone();
            let called_inputs = match call.func.as_ref() {
                Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                    last_path_name(&path.path)
                        .and_then(|name| self.symbols.function_called_inputs.get(&name))
                        .cloned()
                        .unwrap_or_default()
                }
                _ => BTreeSet::new(),
            };
            for index in called_inputs {
                let Some(info) = argument_infos.get(index) else {
                    continue;
                };
                for (captured, mutation) in self.closure_mutations_for_args(info, &[]) {
                    self.assign(&captured, mutation);
                }
            }
            let after_callback = self.scopes.clone();
            self.scopes = before_callback;
            merge_scope_stacks(&mut self.scopes, &after_callback);
            self.bump_context();
        }
        self.visit_expr(&call.func);
        for argument in &call.args {
            if !matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
    }

    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        if !self.allows_source_class(&method.attrs, "method call") {
            return;
        }
        for argument in &method.args {
            if matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
        let cfg = item_cfg(&self.cfg, &method.attrs);
        let name = normalized_ident(&method.method);
        let receiver = self.flow_of_expr(&method.receiver);
        if receiver.is_empty()
            && self.is_unresolved_opaque_call(&method.receiver)
            && matches!(
                name.as_str(),
                "pool"
                    | "acquire"
                    | "begin"
                    | "prepare"
                    | "query"
                    | "execute"
                    | "fetch"
                    | "fetch_all"
                    | "fetch_many"
                    | "fetch_one"
                    | "fetch_optional"
            )
        {
            self.errors.push(format!(
                "{} invokes persistence-shaped method {name} on a zero-argument opaque return whose concrete flow is not represented",
                self.enclosing
            ));
        }
        let mut bound_parameter = false;
        let validated_flow_passthrough = FLOW_PASSTHROUGH_METHODS.contains(&name.as_str())
            || (name == "bind" && receiver.has_stage(FlowStage::Query));
        let operation = if is_query_name(&name) && !receiver.0.is_empty() {
            Some(PersistenceOperation::Query)
        } else if matches!(name.as_str(), "push" | "push_unseparated" | "separated")
            && receiver.targets().contains(&PersistenceTarget::Sqlx)
        {
            Some(PersistenceOperation::RawSql)
        } else if matches!(
            name.as_str(),
            "push_bind" | "push_bind_unseparated" | "push_bindings" | "push_values" | "push_tuples"
        ) && (receiver.targets().contains(&PersistenceTarget::Sqlx)
            || receiver.has_stage(FlowStage::Query))
        {
            // A bind changes what the builder sends, so the call carries a row
            // — but the value is a parameter, not a statement, and it is kept
            // out of SQL-content classification below.
            bound_parameter = true;
            Some(PersistenceOperation::RawSql)
        } else {
            PersistenceOperation::from_executor_method(&name)
        };
        let mut valid_persistence_method = false;
        if let Some(operation) = operation {
            let valid = match operation {
                PersistenceOperation::Commit | PersistenceOperation::Rollback => {
                    receiver.has_stage(FlowStage::Transaction)
                        || receiver
                            .targets()
                            .contains(&PersistenceTarget::SqlTransaction)
                        || receiver
                            .targets()
                            .contains(&PersistenceTarget::SqlxTransaction)
                        || (matches!(
                            name.as_str(),
                            "commit_transaction" | "commit_transaction_with_outcome_like_cpp"
                        ) && !receiver.0.is_empty())
                }
                PersistenceOperation::Begin => !receiver.pool_targets().is_empty(),
                PersistenceOperation::Query
                | PersistenceOperation::PoolAccess
                | PersistenceOperation::PrepareStatement
                | PersistenceOperation::DirectQuery
                | PersistenceOperation::DirectExecute
                | PersistenceOperation::RawSql
                | PersistenceOperation::TransactionAppend
                | PersistenceOperation::GeneratedIdRead
                | PersistenceOperation::AdvisoryLock
                | PersistenceOperation::NonliteralSql
                | PersistenceOperation::InterpolatedSql => !receiver.0.is_empty(),
                _ => {
                    receiver.has_stage(FlowStage::Query)
                        || receiver.has_stage(FlowStage::Transaction)
                        || !receiver.pool_targets().is_empty()
                }
            };
            if valid {
                valid_persistence_method = true;
                // An executor is handed either a statement or an already built
                // query. `pool.execute(sqlx::query("…"))` is the second, and
                // reading it as raw SQL would report ordinary typed execution
                // as dynamic.
                let executor_consumes_raw_sql = matches!(
                    operation,
                    PersistenceOperation::Execute
                        | PersistenceOperation::Fetch
                        | PersistenceOperation::FetchAll
                        | PersistenceOperation::FetchMany
                        | PersistenceOperation::FetchOne
                        | PersistenceOperation::FetchOptional
                ) && !receiver.has_stage(FlowStage::Query)
                    && method.args.first().is_some_and(|argument| {
                        !self.flow_of_expr(argument).has_stage(FlowStage::Query)
                    });
                let mut targets = receiver.targets();
                for argument in &method.args {
                    targets.extend(self.flow_of_expr(argument).targets());
                }
                for target in targets {
                    let fingerprint = self
                        .fingerprint_with_sql_source(canonical_method(method), method.args.first());
                    self.add(target, operation, &name, &cfg, fingerprint.clone());
                    if executor_consumes_raw_sql
                        || operation == PersistenceOperation::PrepareStatement
                    {
                        self.add(
                            target,
                            PersistenceOperation::RawSql,
                            &name,
                            &cfg,
                            fingerprint.clone(),
                        );
                    }
                    if (matches!(
                        operation,
                        PersistenceOperation::DirectQuery
                            | PersistenceOperation::RawSql
                            // `prepare` receives the statement itself, so its
                            // text belongs to the inventory like any other.
                            | PersistenceOperation::PrepareStatement
                    ) || executor_consumes_raw_sql)
                        && !bound_parameter
                    {
                        if self.statement_takes_advisory_lock(&fingerprint) {
                            self.add(
                                target,
                                PersistenceOperation::AdvisoryLock,
                                &name,
                                &cfg,
                                fingerprint.clone(),
                            );
                        }
                    }
                    if (matches!(
                        operation,
                        PersistenceOperation::DirectQuery
                            | PersistenceOperation::DirectExecute
                            | PersistenceOperation::RawSql
                            // A prepared statement is supplied the same way any
                            // other raw SQL is, so it is classified the same:
                            // dynamic text is recorded, and text the snapshot
                            // cannot see is refused.
                            | PersistenceOperation::PrepareStatement
                    ) || executor_consumes_raw_sql)
                        && !bound_parameter
                    {
                        let Some(argument) = method.args.first() else {
                            continue;
                        };
                        match self.sql_expression_kind(argument) {
                            SqlExpressionKind::Static => {}
                            SqlExpressionKind::Included => self.errors.push(format!(
                                "{} passes include_str! SQL whose content is outside the persistence snapshot; mount and fingerprint the included SQL source explicitly",
                                self.enclosing
                            )),
                            SqlExpressionKind::Environment => self.errors.push(format!(
                                "{} passes env! SQL whose expanded content is outside the persistence snapshot; pin the SQL in reviewed source",
                                self.enclosing
                            )),
                            kind @ (SqlExpressionKind::Nonliteral
                            | SqlExpressionKind::Interpolated) => self.add(
                                target,
                                match kind {
                                    SqlExpressionKind::Interpolated => {
                                        PersistenceOperation::InterpolatedSql
                                    }
                                    SqlExpressionKind::Nonliteral => {
                                        PersistenceOperation::NonliteralSql
                                    }
                                    SqlExpressionKind::Static
                                    | SqlExpressionKind::Included
                                    | SqlExpressionKind::Environment => {
                                        unreachable!()
                                    }
                                },
                                &name,
                                &cfg,
                                normalized_tokens(argument),
                            ),
                        }
                    }
                }
            }
        }
        if !valid_persistence_method && !validated_flow_passthrough {
            if receiver.has_stage(FlowStage::Transaction) {
                self.record_persistence_escape(
                    &receiver,
                    PersistenceOperation::ArgumentEscape,
                    &format!("receiver:{name}"),
                    &cfg,
                    normalized_tokens(&method.receiver),
                );
            } else {
                self.record_pool_escape(
                    &receiver,
                    PersistenceOperation::ArgumentEscape,
                    &format!("receiver:{name}"),
                    &cfg,
                    normalized_tokens(&method.receiver),
                );
            }
        }

        if !valid_persistence_method && !validated_flow_passthrough {
            for argument in &method.args {
                let flow = self.flow_of_expr(argument);
                self.record_pool_escape(
                    &flow,
                    PersistenceOperation::ArgumentEscape,
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
        }
        if matches!(
            name.as_str(),
            "push"
                | "push_back"
                | "push_front"
                | "insert"
                | "get_or_insert"
                | "get_or_insert_with"
                | "extend"
                | "append"
                | "replace"
        ) {
            let mut stored = VariableInfo::default();
            for argument in &method.args {
                stored.union(&self.info_from_expr(argument));
            }
            union_into_assignment_place(self, &method.receiver, &stored);
        }
        let sql_mutation_argument = match name.as_str() {
            "push_str" | "push" | "extend" | "append" => method.args.first(),
            "insert_str" | "insert" | "replace_range" => method.args.get(1),
            "clear" | "truncate" | "remove" | "pop" | "retain" | "drain" | "split_off" => None,
            _ => None,
        };
        if matches!(
            name.as_str(),
            "push_str"
                | "push"
                | "extend"
                | "append"
                | "insert_str"
                | "insert"
                | "replace_range"
                | "clear"
                | "truncate"
                | "remove"
                | "pop"
                | "retain"
                | "drain"
                | "split_off"
        ) {
            let mut sql_sources = BTreeSet::from([normalized_tokens(method)]);
            if let Some(argument) = sql_mutation_argument {
                sql_sources.extend(self.sql_sources(argument));
            }
            let appended = VariableInfo {
                sql_expression: sql_mutation_argument
                    .map(|argument| self.sql_expression_kind(argument))
                    .unwrap_or(SqlExpressionKind::Nonliteral),
                sql_sources,
                ..VariableInfo::default()
            };
            union_into_assignment_place(self, &method.receiver, &appended);
        }
        let (receiver_effect, parameter_effects) = self.method_mutation_effects(method);
        if receiver_effect != VariableInfo::default() {
            union_into_assignment_place(self, &method.receiver, &receiver_effect);
        }
        for (index, effect) in parameter_effects {
            let Some(place) = method.args.get(index).and_then(mutable_storage_place) else {
                continue;
            };
            union_into_assignment_place(self, place, &effect);
        }
        self.visit_expr(&method.receiver);
        for argument in &method.args {
            if !matches!(argument, Expr::Closure(_)) {
                self.visit_expr(argument);
            }
        }
        if CLOSURE_INVOKING_METHODS.contains(&name.as_str())
            || (!valid_persistence_method
                && method
                    .args
                    .iter()
                    .any(|argument| matches!(argument, Expr::Closure(_))))
        {
            let pre_call_scopes = self.scopes.clone();
            let mut callback_argument = self.info_from_expr(&method.receiver);
            for argument in &method.args {
                if !matches!(argument, Expr::Closure(_)) {
                    callback_argument.union(&self.info_from_expr(argument));
                }
            }
            for argument in &method.args {
                let info = self.info_from_expr(argument);
                let mutations = self
                    .closure_mutations_for_args(&info, std::slice::from_ref(&callback_argument));
                for (captured, info) in mutations {
                    self.assign(&captured, info);
                }
            }
            let post_call_scopes = self.scopes.clone();
            self.scopes = pre_call_scopes;
            merge_scope_stacks(&mut self.scopes, &post_call_scopes);
            self.bump_context();
        }
    }

    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        if !self.allows_source_class(&assignment.attrs, "assignment") {
            return;
        }
        self.visit_expr(&assignment.right);
        self.visit_expr(&assignment.left);
        let cfg = item_cfg(&self.cfg, &assignment.attrs);
        let info = self.info_from_expr(&assignment.right);
        if let Some(name) = simple_assignment_name(&assignment.left) {
            self.assign(&name, info.clone());
            self.record_flow(
                &info.flow,
                PersistenceOperation::ValueAlias,
                &name,
                &cfg,
                normalized_tokens(assignment),
            );
        } else if assign_destructured_expr(self, &assignment.left, &info) {
            self.record_flow(
                &info.flow,
                PersistenceOperation::ValueAlias,
                "destructuring_assignment",
                &cfg,
                normalized_tokens(assignment),
            );
        } else {
            if let Expr::Unary(unary) = assignment.left.as_ref()
                && matches!(unary.op, syn::UnOp::Deref(_))
            {
                let pointee = self.info_from_expr(&unary.expr);
                for name in pointee.mutable_pointees {
                    self.assign(&name, info.clone());
                }
                for place in pointee.mutable_places {
                    let mut aggregate = self.lookup(&place.root).cloned().unwrap_or_default();
                    assign_place_projection(&mut aggregate, &place.projections, &info);
                    self.assign(&place.root, aggregate);
                }
            }
            if let Some((root, projections)) = assignment_place(&assignment.left) {
                let mut aggregate = self.lookup(&root).cloned().unwrap_or_default();
                assign_place_projection(&mut aggregate, &projections, &info);
                self.assign(&root, aggregate);
            }
            self.record_pool_escape(
                &info.flow,
                PersistenceOperation::StoreEscape,
                "assignment",
                &cfg,
                normalized_tokens(assignment),
            );
        }
    }

    fn visit_expr_struct(&mut self, structure: &'ast ExprStruct) {
        if !self.allows_source_class(&structure.attrs, "struct expression") {
            return;
        }
        let cfg = item_cfg(&self.cfg, &structure.attrs);
        for target in targets_for_path(&structure.path, self.symbols) {
            self.add(
                target,
                PersistenceOperation::PathReference,
                &last_path_name(&structure.path).unwrap_or_default(),
                &cfg,
                canonical_path(&structure.path),
            );
        }
        for field in &structure.fields {
            self.visit_expr(&field.expr);
            let flow = self.flow_of_expr(&field.expr);
            let symbol = match &field.member {
                Member::Named(ident) => normalized_ident(ident),
                Member::Unnamed(index) => index.index.to_string(),
            };
            self.record_pool_escape(
                &flow,
                PersistenceOperation::StoreEscape,
                &symbol,
                &cfg,
                normalized_tokens(&field.expr),
            );
        }
        if let Some(rest) = &structure.rest {
            self.visit_expr(rest);
            let flow = self.flow_of_expr(rest);
            self.record_pool_escape(
                &flow,
                PersistenceOperation::StoreEscape,
                "rest",
                &cfg,
                normalized_tokens(rest),
            );
        }
    }

    fn visit_expr_return(&mut self, returned: &'ast ExprReturn) {
        if !self.allows_source_class(&returned.attrs, "return expression") {
            return;
        }
        if let Some(expression) = &returned.expr {
            self.visit_expr(expression);
            let info = self.info_from_expr(expression);
            let flow = info.flow.clone();
            let cfg = item_cfg(&self.cfg, &returned.attrs);
            self.record_pool_escape(
                &flow,
                PersistenceOperation::ReturnEscape,
                "pool",
                &cfg,
                normalized_tokens(expression),
            );
            if let Some(result) = self.return_value_collectors.last_mut() {
                result.union(&info);
            }
        }
        self.capture_return_exit();
    }

    fn visit_expr_try(&mut self, expression: &'ast syn::ExprTry) {
        if !self.allows_source_class(&expression.attrs, "try expression") {
            return;
        }
        self.visit_expr(&expression.expr);
        // On `Err`, `?` returns the operand's residual from the function, so
        // persistence carried there leaves by the same door as an explicit
        // `return`.
        //
        // Known over-report: this records the operand's whole flow, so a pool
        // held in the *success* payload is reported as escaping although `?`
        // consumes it. Extracting only the residual needs shape information
        // that is present for some operands and absent for others, and two
        // attempts at it each lost a real escape. Reporting an escape that does
        // not happen is the lesser fault for a ratchet whose purpose is to
        // notice change; missing one is the fault that matters.
        let flow = self.info_from_expr(&expression.expr).flow;
        let cfg = item_cfg(&self.cfg, &expression.attrs);
        self.record_pool_escape(
            &flow,
            PersistenceOperation::ReturnEscape,
            "pool",
            &cfg,
            normalized_tokens(&expression.expr),
        );
        // `?` can return from the surrounding closure/async body immediately
        // after evaluating its operand. Preserve that exit state before later
        // statements on the success path can clear the captured binding.
        self.capture_return_exit();
    }

    fn visit_expr_break(&mut self, expression: &'ast syn::ExprBreak) {
        if !self.allows_source_class(&expression.attrs, "break expression") {
            return;
        }
        if let Some(value) = &expression.expr {
            self.visit_expr(value);
            if let Some(label) = &expression.label {
                let label = normalized_ident(&label.ident);
                let info = self.info_from_expr(value);
                if let Some(collector) = self
                    .block_exit_collectors
                    .iter_mut()
                    .rev()
                    .find(|collector| collector.label == label)
                {
                    collector.result.union(&info);
                }
            }
        }
        self.capture_loop_control(expression.label.as_ref(), true);
    }

    fn visit_expr_continue(&mut self, expression: &'ast syn::ExprContinue) {
        if !self.allows_source_class(&expression.attrs, "continue expression") {
            return;
        }
        self.capture_loop_control(expression.label.as_ref(), false);
    }

    fn visit_expr_block(&mut self, expression: &'ast syn::ExprBlock) {
        if !self.allows_source_class(&expression.attrs, "block expression") {
            return;
        }
        let Some(label) = &expression.label else {
            self.visit_block(&expression.block);
            return;
        };
        self.block_exit_collectors.push(BlockExitCollector {
            label: normalized_ident(&label.name.ident),
            ..BlockExitCollector::default()
        });
        self.visit_block(&expression.block);
        let collector = self
            .block_exit_collectors
            .pop()
            .expect("labeled block collector was installed");
        self.block_result_infos
            .borrow_mut()
            .entry(&expression.block as *const syn::Block as usize)
            .or_default()
            .union(&collector.result);
        if let Some(exits) = collector.exits {
            merge_scope_stacks(&mut self.scopes, &exits);
            self.bump_context();
        }
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if !self.allows_source_class(&expression.attrs, "match expression") {
            return;
        }
        self.visit_expr(&expression.expr);
        let scrutinee = self.info_from_expr(&expression.expr);
        // Arms are mutually exclusive: visiting them against one shared
        // scope lets a later arm overwrite (or clear) an earlier arm's
        // assignment to an outer local. Snapshot the pre-match state and
        // conservatively union every arm's resulting flow instead.
        let pre_match_scopes = self.scopes.clone();
        let mut next_arm_scopes = pre_match_scopes.clone();
        let mut merged: Option<Vec<BTreeMap<String, VariableInfo>>> = None;
        for arm in &expression.arms {
            let arm_entry_scopes = next_arm_scopes.clone();
            self.scopes = next_arm_scopes.clone();
            self.push_scope();
            self.bind_pattern(&arm.pat, &scrutinee);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
                next_arm_scopes = self.scopes.clone();
                next_arm_scopes.pop();
                // The pattern can fail before the guard runs, so a later arm
                // must also represent the path on which the guard never
                // executed and its side effects never happened.
                merge_scope_stacks(&mut next_arm_scopes, &arm_entry_scopes);
            }
            self.visit_expr(&arm.body);
            self.pop_scope();
            let post_arm = self.scopes.clone();
            match &mut merged {
                None => merged = Some(post_arm),
                Some(accumulated) => merge_scope_stacks(accumulated, &post_arm),
            }
        }
        self.scopes = merged.unwrap_or(pre_match_scopes);
        merge_scope_stacks(&mut self.scopes, &next_arm_scopes);
        self.bump_context();
    }

    fn visit_expr_if(&mut self, expression: &'ast syn::ExprIf) {
        if !self.allows_source_class(&expression.attrs, "if expression") {
            return;
        }
        // The then/else branches are mutually exclusive: visiting them
        // sequentially against one shared scope lets the else branch erase
        // flow the then branch assigned to an outer local. Like match arms,
        // snapshot the pre-if state and conservatively union both outcomes,
        // including the no-`else` path.
        self.push_scope();
        visit_let_chain_condition(self, &expression.cond, false);
        let mut post_condition_scopes = self.scopes.clone();
        post_condition_scopes.pop();
        self.visit_block(&expression.then_branch);
        self.pop_scope();
        let post_then = self.scopes.clone();
        self.scopes = post_condition_scopes;
        if let Some((_, else_expression)) = &expression.else_branch {
            self.visit_expr(else_expression);
        }
        let post_else = self.scopes.clone();
        self.scopes = post_then;
        merge_scope_stacks(&mut self.scopes, &post_else);
        self.bump_context();
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        let cfg = item_cfg(&self.cfg, &item.attrs);
        let symbol = item
            .ident
            .as_ref()
            .map(normalized_ident)
            .or_else(|| last_path_name(&item.mac.path))
            .unwrap_or_else(|| "macro".to_owned());
        let mut targets = targets_in_tokens(item.mac.tokens.clone(), self.symbols);
        for scope in &self.scopes {
            for (local, info) in scope {
                if tokens_contain_identifier(
                    item.mac.tokens.clone(),
                    &BTreeSet::from([local.clone()]),
                ) {
                    targets.extend(info.flow.targets());
                }
            }
        }
        if targets.is_empty() {
            return;
        }
        for target in targets {
            self.add_generated(
                target,
                PersistenceOperation::MacroReference,
                &symbol,
                &cfg,
                normalized_tokens(&item.mac),
            );
        }
    }

    fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
        self.audit_macro(&expression.mac, &expression.attrs, "macro expression");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.audit_macro(&statement.mac, &statement.attrs, "statement macro");
    }

    fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
        if !self.allows_source_class(&closure.attrs, "closure") {
            return;
        }
        let previous_cfg = self.cfg.clone();
        let declaration_scopes = self.scopes.clone();
        self.cfg = item_cfg(&self.cfg, &closure.attrs);
        self.push_scope();
        let (_, parameter_infos) = closure_callable_model(closure);
        for (input, info) in closure.inputs.iter().zip(&parameter_infos) {
            self.bind_pattern(input, info);
        }
        self.return_exit_collectors.push(None);
        self.return_value_collectors.push(VariableInfo::default());
        self.visit_expr(&closure.body);
        let mut result_info = self.info_from_expr(&closure.body);
        let returned = self
            .return_value_collectors
            .pop()
            .expect("closure return value collector was installed");
        result_info.union(&returned);
        self.closure_result_infos
            .borrow_mut()
            .insert(closure as *const ExprClosure as usize, result_info);
        if let Some(exits) = self
            .return_exit_collectors
            .pop()
            .expect("closure return collector was installed")
        {
            merge_scope_stacks(&mut self.scopes, &exits);
            self.bump_context();
        }
        self.pop_scope();
        let mut effects = BTreeMap::new();
        for (before_scope, after_scope) in declaration_scopes.iter().zip(&self.scopes).rev() {
            for (name, after) in after_scope {
                if before_scope.get(name).is_some_and(|before| before != after) {
                    effects.entry(name.clone()).or_insert_with(|| after.clone());
                }
            }
        }
        self.closure_effects
            .borrow_mut()
            .insert(closure as *const ExprClosure as usize, effects);
        self.scopes = declaration_scopes;
        self.bump_context();
        self.cfg = previous_cfg;
    }
}
