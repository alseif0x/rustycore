//! Distinguish a proven local inventory data module from registration capability.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::{Item, UseTree, visit::Visit};

use super::{
    ident_is, use_tree_can_alias_expected_registration_macro, use_tree_can_alias_inventory_submit,
};

/// Production reverse dependency closure of every resolved `inventory` crate.
/// Package identity, rather than the dependency's local alias, controls capability.
pub(crate) fn inventory_dependency_packages(
    metadata: &serde_json::Value,
) -> Result<BTreeSet<String>, String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing metadata packages")?;
    let mut capable = BTreeSet::new();
    let mut package_ids = BTreeSet::new();
    for package in packages {
        let id = package["id"]
            .as_str()
            .ok_or("package without id")?
            .to_owned();
        if !package_ids.insert(id.clone()) {
            return Err(format!("duplicate package {id}"));
        }
        if package["name"].as_str().ok_or("package without name")? == "inventory" {
            capable.insert(id);
        }
    }
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("missing resolved metadata graph")?;
    let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in nodes {
        let id = node["id"].as_str().ok_or("resolved node without id")?;
        if !package_ids.contains(id) {
            return Err(format!("unknown resolved package {id}"));
        }
        for dependency in node["deps"]
            .as_array()
            .ok_or("resolved node without deps")?
        {
            let provider = dependency["pkg"]
                .as_str()
                .ok_or("dependency without package id")?;
            if !package_ids.contains(provider) {
                return Err(format!("unknown inventory capability provider {provider}"));
            }
            let kinds = dependency["dep_kinds"]
                .as_array()
                .ok_or("dependency without kinds")?;
            if kinds.is_empty() {
                return Err("dependency without kinds".to_owned());
            }
            let mut normal = false;
            for kind in kinds {
                match kind.get("kind") {
                    Some(serde_json::Value::Null) => normal = true,
                    Some(serde_json::Value::String(value))
                        if matches!(value.as_str(), "dev" | "build") => {}
                    _ => return Err("unsupported inventory dependency kind".to_owned()),
                }
            }
            if normal {
                reverse
                    .entry(provider.to_owned())
                    .or_default()
                    .insert(id.to_owned());
            }
        }
    }
    let mut pending = capable.iter().cloned().collect::<Vec<_>>();
    while let Some(provider) = pending.pop() {
        for dependent in reverse.get(&provider).into_iter().flatten() {
            if capable.insert(dependent.clone()) {
                pending.push(dependent.clone());
            }
        }
    }
    Ok(capable)
}

#[derive(Default)]
struct InventoryAliasCollector {
    violations: Vec<String>,
    allow_local_data_module: bool,
    local_inventory: Vec<bool>,
}

impl InventoryAliasCollector {
    fn enter_scope(&mut self, items: &[Item]) {
        self.local_inventory.push(
            items.iter().any(
                |item| matches!(item, Item::Mod(module) if ident_is(&module.ident, "inventory")),
            ),
        );
    }

    fn is_local_data_glob(&self, item: &syn::ItemUse) -> bool {
        self.allow_local_data_module
            && self.local_inventory.last() == Some(&true)
            && item.leading_colon.is_none()
            && matches!(&item.tree, UseTree::Path(path)
                if ident_is(&path.ident, "inventory") && matches!(&*path.tree, UseTree::Glob(_)))
    }
}

impl<'ast> Visit<'ast> for InventoryAliasCollector {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.enter_scope(&file.items);
        syn::visit::visit_file(self, file);
        self.local_inventory.pop();
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if use_tree_can_alias_inventory_submit(&item.tree) && !self.is_local_data_glob(item) {
            self.violations.push(format!(
                "import {} can alias an inventory registration macro; use only canonical \
                 inventory::collect!/inventory::submit! paths",
                item.to_token_stream()
            ));
        }
        if use_tree_can_alias_expected_registration_macro(&item.tree) {
            self.violations.push(format!(
                "import {} aliases or reexports an audited handler registration macro; \
                 registration macros must remain private to the declared handler-registration owner and use their \
                 unqualified audited names", item.to_token_stream()
            ));
        }
        syn::visit::visit_item_use(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        if ident_is(&item.ident, "inventory")
            || item
                .rename
                .as_ref()
                .is_some_and(|(_, rename)| ident_is(rename, "inventory"))
        {
            self.violations.push(format!(
                "{} is not allowed because #[macro_use] or a crate alias can hide inventory \
                 registration macros; use only canonical qualified paths",
                item.to_token_stream()
            ));
        }
        syn::visit::visit_item_extern_crate(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if ident_is(&item.ident, "inventory") && !self.allow_local_data_module {
            self.violations.push(format!(
                "module {} shadows the canonical inventory crate namespace",
                item.ident
            ));
        }
        if let Some((_, items)) = &item.content {
            self.enter_scope(items);
            syn::visit::visit_item_mod(self, item);
            self.local_inventory.pop();
        }
    }
}

pub(crate) fn registration_alias_violations(source: &str) -> Result<Vec<String>, String> {
    collect(source, false)
}

/// Only for packages outside the handler registry closure and without any normal
/// dependency path to inventory. The caller still audits every source's macros,
/// includes, exports and registration invocations; this is no package exemption.
pub(crate) fn data_module_alias_violations(source: &str) -> Result<Vec<String>, String> {
    collect(source, true)
}

fn collect(source: &str, allow_local_data_module: bool) -> Result<Vec<String>, String> {
    let syntax =
        syn::parse_file(source).map_err(|error| format!("cannot parse Rust source: {error}"))?;
    let mut collector = InventoryAliasCollector {
        allow_local_data_module,
        ..Default::default()
    };
    collector.visit_file(&syntax);
    Ok(collector.violations)
}

#[cfg(test)]
mod tests;
