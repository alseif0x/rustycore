//! Resolve authority provenance from the supplied lexical module graph.
//!
//! Resolution is keyed by the enclosing item's cfg, including negative guards
//! for shadowed globs. Unknown relative provenance survives aliases and is only
//! reported when a candidate actually uses it. This is not a Rust type checker.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::{Item, ItemMod, Path, Type};

use super::{BridgeSide, BridgeSource, Symbols};
use crate::ownership::{cfg_context_allows_production, cfg_context_allows_test};

mod candidate_visitor;
mod cfg_context;
mod glob_context;
mod scope;
mod source_graph;
use scope::{LocalBinding, ModuleScope, collect_scope, collect_use_specs};
pub(super) use source_graph::ModuleIndex;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Provenance {
    Authority(BTreeSet<BridgeSide>),
    Module(String),
    NonAuthority,
    Unknown,
}

#[derive(Clone, Debug)]
struct Candidate {
    provenance: Provenance,
    cfg: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct Resolution {
    candidates: Vec<Candidate>,
    issues: BTreeSet<String>,
    // A glob search may revisit a root before reaching the defining child.
    // Keep that back-edge pending until the complete search has a source.
    cycles: BTreeSet<String>,
}

impl Resolution {
    fn one(provenance: Provenance, cfg: &[String]) -> Self {
        Self {
            candidates: vec![Candidate {
                provenance,
                cfg: cfg.to_vec(),
            }],
            issues: BTreeSet::new(),
            cycles: BTreeSet::new(),
        }
    }

    fn append(&mut self, other: Self) {
        self.candidates.extend(other.candidates);
        self.issues.extend(other.issues);
        self.cycles.extend(other.cycles);
    }
}

type ResolutionKey = (usize, String, Vec<String>);

struct Resolver<'a> {
    index: &'a ModuleIndex,
    scopes: Vec<ModuleScope>,
    memo: BTreeMap<ResolutionKey, Resolution>,
    active: BTreeSet<ResolutionKey>,
}

fn canonical_cfg(cfg: &[String]) -> Vec<String> {
    let mut cfg = cfg.to_vec();
    cfg.sort();
    cfg.dedup();
    cfg
}

fn possible(cfg: &[String]) -> bool {
    cfg_context_allows_production(cfg, &[]).unwrap_or(true)
        || cfg_context_allows_test(cfg, &[]).unwrap_or(true)
}

fn combine_cfg(left: &[String], right: &[String]) -> Option<Vec<String>> {
    let mut cfg = left.to_vec();
    cfg.extend_from_slice(right);
    let cfg = canonical_cfg(&cfg);
    possible(&cfg).then_some(cfg)
}

// Convert a cfg/cfg_attr presence condition to a predicate that can be negated.
// The shared ownership solver still decides satisfiability; this only expresses
// "none of these stronger lexical bindings exists" for fallback to a glob.
fn presence_predicate(meta: &syn::Meta) -> Option<String> {
    use syn::parse::Parser;
    let syn::Meta::List(list) = meta else {
        return Some("all()".to_owned());
    };
    if list.path.is_ident("cfg") {
        return Some(list.tokens.to_string());
    }
    if !list.path.is_ident("cfg_attr") {
        return Some("all()".to_owned());
    }
    let metas = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?;
    let mut metas = metas.iter();
    let condition = metas.next()?.to_token_stream().to_string();
    let effects = metas.map(presence_predicate).collect::<Option<Vec<_>>>()?;
    Some(format!(
        "any(not({condition}), all({}))",
        effects.join(", ")
    ))
}

