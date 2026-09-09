//! Bridge access scan state definitions, part 2 of 3.
//!
//! Separated from the bridge_access.rs root under #660. Behaviour is preserved.

use super::*;

impl<'a> CandidateAnalyzer<'a> {
    pub(super) fn new(
        context: &'a ModuleContext<'a>,
        enclosing: String,
        symbols: &Symbols,
        errors: &'a mut Vec<String>,
    ) -> Self {
        Self {
            context,
            enclosing,
            symbols: symbols.clone(),
            variables: BTreeMap::new(),
            evidence: Vec::new(),
            directions: BTreeSet::new(),
            opaque_authority_macros: Vec::new(),
            errors,
        }
    }

    pub(super) fn add(
        &mut self,
        side: BridgeSide,
        kind: BridgeEvidenceKind,
        symbol: impl Into<String>,
        fingerprint: impl Into<String>,
    ) {
        let fingerprint = fingerprint.into();
        self.evidence.push(RawEvidence {
            side,
            kind,
            symbol: symbol.into(),
            fingerprint: compact_fingerprint(&fingerprint),
        });
    }

    pub(super) fn add_sides(
        &mut self,
        sides: &BTreeSet<BridgeSide>,
        kind: BridgeEvidenceKind,
        symbol: impl Into<String>,
        fingerprint: impl Into<String>,
    ) {
        let symbol = symbol.into();
        let fingerprint = fingerprint.into();
        for side in sides {
            self.add(*side, kind, symbol.clone(), fingerprint.clone());
        }
    }

    pub(super) fn seed_anchor(
        &mut self,
        anchor: &CuratedAnchor,
        kind: BridgeEvidenceKind,
        fingerprint: String,
    ) {
        self.directions.insert(anchor.direction);
        for side in direction_sides(anchor.direction) {
            self.add(*side, kind, anchor.name, fingerprint.clone());
        }
    }

    pub(super) fn seed_definition_anchor(&mut self, name: &str) {
        if let Some(anchor) = curated_definition(self.context.package, &self.context.module, name) {
            self.seed_anchor(
                anchor,
                BridgeEvidenceKind::CuratedAnchorDefinition,
                format!("definition:{name}"),
            );
        }
    }

