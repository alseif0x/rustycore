//! Render named local initializers and a conservative persistence projection.
use quote::ToTokens;
use std::{
    env,
    io::{self, Read},
    process::ExitCode,
};
use syn::{
    Expr, ExprLit, ExprStruct, Item, Lit, Local, Member, Pat,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
};

struct Binding<'a> {
    name: &'a str,
    project_config: bool,
    values: Vec<(String, String)>,
}

impl<'ast> Visit<'ast> for Binding<'_> {
    // A nested item is a different owner.
    fn visit_item(&mut self, _: &'ast Item) {}

    fn visit_local(&mut self, local: &'ast Local) {
        if let Pat::Ident(pat) = &local.pat
            && pat.ident == self.name
            && pat.subpat.is_none()
            && let Some(init) = &local.init
        {
            let spelling = format!("{}={}", pat.ident, init.expr.to_token_stream());
            let projection = initializer_projection(&init.expr, self.project_config);
            self.values.push((spelling, projection));
        }
        visit::visit_local(self, local);
    }
}

struct PersistenceShape {
    project_config: bool,
}

impl VisitMut for PersistenceShape {
    fn visit_expr_struct_mut(&mut self, node: &mut ExprStruct) {
        let is_session_config =
            node.path.segments.len() == 1
                && node.path.segments.first().is_some_and(|segment| {
                    segment.ident == "SessionRuntimePolicyCapabilitiesLikeCpp"
                });
        if self.project_config
            && is_session_config
            && node.attrs.is_empty()
            && node.qself.is_none()
            && node.rest.is_none()
        {
            node.fields = node
                .fields
                .iter()
                .filter(|field| {
                    !field.attrs.is_empty()
                        || !matches!(&field.member, Member::Named(_))
                        || !is_config_bool(&field.expr)
                })
                .cloned()
                .collect();
        }
        visit_mut::visit_expr_struct_mut(self, node);
    }
}

fn initializer_projection(expr: &Expr, project_config: bool) -> String {
    if let Some(scalar) = scalar_shape(expr) {
        return scalar;
    }
    let mut projected = expr.clone();
    PersistenceShape { project_config }.visit_expr_mut(&mut projected);
    projected.to_token_stream().to_string()
}

fn scalar_shape(expr: &Expr) -> Option<String> {
    let Expr::Lit(ExprLit { lit, .. }) = expr else {
        return None;
    };
    match lit {
        Lit::Int(value) => Some(format!("scalar:int:{}", value.suffix())),
        Lit::Float(value) => Some(format!("scalar:float:{}", value.suffix())),
        Lit::Bool(_) => Some("scalar:bool".into()),
        _ => None,
    }
}

fn is_config_bool(expr: &Expr) -> bool {
    let Expr::Call(call) = expr else { return false };
    let Expr::Path(function) = &*call.func else {
        return false;
    };
    if function.qself.is_some()
        || function.path.segments.len() != 1
        || function.path.segments[0].ident != "world_config_bool"
        || call.args.len() != 3
    {
        return false;
    }
    let world_configs = matches!(
        &call.args[0],
        Expr::Reference(reference)
            if reference.mutability.is_none()
                && matches!(&*reference.expr, Expr::Path(path)
                    if path.qself.is_none()
                        && path.path.segments.len() == 1
                        && path.path.segments[0].ident == "world_configs")
    );
    let config_key = matches!(&call.args[1], Expr::Lit(ExprLit { lit: Lit::Str(value), .. })
        if value.value().starts_with("CONFIG_") && value.value().len() > "CONFIG_".len());
    let default_is_bool = matches!(
        &call.args[2],
        Expr::Lit(ExprLit {
            lit: Lit::Bool(_),
            ..
        })
    );
    world_configs && config_key && default_is_bool
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let (function, name, project_config) = match args.as_slice() {
        [function, name] => (function, name, false),
        [function, name, flag] if flag == "--session-config" => (function, name, true),
        _ => return Err("expected a function, binding and optional --session-config".into()),
    };
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|e| e.to_string())?;
    let syntax = syn::parse_file(&source).map_err(|e| e.to_string())?;
    let mut functions = syntax.items.iter().filter_map(|item| match item {
        Item::Fn(item) if item.sig.ident == function.as_str() => Some(item),
        _ => None,
    });
    let owner = functions.next().ok_or("function absent")?;
    if functions.next().is_some() {
        return Err("ambiguous function".into());
    }
    let mut binding = Binding {
        name,
        project_config,
        values: Vec::new(),
    };
    binding.visit_block(&owner.block);
    if binding.values.is_empty()
        || binding
            .values
            .iter()
            .any(|(spelling, projection)| spelling.contains('\0') || projection.contains('\0'))
    {
        return Err("binding unavailable or spelling contains a NUL byte".into());
    }
    for (spelling, projection) in &binding.values {
        print!("{spelling}\0{projection}\0");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::from(2)
        }
    }
}
