//! Registry access scan state definitions, part 3 of 4.
//!
//! Separated from the registry_access.rs root under #660. Behaviour is preserved.

use super::*;

impl<'ast> Visit<'ast> for BodyAnalyzer<'_, '_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scopes.push(BTreeMap::new());
        for statement in &block.stmts {
            self.visit_stmt(statement);
        }
        self.scopes.pop();
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if !self.allows_production(&local.attrs, "local binding") {
            return;
        }
        let previous_cfg = self.cfg.clone();
        self.cfg = item_cfg(&self.cfg, &local.attrs);
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
            let flow = self.flow_of_expr(&init.expr);
            self.record_aliases(&local.pat, &flow, RegistryOperation::LocalAlias);
            self.bind_pattern_from_expr(&local.pat, &init.expr);
        } else {
            let info = match &local.pat {
                Pat::Type(typed) => self.info_from_type(&typed.ty),
                _ => VariableInfo::default(),
            };
            self.bind_pattern(&local.pat, &info);
        }
        if let Pat::Type(typed) = &local.pat {
            let visibility = self.visibility.clone();
            let cfg = self.cfg.clone();
            add_type_records(
                self.accumulator,
                &self.context,
                self.symbols,
                &typed.ty,
                &self.enclosing,
                &normalized_tokens(&typed.pat),
                &visibility,
                &cfg,
            );
        }
        self.cfg = previous_cfg;
    }

    fn visit_expr_assign(&mut self, assignment: &'ast syn::ExprAssign) {
        if !self.allows_production(&assignment.attrs, "assignment") {
            return;
        }
        self.visit_expr(&assignment.right);
        self.visit_expr(&assignment.left);
        let Expr::Path(path) = assignment.left.as_ref() else {
            return;
        };
        let Some(name) = last_path_ident(&path.path) else {
            return;
        };
        let info = self.info_from_expr(&assignment.right);
        if info.flow.has_registry() {
            let cfg = item_cfg(&self.cfg, &assignment.attrs);
            for registry in info.flow.registry_kinds() {
                self.add(
                    registry,
                    RegistryOperation::AssignmentAlias,
                    &name,
                    &cfg,
                    normalized_tokens(assignment),
                );
            }
        }
        self.assign(&name, info);
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        if !self.allows_production(&field.attrs, "field expression") {
            return;
        }
        let flow = self.field_flow(field);
        let symbol = match &field.member {
            Member::Named(member) => normalized_ident(member),
            Member::Unnamed(index) => index.index.to_string(),
        };
        let cfg = item_cfg(&self.cfg, &field.attrs);
        for registry in flow.registry_kinds() {
            self.add(
                registry,
                RegistryOperation::Member,
                &symbol,
                &cfg,
                normalized_tokens(field),
            );
        }
        visit::visit_expr_field(self, field);
    }

    fn visit_expr_method_call(&mut self, method: &'ast ExprMethodCall) {
        if !self.allows_production(&method.attrs, "method call") {
            return;
        }
        let name = normalized_ident(&method.method);
        let receiver = self.flow_of_expr(&method.receiver);
        let cfg = item_cfg(&self.cfg, &method.attrs);
        if let Some(kind) = RegistryKind::from_member_or_accessor(&name) {
            self.add(
                kind,
                RegistryOperation::Accessor,
                &name,
                &cfg,
                method_fingerprint(method),
            );
        }
        if let Some(operation) = RegistryOperation::from_method(&name) {
            for registry in receiver.registry_kinds() {
                self.add(registry, operation, &name, &cfg, method_fingerprint(method));
            }
        } else if matches!(name.as_str(), "clone" | "cloned") {
            for registry in receiver.registry_kinds() {
                self.add(
                    registry,
                    RegistryOperation::Clone,
                    &name,
                    &cfg,
                    method_fingerprint(method),
                );
            }
        }

        self.visit_expr(&method.receiver);
        let combinator_input = Flow::from_kinds(&receiver.registry_kinds());
        for argument in &method.args {
            if let Expr::Closure(closure) = argument {
                if matches!(name.as_str(), "map" | "and_then" | "filter" | "inspect")
                    && combinator_input.has_registry()
                {
                    self.visit_closure_with_input(closure, &combinator_input);
                    continue;
                }
            }
            let argument_flow = self.flow_of_expr(argument);
            for registry in argument_flow.registry_kinds() {
                self.add(
                    registry,
                    RegistryOperation::ArgumentEscape,
                    &name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
            self.visit_expr(argument);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if !self.allows_production(&call.attrs, "function call") {
            return;
        }
        let function_name = match call.func.as_ref() {
            Expr::Path(path) => last_path_ident(&path.path).unwrap_or_else(|| "<call>".to_owned()),
            _ => "<call>".to_owned(),
        };
        let clone_call = matches!(function_name.as_str(), "clone" | "cloned");
        let constructor_kinds = match call.func.as_ref() {
            Expr::Path(path)
                if matches!(
                    function_name.as_str(),
                    "new" | "default" | "with_capacity" | "from_iter"
                ) =>
            {
                path.path
                    .segments
                    .iter()
                    .filter_map(|segment| {
                        self.symbols
                            .type_aliases
                            .get(&normalized_ident(&segment.ident))
                    })
                    .flatten()
                    .copied()
                    .collect()
            }
            _ => KindSet::new(),
        };
        let cfg = item_cfg(&self.cfg, &call.attrs);
        for registry in constructor_kinds {
            self.add(
                registry,
                RegistryOperation::Construct,
                &function_name,
                &cfg,
                normalized_tokens(call),
            );
        }
        for argument in &call.args {
            let flow = self.flow_of_expr(argument);
            for registry in flow.registry_kinds() {
                self.add(
                    registry,
                    if clone_call {
                        RegistryOperation::Clone
                    } else {
                        RegistryOperation::ArgumentEscape
                    },
                    &function_name,
                    &cfg,
                    normalized_tokens(argument),
                );
            }
            self.visit_expr(argument);
        }
        self.visit_expr(&call.func);
    }

    fn visit_expr_index(&mut self, index: &'ast syn::ExprIndex) {
        if !self.allows_production(&index.attrs, "index expression") {
            return;
        }
        let receiver = self.flow_of_expr(&index.expr);
        let cfg = item_cfg(&self.cfg, &index.attrs);
        for registry in receiver.registry_kinds() {
            self.add(
                registry,
                RegistryOperation::Index,
                "index",
                &cfg,
                normalized_tokens(index),
            );
        }
        visit::visit_expr_index(self, index);
    }

    fn visit_expr_return(&mut self, returned: &'ast ExprReturn) {
        if !self.allows_production(&returned.attrs, "return expression") {
            return;
        }
        if let Some(expression) = &returned.expr {
            self.visit_expr(expression);
            let flow = self.flow_of_expr(expression);
            let cfg = item_cfg(&self.cfg, &returned.attrs);
            self.record_return_flow(&flow, normalized_tokens(expression), &cfg);
        }
    }

    fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
        self.audit_macro(&expression.mac, &expression.attrs, "macro expression");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.audit_macro(&statement.mac, &statement.attrs, "statement macro");
    }

    fn visit_expr_if(&mut self, if_expression: &'ast ExprIf) {
        if !self.allows_production(&if_expression.attrs, "if expression") {
            return;
        }
        self.visit_expr(&if_expression.cond);
        self.scopes.push(BTreeMap::new());
        if let Expr::Let(let_expression) = if_expression.cond.as_ref() {
            self.bind_pattern_from_expr(&let_expression.pat, &let_expression.expr);
        }
        self.visit_block(&if_expression.then_branch);
        self.scopes.pop();
        if let Some((_, else_expression)) = &if_expression.else_branch {
            self.visit_expr(else_expression);
        }
    }

    fn visit_expr_match(&mut self, match_expression: &'ast ExprMatch) {
        if !self.allows_production(&match_expression.attrs, "match expression") {
            return;
        }
        self.visit_expr(&match_expression.expr);
        for arm in &match_expression.arms {
            if !self.allows_production(&arm.attrs, "match arm") {
                continue;
            }
            self.scopes.push(BTreeMap::new());
            self.bind_pattern_from_expr(&arm.pat, &match_expression.expr);
            if let Some((_, guard)) = &arm.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&arm.body);
            self.scopes.pop();
        }
    }

    fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
        if !self.allows_production(&closure.attrs, "closure") {
            return;
        }
        self.scopes.push(BTreeMap::new());
        for input in &closure.inputs {
            let info = match input {
                Pat::Type(typed) => self.info_from_type(&typed.ty),
                _ => VariableInfo::default(),
            };
            self.bind_pattern(input, &info);
        }
        self.visit_expr(&closure.body);
        self.scopes.pop();
    }
}