    pub(super) fn bind_pattern(&mut self, pattern: &Pat, sides: &BTreeSet<BridgeSide>) {
        match pattern {
            Pat::Ident(identifier) => {
                self.variables
                    .insert(identifier.ident.to_string(), sides.clone());
                if let Some((_, subpattern)) = &identifier.subpat {
                    self.bind_pattern(subpattern, sides);
                }
            }
            Pat::Type(typed) => {
                let typed_sides = sides_in_type(&self.symbols, &typed.ty);
                let bound = if typed_sides.is_empty() {
                    sides.clone()
                } else {
                    typed_sides
                };
                self.bind_pattern(&typed.pat, &bound);
            }
            Pat::Reference(reference) => self.bind_pattern(&reference.pat, sides),
            Pat::Paren(paren) => self.bind_pattern(&paren.pat, sides),
            Pat::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, sides);
                }
            }
            Pat::TupleStruct(tuple) => {
                for element in &tuple.elems {
                    self.bind_pattern(element, sides);
                }
            }
            Pat::Struct(structure) => {
                for field in &structure.fields {
                    self.bind_pattern(&field.pat, sides);
                }
            }
            Pat::Slice(slice) => {
                for element in &slice.elems {
                    self.bind_pattern(element, sides);
                }
            }
            Pat::Or(or) => {
                for case in &or.cases {
                    self.bind_pattern(case, sides);
                }
            }
            _ => {}
        }
    }

    pub(super) fn bind_signature(&mut self, signature: &Signature) {
        for input in &signature.inputs {
            match input {
                FnArg::Receiver(_) => {}
                FnArg::Typed(typed) => {
                    let sides = sides_in_type(&self.symbols, &typed.ty);
                    self.bind_pattern(&typed.pat, &sides);
                }
            }
        }
    }

    pub(super) fn sides_of_expr(&self, expression: &Expr) -> BTreeSet<BridgeSide> {
        match expression {
            Expr::Path(path) => {
                let mut sides = self.symbols.sides_for_path(&path.path);
                if let Some(name) = last_path_ident(&path.path) {
                    if let Some(variable_sides) = self.variables.get(&name) {
                        sides.extend(variable_sides);
                    }
                }
                sides
            }
            Expr::Field(field) => {
                let mut sides = self.sides_of_expr(&field.base);
                if let Member::Named(member) = &field.member {
                    match member.to_string().as_str() {
                        "canonical_map_manager" => {
                            sides.insert(BridgeSide::Canonical);
                        }
                        "map_manager" => {
                            sides.insert(BridgeSide::Legacy);
                        }
                        _ => {}
                    }
                }
                sides
            }
            Expr::MethodCall(call) => self.sides_of_expr(&call.receiver),
            Expr::Reference(reference) => self.sides_of_expr(&reference.expr),
            Expr::Paren(paren) => self.sides_of_expr(&paren.expr),
            Expr::Group(group) => self.sides_of_expr(&group.expr),
            Expr::Try(tried) => self.sides_of_expr(&tried.expr),
            Expr::Await(awaited) => self.sides_of_expr(&awaited.base),
            Expr::Index(index) => self.sides_of_expr(&index.expr),
            Expr::Call(call) => {
                let mut sides = BTreeSet::new();
                if let Expr::Path(path) = call.func.as_ref() {
                    sides.extend(self.symbols.sides_for_path(&path.path));
                    let passthrough = last_path_ident(&path.path)
                        .is_some_and(|name| matches!(name.as_str(), "clone" | "from"));
                    if passthrough {
                        for argument in &call.args {
                            sides.extend(self.sides_of_expr(argument));
                        }
                    }
                }
                sides
            }
            _ => BTreeSet::new(),
        }
    }

    pub(super) fn audit_macro(&mut self, mac: &syn::Macro, label: &str) {
        let name = last_path_ident(&mac.path).unwrap_or_else(|| "<macro>".to_owned());
        let sides = token_sides(&mac.tokens, &self.symbols, &self.variables);
        let has_both =
            sides.contains(&BridgeSide::Canonical) && sides.contains(&BridgeSide::Legacy);
        let mentions_anchor = tokens_mention_curated_anchor(&mac.tokens);
        if mac.path.is_ident("macro_rules") && !sides.is_empty() {
            self.errors.push(format!(
                "{label} {name}! in {} can hide authority references in generated syntax",
                self.enclosing
            ));
            return;
        }
        if (has_both || mentions_anchor || bridge_shaped_name(&name)) && !transparent_macro(&name) {
            self.errors.push(format!(
                "unknown {label} {name}! in {} can hide a legacy/canonical bridge",
                self.enclosing
            ));
            return;
        }
        if !sides.is_empty() {
            let fingerprint = normalized_tokens(&mac.tokens);
            self.add_sides(
                &sides,
                BridgeEvidenceKind::MacroArgument,
                name.clone(),
                fingerprint,
            );
            if !transparent_macro(&name) {
                self.opaque_authority_macros
                    .push(format!("{label} {name}!"));
            }
        }
    }

    pub(super) fn finish(mut self, header: String, cfg: Vec<String>) -> Option<BridgeAccessRecord> {
        let sides: BTreeSet<_> = self.evidence.iter().map(|evidence| evidence.side).collect();
        let structural =
            sides.contains(&BridgeSide::Canonical) && sides.contains(&BridgeSide::Legacy);
        if !structural && self.directions.is_empty() {
            return None;
        }
        if !self.opaque_authority_macros.is_empty() {
            self.opaque_authority_macros.sort();
            self.opaque_authority_macros.dedup();
            self.errors.push(format!(
                "unknown {} in {} can hide part of a legacy/canonical bridge",
                self.opaque_authority_macros.join(", "),
                self.enclosing
            ));
            return None;
        }
        if self.directions.is_empty() {
            self.directions.insert(
                if is_loot_compatibility_module(self.context.package, &self.context.module) {
                    BridgeDirection::DualAuthorityCompatibility
                } else {
                    BridgeDirection::UnresolvedDualSide
                },
            );
        }

        let ordered_evidence = self
            .evidence
            .iter()
            .map(|evidence| {
                format!(
                    "{:?}:{:?}:{}:{}",
                    evidence.side, evidence.kind, evidence.symbol, evidence.fingerprint
                )
            })
            .collect::<Vec<_>>()
            .join("\u{1f}");
        let fingerprint = format!(
            "signature={header}|evidence={}",
            compact_fingerprint(&ordered_evidence)
        );

        let mut grouped = BTreeMap::<EvidenceGroupIdentity, Vec<String>>::new();
        for evidence in self.evidence {
            let identity = EvidenceGroupIdentity {
                side: evidence.side,
                kind: evidence.kind,
                symbol: evidence.symbol,
            };
            grouped
                .entry(identity)
                .or_default()
                .push(evidence.fingerprint);
        }
        let evidence = grouped
            .into_iter()
            .map(|(identity, occurrences)| BridgeEvidenceMarker {
                side: identity.side,
                kind: identity.kind,
                symbol: identity.symbol,
                fingerprint: compact_fingerprint(&occurrences.join("\u{1f}")),
                multiplicity: occurrences.len(),
            })
            .collect();
        Some(BridgeAccessRecord {
            package: self.context.package.to_owned(),
            module: self.context.module.clone(),
            path: self.context.path.to_owned(),
            enclosing: self.enclosing,
            direction_markers: self.directions.into_iter().collect(),
            evidence,
            fingerprint,
            cfg,
            multiplicity: 1,
        })
    }
}

