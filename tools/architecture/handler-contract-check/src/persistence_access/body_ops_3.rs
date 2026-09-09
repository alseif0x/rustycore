//! Body-analysis methods, part 3 of 5.
//!
//! The inherent `BodyAnalyzer` impl is divided by the analysis phase its
//! methods serve under #634; every method keeps its original body.

use super::*;

impl<'a, 'b> BodyAnalyzer<'a, 'b> {
    pub(super) fn union_into_declared_projections(
        &self,
        aggregate: &mut VariableInfo,
        value: &VariableInfo,
    ) {
        for owner in aggregate.nominal_types.clone() {
            let suffix = format!("::{owner}");
            for (key, declared) in self
                .symbols
                .named_type_info
                .iter()
                .chain(self.symbols.workspace_named_type_info.iter())
            {
                if key != &owner && !key.ends_with(&suffix) {
                    continue;
                }
                if aggregate.tuple_items.len() < declared.tuple_items.len() {
                    aggregate
                        .tuple_items
                        .resize_with(declared.tuple_items.len(), VariableInfo::default);
                }
                for item in &mut aggregate.tuple_items {
                    item.union(value);
                }
                for field in declared.field_items.keys() {
                    aggregate
                        .field_items
                        .entry(field.clone())
                        .or_default()
                        .union(value);
                }
            }
        }
    }
    pub(super) fn has_declared_fields(&self, owner: &str) -> bool {
        let owners = self
            .symbols
            .nominal_type_aliases
            .get(owner)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([owner.to_owned()]));
        owners.iter().any(|owner| {
            self.symbols
                .field_targets
                .keys()
                .any(|(field_owner, _)| field_owner == owner)
                || self
                    .symbols
                    .tuple_field_targets
                    .keys()
                    .any(|(field_owner, _)| field_owner == owner)
                || self
                    .symbols
                    .field_nominal_types
                    .keys()
                    .any(|(field_owner, _)| field_owner == owner)
        }) || {
            let suffix = format!("::{owner}");
            let local = self.symbols.named_type_info.iter().any(|(key, info)| {
                (key == &owner || key.ends_with(&suffix)) && !info.field_items.is_empty()
            });
            let canonical =
                canonical_path_names(owner.split("::").map(str::to_owned).collect(), self.symbols)
                    .join("::");
            local
                || [owner, canonical.as_str()].into_iter().any(|key| {
                    self.symbols
                        .workspace_named_type_info
                        .get(key)
                        .is_some_and(|info| !info.field_items.is_empty())
                })
        }
    }
    pub(super) fn pattern_owner(&self, path: &syn::Path) -> String {
        let names = path_names(path);
        if names.len() < 2 {
            return names.last().cloned().unwrap_or_default();
        }
        let variant = names.last().cloned().unwrap_or_default();
        let owner = names.get(names.len() - 2).cloned().unwrap_or_default();
        let owner = self
            .symbols
            .nominal_type_aliases
            .get(&owner)
            .and_then(|owners| {
                (owners.len() == 1)
                    .then(|| owners.iter().next().cloned())
                    .flatten()
            })
            .unwrap_or(owner);
        format!("{owner}::{variant}")
    }
    pub(super) fn wrapper_payload_info(
        &self,
        owner: &str,
        info: &VariableInfo,
    ) -> Option<VariableInfo> {
        let argument_index = match owner.rsplit("::").next().unwrap_or(owner) {
            "Some" | "Ok" => 0,
            "Err" => 1,
            _ => return None,
        };
        let shapes = info
            .payload_variants
            .iter()
            .filter_map(|arguments| arguments.get(argument_index))
            .collect::<Vec<_>>();
        if shapes.is_empty() {
            return None;
        }
        let mut payload = VariableInfo {
            flow: info.flow.clone(),
            sql_expression: info.sql_expression,
            trait_bounds: info.trait_bounds.clone(),
            ..VariableInfo::default()
        };
        payload
            .trait_bounds
            .extend(info.trait_bounds.iter().cloned());
        for shape in shapes {
            payload
                .nominal_types
                .extend(shape.nominal_types.iter().cloned());
            if !shape.arguments.is_empty() {
                if shape.nominal_types.is_empty() {
                    payload.tuple_items =
                        shape.arguments.iter().map(Self::info_from_shape).collect();
                } else {
                    payload.payload_variants.insert(shape.arguments.clone());
                }
            }
        }
        for nominal in payload.nominal_types.clone() {
            if let Some(alias) = self.symbols.type_alias_info.get(&nominal) {
                payload.union(alias);
            }
        }
        Some(payload)
    }
    pub(super) fn info_from_shape(shape: &NominalShape) -> VariableInfo {
        let mut info = VariableInfo {
            nominal_types: shape.nominal_types.clone(),
            ..VariableInfo::default()
        };
        if shape.nominal_types.is_empty() {
            info.tuple_items = shape.arguments.iter().map(Self::info_from_shape).collect();
        } else if !shape.arguments.is_empty() {
            info.payload_variants.insert(shape.arguments.clone());
        }
        info
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
                typed_info
                    .nominal_types
                    .extend(info.nominal_types.iter().cloned());
                typed_info
                    .payload_variants
                    .extend(info.payload_variants.iter().cloned());
                typed_info.sql_expression = typed_info.sql_expression.max(info.sql_expression);
                typed_info
                    .sql_sources
                    .extend(info.sql_sources.iter().cloned());
                self.bind_pattern(&typed.pat, &typed_info);
            }
            Pat::Tuple(tuple) => {
                for (index, element) in tuple.elems.iter().enumerate() {
                    if matches!(element, Pat::Rest(_)) {
                        continue;
                    }
                    let source_index = tuple
                        .elems
                        .iter()
                        .position(|pattern| matches!(pattern, Pat::Rest(_)))
                        .filter(|rest| index > *rest)
                        .and_then(|_| {
                            info.tuple_items
                                .len()
                                .checked_sub(tuple.elems.len().saturating_sub(index))
                        })
                        .unwrap_or(index);
                    self.bind_pattern(element, info.tuple_items.get(source_index).unwrap_or(info));
                }
            }
            Pat::TupleStruct(tuple) => {
                let owner = self.pattern_owner(&tuple.path);
                for (index, element) in tuple.elems.iter().enumerate() {
                    if matches!(element, Pat::Rest(_)) {
                        continue;
                    }
                    let source_index = tuple
                        .elems
                        .iter()
                        .position(|pattern| matches!(pattern, Pat::Rest(_)))
                        .filter(|rest| index > *rest)
                        .and_then(|_| {
                            let declared_len = self
                                .symbols
                                .named_type_info
                                .iter()
                                .filter(|(key, _)| {
                                    *key == &owner || key.ends_with(&format!("::{owner}"))
                                })
                                .map(|(_, info)| info.tuple_items.len())
                                .max()
                                .into_iter()
                                .chain(
                                    self.symbols
                                        .tuple_field_targets
                                        .keys()
                                        .chain(self.symbols.field_nominal_types.keys())
                                        .filter(|(candidate, _)| candidate == &owner)
                                        .filter_map(|(_, field)| field.parse::<usize>().ok())
                                        .map(|index| index + 1),
                                )
                                .max()
                                .unwrap_or(tuple.elems.len());
                            declared_len.checked_sub(tuple.elems.len().saturating_sub(index))
                        })
                        .unwrap_or(index);
                    if self.has_declared_fields(&owner) {
                        let field_info =
                            self.declared_field_info(&owner, &source_index.to_string(), false);
                        self.bind_pattern(element, &field_info);
                    } else if let Some(payload_info) = self.wrapper_payload_info(&owner, info) {
                        self.bind_pattern(element, &payload_info);
                    } else {
                        self.bind_pattern(element, info);
                    }
                }
            }
            Pat::Struct(structure) => {
                let owner = self.pattern_owner(&structure.path);
                for field in &structure.fields {
                    let field_name = match &field.member {
                        Member::Named(ident) => normalized_ident(ident),
                        Member::Unnamed(index) => index.index.to_string(),
                    };
                    if self.has_declared_fields(&owner) {
                        let field_info = self.declared_field_info(
                            &owner,
                            &field_name,
                            matches!(field.member, Member::Named(_)),
                        );
                        self.bind_pattern(&field.pat, &field_info);
                    } else {
                        self.bind_pattern(&field.pat, info);
                    }
                }
            }
            Pat::Slice(slice) => {
                for element in &slice.elems {
                    self.bind_pattern(element, info);
                }
            }
            Pat::Paren(paren) => self.bind_pattern(&paren.pat, info),
            Pat::Or(or_pattern) => {
                for case in &or_pattern.cases {
                    self.bind_pattern(case, info);
                }
            }
            _ => {}
        }
    }
    pub(super) fn bind_pattern_from_expr(&mut self, pattern: &Pat, expression: &Expr) {
        match (pattern, expression) {
            (Pat::Tuple(pattern), Expr::Tuple(tuple))
                if pattern.elems.len() == tuple.elems.len() =>
            {
                for (pattern, expression) in pattern.elems.iter().zip(&tuple.elems) {
                    let mut info = self.info_from_expr(expression);
                    info.sql_expression = self.sql_expression_kind(expression);
                    info.sql_sources = self.sql_sources(expression);
                    self.bind_pattern(pattern, &info);
                }
            }
            _ => {
                let mut info = self.info_from_expr(expression);
                info.sql_expression = self.sql_expression_kind(expression);
                info.sql_sources = self.sql_sources(expression);
                self.bind_pattern(pattern, &info);
            }
        }
    }
    pub(super) fn sql_expression_kind(&self, expression: &Expr) -> SqlExpressionKind {
        match expression {
            Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => {
                SqlExpressionKind::Static
            }
            Expr::Reference(reference) => self.sql_expression_kind(&reference.expr),
            Expr::Paren(paren) => self.sql_expression_kind(&paren.expr),
            Expr::Group(group) => self.sql_expression_kind(&group.expr),
            // Control flow is classified from its own syntax: whether a branch
            // interpolates is visible without evaluating which branch runs.
            Expr::Block(_) | Expr::If(_) | Expr::Match(_) => {
                control_flow_sql_kind(expression.to_token_stream())
            }
            Expr::Path(path) => {
                // A pinned constant keeps its kind however it is named: a local
                // binding, a module item, an imported name, or a qualified
                // path. Reading only body scopes reported it as
                // runtime-assembled and forced a needless workflow
                // classification.
                self.pinned_path_info(path)
                    .map(|info| info.sql_expression)
                    .unwrap_or(SqlExpressionKind::Nonliteral)
            }
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path)
                    if path.path.segments.iter().rev().take(2).map(|segment| normalized_ident(&segment.ident)).collect::<Vec<_>>()
                        == ["from", "String"]) =>
            {
                call.args
                    .first()
                    .map(|argument| self.sql_expression_kind(argument))
                    .unwrap_or(SqlExpressionKind::Nonliteral)
            }
            Expr::Macro(mac)
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| matches!(name.as_str(), "format" | "format_args")) =>
            {
                SqlExpressionKind::Interpolated
            }
            Expr::Macro(mac)
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| name == "include_str") =>
            {
                SqlExpressionKind::Included
            }
            Expr::Macro(mac)
                if self
                    .canonical_local_path_names(path_names(&mac.mac.path))
                    .last()
                    .is_some_and(|name| name == "env") =>
            {
                SqlExpressionKind::Environment
            }
            Expr::Macro(mac)
                if self
                    .standard_string_macro(path_names(&mac.mac.path))
                    .is_some_and(|name| name == "concat") =>
            {
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.mac.tokens.clone())
                    .ok()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .fold(SqlExpressionKind::Static, |kind, argument| {
                                kind.max(self.sql_expression_kind(argument))
                            })
                    })
                    .unwrap_or(SqlExpressionKind::Nonliteral)
            }
            Expr::Macro(mac)
                if self
                    .standard_string_macro(path_names(&mac.mac.path))
                    .is_some_and(|name| name == "stringify") =>
            {
                SqlExpressionKind::Static
            }
            Expr::MethodCall(method)
                if matches!(
                    normalized_ident(&method.method).as_str(),
                    "as_str" | "as_ref" | "to_owned" | "to_string"
                ) =>
            {
                self.sql_expression_kind(&method.receiver)
            }
            Expr::MethodCall(method) if normalized_ident(&method.method) == "sql" => {
                let mut targets = self.flow_of_expr(&method.receiver).targets();
                if let Expr::Path(path) = method.receiver.as_ref() {
                    targets.extend(targets_for_path(&path.path, self.symbols));
                }
                if targets.iter().any(|target| {
                    matches!(
                        target,
                        PersistenceTarget::PreparedStatement
                            | PersistenceTarget::LoginStatements
                            | PersistenceTarget::WorldStatements
                            | PersistenceTarget::CharStatements
                            | PersistenceTarget::HotfixStatements
                    )
                }) {
                    SqlExpressionKind::Static
                } else {
                    SqlExpressionKind::Nonliteral
                }
            }
            _ => SqlExpressionKind::Nonliteral,
        }
    }
    /// The SQL a query argument pins, when a single local rule can prove it.
    ///
    /// This grammar reads pinned statements; it does not evaluate what a Rust
    /// expression would build. A literal, a `concat!`, and a name bound to one
    /// of those are provable here. Anything assembled at run time — a `+`
    /// chain, a `format!` template, a branch, a helper's return, a projection —
    /// yields nothing, and the call site is then ratcheted as interpolated or
    /// nonliteral SQL whose connection affinity is a reviewed workflow
    /// annotation rather than a guess made from token text.
    /// The value a path names, wherever it is declared: a local binding, an
    /// item of this module, an imported name, or a qualified path. An imported
    /// or qualified name is resolved through the canonical package registry,
    /// which is where a constant declared in another module is recorded.
    pub(super) fn pinned_path_info(&self, path: &syn::ExprPath) -> Option<&VariableInfo> {
        // `<Statements as Sql>::SQL` names the same constant as
        // `Statements::SQL`; the qualification says which impl, not which value.
        // `Self::SQL` inside an impl names that impl's type.
        let names = path_names(&path.path);
        if path.qself.is_none()
            && names.len() == 2
            && names[0] == "Self"
            && let Some(info) = self.lookup("Self")
        {
            let member = names[1].clone();
            // Resolution order matches Rust's: this impl's override, then the
            // trait's default, then an inherent constant of the same name.
            if let Some(active_trait) = &self.active_trait
                && let Some(found) = info.nominal_types.iter().find_map(|owner| {
                    let owner = self.package_function_key(vec![owner.clone()]);
                    self.symbols
                        .package_item_values
                        .get(&format!("<{owner} as {active_trait}>::{member}"))
                })
            {
                return Some(found);
            }
            if let Some(active_trait) = &self.active_trait
                && let Some(found) = self
                    .symbols
                    .package_item_values
                    .get(&format!("{active_trait}::{member}"))
            {
                return Some(found);
            }
            if let Some(found) = info.nominal_types.iter().find_map(|owner| {
                let owner = self.package_function_key(vec![owner.clone()]);
                self.symbols
                    .package_item_values
                    .get(&format!("{owner}::{member}"))
            }) {
                return Some(found);
            }
        }
        if let Some(qself) = &path.qself {
            // `<Statements as Sql>::SQL` names the value that `Sql` defines,
            // which another trait's impl for the same type may not share.
            let member = last_path_name(&path.path)?;
            let trait_path = (path.path.segments.len() > 1).then(|| {
                let names = path_names(&path.path);
                self.canonical_names(names[..names.len() - 1].to_vec())
                    .join("::")
            });
            return nominal_types_in_type(&qself.ty)
                .into_iter()
                .chain(receiver_nominal_types_in_type(&qself.ty))
                .find_map(|owner| {
                    let owner = self.package_function_key(vec![owner]);
                    let key = match &trait_path {
                        Some(trait_path) => format!("<{owner} as {trait_path}>::{member}"),
                        None => format!("{owner}::{member}"),
                    };
                    self.symbols
                        .package_item_values
                        .get(&key)
                        // An impl that does not override the constant inherits
                        // the trait's default.
                        .or_else(|| {
                            trait_path.as_ref().and_then(|trait_path| {
                                self.symbols
                                    .package_item_values
                                    .get(&format!("{trait_path}::{member}"))
                            })
                        })
                });
        }
        if names.len() == 1
            && let Some(name) = names.first()
            && let Some(info) = self
                .lookup(name)
                .or_else(|| self.symbols.item_values.get(name))
        {
            return Some(info);
        }
        self.symbols
            .package_item_values
            .get(&self.package_function_key(names))
    }
    /// Whether the statement `text` carries takes an advisory lock, reading it
    /// with this scope's knowledge.
    ///
    /// A compile-time string macro can be invoked under an alias
    /// (`use std::concat as c`), and the reader below recognizes only the macro
    /// it actually is. Rename them first so a pinned statement is not split
    /// into unrelated pieces.
    /// Whether the statement `fingerprint` carries takes an advisory lock,
    /// read with this scope's imports in hand.
    ///
    /// A compile-time string macro can be invoked under an alias
    /// (`use std::concat as c`), and only an *unqualified* name is the alias:
    /// `other::c!` is a different macro that happens to share a leaf name.
    /// The standard compile-time string macro a path names, if it names one.
    ///
    /// Only shadows declared above this body are in scope for it, so the set is
    /// the one recorded where the enclosing item was reached.
    pub(super) fn standard_string_macro(&self, written: Vec<String>) -> Option<String> {
        standard_string_macro_of(
            written,
            &|path| self.canonical_names(path),
            &self.symbols.module_path,
            &self.visible_macro_shadows,
        )
    }
    pub(super) fn statement_takes_advisory_lock(&self, fingerprint: &str) -> bool {
        sql_is_advisory_lock(fingerprint, &|path: Vec<String>| {
            self.standard_string_macro(path)
        })
    }
    /// The path a name really refers to: a block-local `use` alias first, then
    /// the module's own imports.
    pub(super) fn canonical_names(&self, names: Vec<String>) -> Vec<String> {
        canonical_path_names(self.canonical_local_path_names(names), self.symbols)
    }
    pub(super) fn sql_sources(&self, expression: &Expr) -> BTreeSet<String> {
        match expression {
            Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => {
                BTreeSet::from([normalized_tokens(expression)])
            }
            Expr::Reference(reference) => self.sql_sources(&reference.expr),
            Expr::Paren(paren) => self.sql_sources(&paren.expr),
            Expr::Group(group) => self.sql_sources(&group.expr),
            Expr::Path(path) => self
                .pinned_path_info(path)
                .map(|info| info.sql_sources.clone())
                .unwrap_or_default(),
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path)
                    if path.path.segments.iter().rev().take(2).map(|segment| normalized_ident(&segment.ident)).collect::<Vec<_>>()
                        == ["from", "String"]) =>
            {
                call.args
                    .first()
                    .map(|argument| self.sql_sources(argument))
                    .unwrap_or_default()
            }
            // A `QueryBuilder` opens with the first fragment of one statement,
            // and the rest is pushed onto it.
            Expr::Call(call)
                if matches!(call.func.as_ref(), Expr::Path(path)
                    if path.path.segments.iter().rev().take(2).map(|segment| normalized_ident(&segment.ident)).collect::<Vec<_>>()
                        == ["new", "QueryBuilder"]) =>
            {
                call.args
                    .first()
                    .map(|argument| self.sql_sources(argument))
                    .unwrap_or_default()
            }
            // `concat!` and `stringify!` are pinned by their own tokens, which
            // already carry their arguments in order.
            Expr::Macro(mac)
                if self
                    .standard_string_macro(path_names(&mac.mac.path))
                    .is_some() =>
            {
                BTreeSet::from([normalized_tokens(expression)])
            }
            Expr::MethodCall(method)
                if matches!(
                    normalized_ident(&method.method).as_str(),
                    "as_str" | "as_ref" | "to_owned" | "to_string"
                ) =>
            {
                self.sql_sources(&method.receiver)
            }
            _ => BTreeSet::new(),
        }
    }
    pub(super) fn fingerprint_with_sql_source(
        &self,
        base: String,
        argument: Option<&Expr>,
    ) -> String {
        let Some(argument) = argument else {
            return base;
        };
        fn is_indirect_with(
            expression: &Expr,
            resolve_path: &dyn Fn(Vec<String>) -> Vec<String>,
            local_types: &BTreeSet<String>,
        ) -> bool {
            let is_indirect =
                |expression: &Expr| is_indirect_with(expression, resolve_path, local_types);
            match expression {
                Expr::Path(_) => true,
                Expr::Reference(reference) => is_indirect(&reference.expr),
                Expr::Paren(paren) => is_indirect(&paren.expr),
                Expr::Group(group) => is_indirect(&group.expr),
                // `String::from(SQL)` is the same conversion in call form —
                // but only the standard one. A local type named `String` can
                // return anything.
                Expr::Call(call)
                    if matches!(call.func.as_ref(), Expr::Path(path)
                    if is_standard_string_conversion(
                        &path.path,
                        resolve_path,
                        local_types,
                    )) =>
                {
                    call.args.first().is_some_and(is_indirect)
                }
                // The same conversions `sql_sources` follows: a pinned source
                // is no less pinned for having been converted on the way in.
                Expr::MethodCall(method)
                    if matches!(
                        normalized_ident(&method.method).as_str(),
                        "as_str" | "as_ref" | "to_owned" | "to_string"
                    ) =>
                {
                    is_indirect(&method.receiver)
                }
                _ => false,
            }
        }
        let resolve_path = |path: Vec<String>| self.canonical_names(path);
        if !is_indirect_with(
            argument,
            &resolve_path,
            &self.symbols.local_type_definitions,
        ) {
            return base;
        }
        let sources = self.sql_sources(argument);
        if sources.is_empty() {
            base
        } else {
            format!(
                "{base}|sql-source:{}",
                sources.into_iter().collect::<Vec<_>>().join("|")
            )
        }
    }
    pub(super) fn field_flow(&self, field: &ExprField) -> Flow {
        let name = match &field.member {
            Member::Named(ident) => normalized_ident(ident),
            Member::Unnamed(index) => index.index.to_string(),
        };
        let base_info = self.info_from_expr(&field.base);
        if let Some(info) = base_info.field_items.get(&name) {
            // A declared projection is authoritative even when it is clean.
            // Falling through for an empty field inherited the aggregate's
            // persistence flow and tainted sibling booleans/counters.
            return info.flow.clone();
        }
        let mut targets = TargetSet::new();
        let mut declared = false;
        for owner in self.nominal_types_of_expr(&field.base) {
            let field_targets = match &field.member {
                Member::Named(_) => self
                    .symbols
                    .field_targets
                    .get(&(owner.clone(), name.clone())),
                Member::Unnamed(_) => self
                    .symbols
                    .tuple_field_targets
                    .get(&(owner.clone(), name.clone())),
            };
            if let Some(field_targets) = field_targets {
                declared = true;
                targets.extend(field_targets);
            }
            declared |= self
                .symbols
                .field_nominal_types
                .contains_key(&(owner.clone(), name.clone()));
            declared |= self.named_field_is_declared(&owner, &name);
        }
        if declared && targets.is_empty() {
            return Flow::default();
        }
        if targets.is_empty()
            && matches!(field.member, Member::Named(_))
            && let Some(owners) = self.symbols.field_owners.get(&name)
            && !owners.is_empty()
            && owners.iter().all(|owner| {
                self.symbols
                    .field_targets
                    .get(&(owner.clone(), name.clone()))
                    .is_some_and(|targets| !targets.is_empty())
            })
        {
            for owner in owners {
                if let Some(field_targets) = self
                    .symbols
                    .field_targets
                    .get(&(owner.clone(), name.clone()))
                {
                    targets.extend(field_targets);
                }
            }
        }
        if !targets.is_empty() {
            return Flow::pools(&targets);
        }
        if let Some(target) = database_field_target(&name) {
            return Flow::pools(&BTreeSet::from([target]));
        }
        self.flow_of_expr(&field.base)
            .map_pool_stage(FlowStage::DerivedPool)
    }
    /// The flow a path names. An imported alias is resolved first, so
    /// `use sqlx::mysql::MySqlPoolOptions as Opt` keeps the provider that
    /// `Opt::new()` builds.
    pub(super) fn flow_of_path(&self, path: &syn::ExprPath) -> Flow {
        if path.qself.is_none() && path.path.segments.len() == 1 {
            if let Some(name) = last_path_name(&path.path) {
                if let Some(info) = self.lookup(&name) {
                    return info.flow.clone();
                }
                if let Some(info) = self.symbols.item_values.get(&name) {
                    return info.flow.clone();
                }
            }
        }
        let key = self.package_function_key(path_names(&path.path));
        if let Some(info) = self.symbols.package_item_values.get(&key) {
            return info.flow.clone();
        }
        Flow::pools(&targets_for_names(
            &self.canonical_names(path_names(&path.path)),
            self.symbols,
        ))
    }
}
