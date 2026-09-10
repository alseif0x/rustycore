//! Candidate AST traversal, including lexical cfg transitions.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprField, ExprMacro, ExprMethodCall, FnArg, Item, Local, Member,
    Pat, Path,
};

use super::super::{
    BridgeEvidenceKind, CandidateAnalyzer, add_use_to_symbols, collect_use_bindings,
    last_path_ident, normalized_tokens, sides_in_type_with_cfg, validate_cfg,
};
use crate::ownership::extend_cfg_context;

impl CandidateAnalyzer<'_> {
    pub(in crate::bridge_access) fn apply_local_imports(&mut self) {
        for (guard, import) in self.local_imports.clone() {
            if super::combine_cfg(&self.active_cfg, &guard).is_none() {
                continue;
            }
            add_use_to_symbols(&import, &mut self.symbols, self.errors);
            if super::uncovered_cfg(&self.active_cfg, &[guard]).is_some() {
                let mut bindings = Vec::new();
                let mut globs = Vec::new();
                collect_use_bindings(&import.tree, &mut Vec::new(), &mut bindings, &mut globs);
                for (name, _) in bindings {
                    self.symbols.bind_local_import(
                        &name, BTreeSet::new(), Some(format!(
                            "cannot resolve bridge provenance for {} local import {name}: conditional binding under enclosing cfg",
                            self.enclosing,
                        )),
                    );
                }
            }
        }
    }

    pub(in crate::bridge_access) fn within_cfg(
        &mut self,
        attrs: &[Attribute],
        visit: impl FnOnce(&mut Self),
    ) {
        if !attrs
            .iter()
            .any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
        {
            visit(self);
            return;
        }
        let next = extend_cfg_context(&self.active_cfg, attrs);
        if next == self.active_cfg {
            visit(self);
            return;
        }
        validate_cfg(&self.active_cfg, attrs, &self.enclosing, self.errors);
        let previous_cfg = self.active_cfg.clone();
        let previous_symbols = std::mem::take(&mut self.symbols);
        self.set_active_cfg(next);
        visit(self);
        self.active_cfg = previous_cfg;
        self.symbols = previous_symbols;
    }

    pub(in crate::bridge_access) fn report_missing_glob_path(
        &mut self,
        path: &Path,
        value_position: bool,
    ) {
        let Some(first) = path
            .segments
            .first()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if self.generic_parameters.contains(&first)
            || (value_position && self.variables.contains_key(&first))
            || first == "Self"
        {
            return;
        }
        let key = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if let Some(issue) = self.symbols.unresolved_glob_paths.get(&key)
            && self.reported_resolution_issues.insert(key)
        {
            self.errors.push(issue.clone());
        }
    }

    pub(in crate::bridge_access) fn report_path_issue(&mut self, path: &Path) {
        let key = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if let Some(issue) = self.symbols.path_issue_with_cfg(path, &self.active_cfg)
            && self.reported_resolution_issues.insert(key)
        {
            self.errors.push(issue.to_owned());
        }
    }
}

impl<'ast> Visit<'ast> for CandidateAnalyzer<'_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let previous_symbols = self.symbols.clone();
        let inherited_imports = self.local_imports.len();
        for statement in &block.stmts {
            if let syn::Stmt::Item(Item::Use(import)) = statement {
                self.local_imports.push((
                    extend_cfg_context(&self.active_cfg, &import.attrs),
                    import.clone(),
                ));
            }
        }
        self.apply_local_imports();
        visit::visit_block(self, block);
        self.local_imports.truncate(inherited_imports);
        self.symbols = previous_symbols;
    }

    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.within_cfg(
            super::cfg_context::expression_attributes(expression),
            |analyzer| {
                visit::visit_expr(analyzer, expression);
            },
        );
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        self.within_cfg(
            super::cfg_context::statement_attributes(statement),
            |analyzer| {
                visit::visit_stmt(analyzer, statement);
            },
        );
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        self.within_cfg(&field.attrs, |analyzer| visit::visit_field(analyzer, field));
    }

    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        self.within_cfg(&variant.attrs, |analyzer| {
            visit::visit_variant(analyzer, variant)
        });
    }

    fn visit_field_value(&mut self, field: &'ast syn::FieldValue) {
        self.within_cfg(&field.attrs, |analyzer| {
            visit::visit_field_value(analyzer, field)
        });
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.within_cfg(&arm.attrs, |analyzer| visit::visit_arm(analyzer, arm));
    }

    fn visit_fn_arg(&mut self, argument: &'ast FnArg) {
        self.within_cfg(
            super::cfg_context::argument_attributes(argument),
            |analyzer| {
                visit::visit_fn_arg(analyzer, argument);
            },
        );
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        self.report_path_issue(&path.path);
        self.report_missing_glob_path(&path.path, false);
        let sides = self
            .symbols
            .sides_for_path_with_cfg(&path.path, &self.active_cfg);
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
        let local_value = path.path.segments.len() == 1
            && path
                .path
                .segments
                .first()
                .is_some_and(|segment| self.variables.contains_key(&segment.ident.to_string()));
        if !local_value {
            self.report_path_issue(&path.path);
        }
        self.report_missing_glob_path(&path.path, true);
        let mut sides = self
            .symbols
            .sides_for_path_with_cfg(&path.path, &self.active_cfg);
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
            let sides = self
                .symbols
                .sides_for_path_with_cfg(&path.path, &self.active_cfg);
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
            sides.extend(sides_in_type_with_cfg(
                &self.symbols,
                &typed.ty,
                &self.active_cfg,
            ));
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
            // Rust block imports are in scope throughout the block. They
            // were bound on block entry, including their cfg guard.
            Item::Use(_) => {}
            Item::Macro(item_macro) => {
                self.audit_macro(&item_macro.mac, "nested item macro");
            }
            // Nested items have their own enclosing identity and must be passed
            // as source/module items rather than folded into this function.
            _ => {}
        }
    }
}