impl<'ast> Visit<'ast> for CandidateAnalyzer<'_> {
    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        let sides = self.symbols.sides_for_path(&path.path);
        if !sides.is_empty() {
            self.add_sides(
                &sides,
                BridgeEvidenceKind::TypeReference,
                last_path_ident(&path.path).unwrap_or_else(|| "<type>".to_owned()),
                normalized_tokens(path),
            );
        }
        visit::visit_type_path(self, path);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        let mut sides = self.symbols.sides_for_path(&path.path);
        if let Some(name) = last_path_ident(&path.path) {
            if let Some(variable_sides) = self.variables.get(&name) {
                sides.extend(variable_sides);
            }
        }
        if !sides.is_empty() {
            self.add_sides(
                &sides,
                BridgeEvidenceKind::ValueReference,
                last_path_ident(&path.path).unwrap_or_else(|| "<value>".to_owned()),
                normalized_tokens(path),
            );
        }
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_field(&mut self, field: &'ast ExprField) {
        let sides = self.sides_of_expr(&Expr::Field(field.clone()));
        if !sides.is_empty() {
            let symbol = match &field.member {
                Member::Named(member) => member.to_string(),
                Member::Unnamed(index) => index.index.to_string(),
            };
            self.add_sides(
                &sides,
                BridgeEvidenceKind::FieldAccess,
                symbol,
                normalized_tokens(field),
            );
        }
        visit::visit_expr_field(self, field);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = call.func.as_ref() {
            let name = last_path_ident(&path.path).unwrap_or_else(|| "<call>".to_owned());
            let sides = self.symbols.sides_for_path(&path.path);
            if !sides.is_empty() {
                self.add_sides(
                    &sides,
                    BridgeEvidenceKind::FunctionCall,
                    name.clone(),
                    normalized_tokens(call),
                );
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let name = call.method.to_string();
        let sides = self.sides_of_expr(&call.receiver);
        if !sides.is_empty() {
            self.add_sides(
                &sides,
                BridgeEvidenceKind::MethodCall,
                name.clone(),
                normalized_tokens(call),
            );
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if let Pat::Type(typed) = &local.pat {
            self.visit_type(&typed.ty);
        }
        let mut sides = BTreeSet::new();
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            sides.extend(self.sides_of_expr(&init.expr));
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        if let Pat::Type(typed) = &local.pat {
            sides.extend(sides_in_type(&self.symbols, &typed.ty));
        }
        self.bind_pattern(&local.pat, &sides);
    }

    fn visit_expr_macro(&mut self, expression: &'ast ExprMacro) {
        self.audit_macro(&expression.mac, "expression macro");
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        self.audit_macro(&statement.mac, "statement macro");
    }

    fn visit_type_macro(&mut self, type_macro: &'ast syn::TypeMacro) {
        self.audit_macro(&type_macro.mac, "type macro");
    }

    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Use(item_use) => add_use_to_symbols(item_use, &mut self.symbols, self.errors),
            Item::Macro(item_macro) => {
                self.audit_macro(&item_macro.mac, "nested item macro");
            }
            // Nested items have their own enclosing identity and must be passed
            // as source/module items rather than folded into this function.
            _ => {}
        }
    }
}

