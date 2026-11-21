//! Render named local initializers in source order using Rust's parsed token spelling.
use quote::ToTokens;
use std::{
    env,
    io::{self, Read},
    process::ExitCode,
};
use syn::{
    Item, Local, Pat,
    visit::{self, Visit},
};

struct Binding<'a> {
    name: &'a str,
    values: Vec<String>,
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
            self.values
                .push(format!("{}={}", pat.ident, init.expr.to_token_stream()));
        }
        visit::visit_local(self, local);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [function, name] = args.as_slice() else {
        return Err("expected a free function name and a binding name".into());
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
        values: Vec::new(),
    };
    binding.visit_block(&owner.block);
    if binding.values.is_empty() || binding.values.iter().any(|value| value.contains('\0')) {
        return Err("binding unavailable or spelling contains a NUL byte".into());
    }
    for initializer in &binding.values {
        print!("{initializer}\0");
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