fn uncovered_cfg(context: &[String], guards: &[Vec<String>]) -> Option<Vec<String>> {
    if guards.is_empty() {
        return Some(context.to_vec());
    }
    let alternatives = guards
        .iter()
        .map(|guard| {
            let predicates = guard
                .iter()
                .map(|text| {
                    syn::parse_str::<syn::Meta>(text)
                        .ok()
                        .and_then(|meta| presence_predicate(&meta))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(format!("all({})", predicates.join(", ")))
        })
        .collect::<Option<Vec<_>>>();
    // Invalid cfg is independently rejected by the inventory's validation.
    let alternatives = alternatives?;
    combine_cfg(
        context,
        &[format!("cfg(not(any({})))", alternatives.join(", "))],
    )
}

impl<'a> Resolver<'a> {
    fn new(index: &'a ModuleIndex, errors: &mut Vec<String>) -> Self {
        Self {
            index,
            scopes: index
                .modules
                .iter()
                .map(|module| collect_scope(module, errors))
                .collect(),
            memo: BTreeMap::new(),
            active: BTreeSet::new(),
        }
    }

    fn diagnostic(&self, node: usize, name: &str, reason: &str) -> String {
        let module = &self.index.modules[node];
        format!(
            "cannot resolve bridge provenance for {} {} {} name `{name}`: {reason}",
            module.package, module.module, module.source_path
        )
    }

    fn unresolved(&self, node: usize, name: &str, cfg: &[String], reason: &str) -> Resolution {
        let mut result = Resolution::one(Provenance::Unknown, cfg);
        result.issues.insert(self.diagnostic(node, name, reason));
        result
    }

    fn resolve_name(&mut self, node: usize, name: &str, cfg: &[String]) -> Resolution {
        let Some(cfg) = combine_cfg(cfg, &self.index.modules[node].cfg) else {
            return Resolution::default();
        };
        // A parent may re-export a child that imports super::*. That ordinary
        // glob graph is cyclic, but it is not an alias cycle for every
        // unrelated identifier in the program.
        if !known_authority_name(name)
            && !self.has_visible_binding(node, name, &mut BTreeSet::new())
        {
            return Resolution::default();
        }
        let key = (node, name.to_owned(), cfg.clone());
        if let Some(result) = self.memo.get(&key) {
            return result.clone();
        }
        if !self.active.insert(key.clone()) {
            return Resolution {
                cycles: BTreeSet::from([self.diagnostic(
                    node,
                    name,
                    "relative import or type alias resolution cycle",
                )]),
                ..Resolution::default()
            };
        }

        let scope = self.scopes[node].clone();
        let mut result = Resolution::default();
        let mut guards = Vec::new();
        for declaration in scope.declarations.get(name).into_iter().flatten() {
            let Some(branch_cfg) = combine_cfg(&cfg, &declaration.cfg) else {
                continue;
            };
            guards.push(declaration.cfg.clone());
            let resolved = match &declaration.binding {
                LocalBinding::NonAuthority => {
                    Resolution::one(Provenance::NonAuthority, &branch_cfg)
                }
                LocalBinding::Authority(side) => {
                    Resolution::one(Provenance::Authority(BTreeSet::from([*side])), &branch_cfg)
                }
                LocalBinding::Module(module) => {
                    Resolution::one(Provenance::Module(module.clone()), &branch_cfg)
                }
                LocalBinding::Alias(ty) => self.resolve_type(node, ty, &branch_cfg),
            };
            result.append(resolved);
        }
        if let Some(import_cfg) = uncovered_cfg(&cfg, &guards) {
            let mut import_guards = Vec::new();
            for import in scope.explicit.get(name).into_iter().flatten() {
                let Some(branch_cfg) = combine_cfg(&import_cfg, &import.cfg) else {
                    continue;
                };
                import_guards.push(import.cfg.clone());
                let resolved = if import.path.len() > 1
                    && import.path.first().is_some_and(|first| first == name)
                    && !matches!(name, "crate" | "self" | "super")
                {
                    // In use anyhow::{Result, anyhow}, the imported terminal
                    // name does not replace its own external namespace root.
                    Resolution::one(
                        recognized_absolute_provenance(&import.path)
                            .unwrap_or(Provenance::NonAuthority),
                        &branch_cfg,
                    )
                } else {
                    self.resolve_path(node, &import.path, &branch_cfg)
                };
                result.append(resolved);
            }
            if let Some(glob_cfg) = uncovered_cfg(&import_cfg, &import_guards) {
                let mut glob_result = Resolution::default();
                for glob in &scope.globs {
                    let Some(branch_cfg) = combine_cfg(&glob_cfg, &glob.cfg) else {
                        continue;
                    };
                    let targets = self.module_targets(node, &glob.path);
                    if targets.is_empty() {
                        if known_authority_name(name)
                            && glob
                                .path
                                .first()
                                .is_some_and(|s| matches!(s.as_str(), "super" | "self"))
                        {
                            glob_result.append(self.unresolved(
                                node,
                                name,
                                &branch_cfg,
                                "a relative glob module is missing or unavailable",
                            ));
                        }
                    } else {
                        for target in targets {
                            if self.active.iter().any(|(active_node, active_name, _)| {
                                *active_node == target && active_name == name
                            }) {
                                // Glob re-export loops are ordinary Rust module
                                // wiring. An explicit import/alias edge still
                                // enters resolve_name and diagnoses its cycle.
                                glob_result.cycles.insert(self.diagnostic(
                                    target,
                                    name,
                                    "relative glob resolution cycle has no defining source yet",
                                ));
                                continue;
                            }
                            glob_result.append(self.resolve_name(target, name, &branch_cfg));
                        }
                    }
                }
                if glob_result.candidates.is_empty() {
                    if let Some(side) = scope.builtins.get(name) {
                        glob_result = Resolution::one(
                            Provenance::Authority(BTreeSet::from([*side])),
                            &glob_cfg,
                        );
                    }
                }
                if !glob_result.candidates.is_empty() {
                    let guards = glob_result
                        .candidates
                        .iter()
                        .map(|candidate| candidate.cfg.clone())
                        .collect::<Vec<_>>();
                    if uncovered_cfg(&glob_cfg, &guards).is_none() {
                        glob_result.cycles.clear();
                    }
                }
                result.append(glob_result);
            }
        }
        self.active.remove(&key);
        // An intermediate re-export can be unresolved only because its root
        // is still being searched. Caching it would make source order matter.
        if result.cycles.is_empty() {
            self.memo.insert(key, result.clone());
        }
        result
    }

    fn has_visible_binding(&self, node: usize, name: &str, active: &mut BTreeSet<usize>) -> bool {
        if !active.insert(node) {
            return false;
        }
        let scope = &self.scopes[node];
        let found = scope.declarations.contains_key(name)
            || scope.explicit.contains_key(name)
            || scope.builtins.contains_key(name)
            || scope.globs.iter().any(|glob| {
                self.module_targets(node, &glob.path)
                    .into_iter()
                    .any(|target| self.has_visible_binding(target, name, active))
            });
        active.remove(&node);
        found
    }

    fn resolve_type(&mut self, node: usize, ty: &Type, cfg: &[String]) -> Resolution {
        let mut collector = PathCollector::default();
        collector.visit_type(ty);
        // Type components coexist, whereas import candidates are alternatives.
        // Arc<Creature> aggregates Creature's side; Arc is not a conflicting
        // unknown alternative. A diagnostic from any component is preserved.
        let mut aggregate = Resolution::one(Provenance::NonAuthority, cfg);
        for path in collector.paths {
            let resolved = self.resolve_path(node, &path, cfg);
            aggregate.issues.extend(resolved.issues);
            // A cycle within a type expression is a real alias cycle, not a
            // glob forwarding edge that another defining child can discharge.
            aggregate.issues.extend(resolved.cycles);
            if resolved.candidates.is_empty() {
                continue;
            }
            let mut next = Vec::new();
            for left in &aggregate.candidates {
                for right in &resolved.candidates {
                    let Some(cfg) = combine_cfg(&left.cfg, &right.cfg) else {
                        continue;
                    };
                    let mut sides = BTreeSet::new();
                    for provenance in [&left.provenance, &right.provenance] {
                        if let Provenance::Authority(found) = provenance {
                            sides.extend(found);
                        }
                    }
                    next.push(Candidate {
                        provenance: if sides.is_empty() {
                            if matches!(left.provenance, Provenance::Unknown)
                                || matches!(right.provenance, Provenance::Unknown)
                            {
                                Provenance::Unknown
                            } else {
                                Provenance::NonAuthority
                            }
                        } else {
                            Provenance::Authority(sides)
                        },
                        cfg,
                    });
                }
            }
            aggregate.candidates = next;
        }
        aggregate
    }

    fn resolve_path(&mut self, node: usize, segments: &[String], cfg: &[String]) -> Resolution {
        let Some(first) = segments.first().map(String::as_str) else {
            return Resolution::default();
        };
        if matches!(first, "crate" | "self" | "super") {
            let Some((base, rest)) = self.relative_base(node, segments) else {
                return self.unresolved(
                    node,
                    &segments.join("::"),
                    cfg,
                    "relative path escapes crate root",
                );
            };
            return self.resolve_from_module(node, &base, rest, cfg);
        }
        // A local module/import named like an external crate takes lexical
        // precedence. Only unbound crate roots use exact external identities.
        let local = self.resolve_name(node, first, cfg);
        if !local.candidates.is_empty() || !local.issues.is_empty() || !local.cycles.is_empty() {
            return self.follow_resolution(node, local, &segments[1..], cfg);
        }
        let module = format!("{}::{first}", self.index.modules[node].module);
        if !self.module_ids(node, &module).is_empty() {
            return self.resolve_from_module(node, &module, &segments[1..], cfg);
        }
        if let Some(provenance) = recognized_absolute_provenance(segments) {
            return Resolution::one(provenance, cfg);
        }
        if segments.len() > 1 {
            // An explicit ordinary external import or enum/DTO path is known
            // not to be one of the exact authority surfaces.
            return Resolution::one(Provenance::NonAuthority, cfg);
        }
        Resolution::default()
    }

    fn follow_resolution(
        &mut self,
        node: usize,
        resolution: Resolution,
        rest: &[String],
        cfg: &[String],
    ) -> Resolution {
        let mut result = Resolution {
            candidates: Vec::new(),
            issues: resolution.issues,
            cycles: resolution.cycles,
        };
        for candidate in resolution.candidates {
            if let Provenance::Module(module) = &candidate.provenance {
                if !rest.is_empty() {
                    result.append(self.resolve_from_module(node, module, rest, &candidate.cfg));
                    continue;
                }
            }
            result.candidates.push(candidate);
        }
        let _ = cfg;
        result
    }

    fn resolve_from_module(
        &mut self,
        node: usize,
        module: &str,
        rest: &[String],
        cfg: &[String],
    ) -> Resolution {
        let targets = self.module_ids(node, module);
        if targets.is_empty() {
            return self.unresolved(
                node,
                &format!("{module}::{}", rest.join("::")),
                cfg,
                "relative module or authority declaration is unavailable",
            );
        }
        let mut result = Resolution::default();
        for target in targets {
            let Some(branch_cfg) = combine_cfg(cfg, &self.index.modules[target].cfg) else {
                continue;
            };
            if rest.is_empty() {
                result.append(Resolution::one(
                    Provenance::Module(module.to_owned()),
                    &branch_cfg,
                ));
            } else {
                let name = &rest[0];
                let resolved = self.resolve_name(target, name, &branch_cfg);
                if resolved.candidates.is_empty()
                    && resolved.issues.is_empty()
                    && resolved.cycles.is_empty()
                {
                    let child = format!("{module}::{name}");
                    if !self.module_ids(node, &child).is_empty() {
                        result.append(self.resolve_from_module(
                            node,
                            &child,
                            &rest[1..],
                            &branch_cfg,
                        ));
                    } else {
                        result.append(self.unresolved(
                            node,
                            name,
                            &branch_cfg,
                            "relative authority declaration is unavailable",
                        ));
                    }
                } else {
                    result.append(self.follow_resolution(node, resolved, &rest[1..], &branch_cfg));
                }
            }
        }
        if result.candidates.is_empty()
            && result.issues.is_empty()
            && result.cycles.is_empty()
            && possible(cfg)
        {
            return self.unresolved(
                node,
                &format!("{module}::{}", rest.join("::")),
                cfg,
                "relative declaration is unavailable under the enclosing cfg context",
            );
        }
        result
    }

    fn relative_base<'s>(&self, node: usize, path: &'s [String]) -> Option<(String, &'s [String])> {
        let mut base = self.index.modules[node].module.clone();
        let mut cursor = 1;
        match path.first()?.as_str() {
            "crate" => base = "crate".to_owned(),
            "self" => {}
            "super" => {
                base = base.rsplit_once("::")?.0.to_owned();
                while path.get(cursor).is_some_and(|segment| segment == "super") {
                    base = base.rsplit_once("::")?.0.to_owned();
                    cursor += 1;
                }
            }
            _ => return None,
        }
        Some((base, &path[cursor..]))
    }

    fn module_targets(&self, node: usize, path: &[String]) -> Vec<usize> {
        self.module_targets_inner(node, path, &mut BTreeSet::new())
    }

    fn module_targets_inner(
        &self,
        node: usize,
        path: &[String],
        active: &mut BTreeSet<(usize, Vec<String>)>,
    ) -> Vec<usize> {
        let key = (node, path.to_vec());
        if !active.insert(key.clone()) {
            return Vec::new();
        }
        let module = if path
            .first()
            .is_some_and(|s| matches!(s.as_str(), "crate" | "self" | "super"))
        {
            let Some((base, rest)) = self.relative_base(node, path) else {
                return Vec::new();
            };
            if rest.is_empty() {
                base
            } else {
                format!("{base}::{}", rest.join("::"))
            }
        } else {
            format!("{}::{}", self.index.modules[node].module, path.join("::"))
        };
        let mut targets = self.module_ids(node, &module);
        if targets.is_empty()
            && let Some(first) = path.first()
            && let Some(imports) = self.scopes[node].explicit.get(first)
        {
            for import in imports {
                let mut expanded = import.path.clone();
                expanded.extend_from_slice(&path[1..]);
                targets.extend(self.module_targets_inner(node, &expanded, active));
            }
        }
        active.remove(&key);
        targets.sort();
        targets.dedup();
        targets
    }

    fn module_ids(&self, node: usize, module: &str) -> Vec<usize> {
        self.index
            .by_module
            .get(&(self.index.modules[node].package.clone(), module.to_owned()))
            .cloned()
            .unwrap_or_default()
    }

    fn add_resolution(
        &self,
        symbols: &mut Symbols,
        node: usize,
        path: &[String],
        cfg: &[String],
        mut resolution: Resolution,
    ) {
        let key = path.join("::");
        resolution.issues.append(&mut resolution.cycles);
        let guards = resolution
            .candidates
            .iter()
            .map(|c| c.cfg.clone())
            .collect::<Vec<_>>();
        // A conditional binding with no fallback leaves the name unresolved in
        // part of the candidate's context. Keep that alternative, too.
        if !resolution.candidates.is_empty()
            && let Some(uncovered) = uncovered_cfg(cfg, &guards)
        {
            resolution.candidates.push(Candidate {
                provenance: Provenance::Unknown,
                cfg: uncovered,
            });
        }
        let alternatives = resolution
            .candidates
            .iter()
            .map(|candidate| candidate.provenance.clone())
            .collect::<BTreeSet<_>>();
        let authority = alternatives
            .iter()
            .filter_map(|provenance| {
                if let Provenance::Authority(sides) = provenance {
                    Some(sides)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !authority.is_empty() && alternatives.len() > 1 {
            resolution.issues.insert(self.diagnostic(
                node,
                &key,
                "ambiguous authority candidates under the enclosing cfg context",
            ));
        }
        if alternatives.len() == 1 && resolution.issues.is_empty() {
            if let Some(sides) = authority.first() {
                symbols.qualified.insert(key.clone(), (**sides).clone());
                if path.len() == 1 {
                    symbols.named.insert(key.clone(), (**sides).clone());
                }
            } else {
                symbols.qualified.insert(key.clone(), BTreeSet::new());
                if path.len() == 1 {
                    symbols.non_authority.insert(key.clone());
                }
            }
        } else {
            // Resolved unknowns must not fall back to curated spelling rules.
            symbols.qualified.insert(key.clone(), BTreeSet::new());
        }
        if !resolution.issues.is_empty() {
            symbols.path_issues.insert(
                key,
                resolution.issues.into_iter().collect::<Vec<_>>().join("; "),
            );
        }
    }

    fn symbols_for(&mut self, node: usize, cfg: &[String], paths: &[Vec<String>]) -> Symbols {
        let mut symbols = Symbols::default();
        let missing_globs = self.missing_relative_globs(node, cfg, &mut BTreeSet::new());
        for path in paths {
            let result = self.resolve_path(node, path, cfg);
            if !missing_globs.is_empty()
                && let Some(first) = path.first()
                && !self.has_visible_binding(node, first, &mut BTreeSet::new())
                && recognized_absolute_provenance(path).is_none()
                && !matches!(first.as_str(), "crate" | "self" | "super" | "Self")
            {
                symbols.unresolved_glob_paths.insert(
                    path.join("::"),
                    self.diagnostic(
                        node,
                        &path.join("::"),
                        &format!(
                            "relative glob context is unavailable: {}",
                            missing_globs.iter().cloned().collect::<Vec<_>>().join(", "),
                        ),
                    ),
                );
            }
            self.add_resolution(&mut symbols, node, path, cfg, result);
        }
        symbols
    }

    fn all_symbols(&mut self) -> Vec<Symbols> {
        let mut all = Vec::new();
        for node in 0..self.index.modules.len() {
            let module = &self.index.modules[node];
            let cfg = module.cfg.clone();
            let contexts = cfg_context::item_cfg_contexts(&module.items, &cfg);
            let mut collector = PathCollector::default();
            for item in &module.items {
                if matches!(item, Item::Use(_)) {
                    continue;
                }
                collector.visit_item(item);
            }
            let paths = collector.paths.into_iter().collect::<Vec<_>>();
            let mut symbols = self.symbols_for(node, &cfg, &paths);
            for context in contexts {
                let table = self.symbols_for(node, &context, &paths);
                symbols.contexts.insert(context, table);
            }
            all.push(symbols);
        }
        all
    }
}

#[derive(Default)]
struct PathCollector {
    paths: BTreeSet<Vec<String>>,
}

impl<'ast> Visit<'ast> for PathCollector {
    fn visit_path(&mut self, path: &'ast Path) {
        self.paths
            .insert(path.segments.iter().map(|s| s.ident.to_string()).collect());
        visit::visit_path(self, path);
    }

    fn visit_item_mod(&mut self, _item: &'ast ItemMod) {}

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let mut explicit = Vec::new();
        let mut globs = Vec::new();
        collect_use_specs(&item.tree, &mut Vec::new(), &mut explicit, &mut globs);
        self.paths
            .extend(explicit.into_iter().map(|(_, path)| path));
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.paths.extend(token_paths(&mac.tokens));
    }
}

pub(super) fn token_paths(tokens: &proc_macro2::TokenStream) -> BTreeSet<Vec<String>> {
    use proc_macro2::TokenTree;
    let trees = tokens.clone().into_iter().collect::<Vec<_>>();
    let mut paths = BTreeSet::new();
    for tree in &trees {
        if let TokenTree::Group(group) = tree {
            paths.extend(token_paths(&group.stream()));
        }
    }
    for start in 0..trees.len() {
        if start >= 2
            && matches!(&trees[start - 2], TokenTree::Punct(p) if p.as_char() == ':')
            && matches!(&trees[start - 1], TokenTree::Punct(p) if p.as_char() == ':')
        {
            continue;
        }
        let TokenTree::Ident(first) = &trees[start] else {
            continue;
        };
        let mut path = vec![first.to_string()];
        let mut cursor = start + 1;
        while cursor + 2 < trees.len()
            && matches!(&trees[cursor], TokenTree::Punct(p) if p.as_char() == ':')
            && matches!(&trees[cursor + 1], TokenTree::Punct(p) if p.as_char() == ':')
        {
            let TokenTree::Ident(next) = &trees[cursor + 2] else {
                break;
            };
            path.push(next.to_string());
            cursor += 3;
        }
        paths.insert(path);
    }
    paths
}

fn known_authority_name(name: &str) -> bool {
    matches!(
        name,
        "Creature"
            | "WorldCreature"
            | "MapManager"
            | "SharedMapManager"
            | "LegacyMapManager"
            | "SharedCanonicalMapManager"
    )
}

fn recognized_absolute_provenance(segments: &[String]) -> Option<Provenance> {
    let first = segments.first()?.as_str();
    let canonical_map = segments
        .get(1)
        .is_some_and(|s| matches!(s.as_str(), "MapManager" | "ManagedMap" | "Map"))
        || (segments
            .get(1)
            .is_some_and(|s| matches!(s.as_str(), "manager" | "map"))
            && segments
                .get(2)
                .is_some_and(|s| matches!(s.as_str(), "MapManager" | "ManagedMap" | "Map")));
    if (first == "wow_map" && canonical_map)
        || (first == "wow_entities" && segments.get(1).is_some_and(|s| s == "Creature"))
        || (first == "wow_world"
            && (segments
                .get(1)
                .is_some_and(|s| s == "SharedCanonicalMapManager")
                || (segments.get(1).is_some_and(|s| s == "session")
                    && segments
                        .get(2)
                        .is_some_and(|s| s == "SharedCanonicalMapManager"))))
    {
        return Some(Provenance::Authority(BTreeSet::from([
            BridgeSide::Canonical,
        ])));
    }
    let legacy = |name: &str| {
        matches!(
            name,
            "MapManager" | "SharedMapManager" | "LegacyMapManager" | "WorldCreature"
        )
    };
    if first == "wow_world"
        && (segments.get(1).is_some_and(|s| legacy(s))
            || (segments.get(1).is_some_and(|s| s == "map_manager")
                && segments.get(2).is_some_and(|s| legacy(s))))
    {
        return Some(Provenance::Authority(BTreeSet::from([BridgeSide::Legacy])));
    }
    None
}

pub(super) fn resolve_module_symbols(
    index: &ModuleIndex,
    errors: &mut Vec<String>,
) -> Vec<Symbols> {
    let mut mounts = BTreeSet::new();
    for module in &index.modules {
        if !mounts.insert((
            &module.package,
            &module.module,
            &module.source_path,
            &module.cfg,
        )) {
            errors.push(format!(
                "duplicate bridge source mount {} {} {}",
                module.package, module.module, module.source_path,
            ));
        }
    }
    Resolver::new(index, errors).all_symbols()
}

pub(super) fn build_module_index(sources: &[(BridgeSource<'_>, syn::File)]) -> ModuleIndex {
    let mut index = ModuleIndex::default();
    for (source, syntax) in sources {
        index.add_source(*source, syntax.clone());
    }
    index
}
