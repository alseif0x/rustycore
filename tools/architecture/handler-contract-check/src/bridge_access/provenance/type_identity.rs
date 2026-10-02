//! Opt-in nominal type identity queries over the bridge resolver's source graph.

use std::collections::BTreeSet;
use std::fmt;

use quote::ToTokens;
use syn::{PathArguments, Type};

use crate::bridge_access::BridgeSource;
use crate::ownership::{cfg_context_allows_production, cfg_context_allows_test};

use super::scope::collect_scope;
use super::source_graph::ModuleIndex;
use super::{
    ModuleIdentity, Provenance, Resolution, Resolver, ResolverMode, build_module_index,
    canonical_cfg, combine_cfg, uncovered_cfg,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum TypeIdentityKind {
    Struct,
    Enum,
    Union,
}

impl fmt::Display for TypeIdentityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Union => "union",
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct SuppliedTypeIdentity {
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) symbol: String,
    pub(crate) kind: TypeIdentityKind,
    pub(crate) source_path: String,
    pub(crate) guards: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) struct TypeIdentityQuery<'a> {
    pub(crate) package: &'a str,
    pub(crate) module: &'a str,
    pub(crate) ty: &'a Type,
    pub(crate) cfg: &'a [String],
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct NominalProvider {
    pub(super) package: String,
    pub(super) module: String,
    pub(super) symbol: String,
    pub(super) kind: TypeIdentityKind,
    pub(super) source_path: String,
    pub(super) declaration_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TypeIdentityError {
    InvalidSources(String),
    Missing(String),
    Ambiguous(String),
    Cycle(String),
    NotNominal(String),
    Unsupported(String),
}

impl fmt::Display for TypeIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, detail) = match self {
            Self::InvalidSources(detail) => ("invalid supplied sources", detail),
            Self::Missing(detail) => ("missing nominal type identity", detail),
            Self::Ambiguous(detail) => ("ambiguous nominal type identity", detail),
            Self::Cycle(detail) => ("nominal type identity cycle", detail),
            Self::NotNominal(detail) => ("supplied name is not a nominal type", detail),
            Self::Unsupported(detail) => ("unsupported nominal type expression", detail),
        };
        write!(f, "{kind}: {detail}")
    }
}

impl std::error::Error for TypeIdentityError {}

impl<'a> Resolver<'a> {
    pub(super) fn new(index: &'a ModuleIndex, errors: &mut Vec<String>) -> Self {
        Self::with_mode(index, errors, ResolverMode::Bridge)
    }

    pub(super) fn new_type_identity(index: &'a ModuleIndex, errors: &mut Vec<String>) -> Self {
        Self::with_mode(index, errors, ResolverMode::TypeIdentity)
    }

    fn with_mode(index: &'a ModuleIndex, errors: &mut Vec<String>, mode: ResolverMode) -> Self {
        Self {
            index,
            scopes: index
                .modules
                .iter()
                .map(|module| collect_scope(module, index, errors, mode))
                .collect(),
            mode,
            memo: Default::default(),
            active: Default::default(),
        }
    }

    pub(super) fn resolve_nominal_type(
        &mut self,
        node: usize,
        ty: &Type,
        cfg: &[String],
    ) -> Resolution {
        let Type::Path(type_path) = ty else {
            return self.unresolved(
                node,
                &ty.to_token_stream().to_string(),
                cfg,
                "unsupported non-path type in nominal identity query",
            );
        };
        if type_path.qself.is_some()
            || type_path
                .path
                .segments
                .iter()
                .any(|segment| !matches!(&segment.arguments, &PathArguments::None))
        {
            return self.unresolved(
                node,
                &type_path.to_token_stream().to_string(),
                cfg,
                "unsupported qualified or generic type path in nominal identity query",
            );
        }
        let path = type_path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        let result = self.resolve_path(node, &path, cfg);
        if result.candidates.is_empty() && result.issues.is_empty() && result.cycles.is_empty() {
            return self.unresolved(
                node,
                &path.join("::"),
                cfg,
                "nominal type target is missing or unavailable",
            );
        }
        result
    }