pub(super) fn impl_self_name(item_impl: &ItemImpl) -> String {
    match item_impl.self_ty.as_ref() {
        Type::Path(path) => last_path_ident(&path.path).unwrap_or_else(|| "<impl>".to_owned()),
        ty => normalized_tokens(ty),
    }
}

pub(super) fn register_function_parameters(
    analyzer: &mut BodyAnalyzer<'_, '_>,
    inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    cfg: &[String],
) {
    for argument in inputs {
        let FnArg::Typed(typed) = argument else {
            continue;
        };
        if !analyzer.allows_production(&typed.attrs, "function parameter") {
            continue;
        }
        let info = analyzer.info_from_type(&typed.ty);
        analyzer.bind_pattern(&typed.pat, &info);
        add_type_records(
            analyzer.accumulator,
            &analyzer.context,
            analyzer.symbols,
            &typed.ty,
            &analyzer.enclosing,
            &normalized_tokens(&typed.pat),
            &analyzer.visibility,
            &item_cfg(cfg, &typed.attrs),
        );
    }
}

pub(super) fn analyze_function(
    function: &ItemFn,
    context: RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enclosing = format!("fn {}", normalized_ident(&function.sig.ident));
    let visibility = normalized_visibility(&function.vis);
    if let ReturnType::Type(_, ty) = &function.sig.output {
        add_type_records(
            accumulator,
            &context,
            symbols,
            ty,
            &enclosing,
            "return",
            &visibility,
            &cfg,
        );
    }
    let mut analyzer = BodyAnalyzer::new(
        context,
        accumulator,
        errors,
        symbols,
        enclosing,
        visibility,
        cfg.clone(),
    );
    register_function_parameters(&mut analyzer, &function.sig.inputs, &cfg);
    for statement in &function.block.stmts {
        analyzer.visit_stmt(statement);
    }
    let tail = implicit_tail_flow(&function.block, &analyzer);
    if !tail.0.is_empty() {
        let fingerprint = function
            .block
            .stmts
            .last()
            .map(normalized_tokens)
            .unwrap_or_default();
        analyzer.record_return_flow(&tail, fingerprint, &cfg);
    }
}

