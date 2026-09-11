//! Collect the same nested cfg contexts that candidate traversal enters.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{Attribute, Expr, FnArg, ImplItem, Item};

use super::scope::item_attributes;
use crate::ownership::extend_cfg_context;

pub(super) fn expression_attributes(expression: &Expr) -> &[Attribute] {
    match expression {
        Expr::Array(value) => &value.attrs,
        Expr::Assign(value) => &value.attrs,
        Expr::Async(value) => &value.attrs,
        Expr::Await(value) => &value.attrs,
        Expr::Binary(value) => &value.attrs,
        Expr::Block(value) => &value.attrs,
        Expr::Break(value) => &value.attrs,
        Expr::Call(value) => &value.attrs,
        Expr::Cast(value) => &value.attrs,
        Expr::Closure(value) => &value.attrs,
        Expr::Const(value) => &value.attrs,
        Expr::Continue(value) => &value.attrs,
        Expr::Field(value) => &value.attrs,
        Expr::ForLoop(value) => &value.attrs,
        Expr::Group(value) => &value.attrs,
        Expr::If(value) => &value.attrs,
        Expr::Index(value) => &value.attrs,
        Expr::Infer(value) => &value.attrs,
        Expr::Let(value) => &value.attrs,
        Expr::Lit(value) => &value.attrs,
        Expr::Loop(value) => &value.attrs,
        Expr::Macro(value) => &value.attrs,
        Expr::Match(value) => &value.attrs,
        Expr::MethodCall(value) => &value.attrs,
        Expr::Paren(value) => &value.attrs,
        Expr::Path(value) => &value.attrs,
        Expr::Range(value) => &value.attrs,
        Expr::RawAddr(value) => &value.attrs,
        Expr::Reference(value) => &value.attrs,
        Expr::Repeat(value) => &value.attrs,
        Expr::Return(value) => &value.attrs,
        Expr::Struct(value) => &value.attrs,
        Expr::Try(value) => &value.attrs,
        Expr::TryBlock(value) => &value.attrs,
        Expr::Tuple(value) => &value.attrs,
        Expr::Unary(value) => &value.attrs,
        Expr::Unsafe(value) => &value.attrs,
        Expr::While(value) => &value.attrs,
        Expr::Yield(value) => &value.attrs,
        _ => &[],
    }
}

pub(super) fn statement_attributes(statement: &syn::Stmt) -> &[Attribute] {
    match statement {
        syn::Stmt::Local(local) => &local.attrs,
        syn::Stmt::Macro(mac) => &mac.attrs,
        syn::Stmt::Item(item) => item_attributes(item),
        syn::Stmt::Expr(_, _) => &[],
    }
}

pub(super) fn argument_attributes(argument: &FnArg) -> &[Attribute] {
    match argument {
        FnArg::Receiver(receiver) => &receiver.attrs,
        FnArg::Typed(typed) => &typed.attrs,
    }
}

fn impl_item_attributes(item: &ImplItem) -> &[Attribute] {
    match item {
        ImplItem::Fn(item) => &item.attrs,
        ImplItem::Const(item) => &item.attrs,
        ImplItem::Type(item) => &item.attrs,
        ImplItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

struct ContextCollector {
    current: Vec<String>,
    contexts: BTreeSet<Vec<String>>,
}

impl ContextCollector {
    fn within(&mut self, attrs: &[Attribute], visit: impl FnOnce(&mut Self)) {
        if !attrs
            .iter()
            .any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
        {
            visit(self);
            return;
        }
        let next = extend_cfg_context(&self.current, attrs);
        if next == self.current {
            visit(self);
            return;
        }
        self.contexts.insert(next.clone());
        let previous = std::mem::replace(&mut self.current, next);
        visit(self);
        self.current = previous;
    }
}

impl<'ast> Visit<'ast> for ContextCollector {
    fn visit_item(&mut self, item: &'ast Item) {
        if matches!(item, Item::Mod(_)) {
            return;
        }
        self.within(item_attributes(item), |collector| {
            visit::visit_item(collector, item)
        });
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        self.within(impl_item_attributes(item), |collector| {
            visit::visit_impl_item(collector, item)
        });
    }

    fn visit_expr(&mut self, expression: &'ast Expr) {
        self.within(expression_attributes(expression), |collector| {
            visit::visit_expr(collector, expression);
        });
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        self.within(statement_attributes(statement), |collector| {
            visit::visit_stmt(collector, statement);
        });
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        self.within(&field.attrs, |collector| {
            visit::visit_field(collector, field)
        });
    }

    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        self.within(&variant.attrs, |collector| {
            visit::visit_variant(collector, variant)
        });
    }

    fn visit_field_value(&mut self, field: &'ast syn::FieldValue) {
        self.within(&field.attrs, |collector| {
            visit::visit_field_value(collector, field)
        });
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.within(&arm.attrs, |collector| visit::visit_arm(collector, arm));
    }

    fn visit_fn_arg(&mut self, argument: &'ast FnArg) {
        self.within(argument_attributes(argument), |collector| {
            visit::visit_fn_arg(collector, argument)
        });
    }
}

pub(super) fn item_cfg_contexts(items: &[Item], cfg: &[String]) -> BTreeSet<Vec<String>> {
    let mut collector = ContextCollector {
        current: cfg.to_vec(),
        contexts: BTreeSet::from([cfg.to_vec()]),
    };
    for item in items {
        collector.visit_item(item);
    }
    collector.contexts
}
