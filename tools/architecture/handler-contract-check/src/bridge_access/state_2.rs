//! Bridge access scan state definitions, part 2 of 3.
//!
//! Separated from the bridge_access.rs root under #660. Behaviour is preserved.

use super::*;

impl<'a> CandidateAnalyzer<'a> {
    pub(super) fn new(
        context: &'a ModuleContext<'a>,
        enclosing: String,
        symbols: &'a Symbols,
        errors: &'a mut Vec<String>,
    ) -> Self {
        Self {
            context,
            enclosing,
            symbols: symbols.for_cfg(&context.cfg).clone(),
            module_symbols: symbols,
            local_imports: Vec::new(),
            active_cfg: context.cfg.clone(),
            variables: BTreeMap::new(),
            evidence: Vec::new(),
            directions: BTreeSet::new(),
            opaque_authority_macros: Vec::new(),
            reported_resolution_issues: BTreeSet::new(),
            generic_parameters: BTreeSet::new(),
            errors,
        }
    }

    pub(super) fn set_active_cfg(&mut self, cfg: Vec<String>) {
        self.symbols = self.module_symbols.for_cfg(&cfg).clone();
        for name in &self.generic_parameters {
            self.symbols.shadow_generic(name);
        }
        self.active_cfg = cfg;
        self.apply_local_imports();
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
                let typed_sides =
                    sides_in_type_with_cfg(&self.symbols, &typed.ty, &self.active_cfg);
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
        self.bind_generics(&signature.generics);
        for input in &signature.inputs {
            let attrs = match input {
                FnArg::Receiver(receiver) => &receiver.attrs,
                FnArg::Typed(typed) => &typed.attrs,
            };
            self.within_cfg(attrs, |analyzer| {
                if let FnArg::Typed(typed) = input {
                    let sides =
                        sides_in_type_with_cfg(&analyzer.symbols, &typed.ty, &analyzer.active_cfg);
                    analyzer.bind_pattern(&typed.pat, &sides);
                }
            });
        }
    }

    fn bind_generics(&mut self, generics: &syn::Generics) {
        for parameter in &generics.params {
            let name = match parameter {
                syn::GenericParam::Type(parameter) => parameter.ident.to_string(),
                syn::GenericParam::Const(parameter) => parameter.ident.to_string(),
                syn::GenericParam::Lifetime(_) => continue,
            };
            self.generic_parameters.insert(name.clone());
            self.symbols.shadow_generic(&name);
        }
    }

    pub(super) fn sides_of_expr(&self, expression: &Expr) -> BTreeSet<BridgeSide> {
        match expression {
            Expr::Path(path) => {
                let mut sides = self
                    .symbols
                    .sides_for_path_with_cfg(&path.path, &self.active_cfg);
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
                    sides.extend(
                        self.symbols
                            .sides_for_path_with_cfg(&path.path, &self.active_cfg),
                    );
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
        for path in provenance::token_paths(&mac.tokens) {
            if let Some(issue) = self.symbols.path_issue_for_segments(&path)
                && self.reported_resolution_issues.insert(path.join("::"))
            {
                self.errors.push(issue.to_owned());
            }
            if path.len() > 1
                && let Ok(path) = syn::parse_str::<Path>(&path.join("::"))
            {
                self.report_missing_glob_path(&path, true);
            }
        }
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
    analyzer.set_active_cfg(cfg.clone());
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
                let self_sides = sides_in_type_with_cfg(symbols, &item_impl.self_ty, &cfg);
                let trait_sides = item_impl
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| symbols.sides_for_path_with_cfg(path, &cfg))
                    .unwrap_or_default();
                let enclosing = method_enclosing(item_impl, &method.sig);
                let mut analyzer = CandidateAnalyzer::new(context, enclosing, symbols, errors);
                analyzer.set_active_cfg(cfg.clone());
                analyzer.bind_generics(&item_impl.generics);
                if let Type::Path(path) = item_impl.self_ty.as_ref() {
                    analyzer.report_path_issue(&path.path);
                }
                if let Some((_, path, _)) = &item_impl.trait_ {
                    analyzer.report_path_issue(path);
                }
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
                analyzer.set_active_cfg(cfg.clone());
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
                analyzer.set_active_cfg(cfg.clone());
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
    analyzer.set_active_cfg(cfg.clone());
    match item {
        Item::Struct(item) => analyzer.bind_generics(&item.generics),
        Item::Enum(item) => analyzer.bind_generics(&item.generics),
        Item::Type(item) => analyzer.bind_generics(&item.generics),
        _ => {}
    }
    match item {
        Item::Struct(value) => {
            for field in &value.fields {
                analyzer.visit_field(field);
            }
        }
        Item::Enum(value) => {
            for variant in &value.variants {
                analyzer.visit_variant(variant);
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
    let cfg = extend_cfg_context(&context.cfg, &item_macro.attrs);
    let symbols = symbols.for_cfg(&cfg);
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
    symbols: &Symbols,
    accumulator: &mut BridgeAccumulator,
    errors: &mut Vec<String>,
) {
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
                // Inline children are indexed and analyzed as independent
                // lexical modules by `provenance.rs`.  Re-entering them here
                // would use the parent's symbol table and double-count the
                // same source mount.
                let _ = child_items;
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
    let mut parsed_sources = Vec::new();
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
        parsed_sources.push((source, syntax));
    }

    // The repository caller has already resolved source mounts and logical
    // module paths.  Build the lexical graph from exactly those inputs so
    // external and inline children receive identical `super`/glob handling.
    let index = build_module_index(&parsed_sources);
    let symbols = resolve_module_symbols(&index, &mut errors);
    for (node, module_symbols) in index.modules.iter().zip(symbols.iter()) {
        let context = ModuleContext {
            package: &node.package,
            module: node.module.clone(),
            path: &node.source_path,
            cfg: node.cfg.clone(),
        };
        analyze_module_items(
            &node.items,
            &context,
            module_symbols,
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
