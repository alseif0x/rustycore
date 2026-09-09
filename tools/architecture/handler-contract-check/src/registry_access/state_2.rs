//! Registry access scan state definitions, part 2 of 4.
//!
//! Separated from the registry_access.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) fn collect_use_bindings(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    symbols: &ModuleSymbols,
    bindings: &mut Vec<ImportBinding>,
    errors: &mut Vec<String>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            collect_use_bindings(&path.tree, prefix, symbols, bindings, errors);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let source_name = normalized_ident(&name.ident);
            let local_name = if source_name == "self" {
                prefix
                    .last()
                    .cloned()
                    .unwrap_or_else(|| source_name.clone())
            } else {
                source_name.clone()
            };
            let kinds = RegistryKind::from_source_name(&source_name)
                .map(|kind| BTreeSet::from([kind]))
                .or_else(|| symbols.type_aliases.get(&source_name).cloned())
                .or_else(|| symbols.type_aliases.get(&local_name).cloned())
                .unwrap_or_default();
            for registry in kinds {
                bindings.push(ImportBinding {
                    local_name: local_name.clone(),
                    registry,
                });
            }
        }
        UseTree::Rename(rename) => {
            let source_name = normalized_ident(&rename.ident);
            let local_name = normalized_ident(&rename.rename);
            let kinds = RegistryKind::from_source_name(&source_name)
                .map(|kind| BTreeSet::from([kind]))
                .or_else(|| symbols.type_aliases.get(&source_name).cloned())
                .or_else(|| symbols.type_aliases.get(&local_name).cloned())
                .unwrap_or_default();
            for registry in kinds {
                bindings.push(ImportBinding {
                    local_name: local_name.clone(),
                    registry,
                });
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_bindings(item, prefix, symbols, bindings, errors);
            }
        }
        UseTree::Glob(_) => {
            // `directory` is the relocated player-directory owner module from
            // issue #138 and `wow_social`/`group` the relocated Group owner from
            // issue #137; `player_registry` remains the `wow-network` mailbox.
            let hides_registry = prefix.iter().any(|segment| {
                matches!(
                    segment.as_str(),
                    "wow_network"
                        | "wow_world"
                        | "wow_social"
                        | "player_registry"
                        | "group_registry"
                        | "group"
                        | "directory"
                        | "mailbox"
                )
            });
            if hides_registry {
                errors.push(format!(
                    "glob import {}::* can hide a registry alias; import each registry explicitly",
                    prefix.join("::")
                ));
            }
        }
    }
}

pub(super) fn item_cfg(parent: &[String], attributes: &[Attribute]) -> Vec<String> {
    extend_cfg_context(parent, attributes)
}

pub(super) fn production(
    parent: &[String],
    attributes: &[Attribute],
    errors: &mut Vec<String>,
    owner: &str,
) -> bool {
    match cfg_context_allows_production(parent, attributes) {
        Ok(production) => production,
        Err(error) => {
            errors.push(format!("invalid cfg on {owner}: {error}"));
            false
        }
    }
}

pub(super) fn pat_identifiers(pattern: &Pat, output: &mut Vec<String>) {
    match pattern {
        Pat::Ident(ident) => {
            output.push(normalized_ident(&ident.ident));
            if let Some((_, subpat)) = &ident.subpat {
                pat_identifiers(subpat, output);
            }
        }
        Pat::Reference(reference) => pat_identifiers(&reference.pat, output),
        Pat::Type(typed) => pat_identifiers(&typed.pat, output),
        Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                pat_identifiers(element, output);
            }
        }
        Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                pat_identifiers(element, output);
            }
        }
        Pat::Struct(structure) => {
            for field in &structure.fields {
                pat_identifiers(&field.pat, output);
            }
        }
        Pat::Slice(slice) => {
            for element in &slice.elems {
                pat_identifiers(element, output);
            }
        }
        Pat::Or(or) => {
            for case in &or.cases {
                pat_identifiers(case, output);
            }
        }
        Pat::Paren(paren) => pat_identifiers(&paren.pat, output),
        _ => {}
    }
}

pub(super) fn tokens_contain_identifier(tokens: TokenStream, names: &BTreeSet<String>) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => names.contains(&normalized_ident(&ident)),
        TokenTree::Group(group) => tokens_contain_identifier(group.stream(), names),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

pub(super) fn macro_name(path: &syn::Path) -> String {
    last_path_ident(path).unwrap_or_else(|| normalized_tokens(path))
}

pub(super) fn is_known_opaque_value_macro(name: &str) -> bool {
    KNOWN_OPAQUE_VALUE_MACROS.contains(&name)
}