pub(super) fn analyze_impl(
    item_impl: &ItemImpl,
    context: RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let self_name = impl_self_name(item_impl);
    for item in &item_impl.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };
        if !production(&cfg, &method.attrs, errors, "impl method") {
            continue;
        }
        let method_cfg = item_cfg(&cfg, &method.attrs);
        let enclosing = format!("impl {self_name}::{}", normalized_ident(&method.sig.ident));
        let visibility = normalized_visibility(&method.vis);
        if let ReturnType::Type(_, ty) = &method.sig.output {
            add_type_records(
                accumulator,
                &context,
                symbols,
                ty,
                &enclosing,
                "return",
                &visibility,
                &method_cfg,
            );
        }
        let mut analyzer = BodyAnalyzer::new(
            RecordContext {
                package: context.package,
                module: context.module,
                source: context.source,
            },
            accumulator,
            errors,
            symbols,
            enclosing,
            visibility,
            method_cfg.clone(),
        );
        register_function_parameters(&mut analyzer, &method.sig.inputs, &method_cfg);
        for statement in &method.block.stmts {
            analyzer.visit_stmt(statement);
        }
        let tail = implicit_tail_flow(&method.block, &analyzer);
        if !tail.0.is_empty() {
            let fingerprint = method
                .block
                .stmts
                .last()
                .map(normalized_tokens)
                .unwrap_or_default();
            analyzer.record_return_flow(&tail, fingerprint, &method_cfg);
        }
    }
}