pub(super) fn method_enclosing(item_impl: &ItemImpl, signature: &Signature) -> String {
    let self_type = normalized_tokens(&item_impl.self_ty);
    match &item_impl.trait_ {
        Some((_, trait_path, _)) => format!(
            "<{} as {}>::{}",
            self_type,
            normalized_tokens(trait_path),
            signature.ident
        ),
        None => format!("{}::{}", self_type, signature.ident),
    }
}

pub(super) fn analyze_function(
    function: &ItemFn,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    validate_cfg(
        &context.cfg,
        &function.attrs,
        &format!("function {}", function.sig.ident),
        errors,
    );
    let cfg = extend_cfg_context(&context.cfg, &function.attrs);
    let enclosing = format!("fn::{}", function.sig.ident);
    let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
    analyzer.seed_definition_anchor(&function.sig.ident.to_string());
    analyzer.bind_signature(&function.sig);
    analyzer.visit_signature(&function.sig);
    analyzer.visit_block(&function.block);
    let header = format!(
        "{}|body={}",
        normalized_tokens(&function.sig),
        compact_token_fingerprint(&function.block)
    );
    if let Some(record) = analyzer.finish(header, cfg) {
        accumulator.add(record);
    }
}

pub(super) fn analyze_impl(
    item_impl: &ItemImpl,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    validate_cfg(&context.cfg, &item_impl.attrs, "impl", errors);
    let impl_cfg = extend_cfg_context(&context.cfg, &item_impl.attrs);
    let self_sides = sides_in_type(symbols, &item_impl.self_ty);
    let trait_sides = item_impl
        .trait_
        .as_ref()
        .map(|(_, path, _)| symbols.sides_for_path(path))
        .unwrap_or_default();
    for item in &item_impl.items {
        match item {
            ImplItem::Fn(method) => {
                validate_cfg(
                    &impl_cfg,
                    &method.attrs,
                    &format!("method {}", method.sig.ident),
                    errors,
                );
                let cfg = extend_cfg_context(&impl_cfg, &method.attrs);
                let enclosing = method_enclosing(item_impl, &method.sig);
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.seed_definition_anchor(&method.sig.ident.to_string());
                analyzer.bind_signature(&method.sig);
                if !self_sides.is_empty() {
                    analyzer
                        .variables
                        .insert("self".to_owned(), self_sides.clone());
                    analyzer.add_sides(
                        &self_sides,
                        BridgeEvidenceKind::SelfType,
                        "Self",
                        normalized_tokens(&item_impl.self_ty),
                    );
                }
                if !trait_sides.is_empty() {
                    analyzer.add_sides(
                        &trait_sides,
                        BridgeEvidenceKind::SelfType,
                        "Trait",
                        item_impl
                            .trait_
                            .as_ref()
                            .map(|(_, path, _)| normalized_tokens(path))
                            .unwrap_or_default(),
                    );
                }
                analyzer.visit_signature(&method.sig);
                analyzer.visit_block(&method.block);
                let header = format!(
                    "impl:{}:{}:{}|body={}",
                    normalized_tokens(&item_impl.self_ty),
                    item_impl
                        .trait_
                        .as_ref()
                        .map(|(_, path, _)| normalized_tokens(path))
                        .unwrap_or_default(),
                    normalized_tokens(&method.sig),
                    compact_token_fingerprint(&method.block)
                );
                if let Some(record) = analyzer.finish(header, cfg) {
                    accumulator.add(record);
                }
            }
            ImplItem::Const(constant) => {
                let cfg = extend_cfg_context(&impl_cfg, &constant.attrs);
                let enclosing = format!(
                    "{}::const::{}",
                    normalized_tokens(&item_impl.self_ty),
                    constant.ident
                );
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.visit_type(&constant.ty);
                analyzer.visit_expr(&constant.expr);
                let header = format!(
                    "const {}:{}|value={}",
                    constant.ident,
                    normalized_tokens(&constant.ty),
                    compact_token_fingerprint(&constant.expr)
                );
                if let Some(record) = analyzer.finish(header, cfg) {
                    accumulator.add(record);
                }
            }
            ImplItem::Type(item_type) => {
                let cfg = extend_cfg_context(&impl_cfg, &item_type.attrs);
                let enclosing = format!(
                    "{}::type::{}",
                    normalized_tokens(&item_impl.self_ty),
                    item_type.ident
                );
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.visit_type(&item_type.ty);
                let header = format!(
                    "type {}={}",
                    item_type.ident,
                    normalized_tokens(&item_type.ty)
                );
                if let Some(record) = analyzer.finish(header, cfg) {
                    accumulator.add(record);
                }
            }
            ImplItem::Macro(item_macro) => {
                audit_item_macro(
                    &ItemMacro {
                        attrs: item_macro.attrs.clone(),
                        ident: None,
                        mac: item_macro.mac.clone(),
                        semi_token: item_macro.semi_token,
                    },
                    context,
                    symbols,
                    errors,
                    "impl item macro",
                );
            }
            _ => {}
        }
    }
}