pub(super) fn add_type_records(
    accumulator: &mut AccessAccumulator,
    context: &RecordContext<'_>,
    symbols: &ModuleSymbols,
    ty: &Type,
    enclosing: &str,
    symbol: &str,
    visibility: &str,
    cfg: &[String],
) {
    for registry in registry_kinds_in_type(ty, symbols) {
        accumulator.add(
            context,
            NewAccess {
                enclosing,
                registry,
                operation: RegistryOperation::TypeReference,
                symbol,
                visibility,
                cfg,
                fingerprint: normalized_tokens(ty),
            },
        );
    }
}

pub(super) fn collect_module_symbols(
    items: &[Item],
    parent: Option<&ModuleSymbols>,
    global_aliases: &GlobalAliasIndex,
    package: &str,
    module: &str,
    cfg: &[String],
    errors: &mut Vec<String>,
) -> ModuleSymbols {
    let mut symbols = parent.cloned().unwrap_or_default();
    global_aliases.extend_module_symbols(package, module, &mut symbols);

    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if !production(cfg, &item_use.attrs, errors, "use item") {
            continue;
        }
        let mut bindings = Vec::new();
        collect_use_bindings(
            &item_use.tree,
            &mut Vec::new(),
            &symbols,
            &mut bindings,
            errors,
        );
        for binding in bindings {
            symbols
                .type_aliases
                .entry(binding.local_name)
                .or_default()
                .insert(binding.registry);
        }
    }

    // Resolve explicit type aliases to a fixed point. This covers chains such
    // as `type Players = PlayerRegistry; type Shared = Arc<Players>`.
    let mut aliases = Vec::new();
    for item in items {
        if let Item::Type(alias) = item
            && production(cfg, &alias.attrs, errors, "type alias")
        {
            aliases.push(alias);
        }
    }
    for _ in 0..=aliases.len() {
        let mut changed = false;
        for alias in &aliases {
            let name = normalized_ident(&alias.ident);
            let mut kinds = registry_kinds_in_type(&alias.ty, &symbols);
            if let Some(canonical) = RegistryKind::from_source_name(&name) {
                kinds.insert(canonical);
            }
            if !kinds.is_empty() {
                let entry = symbols.type_aliases.entry(name).or_default();
                let before = entry.len();
                entry.extend(kinds);
                changed |= entry.len() != before;
            }
        }
        if !changed {
            break;
        }
    }

    for item in items {
        match item {
            Item::Struct(item_struct) if production(cfg, &item_struct.attrs, errors, "struct") => {
                symbols
                    .struct_names
                    .insert(normalized_ident(&item_struct.ident));
            }
            Item::Enum(item_enum) if production(cfg, &item_enum.attrs, errors, "enum") => {
                symbols
                    .struct_names
                    .insert(normalized_ident(&item_enum.ident));
            }
            _ => {}
        }
    }

    for item in items {
        match item {
            Item::Struct(item_struct) if production(cfg, &item_struct.attrs, errors, "struct") => {
                let struct_cfg = item_cfg(cfg, &item_struct.attrs);
                for field in &item_struct.fields {
                    if !production(&struct_cfg, &field.attrs, errors, "struct field") {
                        continue;
                    }
                    let kinds = registry_kinds_in_type(&field.ty, &symbols);
                    if kinds.is_empty() {
                        continue;
                    }
                    let name = field
                        .ident
                        .as_ref()
                        .map(normalized_ident)
                        .unwrap_or_else(|| "<tuple-field>".to_owned());
                    symbols.field_kinds.entry(name).or_default().extend(kinds);
                }
            }
            Item::Enum(item_enum) if production(cfg, &item_enum.attrs, errors, "enum") => {
                let enum_cfg = item_cfg(cfg, &item_enum.attrs);
                for variant in &item_enum.variants {
                    if !production(&enum_cfg, &variant.attrs, errors, "enum variant") {
                        continue;
                    }
                    let variant_cfg = item_cfg(&enum_cfg, &variant.attrs);
                    for field in &variant.fields {
                        if !production(&variant_cfg, &field.attrs, errors, "enum field") {
                            continue;
                        }
                        let kinds = registry_kinds_in_type(&field.ty, &symbols);
                        if kinds.is_empty() {
                            continue;
                        }
                        let name = field
                            .ident
                            .as_ref()
                            .map(normalized_ident)
                            .unwrap_or_else(|| "<tuple-field>".to_owned());
                        symbols.field_kinds.entry(name).or_default().extend(kinds);
                    }
                }
            }
            _ => {}
        }
    }

    for item in items {
        match item {
            Item::Fn(function) if production(cfg, &function.attrs, errors, "function") => {
                if let ReturnType::Type(_, ty) = &function.sig.output {
                    let kinds = registry_kinds_in_type(ty, &symbols);
                    if !kinds.is_empty() {
                        symbols.function_returns.insert(
                            normalized_ident(&function.sig.ident),
                            Flow::from_kinds(&kinds),
                        );
                    }
                }
            }
            Item::Impl(item_impl) if production(cfg, &item_impl.attrs, errors, "impl") => {
                let impl_cfg = item_cfg(cfg, &item_impl.attrs);
                for impl_item in &item_impl.items {
                    let ImplItem::Fn(method) = impl_item else {
                        continue;
                    };
                    if !production(&impl_cfg, &method.attrs, errors, "impl method") {
                        continue;
                    }
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let kinds = registry_kinds_in_type(ty, &symbols);
                        if !kinds.is_empty() {
                            symbols.function_returns.insert(
                                normalized_ident(&method.sig.ident),
                                Flow::from_kinds(&kinds),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    symbols
}

pub(super) struct BodyAnalyzer<'a, 'b> {
    pub(super) context: RecordContext<'a>,
    pub(super) accumulator: &'b mut AccessAccumulator,
    pub(super) errors: &'b mut Vec<String>,
    pub(super) symbols: &'b ModuleSymbols,
    pub(super) enclosing: String,
    pub(super) visibility: String,
    pub(super) cfg: Vec<String>,
    pub(super) scopes: Vec<BTreeMap<String, VariableInfo>>,
}

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
        }
    }

    pub(super) fn add(
        &mut self,
        registry: RegistryKind,
        operation: RegistryOperation,
        symbol: &str,
        cfg: &[String],
        fingerprint: String,
    ) {
        self.accumulator.add(
            &self.context,
            NewAccess {
                enclosing: &self.enclosing,
                registry,
                operation,
                symbol,
                visibility: &self.visibility,
                cfg,
                fingerprint,
            },
        );
    }

    pub(super) fn allows_production(&mut self, attributes: &[Attribute], owner: &str) -> bool {
        production(&self.cfg, attributes, self.errors, owner)
    }

    pub(super) fn audit_macro(&mut self, mac: &syn::Macro, attributes: &[Attribute], owner: &str) {
        if !self.allows_production(attributes, owner) {
            return;
        }
        let names = self.known_registry_token_names();
        if !tokens_contain_identifier(mac.tokens.clone(), &names) {
            return;
        }
        let name = macro_name(&mac.path);
        if !is_known_opaque_value_macro(&name) {
            self.errors.push(format!(
                "{} passes a registry alias/value through unknown macro {name}!; expose ordinary Rust syntax before baselining it",
                self.enclosing
            ));
            return;
        }
        let cfg = item_cfg(&self.cfg, attributes);
        let mut kinds = KindSet::new();
        for token_name in names {
            if tokens_contain_identifier(mac.tokens.clone(), &BTreeSet::from([token_name.clone()]))
            {
                if let Some(info) = self.lookup(&token_name) {
                    kinds.extend(info.flow.all_kinds());
                }
                if let Some(alias_kinds) = self.symbols.type_aliases.get(&token_name) {
                    kinds.extend(alias_kinds);
                }
            }
        }
        for registry in kinds {
            self.add(
                registry,
                RegistryOperation::OpaqueMacroBoundary,
                &name,
                &cfg,
                normalized_tokens(mac),
            );
        }
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&VariableInfo> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub(super) fn bind(&mut self, name: String, info: VariableInfo) {
        self.scopes
            .last_mut()
            .expect("body analyzer always has a scope")
            .insert(name, info);
    }

    pub(super) fn assign(&mut self, name: &str, info: VariableInfo) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(existing) = scope.get_mut(name) {
                *existing = info;
                return;
            }
        }
        self.bind(name.to_owned(), info);
    }

    pub(super) fn info_from_type(&self, ty: &Type) -> VariableInfo {
        VariableInfo {
            flow: Flow::from_kinds(&registry_kinds_in_type(ty, self.symbols)),
            struct_types: struct_names_in_type(ty, self.symbols),
        }
    }

    pub(super) fn info_from_expr(&self, expression: &Expr) -> VariableInfo {
        VariableInfo {
            flow: self.flow_of_expr(expression),
            struct_types: BTreeSet::new(),
        }
    }

    pub(super) fn field_flow(&self, field: &ExprField) -> Flow {
        let Member::Named(member) = &field.member else {
            return Flow::default();
        };
        let name = normalized_ident(member);
        if let Some(kind) = RegistryKind::from_member_or_accessor(&name) {
            return Flow::registry(kind);
        }
        if let Some(kinds) = self.symbols.field_kinds.get(&name) {
            return Flow::from_kinds(kinds);
        }
        Flow::default()
    }

    pub(super) fn flow_of_expr(&self, expression: &Expr) -> Flow {
        match expression {
            Expr::Path(path) => {
                let Some(name) = last_path_ident(&path.path) else {
                    return Flow::default();
                };
                self.lookup(&name)
                    .map(|info| info.flow.clone())
                    .or_else(|| self.symbols.type_aliases.get(&name).map(Flow::from_kinds))
                    .unwrap_or_default()
            }
            Expr::Field(field) => self.field_flow(field),
            Expr::Reference(reference) => self.flow_of_expr(&reference.expr),
            Expr::Paren(paren) => self.flow_of_expr(&paren.expr),
            Expr::Group(group) => self.flow_of_expr(&group.expr),
            Expr::Try(try_expression) => self.flow_of_expr(&try_expression.expr),
            Expr::Await(await_expression) => self.flow_of_expr(&await_expression.base),
            Expr::Cast(cast) => self.flow_of_expr(&cast.expr),
            Expr::Unary(unary) => self.flow_of_expr(&unary.expr),
            Expr::Tuple(tuple) => {
                let mut flow = Flow::default();
                for element in &tuple.elems {
                    flow.union(self.flow_of_expr(element));
                }
                flow
            }
            Expr::Array(array) => {
                let mut flow = Flow::default();
                for element in &array.elems {
                    flow.union(self.flow_of_expr(element));
                }
                flow
            }
            Expr::If(if_expression) => {
                let mut flow = implicit_tail_flow(&if_expression.then_branch, self);
                if let Some((_, else_expression)) = &if_expression.else_branch {
                    flow.union(self.flow_of_expr(else_expression));
                }
                flow
            }
            Expr::Match(match_expression) => {
                let mut flow = Flow::default();
                for arm in &match_expression.arms {
                    flow.union(self.flow_of_expr(&arm.body));
                }
                flow
            }
            Expr::Block(block) => implicit_tail_flow(&block.block, self),
            Expr::Call(call) => self.flow_of_call(call),
            Expr::MethodCall(method) => self.flow_of_method(method),
            Expr::Index(index) => self
                .flow_of_expr(&index.expr)
                .map_registry_stage(FlowStage::Derived),
            _ => Flow::default(),
        }
    }

    pub(super) fn flow_of_call(&self, call: &ExprCall) -> Flow {
        let Expr::Path(function_path) = call.func.as_ref() else {
            return Flow::default();
        };
        let segments = path_segments(&function_path.path);
        let last = segments.last().map(String::as_str).unwrap_or_default();
        if matches!(last, "clone" | "cloned") {
            return call
                .args
                .first()
                .map(|argument| self.flow_of_expr(argument))
                .unwrap_or_default();
        }
        if matches!(last, "Some" | "Ok" | "Box" | "new") {
            let mut flow = Flow::default();
            for argument in &call.args {
                flow.union(self.flow_of_expr(argument));
            }
            if !flow.0.is_empty() {
                return flow;
            }
        }
        for segment in &segments {
            if let Some(kinds) = self.symbols.type_aliases.get(segment) {
                if matches!(last, "new" | "default" | "with_capacity" | "from_iter") {
                    return Flow::from_kinds(kinds);
                }
            }
        }
        self.symbols
            .function_returns
            .get(last)
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn flow_of_method(&self, method: &ExprMethodCall) -> Flow {
        let name = normalized_ident(&method.method);
        if let Some(kind) = RegistryKind::from_member_or_accessor(&name) {
            return Flow::registry(kind);
        }
        let receiver = self.flow_of_expr(&method.receiver);
        match name.as_str() {
            "get" | "get_mut" => receiver.map_registry_stage(FlowStage::Guard),
            "iter" => receiver.map_registry_stage(FlowStage::Iterator),
            "entry" => receiver.map_registry_stage(FlowStage::Entry),
            "insert" | "remove" => receiver.map_registry_stage(FlowStage::Derived),
            "retain" | "clear" => Flow::default(),
            "clone" | "cloned" => Flow::from_kinds(&receiver.registry_kinds()),
            method if PASSTHROUGH_METHODS.contains(&method) => {
                Flow::from_kinds(&receiver.registry_kinds())
            }
            _ => Flow::default(),
        }
    }

    pub(super) fn bind_pattern(&mut self, pattern: &Pat, info: &VariableInfo) {
        match pattern {
            Pat::Ident(ident) => {
                self.bind(normalized_ident(&ident.ident), info.clone());
                if let Some((_, subpat)) = &ident.subpat {
                    self.bind_pattern(subpat, info);
                }
            }
            Pat::Reference(reference) => self.bind_pattern(&reference.pat, info),
            Pat::Type(typed) => {
                let mut typed_info = self.info_from_type(&typed.ty);
                typed_info.flow.union(info.flow.clone());
                typed_info.struct_types.extend(info.struct_types.clone());
                self.bind_pattern(&typed.pat, &typed_info);
            }
            Pat::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::TupleStruct(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::Struct(structure) => {
                for field in &structure.fields {
                    self.bind_pattern(&field.pat, info);
                }
            }
            Pat::Slice(slice) => {
                for element in &slice.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::Or(or) => {
                for case in &or.cases {
                    self.bind_pattern(case, info);
                }
            }
            Pat::Paren(paren) => self.bind_pattern(&paren.pat, info),
            _ => {}
        }
    }

    pub(super) fn bind_pattern_from_expr(&mut self, pattern: &Pat, expression: &Expr) {
        match (pattern, expression) {
            (Pat::Tuple(pattern), Expr::Tuple(expression))
                if pattern.elems.len() == expression.elems.len() =>
            {
                for (pattern, expression) in pattern.elems.iter().zip(&expression.elems) {
                    self.bind_pattern_from_expr(pattern, expression);
                }
            }
            (Pat::TupleStruct(pattern), _) if pattern.elems.len() == 1 => {
                self.bind_pattern_from_expr(
                    pattern.elems.first().expect("one tuple-struct element"),
                    expression,
                );
            }
            (Pat::Reference(pattern), _) => self.bind_pattern_from_expr(&pattern.pat, expression),
            (Pat::Paren(pattern), _) => self.bind_pattern_from_expr(&pattern.pat, expression),
            _ => {
                let info = self.info_from_expr(expression);
                self.bind_pattern(pattern, &info);
            }
        }
    }

    pub(super) fn record_aliases(
        &mut self,
        pattern: &Pat,
        flow: &Flow,
        operation: RegistryOperation,
    ) {
        if !flow.has_registry() {
            return;
        }
        let mut names = Vec::new();
        pat_identifiers(pattern, &mut names);
        let cfg = self.cfg.clone();
        for name in names {
            for registry in flow.registry_kinds() {
                self.add(
                    registry,
                    operation,
                    &name,
                    &cfg,
                    format!("{name}:{}", registry.source_name()),
                );
            }
        }
    }

    pub(super) fn record_return_flow(&mut self, flow: &Flow, fingerprint: String, cfg: &[String]) {
        for (registry, stage) in &flow.0 {
            self.add(
                *registry,
                RegistryOperation::Return,
                match stage {
                    FlowStage::Registry => "registry",
                    FlowStage::Guard => "guard",
                    FlowStage::Iterator => "iterator",
                    FlowStage::Entry => "entry",
                    FlowStage::Derived => "derived",
                },
                cfg,
                fingerprint.clone(),
            );
        }
    }

    pub(super) fn known_registry_token_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<_> = self.symbols.type_aliases.keys().cloned().collect();
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

    pub(super) fn visit_closure_with_input(&mut self, closure: &ExprClosure, input: &Flow) {
        if !self.allows_production(&closure.attrs, "closure") {
            return;
        }
        let next_cfg = item_cfg(&self.cfg, &closure.attrs);
        let previous_cfg = std::mem::replace(&mut self.cfg, next_cfg);
        self.scopes.push(BTreeMap::new());
        let info = VariableInfo {
            flow: Flow::from_kinds(&input.registry_kinds()),
            struct_types: BTreeSet::new(),
        };
        for input_pattern in &closure.inputs {
            self.bind_pattern(input_pattern, &info);
        }
        self.visit_expr(&closure.body);
        self.scopes.pop();
        self.cfg = previous_cfg;
    }
}

pub(super) fn implicit_tail_flow(block: &syn::Block, analyzer: &BodyAnalyzer<'_, '_>) -> Flow {
    match block.stmts.last() {
        Some(Stmt::Expr(expression, None)) => analyzer.flow_of_expr(expression),
        _ => Flow::default(),
    }
}