pub(super) fn analyze_struct(
    item_struct: &ItemStruct,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enclosing = format!("struct {}", normalized_ident(&item_struct.ident));
    for (index, field) in item_struct.fields.iter().enumerate() {
        if !production(cfg, &field.attrs, errors, "struct field") {
            continue;
        }
        let name = field
            .ident
            .as_ref()
            .map(normalized_ident)
            .unwrap_or_else(|| index.to_string());
        add_type_records(
            accumulator,
            context,
            symbols,
            &field.ty,
            &enclosing,
            &name,
            &normalized_visibility(&field.vis),
            &item_cfg(cfg, &field.attrs),
        );
    }
}

pub(super) fn analyze_enum(
    item_enum: &ItemEnum,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let enum_name = normalized_ident(&item_enum.ident);
    for variant in &item_enum.variants {
        if !production(cfg, &variant.attrs, errors, "enum variant") {
            continue;
        }
        let variant_cfg = item_cfg(cfg, &variant.attrs);
        let variant_name = normalized_ident(&variant.ident);
        for (index, field) in variant.fields.iter().enumerate() {
            if !production(&variant_cfg, &field.attrs, errors, "enum field") {
                continue;
            }
            let field_name = field
                .ident
                .as_ref()
                .map(normalized_ident)
                .unwrap_or_else(|| index.to_string());
            add_type_records(
                accumulator,
                context,
                symbols,
                &field.ty,
                &format!("enum {enum_name}::{variant_name}"),
                &field_name,
                &normalized_visibility(&item_enum.vis),
                &item_cfg(&variant_cfg, &field.attrs),
            );
        }
    }
}

pub(super) fn analyze_type_alias(
    alias: &ItemType,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
) {
    let name = normalized_ident(&alias.ident);
    let mut kinds = registry_kinds_in_type(&alias.ty, symbols);
    if let Some(resolved) = symbols.type_aliases.get(&name) {
        kinds.extend(resolved.iter().copied());
    }
    if let Some(canonical) = RegistryKind::from_source_name(&name) {
        kinds.insert(canonical);
    }
    for registry in kinds {
        accumulator.add(
            context,
            NewAccess {
                enclosing: "module",
                registry,
                operation: RegistryOperation::TypeAlias,
                symbol: &name,
                visibility: &normalized_visibility(&alias.vis),
                cfg,
                fingerprint: normalized_tokens(&alias.ty),
            },
        );
    }
}

pub(super) fn analyze_import(
    item_use: &ItemUse,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    cfg: &[String],
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let mut bindings = Vec::new();
    collect_use_bindings(
        &item_use.tree,
        &mut Vec::new(),
        symbols,
        &mut bindings,
        errors,
    );
    for binding in bindings {
        accumulator.add(
            context,
            NewAccess {
                enclosing: "module",
                registry: binding.registry,
                operation: RegistryOperation::ImportAlias,
                symbol: &binding.local_name,
                visibility: &normalized_visibility(&item_use.vis),
                cfg,
                fingerprint: canonical_use_tree(&item_use.tree),
            },
        );
    }
}

pub(super) fn macro_definition_or_invocation_error(
    item: &syn::ItemMacro,
    symbols: &ModuleSymbols,
    module: &str,
) -> Option<String> {
    let names: BTreeSet<_> = symbols.type_aliases.keys().cloned().collect();
    if !tokens_contain_identifier(item.mac.tokens.clone(), &names) {
        return None;
    }
    Some(format!(
        "module {module} hides registry provenance inside item macro {}!; expose ordinary Rust syntax before baselining it",
        macro_name(&item.mac.path)
    ))
}