    pub(super) fn resolve_type_external_path(
        &mut self,
        node: usize,
        segments: &[String],
        cfg: &[String],
    ) -> Option<Resolution> {
        if self.mode != ResolverMode::TypeIdentity {
            return None;
        }
        let first = segments.first()?;
        if first != "wow_world_core" {
            return None;
        }
        Some(self.resolve_from_module(
            node,
            &ModuleIdentity::new("wow-world-core", "crate"),
            &segments[1..],
            cfg,
        ))
    }

    pub(super) fn resolve_type_self_import(
        &mut self,
        node: usize,
        name: &str,
        path: &[String],
        cfg: &[String],
    ) -> Option<Resolution> {
        if self.mode != ResolverMode::TypeIdentity
            || path.len() != 1
            || path[0] != name
            || matches!(name, "crate" | "self" | "super")
        {
            return None;
        }
        Some(
            self.resolve_type_external_path(node, path, cfg)
                .unwrap_or_else(|| {
                    self.unresolved(
                        node,
                        name,
                        cfg,
                        "external self import has no supplied crate root",
                    )
                }),
        )
    }

    pub(super) fn has_visible_binding(
        &self,
        node: usize,
        name: &str,
        active: &mut BTreeSet<usize>,
    ) -> bool {
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

}

/// Resolve a path-shaped type through the supplied lexical modules and reexports.
/// This query does not classify either bridge authority side.
pub(crate) fn resolve_supplied_type_identity(
    sources: &[BridgeSource<'_>],
    package: &str,
    module: &str,
    ty: &syn::Type,
    cfg: &[String],
) -> Result<Vec<SuppliedTypeIdentity>, TypeIdentityError> {
    let mut results = resolve_supplied_type_identities(
        sources,
        &[TypeIdentityQuery {
            package,
            module,
            ty,
            cfg,
        }],
    )?;
    results
        .pop()
        .expect("a singleton identity batch returns one result")
}

/// Resolve several type uses against one parsed graph and one shared resolver memo.
pub(crate) fn resolve_supplied_type_identities(
    sources: &[BridgeSource<'_>],
    queries: &[TypeIdentityQuery<'_>],
) -> Result<Vec<Result<Vec<SuppliedTypeIdentity>, TypeIdentityError>>, TypeIdentityError> {
    if queries.is_empty() {
        return Ok(Vec::new());
    }
    if sources.is_empty() {
        return Ok(queries
            .iter()
            .map(|query| {
                Err(TypeIdentityError::Missing(format!(
                    "no supplied source contains {}::{}",
                    query.package, query.module
                )))
            })
            .collect());
    }

    let mut parsed = Vec::with_capacity(sources.len());
    let mut errors = Vec::new();
    let mut mounts = BTreeSet::new();
    for source in sources {
        if source.package.is_empty() || source.module.is_empty() || source.source_path.is_empty() {
            errors.push("source package/module/path must be non-empty".to_owned());
            continue;
        }
        let mount = (
            source.package.to_owned(),
            source.module.to_owned(),
            source.source_path.to_owned(),
            source.inherited_cfg.to_vec(),
        );
        if !mounts.insert(mount) {
            errors.push(format!(
                "duplicate source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        match syn::parse_file(source.source) {
            Ok(syntax) => {
                for validation in [
                    cfg_context_allows_production(source.inherited_cfg, &syntax.attrs),
                    cfg_context_allows_test(source.inherited_cfg, &syntax.attrs),
                ] {
                    if let Err(error) = validation {
                        errors.push(format!("invalid cfg for {}: {error}", source.source_path));
                    }
                }
                parsed.push((*source, syntax));
            }
            Err(error) => errors.push(format!(
                "cannot parse supplied source {}: {error}",
                source.source_path
            )),
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(TypeIdentityError::InvalidSources(errors.join("; ")));
    }

    let index = build_module_index(&parsed);
    let mut scope_errors = Vec::new();
    let mut resolver = Resolver::new_type_identity(&index, &mut scope_errors);
    if !scope_errors.is_empty() {
        scope_errors.sort();
        scope_errors.dedup();
        return Err(TypeIdentityError::InvalidSources(scope_errors.join("; ")));
    }

    Ok(queries
        .iter()
        .map(|query| resolve_query(&index, &mut resolver, query))
        .collect())
}

fn resolve_query(
    index: &ModuleIndex,
    resolver: &mut Resolver<'_>,
    query: &TypeIdentityQuery<'_>,
) -> Result<Vec<SuppliedTypeIdentity>, TypeIdentityError> {
    let TypeIdentityQuery {
        package,
        module,
        ty,
        cfg,
    } = *query;
    if package.is_empty() || module.is_empty() {
        return Err(TypeIdentityError::InvalidSources(
            "query package and module must be non-empty".to_owned(),
        ));
    }
    cfg_context_allows_production(cfg, &[])
        .and_then(|_| cfg_context_allows_test(cfg, &[]))
        .map_err(TypeIdentityError::InvalidSources)?;
    let requested_cfg = canonical_cfg(cfg);
    let roots = index
        .modules
        .iter()
        .enumerate()
        .filter(|(_, source)| source.package == package && source.module == module)
        .filter_map(|(node, source)| {
            combine_cfg(&requested_cfg, &source.cfg).map(|context| (node, context))
        })
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err(TypeIdentityError::Missing(format!(
            "no available source mount for {package}::{module} under {requested_cfg:?}"
        )));
    }

    let mut resolution = Resolution::default();
    for (node, context) in roots {
        resolution.append(resolver.resolve_type(node, ty, &context));
    }
    if !resolution.cycles.is_empty() {
        return Err(TypeIdentityError::Cycle(
            resolution.cycles.into_iter().collect::<Vec<_>>().join("; "),
        ));
    }
    if !resolution.issues.is_empty() {
        let details = resolution.issues.into_iter().collect::<Vec<_>>().join("; ");
        return Err(if details.contains("ambiguous") {
            TypeIdentityError::Ambiguous(details)
        } else if details.contains("unsupported") {
            TypeIdentityError::Unsupported(details)
        } else {
            TypeIdentityError::Missing(details)
        });
    }
    if resolution.candidates.is_empty() {
        return Err(TypeIdentityError::Missing(format!(
            "{} does not resolve under {requested_cfg:?}",
            ty.to_token_stream()
        )));
    }

    let mut providers = Vec::new();
    for candidate in &resolution.candidates {
        let Provenance::Nominal(provider) = &candidate.provenance else {
            return Err(TypeIdentityError::NotNominal(format!(
                "{} resolves to {:?}, not a supplied struct, enum, or union",
                ty.to_token_stream(), candidate.provenance
            )));
        };
        providers.push((provider.clone(), candidate.cfg.clone()));
    }
    providers.sort();
    providers.dedup();

    let guards = providers
        .iter()
        .map(|(_, guards)| guards.clone())
        .collect::<Vec<_>>();
    if let Some(uncovered) = uncovered_cfg(&requested_cfg, &guards) {
        return Err(TypeIdentityError::Missing(format!(
            "{} has no supplied nominal provider under {uncovered:?}",
            ty.to_token_stream()
        )));
    }
    for (index, (left, left_cfg)) in providers.iter().enumerate() {
        for (right, right_cfg) in providers.iter().skip(index + 1) {
            if left != right && combine_cfg(left_cfg, right_cfg).is_some() {
                return Err(TypeIdentityError::Ambiguous(format!(
                    "{} has compatible providers {}::{} ({}) and {}::{} ({})",
                    ty.to_token_stream(),
                    left.package,
                    left.module,
                    left.source_path,
                    right.package,
                    right.module,
                    right.source_path,
                )));
            }
        }
    }

    Ok(providers
        .into_iter()
        .map(|(provider, guards)| SuppliedTypeIdentity {
            package: provider.package,
            module: provider.module,
            symbol: provider.symbol,
            kind: provider.kind,
            source_path: provider.source_path,
            guards,
        })
        .collect())
}
