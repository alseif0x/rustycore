//! Supplied source mounts and their lexical inline module relationships.

use std::collections::BTreeMap;

use syn::{Item, ItemMod};

use super::super::BridgeSource;
use crate::ownership::extend_cfg_context;

#[derive(Clone)]
pub(in crate::bridge_access) struct IndexedModule {
    pub(in crate::bridge_access) package: String,
    pub(in crate::bridge_access) module: String,
    pub(in crate::bridge_access) source_path: String,
    pub(in crate::bridge_access) cfg: Vec<String>,
    pub(in crate::bridge_access) items: Vec<Item>,
}

#[derive(Default)]
pub(in crate::bridge_access) struct ModuleIndex {
    pub(in crate::bridge_access) modules: Vec<IndexedModule>,
    pub(super) by_module: BTreeMap<(String, String), Vec<usize>>,
}

impl ModuleIndex {
    pub(super) fn add_source(&mut self, source: BridgeSource<'_>, syntax: syn::File) {
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        self.add_module(
            source.package.to_owned(),
            source.module.to_owned(),
            source.source_path.to_owned(),
            cfg,
            syntax.items,
        );
    }

    fn add_module(
        &mut self,
        package: String,
        module: String,
        source_path: String,
        cfg: Vec<String>,
        items: Vec<Item>,
    ) {
        let id = self.modules.len();
        self.by_module
            .entry((package.clone(), module.clone()))
            .or_default()
            .push(id);
        self.modules.push(IndexedModule {
            package: package.clone(),
            module: module.clone(),
            source_path: source_path.clone(),
            cfg: cfg.clone(),
            items: items.clone(),
        });

        // Inline and external modules are intentionally entered through the
        // same graph.  External children arrive as their own BridgeSource;
        // inline children use this source mount and inherit its cfg.
        for item in items {
            let Item::Mod(ItemMod {
                attrs,
                ident,
                content: Some((_, child_items)),
                ..
            }) = item
            else {
                continue;
            };
            let child_cfg = extend_cfg_context(&cfg, &attrs);
            self.add_module(
                package.clone(),
                format!("{module}::{ident}"),
                source_path.clone(),
                child_cfg,
                child_items,
            );
        }
    }
}
