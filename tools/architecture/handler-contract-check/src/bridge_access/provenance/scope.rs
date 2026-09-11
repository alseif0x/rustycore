//! Declarations, imports and cfg guards used by bridge provenance resolution.

use std::collections::BTreeMap;

use syn::{Attribute, Item, Type, UseTree};

use super::super::{
    BridgeSide, Symbols, bridge_capable_namespace_import, sides_for_segments, validate_cfg,
};
use super::source_graph::IndexedModule;
use crate::ownership::{
    cfg_context_allows_production, cfg_context_allows_test, extend_cfg_context,
};

#[derive(Clone)]
pub(super) enum LocalBinding {
    NonAuthority,
    Authority(BridgeSide),
    Module(String),
    Alias(Type),
}

#[derive(Clone)]
pub(super) struct DeclaredBinding {
    pub(super) binding: LocalBinding,
    pub(super) cfg: Vec<String>,
}

#[derive(Clone)]
pub(super) struct ImportBinding {
    pub(super) path: Vec<String>,
    pub(super) cfg: Vec<String>,
}

#[derive(Clone)]
pub(super) struct GlobBinding {
    pub(super) path: Vec<String>,
    pub(super) cfg: Vec<String>,
}

#[derive(Clone, Default)]
pub(super) struct ModuleScope {
    pub(super) declarations: BTreeMap<String, Vec<DeclaredBinding>>,
    pub(super) explicit: BTreeMap<String, Vec<ImportBinding>>,
    pub(super) globs: Vec<GlobBinding>,
    pub(super) builtins: BTreeMap<String, BridgeSide>,
}

pub(super) fn collect_use_specs(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    explicit: &mut Vec<(String, Vec<String>)>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_specs(&path.tree, prefix, explicit, globs);
            prefix.pop();
        }
        UseTree::Name(name) => {
            if name.ident == "self" {
                if let Some(local) = prefix.last() {
                    explicit.push((local.to_string(), prefix.clone()));
                }
            } else {
                let mut path = prefix.clone();
                path.push(name.ident.to_string());
                explicit.push((name.ident.to_string(), path));
            }
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            if rename.ident != "self" {
                path.push(rename.ident.to_string());
            }
            explicit.push((rename.rename.to_string(), path));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_specs(item, prefix, explicit, globs);
            }
        }
        UseTree::Glob(_) => globs.push(prefix.clone()),
    }
}

fn cfg_for_binding(parent: &[String], attrs: &[Attribute]) -> Option<Vec<String>> {
    let cfg = extend_cfg_context(parent, attrs);
    let production = cfg_context_allows_production(&cfg, &[]).ok()?;
    let test = cfg_context_allows_test(&cfg, &[]).ok()?;
    (production || test).then_some(cfg)
}

pub(super) fn collect_scope(module: &IndexedModule, errors: &mut Vec<String>) -> ModuleScope {
    let mut scope = ModuleScope::default();
    for (name, sides) in Symbols::for_module(&module.package, &module.module).named {
        if let Some(side) = sides.iter().next().copied() {
            scope.builtins.insert(name, side);
        }
    }
    for item in &module.items {
        let attrs = item_attributes(item);
        validate_cfg(&module.cfg, attrs, &module.module, errors);
        let Some(cfg) = cfg_for_binding(&module.cfg, attrs) else {
            continue;
        };
        match item {
            Item::Use(item_use) => {
                let mut explicit = Vec::new();
                let mut globs = Vec::new();
                collect_use_specs(&item_use.tree, &mut Vec::new(), &mut explicit, &mut globs);
                for (local, path) in explicit {
                    if sides_for_segments(
                        &Symbols::for_module(&module.package, &module.module),
                        &path,
                    )
                    .is_empty()
                        && bridge_capable_namespace_import(&path)
                        && path.first().is_some_and(|root| {
                            !matches!(root.as_str(), "crate" | "self" | "super")
                        })
                        && !(path.len() == 1 && local == path[0])
                    {
                        errors.push(format!(
                            "bridge-capable namespace import `{}` as `{local}` hides exact legacy/canonical symbols",
                            path.join("::")
                        ));
                    }
                    scope
                        .explicit
                        .entry(local.clone())
                        .or_default()
                        .push(ImportBinding {
                            path,
                            cfg: cfg.clone(),
                        });
                }
                for path in globs {
                    if path.first().is_some_and(|root| {
                        matches!(root.as_str(), "wow_map" | "wow_entities" | "wow_world")
                    }) {
                        errors.push(format!(
                            "bridge-capable glob import `{}` hides exact legacy/canonical symbols",
                            path.join("::")
                        ));
                    }
                    scope.globs.push(GlobBinding {
                        path,
                        cfg: cfg.clone(),
                    });
                }
            }
            Item::Type(item_type) => {
                let binding = if module.package == "wow-world"
                    && module.module == "crate::map_manager"
                    && matches!(
                        item_type.ident.to_string().as_str(),
                        "MapManager" | "WorldCreature" | "SharedMapManager"
                    ) {
                    LocalBinding::Authority(BridgeSide::Legacy)
                } else {
                    LocalBinding::Alias((*item_type.ty).clone())
                };
                scope
                    .declarations
                    .entry(item_type.ident.to_string())
                    .or_default()
                    .push(DeclaredBinding { binding, cfg });
            }
            _ => {
                if let Some(name) = item_declared_name(item) {
                    let binding = if module.package == "wow-world"
                        && module.module == "crate::map_manager"
                        && matches!(
                            name.as_str(),
                            "MapManager" | "WorldCreature" | "SharedMapManager"
                        ) {
                        LocalBinding::Authority(BridgeSide::Legacy)
                    } else if matches!(item, Item::Mod(_)) {
                        LocalBinding::Module(format!("{}::{name}", module.module))
                    } else {
                        LocalBinding::NonAuthority
                    };
                    scope
                        .declarations
                        .entry(name)
                        .or_default()
                        .push(DeclaredBinding { binding, cfg });
                }
            }
        }
    }
    scope
}

pub(super) fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        Item::Verbatim(_) => &[],
        _ => &[],
    }
}

fn item_declared_name(item: &Item) -> Option<String> {
    match item {
        Item::Const(item) => Some(item.ident.to_string()),
        Item::Enum(item) => Some(item.ident.to_string()),
        Item::ExternCrate(item) => Some(item.ident.to_string()),
        Item::Fn(item) => Some(item.sig.ident.to_string()),
        Item::Macro(item) => item.ident.as_ref().map(ToString::to_string),
        Item::Mod(item) => Some(item.ident.to_string()),
        Item::Static(item) => Some(item.ident.to_string()),
        Item::Struct(item) => Some(item.ident.to_string()),
        Item::Trait(item) => Some(item.ident.to_string()),
        Item::TraitAlias(item) => Some(item.ident.to_string()),
        Item::Type(item) => Some(item.ident.to_string()),
        Item::Union(item) => Some(item.ident.to_string()),
        _ => None,
    }
}