pub(super) fn analyze_module_items(
    items: &[Item],
    context: RecordContext<'_>,
    parent_symbols: Option<&ModuleSymbols>,
    global_aliases: &GlobalAliasIndex,
    cfg: Vec<String>,
    accumulator: &mut AccessAccumulator,
    errors: &mut Vec<String>,
) {
    let symbols = collect_module_symbols(
        items,
        parent_symbols,
        global_aliases,
        context.package,
        context.module,
        &cfg,
        errors,
    );
    for item in items {
        match item {
            Item::Use(item_use) => {
                if !production(&cfg, &item_use.attrs, errors, "use item") {
                    continue;
                }
                analyze_import(
                    item_use,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_use.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Type(alias) => {
                if !production(&cfg, &alias.attrs, errors, "type alias") {
                    continue;
                }
                analyze_type_alias(
                    alias,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &alias.attrs),
                    accumulator,
                );
            }
            Item::Struct(item_struct) => {
                if !production(&cfg, &item_struct.attrs, errors, "struct") {
                    continue;
                }
                analyze_struct(
                    item_struct,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_struct.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Enum(item_enum) => {
                if !production(&cfg, &item_enum.attrs, errors, "enum") {
                    continue;
                }
                analyze_enum(
                    item_enum,
                    &context,
                    &symbols,
                    &item_cfg(&cfg, &item_enum.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Fn(function) => {
                if !production(&cfg, &function.attrs, errors, "function") {
                    continue;
                }
                analyze_function(
                    function,
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    &symbols,
                    item_cfg(&cfg, &function.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Impl(item_impl) => {
                if !production(&cfg, &item_impl.attrs, errors, "impl") {
                    continue;
                }
                analyze_impl(
                    item_impl,
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    &symbols,
                    item_cfg(&cfg, &item_impl.attrs),
                    accumulator,
                    errors,
                );
            }
            Item::Const(item_const) => {
                if !production(&cfg, &item_const.attrs, errors, "const") {
                    continue;
                }
                let enclosing = format!("const {}", normalized_ident(&item_const.ident));
                let item_cfg = item_cfg(&cfg, &item_const.attrs);
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_const.ty,
                    &enclosing,
                    &normalized_ident(&item_const.ident),
                    "",
                    &item_cfg,
                );
                let mut analyzer = BodyAnalyzer::new(
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    accumulator,
                    errors,
                    &symbols,
                    enclosing,
                    String::new(),
                    item_cfg,
                );
                analyzer.visit_expr(&item_const.expr);
            }
            Item::Static(item_static) => {
                if !production(&cfg, &item_static.attrs, errors, "static") {
                    continue;
                }
                let enclosing = format!("static {}", normalized_ident(&item_static.ident));
                let item_cfg = item_cfg(&cfg, &item_static.attrs);
                add_type_records(
                    accumulator,
                    &context,
                    &symbols,
                    &item_static.ty,
                    &enclosing,
                    &normalized_ident(&item_static.ident),
                    &normalized_visibility(&item_static.vis),
                    &item_cfg,
                );
                let mut analyzer = BodyAnalyzer::new(
                    RecordContext {
                        package: context.package,
                        module: context.module,
                        source: context.source,
                    },
                    accumulator,
                    errors,
                    &symbols,
                    enclosing,
                    normalized_visibility(&item_static.vis),
                    item_cfg,
                );
                analyzer.visit_expr(&item_static.expr);
            }
            Item::Macro(item_macro) => {
                if !production(&cfg, &item_macro.attrs, errors, "item macro") {
                    continue;
                }
                if let Some(error) =
                    macro_definition_or_invocation_error(item_macro, &symbols, context.module)
                {
                    errors.push(error);
                }
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) => {
                if !production(&cfg, attrs, errors, "inline module") {
                    continue;
                }
                let child_module = format!("{}::{}", context.module, normalized_ident(ident));
                analyze_module_items(
                    child_items,
                    RecordContext {
                        package: context.package,
                        module: &child_module,
                        source: context.source,
                    },
                    Some(&symbols),
                    global_aliases,
                    item_cfg(&cfg, attrs),
                    accumulator,
                    errors,
                );
            }
            _ => {}
        }
    }
}