pub(super) fn analyze_data_item(
    item: &Item,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    let (name, attrs) = match item {
        Item::Struct(value) => (format!("struct::{}", value.ident), value.attrs.as_slice()),
        Item::Enum(value) => (format!("enum::{}", value.ident), value.attrs.as_slice()),
        Item::Type(value) => (format!("type::{}", value.ident), value.attrs.as_slice()),
        Item::Const(value) => (format!("const::{}", value.ident), value.attrs.as_slice()),
        Item::Static(value) => (format!("static::{}", value.ident), value.attrs.as_slice()),
        _ => return,
    };
    validate_cfg(&context.cfg, attrs, &name, errors);
    let cfg = extend_cfg_context(&context.cfg, attrs);
    let header = format!("{name}|surface={}", compact_token_fingerprint(item));
    let mut analyzer = CandidateAnalyzer::new(context, name, symbols, errors);
    match item {
        Item::Struct(value) => {
            for field in &value.fields {
                analyzer.visit_type(&field.ty);
            }
        }
        Item::Enum(value) => {
            for variant in &value.variants {
                for field in &variant.fields {
                    analyzer.visit_type(&field.ty);
                }
                if let Some((_, discriminant)) = &variant.discriminant {
                    analyzer.visit_expr(discriminant);
                }
            }
        }
        Item::Type(value) => analyzer.visit_type(&value.ty),
        Item::Const(value) => {
            analyzer.visit_type(&value.ty);
            analyzer.visit_expr(&value.expr);
        }
        Item::Static(value) => {
            analyzer.visit_type(&value.ty);
            analyzer.visit_expr(&value.expr);
        }
        _ => unreachable!("data item was matched above"),
    }
    if let Some(record) = analyzer.finish(header, cfg) {
        accumulator.add(record);
    }
}

