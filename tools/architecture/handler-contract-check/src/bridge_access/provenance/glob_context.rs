//! Identify unavailable relative glob context without guessing imported names.

use std::collections::BTreeSet;

use super::{Resolver, combine_cfg, uncovered_cfg};

impl Resolver<'_> {
    pub(super) fn missing_relative_globs(
        &self,
        node: usize,
        cfg: &[String],
        active: &mut BTreeSet<usize>,
    ) -> BTreeSet<String> {
        if !active.insert(node) {
            return BTreeSet::new();
        }
        let mut missing = BTreeSet::new();
        for glob in &self.scopes[node].globs {
            let Some(branch_cfg) = combine_cfg(cfg, &glob.cfg) else {
                continue;
            };
            let targets = self.module_targets(node, &glob.path);
            if targets.is_empty() {
                if glob.path.first().is_some_and(|first| {
                    matches!(first.as_str(), "crate" | "self" | "super")
                        || self.scopes[node].declarations.contains_key(first)
                        || self.scopes[node].explicit.contains_key(first)
                }) {
                    missing.insert(format!(
                        "{}::use {}::*",
                        self.index.modules[node].module,
                        glob.path.join("::"),
                    ));
                }
            } else {
                let mut available_cfg = Vec::new();
                for target in targets {
                    if let Some(target_cfg) =
                        combine_cfg(&branch_cfg, &self.index.modules[target].cfg)
                    {
                        available_cfg.push(target_cfg.clone());
                        missing.extend(self.missing_relative_globs(target, &target_cfg, active));
                    }
                }
                if uncovered_cfg(&branch_cfg, &available_cfg).is_some() {
                    missing.insert(format!(
                        "{}::use {}::* (unavailable under enclosing cfg)",
                        self.index.modules[node].module,
                        glob.path.join("::"),
                    ));
                }
            }
        }
        active.remove(&node);
        missing
    }
}