pub(super) fn audit_item_macro(
    item_macro: &ItemMacro,
    context: &ModuleContext<'_>,
    symbols: &Symbols,
    errors: &mut Vec<String>,
    label: &str,
) {
    let name = item_macro
        .ident
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| last_path_ident(&item_macro.mac.path))
        .unwrap_or_else(|| "<macro>".to_owned());
    let sides = token_sides(&item_macro.mac.tokens, symbols, &BTreeMap::new());
    if !item_macro.mac.path.is_ident("include")
        && (!sides.is_empty()
            || tokens_mention_curated_anchor(&item_macro.mac.tokens)
            || bridge_shaped_name(&name))
    {
        errors.push(format!(
            "{label} {name}! in {}::{} can generate or hide a legacy/canonical bridge",
            context.package, context.module
        ));
    }
}

pub(super) fn analyze_module_items(
    items: &[Item],
    context: &ModuleContext<'_>,
    inherited_symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
    let symbols = register_module_symbols(items, context, inherited_symbols, errors);
    for item in items {
        match item {
            Item::Fn(function) => {
                analyze_function(function, context, &symbols, accumulator, errors)
            }
            Item::Impl(item_impl) => {
                analyze_impl(item_impl, context, &symbols, accumulator, errors)
            }
            Item::Struct(_) | Item::Enum(_) | Item::Type(_) | Item::Const(_) | Item::Static(_) => {
                analyze_data_item(item, context, &symbols, accumulator, errors)
            }
            Item::Macro(item_macro) => {
                audit_item_macro(item_macro, context, &symbols, errors, "item macro")
            }
            Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) => {
                validate_cfg(&context.cfg, attrs, &format!("module {ident}"), errors);
                let child = ModuleContext {
                    package: context.package,
                    module: format!("{}::{ident}", context.module),
                    path: context.path,
                    cfg: extend_cfg_context(&context.cfg, attrs),
                };
                analyze_module_items(child_items, &child, &symbols, accumulator, errors);
            }
            _ => {}
        }
    }
}

/// Parse and inventory bridge-bearing items from already resolved world-server
/// and wow-world source mounts. Input order is irrelevant; duplicate exact
/// `(package, module, path, inherited cfg)` mounts fail because they would make
/// multiplicity depend on caller behavior. Distinct cfg mounts remain visible.
pub(crate) fn inventory_bridge_accesses(
    sources: &[BridgeSource<'_>],
) -> Result<BridgeAccessBaseline, String> {
    let mut ordered: Vec<_> = sources.iter().copied().collect();
    ordered.sort_by(|left, right| {
        (
            left.package,
            left.module,
            left.source_path,
            left.inherited_cfg,
        )
            .cmp(&(
                right.package,
                right.module,
                right.source_path,
                right.inherited_cfg,
            ))
    });
    let mut seen = BTreeSet::new();
    let mut accumulator = BridgeAccumulator::default();
    let mut errors = Vec::new();
    for source in ordered {
        if source.package.is_empty() || source.module.is_empty() || source.source_path.is_empty() {
            errors.push("bridge source package/module/path must be non-empty".to_owned());
            continue;
        }
        if !seen.insert((
            source.package,
            source.module,
            source.source_path,
            source.inherited_cfg,
        )) {
            errors.push(format!(
                "duplicate bridge source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        let syntax = match syn::parse_file(source.source) {
            Ok(syntax) => syntax,
            Err(error) => {
                errors.push(format!(
                    "cannot parse bridge source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        };
        validate_cfg(
            source.inherited_cfg,
            &syntax.attrs,
            source.source_path,
            &mut errors,
        );
        let context = ModuleContext {
            package: source.package,
            module: source.module.to_owned(),
            path: source.source_path,
            cfg: extend_cfg_context(source.inherited_cfg, &syntax.attrs),
        };
        analyze_module_items(
            &syntax.items,
            &context,
            &Symbols::for_module(source.package, source.module),
            &mut accumulator,
            &mut errors,
        );
    }
    if errors.is_empty() {
        Ok(accumulator.finish())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors.join("\n"))
    }
}
