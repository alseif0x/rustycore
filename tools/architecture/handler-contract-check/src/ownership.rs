// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Workspace-wide ownership audit for `PacketHandlerEntry` registrations.
//!
//! Cargo metadata identifies the reverse closure of production workspace
//! packages that can name `wow-handler`. Their `lib`/`bin` module trees are
//! parsed without evaluating `cfg`; this is source ownership validation, not
//! arbitrary procedural-macro expansion.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quote::ToTokens;
use serde_json::Value;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Attribute, Item, ItemMod, Meta, Token};

use crate::module_policy::CapabilityOwner;
use crate::registrations::{
    analyze_registration_syntax_outside_handlers, exported_macro_names,
    handler_capable_macro_definitions, handler_capable_macro_invocations, include_macro_bodies,
    inventory_registration_macro_fingerprints, registration_alias_violations,
};

const HANDLER_PACKAGE_NAME: &str = "wow-handler";
const WOW_PROTO_PACKAGE_NAME: &str = "wow-proto";
const WOW_PROTO_GENERATED_INCLUDE_SUFFIXES: &[&str] = &[
    "/bgs.protocol.rs",
    "/bgs.protocol.account.v1.rs",
    "/bgs.protocol.authentication.v1.rs",
    "/bgs.protocol.challenge.v1.rs",
    "/bgs.protocol.connection.v1.rs",
    "/bgs.protocol.game_utilities.v1.rs",
];
const WOW_LOGGING_PACKAGE_NAME: &str = "wow-logging";
const WOW_LOGGING_EXPORTED_MACROS: &[&str] = &[
    "__log_with_filter",
    "log_achievement",
    "log_ai",
    "log_arena",
    "log_battleground",
    "log_chat",
    "log_commands",
    "log_condition",
    "log_database",
    "log_entities",
    "log_guild",
    "log_lfg",
    "log_loading",
    "log_loot",
    "log_maps",
    "log_misc",
    "log_movement",
    "log_network",
    "log_player",
    "log_scripts",
    "log_server",
    "log_spells",
    "log_vehicle",
];
const WOW_SCRIPT_PACKAGE_NAME: &str = "wow-script";
const WOW_SCRIPT_INVENTORY_CALLS: &[&str] = &[
    "inventory::collect!(GivePlayerXpHookLikeCpp);",
    "inventory::collect!(ShutdownHookLikeCpp);",
    "inventory::collect!(StartupHookLikeCpp);",
    r#"inventory::submit! {
        GivePlayerXpHookLikeCpp {
            name: "test_add_seven_xp_like_cpp",
            callback: add_seven_xp_like_cpp,
        }
    };"#,
    r#"inventory::submit! {
        GivePlayerXpHookLikeCpp {
            name: "test_add_three_xp_like_cpp",
            callback: add_three_xp_like_cpp,
        }
    };"#,
    r#"inventory::submit! {
        ShutdownHookLikeCpp {
            name: "test_shutdown_like_cpp",
            callback: record_shutdown_like_cpp,
        }
    };"#,
    r#"inventory::submit! {
        StartupHookLikeCpp {
            name: "test_startup_like_cpp",
            callback: record_startup_like_cpp,
        }
    };"#,
];

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RegistrationOwnershipReport {
    pub(crate) scanned_packages: usize,
    pub(crate) scanned_files: usize,
    pub(crate) macro_scan_packages: usize,
    pub(crate) macro_scan_files: usize,
    pub(crate) explicit_path_modules: usize,
    pub(crate) package_names: Vec<String>,
}

#[derive(Debug)]
struct PackageAuditScope {
    id: String,
    name: String,
    root: PathBuf,
    production_roots: Vec<PathBuf>,
    production_lib_roots: BTreeSet<PathBuf>,
}

fn pinned_generated_include_bodies() -> Result<Vec<String>, String> {
    let mut bodies = Vec::new();
    for suffix in WOW_PROTO_GENERATED_INCLUDE_SUFFIXES {
        let source = format!("include!(concat!(env!(\"OUT_DIR\"), {suffix:?}));");
        bodies.extend(include_macro_bodies(
            Path::new("<pinned-wow-proto>"),
            &source,
        )?);
    }
    bodies.sort();
    Ok(bodies)
}

fn is_pinned_wow_proto_include_surface(
    scope: &PackageAuditScope,
    source_path: &Path,
    include_bodies: &[String],
) -> Result<bool, String> {
    if scope.name != WOW_PROTO_PACKAGE_NAME {
        return Ok(false);
    }
    let pinned_source = scope
        .root
        .join("src/lib.rs")
        .canonicalize()
        .map_err(|error| format!("cannot resolve pinned wow-proto include source: {error}"))?;
    Ok(source_path == pinned_source && include_bodies == pinned_generated_include_bodies()?)
}

fn is_pinned_wow_logging_source(
    scope: &PackageAuditScope,
    source_path: &Path,
) -> Result<bool, String> {
    if scope.name != WOW_LOGGING_PACKAGE_NAME {
        return Ok(false);
    }
    let pinned_source = scope
        .root
        .join("src/lib.rs")
        .canonicalize()
        .map_err(|error| format!("cannot resolve pinned wow-logging macro source: {error}"))?;
    Ok(source_path == pinned_source)
}

fn pinned_wow_script_inventory_fingerprints() -> Result<Vec<String>, String> {
    let source = WOW_SCRIPT_INVENTORY_CALLS.join("\n");
    inventory_registration_macro_fingerprints(Path::new("<pinned-wow-script>"), &source)
}

fn is_pinned_wow_script_inventory_source(
    scope: &PackageAuditScope,
    source_path: &Path,
) -> Result<bool, String> {
    if scope.name != WOW_SCRIPT_PACKAGE_NAME {
        return Ok(false);
    }
    let pinned_source = scope
        .root
        .join("src/lib.rs")
        .canonicalize()
        .map_err(|error| format!("cannot resolve pinned wow-script inventory source: {error}"))?;
    Ok(source_path == pinned_source)
}

fn required_array<'a>(value: &'a Value, field: &str, owner: &str) -> Result<&'a [Value], String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("cargo metadata {owner} is missing array field {field:?}"))
}

fn required_string<'a>(value: &'a Value, field: &str, owner: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("cargo metadata {owner} is missing string field {field:?}"))
}

fn workspace_metadata(repository_root: &Path) -> Result<Value, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--locked",
            "--all-features",
            "--format-version",
            "1",
        ])
        .current_dir(repository_root)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cargo metadata failed with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata returned invalid JSON: {error}"))
}

/// Direct dependency crate aliases relevant to persistence analysis, separated
/// by whether they are available to production code or only to test-capable
/// source. This includes workspace providers and the external persistence
/// providers whose source paths the analyzer recognizes. The alias is the name
/// visible to Rust source (`deps[].name`); the value is the provider package's
/// canonical crate root used by the persistence type registry.
#[derive(Clone, Debug, Default)]
pub(crate) struct WorkspaceDependencyAliases {
    pub(crate) production: BTreeMap<String, BTreeMap<String, String>>,
    pub(crate) test: BTreeMap<String, BTreeMap<String, String>>,
}

pub(crate) fn workspace_dependency_aliases(
    repository_root: &Path,
) -> Result<WorkspaceDependencyAliases, String> {
    let metadata = workspace_metadata(repository_root)?;
    workspace_dependency_aliases_from_metadata(&metadata)
}

pub(crate) fn workspace_dependency_aliases_from_metadata(
    metadata: &Value,
) -> Result<WorkspaceDependencyAliases, String> {
    let workspace_members: BTreeSet<_> = required_array(&metadata, "workspace_members", "root")?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "cargo metadata workspace member is not a string".to_owned())
        })
        .collect::<Result<_, _>>()?;
    let packages_by_id: BTreeMap<_, _> = required_array(&metadata, "packages", "root")?
        .iter()
        .map(|package| {
            Ok((
                required_string(package, "id", "package")?.to_owned(),
                required_string(package, "name", "package")?.to_owned(),
            ))
        })
        .collect::<Result<_, String>>()?;
    let resolve = metadata
        .get("resolve")
        .filter(|resolve| !resolve.is_null())
        .ok_or_else(|| "cargo metadata is missing the resolved dependency graph".to_owned())?;
    let mut aliases = WorkspaceDependencyAliases::default();
    for node in required_array(resolve, "nodes", "resolve")? {
        let package_id = required_string(node, "id", "resolve node")?;
        if !workspace_members.contains(package_id) {
            continue;
        }
        let package = packages_by_id
            .get(package_id)
            .ok_or_else(|| format!("workspace package {package_id} is absent from metadata"))?;
        for dependency in required_array(node, "deps", package_id)? {
            let dependency_id = required_string(dependency, "pkg", "resolved dependency")?;
            let provider = packages_by_id.get(dependency_id).ok_or_else(|| {
                format!("workspace dependency {dependency_id} is absent from metadata")
            })?;
            let alias = required_string(dependency, "name", "resolved dependency")?.to_owned();
            let provider_root = provider.replace('-', "_");
            if !workspace_members.contains(dependency_id) && provider_root != "sqlx" {
                continue;
            }
            let dependency_kinds = required_array(dependency, "dep_kinds", "resolved dependency")?;
            let normal = dependency_kinds
                .iter()
                .any(|kind| matches!(kind.get("kind"), Some(Value::Null)));
            let dev = dependency_kinds
                .iter()
                .any(|kind| kind.get("kind").and_then(Value::as_str) == Some("dev"));
            if normal {
                aliases
                    .production
                    .entry(package.clone())
                    .or_default()
                    .insert(alias.clone(), provider_root.clone());
            }
            if normal || dev {
                aliases
                    .test
                    .entry(package.clone())
                    .or_default()
                    .insert(alias, provider_root);
            }
        }
    }
    Ok(aliases)
}

pub(crate) fn registry_capable_package_ids(metadata: &Value) -> Result<BTreeSet<String>, String> {
    let packages = required_array(metadata, "packages", "root")?;
    let workspace_members: BTreeSet<_> = required_array(metadata, "workspace_members", "root")?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "cargo metadata workspace member is not a string".to_owned())
        })
        .collect::<Result<_, _>>()?;

    let mut packages_by_id = BTreeMap::new();
    for package in packages {
        let id = required_string(package, "id", "package")?.to_owned();
        if packages_by_id.insert(id.clone(), package).is_some() {
            return Err(format!("cargo metadata contains duplicate package id {id}"));
        }
    }

    let handler_ids: Vec<_> = workspace_members
        .iter()
        .filter(|id| {
            packages_by_id
                .get(*id)
                .and_then(|package| package.get("name"))
                .and_then(Value::as_str)
                == Some(HANDLER_PACKAGE_NAME)
        })
        .cloned()
        .collect();
    let [handler_id] = handler_ids.as_slice() else {
        return Err(format!(
            "expected exactly one workspace package named {HANDLER_PACKAGE_NAME}, found {}",
            handler_ids.len()
        ));
    };

    let resolve = metadata
        .get("resolve")
        .filter(|resolve| !resolve.is_null())
        .ok_or_else(|| "cargo metadata is missing the resolved dependency graph".to_owned())?;
    let mut reverse_normal_dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in required_array(resolve, "nodes", "resolve")? {
        let package_id = required_string(node, "id", "resolve node")?.to_owned();
        for dependency in required_array(node, "deps", &package_id)? {
            let dependency_kinds = required_array(dependency, "dep_kinds", "resolved dependency")?;
            if dependency_kinds.is_empty() {
                return Err(format!(
                    "resolved dependency from {package_id} has no dep_kinds entries"
                ));
            }
            let mut normal_dependency = false;
            for kind in dependency_kinds {
                match kind.get("kind") {
                    Some(Value::Null) => normal_dependency = true,
                    Some(Value::String(kind)) if matches!(kind.as_str(), "dev" | "build") => {}
                    Some(Value::String(kind)) => {
                        return Err(format!(
                            "resolved dependency from {package_id} has unsupported dependency kind \
                             {kind:?}; expected normal, dev, or build"
                        ));
                    }
                    Some(other) => {
                        return Err(format!(
                            "resolved dependency from {package_id} has non-string dependency kind \
                             {other}"
                        ));
                    }
                    None => {
                        return Err(format!(
                            "resolved dependency from {package_id} is missing dep_kinds[].kind"
                        ));
                    }
                }
            }
            if !normal_dependency {
                continue;
            }
            let dependency_id =
                required_string(dependency, "pkg", "resolved dependency")?.to_owned();
            reverse_normal_dependencies
                .entry(dependency_id)
                .or_default()
                .insert(package_id.clone());
        }
    }

    let mut capable = BTreeSet::from([handler_id.clone()]);
    let mut pending = VecDeque::from([handler_id.clone()]);
    while let Some(package_id) = pending.pop_front() {
        for dependent in reverse_normal_dependencies
            .get(&package_id)
            .into_iter()
            .flatten()
        {
            if !workspace_members.contains(dependent) {
                let dependent_name = packages_by_id
                    .get(dependent)
                    .and_then(|package| package.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("<unknown>");
                return Err(format!(
                    "non-workspace package {dependent_name} ({dependent}) is in the normal reverse \
                     dependency closure of {HANDLER_PACKAGE_NAME}; its source cannot be covered by \
                     the workspace ownership audit"
                ));
            }
            if capable.insert(dependent.clone()) {
                pending.push_back(dependent.clone());
            }
        }
    }
    Ok(capable)
}

fn package_audit_scopes(
    metadata: &Value,
    selected_package_ids: &BTreeSet<String>,
) -> Result<Vec<PackageAuditScope>, String> {
    let packages = required_array(metadata, "packages", "root")?;
    let packages_by_id: BTreeMap<_, _> = packages
        .iter()
        .map(|package| {
            Ok((
                required_string(package, "id", "package")?.to_owned(),
                package,
            ))
        })
        .collect::<Result<_, String>>()?;

    let mut scopes = Vec::new();
    for package_id in selected_package_ids {
        let package = packages_by_id
            .get(package_id)
            .ok_or_else(|| format!("selected workspace package {package_id} is absent"))?;
        let name = required_string(package, "name", &package_id)?.to_owned();
        let manifest = PathBuf::from(required_string(package, "manifest_path", &package_id)?);
        let root = manifest
            .parent()
            .ok_or_else(|| format!("manifest has no parent: {}", manifest.display()))?
            .canonicalize()
            .map_err(|error| {
                format!(
                    "cannot resolve package root {}: {error}",
                    manifest.display()
                )
            })?;

        let mut production_roots = BTreeSet::new();
        let mut production_lib_roots = BTreeSet::new();
        for target in required_array(package, "targets", &package_id)? {
            let kinds = required_array(target, "kind", "target")?;
            let production = kinds.iter().any(|kind| {
                kind.as_str()
                    .is_some_and(|kind| matches!(kind, "lib" | "bin"))
            });
            if production {
                let source_path = PathBuf::from(required_string(target, "src_path", "target")?);
                production_roots.insert(source_path.clone());
                if kinds.iter().any(|kind| kind.as_str() == Some("lib")) {
                    production_lib_roots.insert(source_path.canonicalize().map_err(|error| {
                        format!(
                            "cannot resolve production lib root {} for {name}: {error}",
                            source_path.display()
                        )
                    })?);
                }
            }
        }
        if production_roots.is_empty() {
            return Err(format!(
                "selected workspace package {name} has no production lib/bin target"
            ));
        }
        scopes.push(PackageAuditScope {
            id: package_id.clone(),
            name,
            root,
            production_roots: production_roots.into_iter().collect(),
            production_lib_roots,
        });
    }
    scopes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(scopes)
}

#[derive(Default)]
struct SourceGraph {
    mounts: BTreeMap<PathBuf, SourceMount>,
    explicit_path_declarations: BTreeSet<(PathBuf, String, PathBuf)>,
    visited_mounts: BTreeSet<(PathBuf, SourceMountContext)>,
    active_sources: Vec<PathBuf>,
}

#[derive(Debug)]
struct SourceMount {
    module_directory: PathBuf,
    contexts: BTreeSet<SourceMountContext>,
}

/// One logical mount of a physical source file.
///
/// The handler audit intentionally inspects every cfg branch. Session surface
/// ratchets additionally need to distinguish source that can participate in a
/// non-test production build from exact `cfg(test)`-only source, while still
/// retaining the normalized cfg ancestry in the checked-in baseline.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct SourceMountContext {
    pub(crate) logical_module_path: String,
    pub(crate) cfg: Vec<String>,
    pub(crate) production_possible: bool,
    pub(crate) test_possible: bool,
}

#[derive(Clone, Debug)]
enum CfgExpression {
    Constant(bool),
    Atom(String),
    Not(Box<CfgExpression>),
    All(Vec<CfgExpression>),
    Any(Vec<CfgExpression>),
}

fn cfg_meta_expression(meta: &Meta, test_enabled: bool) -> Result<CfgExpression, String> {
    match meta {
        Meta::Path(path) if path.is_ident("test") => Ok(CfgExpression::Constant(test_enabled)),
        Meta::Path(_) | Meta::NameValue(_) => {
            Ok(CfgExpression::Atom(meta.to_token_stream().to_string()))
        }
        Meta::List(list) if list.path.is_ident("all") => {
            let items = Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .map_err(|error| format!("cannot parse cfg(all(...)): {error}"))?;
            Ok(CfgExpression::All(
                items
                    .iter()
                    .map(|meta| cfg_meta_expression(meta, test_enabled))
                    .collect::<Result<_, _>>()?,
            ))
        }
        Meta::List(list) if list.path.is_ident("any") => {
            let items = Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .map_err(|error| format!("cannot parse cfg(any(...)): {error}"))?;
            Ok(CfgExpression::Any(
                items
                    .iter()
                    .map(|meta| cfg_meta_expression(meta, test_enabled))
                    .collect::<Result<_, _>>()?,
            ))
        }
        Meta::List(list) if list.path.is_ident("not") => {
            let items = Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .map_err(|error| format!("cannot parse cfg(not(...)): {error}"))?;
            if items.len() != 1 {
                return Err("cfg(not(...)) must contain exactly one predicate".to_owned());
            }
            let item = items.first().expect("one cfg item after length check");
            Ok(CfgExpression::Not(Box::new(cfg_meta_expression(
                item,
                test_enabled,
            )?)))
        }
        Meta::List(_) => Ok(CfgExpression::Atom(meta.to_token_stream().to_string())),
    }
}

fn cfg_attribute_expression(meta: &Meta, test_enabled: bool) -> Result<CfgExpression, String> {
    let Meta::List(list) = meta else {
        return Err("cfg attribute must use list syntax".to_owned());
    };
    let predicate = syn::parse2::<Meta>(list.tokens.clone())
        .map_err(|error| format!("cannot parse cfg predicate: {error}"))?;
    cfg_meta_expression(&predicate, test_enabled)
}

fn cfg_attr_expression(meta: &Meta, test_enabled: bool) -> Result<CfgExpression, String> {
    let Meta::List(list) = meta else {
        return Err("cfg_attr attribute must use list syntax".to_owned());
    };
    let items = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .map_err(|error| format!("cannot parse cfg_attr arguments: {error}"))?;
    let mut items = items.iter();
    let predicate = items
        .next()
        .ok_or_else(|| "cfg_attr requires a predicate and at least one attribute".to_owned())?;
    let nested: Vec<_> = items.collect();
    if nested.is_empty() {
        return Err("cfg_attr requires at least one conditional attribute".to_owned());
    }
    let predicate = cfg_meta_expression(predicate, test_enabled)?;
    let effects = nested
        .into_iter()
        .map(|attribute| {
            if attribute.path().is_ident("cfg") {
                cfg_attribute_expression(attribute, test_enabled)
            } else if attribute.path().is_ident("cfg_attr") {
                cfg_attr_expression(attribute, test_enabled)
            } else {
                Ok(CfgExpression::Constant(true))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CfgExpression::Any(vec![
        CfgExpression::Not(Box::new(predicate)),
        CfgExpression::All(effects),
    ]))
}

fn cfg_atoms(expression: &CfgExpression, atoms: &mut BTreeSet<String>) {
    match expression {
        CfgExpression::Atom(atom) => {
            atoms.insert(atom.clone());
        }
        CfgExpression::Not(inner) => cfg_atoms(inner, atoms),
        CfgExpression::All(items) | CfgExpression::Any(items) => {
            for item in items {
                cfg_atoms(item, atoms);
            }
        }
        CfgExpression::Constant(_) => {}
    }
}

fn evaluate_cfg(expression: &CfgExpression, assignments: &BTreeMap<String, bool>) -> Option<bool> {
    match expression {
        CfgExpression::Constant(value) => Some(*value),
        CfgExpression::Atom(atom) => assignments.get(atom).copied(),
        CfgExpression::Not(inner) => evaluate_cfg(inner, assignments).map(|value| !value),
        CfgExpression::All(items) => {
            let values: Vec<_> = items
                .iter()
                .map(|item| evaluate_cfg(item, assignments))
                .collect();
            if values.contains(&Some(false)) {
                Some(false)
            } else if values.iter().all(Option::is_some) {
                Some(true)
            } else {
                None
            }
        }
        CfgExpression::Any(items) => {
            let values: Vec<_> = items
                .iter()
                .map(|item| evaluate_cfg(item, assignments))
                .collect();
            if values.contains(&Some(true)) {
                Some(true)
            } else if values.iter().all(Option::is_some) {
                Some(false)
            } else {
                None
            }
        }
    }
}

fn cfg_satisfiable(expression: &CfgExpression) -> bool {
    let mut atoms = BTreeSet::new();
    cfg_atoms(expression, &mut atoms);
    let atoms: Vec<_> = atoms.into_iter().collect();

    fn search(
        expression: &CfgExpression,
        atoms: &[String],
        assignments: &mut BTreeMap<String, bool>,
    ) -> bool {
        match evaluate_cfg(expression, assignments) {
            Some(value) => return value,
            None => {}
        }
        let Some(atom) = atoms.iter().find(|atom| !assignments.contains_key(*atom)) else {
            return false;
        };
        let atom = atom.clone();
        for value in [false, true] {
            assignments.insert(atom.clone(), value);
            if search(expression, atoms, assignments) {
                assignments.remove(&atom);
                return true;
            }
        }
        assignments.remove(&atom);
        false
    }

    search(expression, &atoms, &mut BTreeMap::new())
}

pub(crate) fn normalized_cfg_attributes(attributes: &[syn::Attribute]) -> Vec<String> {
    let mut cfg: Vec<_> = attributes
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        })
        .map(|attribute| attribute.meta.to_token_stream().to_string())
        .collect();
    cfg.sort();
    cfg.dedup();
    cfg
}

fn cfg_context_satisfiable(
    parent: &[String],
    attributes: &[syn::Attribute],
    test_enabled: bool,
) -> Result<bool, String> {
    let mut expressions = Vec::new();
    for fingerprint in parent {
        let meta: Meta = syn::parse_str(fingerprint)
            .map_err(|error| format!("cannot parse inherited cfg {fingerprint:?}: {error}"))?;
        if meta.path().is_ident("cfg") {
            expressions.push(cfg_attribute_expression(&meta, test_enabled)?);
        } else if meta.path().is_ident("cfg_attr") {
            expressions.push(cfg_attr_expression(&meta, test_enabled)?);
        } else {
            return Err(format!(
                "unsupported inherited cfg fingerprint {fingerprint:?}"
            ));
        }
    }
    for attribute in attributes {
        if attribute.path().is_ident("cfg") {
            expressions.push(cfg_attribute_expression(&attribute.meta, test_enabled)?);
        } else if attribute.path().is_ident("cfg_attr") {
            expressions.push(cfg_attr_expression(&attribute.meta, test_enabled)?);
        }
    }
    Ok(cfg_satisfiable(&CfgExpression::All(expressions)))
}

pub(crate) fn cfg_context_allows_production(
    parent: &[String],
    attributes: &[syn::Attribute],
) -> Result<bool, String> {
    cfg_context_satisfiable(parent, attributes, false)
}

pub(crate) fn cfg_context_allows_test(
    parent: &[String],
    attributes: &[syn::Attribute],
) -> Result<bool, String> {
    cfg_context_satisfiable(parent, attributes, true)
}

pub(crate) fn extend_cfg_context(parent: &[String], attributes: &[syn::Attribute]) -> Vec<String> {
    let mut cfg = parent.to_vec();
    cfg.extend(normalized_cfg_attributes(attributes));
    cfg.sort();
    cfg.dedup();
    cfg
}

fn meta_controls_presence(meta: &Meta) -> Result<bool, String> {
    if meta.path().is_ident("cfg") {
        return Ok(true);
    }
    if !meta.path().is_ident("cfg_attr") {
        return Ok(false);
    }
    let Meta::List(list) = meta else {
        return Err("cfg_attr attribute must use list syntax".to_owned());
    };
    let items = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .map_err(|error| format!("cannot parse cfg_attr arguments: {error}"))?;
    if items.len() < 2 {
        return Err("cfg_attr requires a predicate and at least one attribute".to_owned());
    }
    items.iter().skip(1).try_fold(false, |controls, item| {
        Ok(controls || meta_controls_presence(item)?)
    })
}

/// Whether the inherited fingerprints or new attributes can remove an item
/// from a production build. Inert cfg_attr effects such as lints do not make
/// ownership ambiguous.
pub(crate) fn cfg_context_controls_presence(
    parent: &[String],
    attributes: &[syn::Attribute],
) -> Result<bool, String> {
    for fingerprint in parent {
        let meta: Meta = syn::parse_str(fingerprint)
            .map_err(|error| format!("cannot parse inherited cfg {fingerprint:?}: {error}"))?;
        if meta_controls_presence(&meta)? {
            return Ok(true);
        }
    }
    for attribute in attributes {
        if meta_controls_presence(&attribute.meta)? {
            return Ok(true);
        }
    }
    Ok(false)
}

struct NestedModuleCollector<'ast> {
    modules: Vec<&'ast ItemMod>,
}

impl<'ast> Visit<'ast> for NestedModuleCollector<'ast> {
    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        self.modules.push(module);
        // Do not recurse here: the graph walker processes inline module bodies
        // with their correct Rust module directory.
    }
}

fn validate_source_file(
    candidate: &Path,
    package_root: &Path,
    context: &str,
) -> Result<PathBuf, String> {
    if candidate
        .extension()
        .is_none_or(|extension| extension != "rs")
    {
        return Err(format!(
            "{context} must reference a .rs file, got {}",
            candidate.display()
        ));
    }
    let metadata = fs::symlink_metadata(candidate)
        .map_err(|error| format!("cannot inspect {context} {}: {error}", candidate.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{context} must not reference a symlink: {}",
            candidate.display()
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "{context} is not a regular file: {}",
            candidate.display()
        ));
    }
    let resolved = candidate
        .canonicalize()
        .map_err(|error| format!("cannot resolve {context} {}: {error}", candidate.display()))?;
    if !resolved.starts_with(package_root) {
        return Err(format!(
            "{context} resolves outside package root {}: {}",
            package_root.display(),
            resolved.display()
        ));
    }
    Ok(resolved)
}

fn explicit_path(attributes: &[syn::Attribute]) -> Result<Option<PathBuf>, String> {
    crate::registrations::path_override(attributes)
}

fn child_module_directory(source: &Path) -> Result<PathBuf, String> {
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    if source.file_name().is_some_and(|name| name == "mod.rs") {
        Ok(parent.to_owned())
    } else {
        Ok(parent.join(
            source
                .file_stem()
                .ok_or_else(|| format!("module source has no file stem: {}", source.display()))?,
        ))
    }
}

/// Read a source file with every `#[path]` module spliced back in as an inline
/// module.
///
/// `#[path]` does not change the module tree: `#[path = "x_tests.rs"] mod tests;`
/// declares exactly the module that `mod tests { .. }` would. The analyzers do
/// not see it that way, because each physical file becomes its own unit with its
/// own symbol table, and provenance is resolved from a file's own `use` items.
/// A child that reaches its parent's imports through `use super::*` -- which is
/// real Rust, the child can name an ancestor's private imports -- loses that
/// provenance, because resolving a glob is beyond a syntactic analyzer. Moving a
/// `mod tests` out of its parent then silently deletes rows from the very
/// inventories that exist to notice deletions: 29 of 71 bridge rows and 1,261 of
/// 23,404 persistence rows vanished this way.
///
/// Splicing removes the indirection before anything is analyzed, so the audit
/// sees the module tree the compiler sees and the extraction is invisible to
/// every ratchet at once, rather than each analyzer needing its own repair.
/// The inner attributes a `#[path]` child declares on itself.
///
/// `walk_source_file` extends a mount's cfg with these, so a key built only
/// from the parent's context and the declaration would not match the context
/// the walker actually produced.
fn child_inner_attributes(child: &Path) -> Result<Vec<syn::Attribute>, String> {
    let source = fs::read_to_string(child)
        .map_err(|error| format!("cannot read {}: {error}", child.display()))?;
    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", child.display()))?;
    Ok(syntax.attrs)
}

/// Which (file, logical module path) pairs are reached through a `#[path]`.
///
/// Matched by logical path, not by file, so a file that is also mounted
/// ordinarily keeps that mount: skipping the whole file would drop the ordinary
/// context from the analysis without saying so.
pub(crate) fn spliced_child_contexts(
    mounts: &BTreeMap<PathBuf, BTreeSet<SourceMountContext>>,
) -> Result<BTreeSet<(PathBuf, String, Vec<String>)>, String> {
    let mut spliced = BTreeSet::new();
    for (parent_path, parent_contexts) in mounts {
        let raw = fs::read_to_string(parent_path)
            .map_err(|error| format!("cannot read {}: {error}", parent_path.display()))?;
        for (child, ident, attributes) in path_module_children(parent_path, &raw) {
            for parent_context in parent_contexts {
                // Keyed by cfg as well as name. One file can be mounted under
                // the same logical name through mutually exclusive
                // declarations -- `#[cfg(not(test))] mod foo;` beside
                // `#[cfg(test)] #[path = "foo.rs"] mod foo;` -- and a
                // name-only key matched both, filtering out the production
                // context that no splice represents.
                // The child's own inner attributes count too: `walk_source_file`
                // extends the mount context with them, so a child carrying
                // `#![cfg(unix)]` has a context this key would otherwise miss --
                // leaving the spliced context unfiltered and its contents
                // counted twice.
                let declared = extend_cfg_context(&parent_context.cfg, &attributes);
                spliced.insert((
                    child.clone(),
                    format!("{}::{ident}", parent_context.logical_module_path),
                    extend_cfg_context(&declared, &child_inner_attributes(&child)?),
                ));
            }
        }
    }
    Ok(spliced)
}

/// Files reachable only as `#[path]` children of `source`.
///
/// A consumer that reads sources through [`read_spliced_source`] already has
/// these inside their parent, so analysing them again would double-count them
/// and reintroduce the separate-unit provenance loss the splice exists to
/// remove. The mount graph still records them, which is where the `#[path]`
/// count and the duplicate-owner rejection live.
pub(crate) fn path_module_children(
    source_path: &Path,
    source: &str,
) -> Vec<(PathBuf, String, Vec<syn::Attribute>)> {
    let Ok(syntax) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut children = Vec::new();
    for item in &syntax.items {
        let Item::Mod(module) = item else { continue };
        if module.content.is_some() {
            continue;
        }
        let Ok(Some(relative)) = explicit_path(&module.attrs) else {
            continue;
        };
        let child = source_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative);
        if let Ok(resolved) = child.canonicalize() {
            children.push((resolved, module.ident.to_string(), module.attrs.clone()));
        }
    }
    children
}

pub(crate) fn read_spliced_source(
    source_path: &Path,
    package_root: &Path,
) -> Result<String, String> {
    read_spliced_source_at_depth(source_path, package_root, 0)
}

/// `#[path]` chains are not expected to nest, but a bound keeps a cycle from
/// becoming a stack overflow; `walk_source_file` reports the cycle properly.
const MAX_PATH_MODULE_DEPTH: usize = 8;

fn read_spliced_source_at_depth(
    source_path: &Path,
    package_root: &Path,
    depth: usize,
) -> Result<String, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
    if depth > MAX_PATH_MODULE_DEPTH {
        return Err(format!(
            "#[path] module nesting deeper than {MAX_PATH_MODULE_DEPTH} while splicing {}",
            source_path.display()
        ));
    }

    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", source_path.display()))?;

    // Collected first and applied last-to-first so earlier byte ranges stay valid.
    let mut replacements: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for item in &syntax.items {
        let Item::Mod(module) = item else { continue };
        if module.content.is_some() {
            continue;
        }
        let Some(relative) = explicit_path(&module.attrs)? else {
            continue;
        };
        let child_path = source_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&relative);
        let context = format!(
            "#[path] module {} declared in {}",
            module.ident,
            source_path.display()
        );
        let resolved = validate_source_file(&child_path, package_root, &context)?;
        let child = read_spliced_source_at_depth(&resolved, package_root, depth + 1)?;
        let child = strip_shebang(&child);
        let body = strip_inner_cfg_test(&child, &module.attrs, &resolved)?;

        // The declaration is rebuilt by slicing the original bytes rather than by
        // re-emitting the parsed tokens: `to_token_stream().to_string()` would
        // print `# [cfg (test)]`, which parses the same but no longer matches
        // what the file says, and this text is what every later audit reads.
        let item_range = item.span().byte_range();
        let item_text = &source[item_range.clone()];
        let attribute_range = module
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("path"))
            .map(|attribute| attribute.span().byte_range())
            .ok_or_else(|| format!("lost the #[path] attribute of module {}", module.ident))?;
        let mut declaration = String::with_capacity(item_text.len());
        declaration.push_str(&item_text[..attribute_range.start - item_range.start]);
        // The attribute occupied its own line, so splicing its range out leaves
        // the newline that followed it and the module's remaining attributes end
        // up separated from the declaration they apply to.
        declaration.push_str(
            item_text[attribute_range.end - item_range.start..].trim_start_matches(['\n', '\r']),
        );
        let declaration = declaration
            .trim_end()
            .strip_suffix(';')
            .ok_or_else(|| {
                format!(
                    "#[path] module {} in {} is not a `mod name;` declaration",
                    module.ident,
                    source_path.display()
                )
            })?
            .trim_end()
            .to_owned();
        replacements.push((item_range, format!("{declaration} {{\n{body}\n}}")));
    }

    if replacements.is_empty() {
        return Ok(source);
    }
    replacements.sort_by_key(|(range, _)| range.start);
    let mut spliced = source;
    for (range, text) in replacements.into_iter().rev() {
        spliced.replace_range(range, &text);
    }
    Ok(spliced)
}

/// Whether a normalized cfg makes its item test-only.
///
/// Not a string comparison: `cfg(all(test))` and `cfg(all(test, unix))` are
/// test-only too, and reading them as production-capable charges an entire
/// test child against a production ceiling. `any(..)` is not, since one of its
/// branches can hold without `test`, and a negation is not.
fn cfg_is_test_only(cfg: &str) -> bool {
    let compact = cfg.replace(' ', "");
    let Some(inner) = compact
        .strip_prefix("cfg(")
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return false;
    };
    cfg_predicate_is_test_only(inner)
}

fn cfg_predicate_is_test_only(predicate: &str) -> bool {
    if predicate == "test" {
        return true;
    }
    if let Some(inner) = predicate
        .strip_prefix("all(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        // `all(..)` holds only if every term does, so one test-only term makes
        // the whole thing test-only.
        return split_top_level_predicates(inner)
            .iter()
            .any(|term| cfg_predicate_is_test_only(term));
    }
    if let Some(inner) = predicate
        .strip_prefix("any(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        // `any(..)` holds if any term does, so it is test-only only when every
        // term is.
        let terms = split_top_level_predicates(inner);
        return !terms.is_empty() && terms.iter().all(|term| cfg_predicate_is_test_only(term));
    }
    if let Some(inner) = predicate
        .strip_prefix("not(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        // Double negation is the only shape that survives: `not(not(test))` is
        // `test`. A single `not(test)` is the opposite of test-only.
        return inner
            .strip_prefix("not(")
            .and_then(|rest| rest.strip_suffix(')'))
            .is_some_and(cfg_predicate_is_test_only);
    }
    // Anything else -- a bare feature, a target predicate -- is not test-only.
    false
}

fn split_top_level_predicates(inner: &str) -> Vec<&str> {
    let mut terms = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, character) in inner.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                terms.push(&inner[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    terms.push(&inner[start..]);
    terms
}

/// Skip leading whitespace and comments, so an attribute written with trivia
/// between its tokens is still recognised as one.
fn strip_rust_trivia_prefix(text: &str) -> &str {
    let mut rest = text.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("/*") {
            // Rust block comments nest, so the first `*/` is not necessarily
            // the end: `/* outer /* inner */ end */` closes at the second.
            // Walked by char boundary, not by byte: a comment may hold
            // non-ASCII text, and slicing `&after[index..index + 2]` inside a
            // multi-byte character panics -- taking the whole mandatory check
            // down with it.
            let mut depth = 1usize;
            let mut cursor = after;
            while depth > 0 {
                if let Some(next) = cursor.strip_prefix("/*") {
                    depth += 1;
                    cursor = next;
                } else if let Some(next) = cursor.strip_prefix("*/") {
                    depth -= 1;
                    cursor = next;
                } else {
                    match cursor.chars().next() {
                        Some(character) => cursor = &cursor[character.len_utf8()..],
                        None => return "",
                    }
                }
            }
            rest = cursor.trim_start();
        } else if let Some(after) = rest.strip_prefix("//") {
            rest = after.find('\n').map_or("", |end| after[end..].trim_start());
        } else {
            return rest;
        }
    }
}

/// Drop a leading `#!` shebang line.
///
/// rustc and rustfmt accept a shebang at the top of an external module file,
/// but inside `mod child { .. }` it is not valid Rust: `#!` there begins an
/// inner attribute. Splicing it through made the reconstructed parent
/// unparseable, so the whole file dropped out of the inventory -- a silent loss
/// of every row it owned.
///
/// Only a first line starting `#!` and not `#![`, which is the shebang rather
/// than an inner attribute.
fn strip_shebang(source: &str) -> String {
    // `#! /* keep */ [cfg(test)]` is an inner attribute, not a shebang: the
    // bracket may be separated from the `#!` by trivia, so a `#![` prefix test
    // strips a real attribute's first line.
    let Some(rest) = source.strip_prefix("#!") else {
        return source.to_owned();
    };
    // Not bounded to the first line: a line comment between the `#!` and its
    // bracket puts them on separate lines, and `#!// keep\n[cfg(test)]` is an
    // inner attribute whose first line a shebang rule would delete.
    if strip_rust_trivia_prefix(rest).starts_with('[') {
        return source.to_owned();
    }
    match source.find('\n') {
        // The newline is kept so byte offsets after it do not shift.
        Some(end) => source[end..].to_owned(),
        None => String::new(),
    }
}

/// Drop a `#![cfg(test)]` that the `mod` declaration already states.
///
/// Inlining would otherwise record the module's cfg twice and change its exact
/// identity in the baseline. Every other inner attribute is carried through
/// untouched: `mod m { #![doc = ".."] .. }` means exactly what the same
/// attribute meant at the top of the module's own file, so rewriting it would
/// change the audited source rather than preserve it.
fn strip_inner_cfg_test(
    child: &str,
    module_attributes: &[Attribute],
    child_path: &Path,
) -> Result<String, String> {
    let syntax = syn::parse_file(child)
        .map_err(|error| format!("cannot parse {}: {error}", child_path.display()))?;
    if syntax.attrs.is_empty() {
        return Ok(child.to_owned());
    }
    let module_is_test_only = module_attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute.to_token_stream().to_string().replace(' ', "") == "#[cfg(test)]"
    });
    let mut body = child.to_owned();
    let mut ranges = Vec::new();
    for attribute in &syntax.attrs {
        let text = attribute.to_token_stream().to_string().replace(' ', "");
        if text == "#![cfg(test)]" && module_is_test_only {
            ranges.push(attribute.span().byte_range());
        }
    }
    ranges.sort_by_key(|range| range.start);
    for range in ranges.into_iter().rev() {
        body.replace_range(range, "");
    }
    Ok(body)
}

fn resolve_module_source(
    module: &ItemMod,
    source_path: &Path,
    module_directory: &Path,
    package_root: &Path,
    graph: &mut SourceGraph,
) -> Result<(PathBuf, PathBuf), String> {
    if let Some(path) = explicit_path(&module.attrs)? {
        graph.explicit_path_declarations.insert((
            source_path.to_owned(),
            module.ident.to_string(),
            path.clone(),
        ));
        let candidate = source_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path);
        let context = format!(
            "#[path] module {} declared in {}",
            module.ident,
            source_path.display()
        );
        let resolved = validate_source_file(&candidate, package_root, &context)?;
        let child_directory = child_module_directory(&resolved)?;
        return Ok((resolved, child_directory));
    }

    let module_name = module.ident.to_string();
    let flat = module_directory.join(format!("{module_name}.rs"));
    let nested = module_directory.join(&module_name).join("mod.rs");
    let flat_exists = flat.exists();
    let nested_exists = nested.exists();
    let candidate = match (flat_exists, nested_exists) {
        (true, false) => flat,
        (false, true) => nested,
        (false, false) => {
            return Err(format!(
                "cannot resolve module {module_name} declared in {}",
                source_path.display()
            ));
        }
        (true, true) => {
            return Err(format!(
                "ambiguous module {module_name} declared in {}: both {} and {} exist",
                source_path.display(),
                flat.display(),
                nested.display()
            ));
        }
    };
    let context = format!("module {module_name} declared in {}", source_path.display());
    let resolved = validate_source_file(&candidate, package_root, &context)?;
    Ok((resolved, module_directory.join(module_name)))
}

fn walk_module(
    module: &ItemMod,
    source_path: &Path,
    module_directory: &Path,
    logical_module_path: &str,
    cfg: &[String],
    production_possible: bool,
    test_possible: bool,
    inside_inline_module: bool,
    package_root: &Path,
    graph: &mut SourceGraph,
) -> Result<(), String> {
    let child_logical_path = format!("{logical_module_path}::{}", module.ident);
    let child_cfg = extend_cfg_context(cfg, &module.attrs);
    let child_context_possible =
        cfg_context_allows_production(&child_cfg, &[]).map_err(|error| {
            format!(
                "cannot evaluate cfg on module {} in {}: {error}",
                module.ident,
                source_path.display()
            )
        })?;
    let child_production_possible = production_possible && child_context_possible;
    let child_test_context_possible =
        cfg_context_allows_test(&child_cfg, &[]).map_err(|error| {
            format!(
                "cannot evaluate test cfg on module {} in {}: {error}",
                module.ident,
                source_path.display()
            )
        })?;
    let child_test_possible = test_possible && child_test_context_possible;
    if let Some((_, inline_items)) = &module.content {
        if explicit_path(&module.attrs)?.is_some() {
            return Err(format!(
                "inline module {} in {} must not use #[path]; move the module to a separate .rs \
                 file before using an explicit path",
                module.ident,
                source_path.display()
            ));
        }
        return walk_items(
            inline_items,
            source_path,
            &module_directory.join(module.ident.to_string()),
            &child_logical_path,
            &child_cfg,
            child_production_possible,
            child_test_possible,
            true,
            package_root,
            graph,
        );
    }

    if inside_inline_module && explicit_path(&module.attrs)?.is_some() {
        return Err(format!(
            "#[path] module {} in {} is declared inside an inline module; this closed audit \
             grammar permits #[path] only in file modules",
            module.ident,
            source_path.display()
        ));
    }
    let (child_source, child_directory) =
        resolve_module_source(module, source_path, module_directory, package_root, graph)?;
    walk_source_file(
        &child_source,
        &child_directory,
        &child_logical_path,
        &child_cfg,
        child_production_possible,
        child_test_possible,
        package_root,
        graph,
    )
}

fn walk_items(
    items: &[Item],
    source_path: &Path,
    module_directory: &Path,
    logical_module_path: &str,
    cfg: &[String],
    production_possible: bool,
    test_possible: bool,
    inside_inline_module: bool,
    package_root: &Path,
    graph: &mut SourceGraph,
) -> Result<(), String> {
    for item in items {
        if let Item::Mod(module) = item {
            walk_module(
                module,
                source_path,
                module_directory,
                logical_module_path,
                cfg,
                production_possible,
                test_possible,
                inside_inline_module,
                package_root,
                graph,
            )?;
            continue;
        }
        let mut nested = NestedModuleCollector {
            modules: Vec::new(),
        };
        nested.visit_item(item);
        if let Some(module) = nested.modules.first() {
            return Err(format!(
                "module {} in {} is declared inside a block/item body; production module \
                 declarations must be module-level so the registration owner and source audit \
                 traverse the same grammar",
                module.ident,
                source_path.display()
            ));
        }
    }
    Ok(())
}

fn walk_source_file(
    source_path: &Path,
    module_directory: &Path,
    logical_module_path: &str,
    inherited_cfg: &[String],
    inherited_production_possible: bool,
    inherited_test_possible: bool,
    package_root: &Path,
    graph: &mut SourceGraph,
) -> Result<(), String> {
    let resolved = validate_source_file(source_path, package_root, "production module source")?;
    if graph.active_sources.contains(&resolved) {
        return Err(format!(
            "recursive Rust module source while mounting {} as {logical_module_path}",
            resolved.display()
        ));
    }
    let source = fs::read_to_string(&resolved)
        .map_err(|error| format!("cannot read {}: {error}", resolved.display()))?;
    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", resolved.display()))?;
    let cfg = extend_cfg_context(inherited_cfg, &syntax.attrs);
    let source_context_possible = cfg_context_allows_production(&cfg, &[])
        .map_err(|error| format!("cannot evaluate cfg in {}: {error}", resolved.display()))?;
    let production_possible = inherited_production_possible && source_context_possible;
    let test_context_possible = cfg_context_allows_test(&cfg, &[]).map_err(|error| {
        format!(
            "cannot evaluate test cfg in {}: {error}",
            resolved.display()
        )
    })?;
    let test_possible = inherited_test_possible && test_context_possible;
    let context = SourceMountContext {
        logical_module_path: logical_module_path.to_owned(),
        cfg,
        production_possible,
        test_possible,
    };
    if let Some(previous_mount) = graph.mounts.get_mut(&resolved) {
        if previous_mount.module_directory != module_directory {
            return Err(format!(
                "source {} is mounted with conflicting module directories {} and {}",
                resolved.display(),
                previous_mount.module_directory.display(),
                module_directory.display()
            ));
        }
        previous_mount.contexts.insert(context.clone());
    } else {
        graph.mounts.insert(
            resolved.clone(),
            SourceMount {
                module_directory: module_directory.to_owned(),
                contexts: BTreeSet::from([context.clone()]),
            },
        );
    }
    if !graph
        .visited_mounts
        .insert((resolved.clone(), context.clone()))
    {
        return Ok(());
    }

    graph.active_sources.push(resolved.clone());
    let result = walk_items(
        &syntax.items,
        &resolved,
        module_directory,
        logical_module_path,
        &context.cfg,
        context.production_possible,
        context.test_possible,
        false,
        package_root,
        graph,
    );
    let active_source = graph
        .active_sources
        .pop()
        .expect("source graph active stack is non-empty");
    debug_assert_eq!(active_source, resolved);
    result
}

pub(crate) fn audit_package_source_mounts(
    package_root: &Path,
    production_roots: &[PathBuf],
) -> Result<(BTreeMap<PathBuf, BTreeSet<SourceMountContext>>, usize), String> {
    let package_root = package_root.canonicalize().map_err(|error| {
        format!(
            "cannot resolve package root {}: {error}",
            package_root.display()
        )
    })?;
    let mut graph = SourceGraph::default();
    for production_root in production_roots {
        let root_parent = production_root.parent().ok_or_else(|| {
            format!(
                "production target source has no parent: {}",
                production_root.display()
            )
        })?;
        walk_source_file(
            production_root,
            root_parent,
            "crate",
            &[],
            true,
            true,
            &package_root,
            &mut graph,
        )?;
    }
    Ok((
        graph
            .mounts
            .into_iter()
            .map(|(source, mount)| (source, mount.contexts))
            .collect(),
        graph.explicit_path_declarations.len(),
    ))
}

pub(crate) fn audit_package_source_graph(
    package_root: &Path,
    production_roots: &[PathBuf],
) -> Result<(BTreeMap<PathBuf, BTreeSet<String>>, usize), String> {
    let (mounts, explicit_paths) = audit_package_source_mounts(package_root, production_roots)?;
    Ok((
        mounts
            .into_iter()
            .map(|(source, contexts)| {
                (
                    source,
                    contexts
                        .into_iter()
                        .map(|context| context.logical_module_path)
                        .collect(),
                )
            })
            .collect(),
        explicit_paths,
    ))
}

/// One physical source and all of its logical production/test mount contexts
/// in a Cargo workspace package.
pub(crate) struct WorkspaceSourceMount {
    pub(crate) package: String,
    pub(crate) source_path: PathBuf,
    pub(crate) contexts: BTreeSet<SourceMountContext>,
    pub(crate) source: String,
}

/// Resolve every workspace production lib/bin module graph using the same
/// locked Cargo metadata and path rules as the handler ownership audit.
/// Every `#[path]` mount in the workspace, as repository-relative paths.
///
/// `parent` mounts `child`, and `test_only` says whether the declaration or the
/// child itself is `cfg(test)`. Emitted for the Python guard, which charges a
/// child's lines to its parent and has no business parsing Rust to find them.
///
/// Only `#[path]` declarations are reported. A plain `mod foo;` resolving to
/// `parent/foo.rs` by the ordinary rules is an extraction too, and its lines
/// currently leave the parent's charged total -- so a hotspot could be split
/// that way and the ratchet would accept the drop. Nothing in the tree does
/// this today; closing it means implementing Rust's default path resolution and
/// re-freezing every ratchet against the wider mount set, tracked as #220 and a
/// prerequisite for the `session.rs` split.
#[derive(Debug, serde::Serialize)]
pub(crate) struct PathModuleMount {
    parent: String,
    child: String,
    test_only: bool,
}

pub(crate) fn workspace_path_module_mounts(
    repository_root: &Path,
) -> Result<Vec<PathModuleMount>, String> {
    let mut mounts = Vec::new();
    // Queued rather than iterated: `workspace_source_mounts` omits a spliced
    // child, so a child that mounts a grandchild would never be read and only
    // the first level of a chain would be reported.
    let mut pending: Vec<PathBuf> = workspace_source_mounts(repository_root)?
        .into_iter()
        .map(|mount| mount.source_path)
        .collect();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    while let Some(source_path) = pending.pop() {
        if !seen.insert(source_path.clone()) {
            continue;
        }
        let raw = fs::read_to_string(&source_path)
            .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
        for (child, ident, attributes) in path_module_children(&source_path, &raw) {
            pending.push(child.clone());
            let _ = ident;
            let declared_test_only = normalized_cfg_attributes(&attributes)
                .iter()
                .any(|cfg| cfg_is_test_only(cfg));
            let child_inner = child_inner_attributes(&child)?;
            let inner_test_only = normalized_cfg_attributes(&child_inner)
                .iter()
                .any(|cfg| cfg_is_test_only(cfg));
            mounts.push(PathModuleMount {
                parent: crate::session_ownership::repository_relative_path(
                    repository_root,
                    &source_path,
                )?,
                child: crate::session_ownership::repository_relative_path(repository_root, &child)?,
                test_only: declared_test_only || inner_test_only,
            });
        }
    }
    mounts.sort_by(|left, right| (&left.parent, &left.child).cmp(&(&right.parent, &right.child)));
    // Conservative merge: a pair mounted both test-only and production-capable
    // is production-capable, because charging its lines as tests would take
    // them out of the production ceiling they belong to.
    mounts.dedup_by(|left, right| {
        if left.parent == right.parent && left.child == right.child {
            right.test_only = right.test_only && left.test_only;
            true
        } else {
            false
        }
    });
    Ok(mounts)
}

pub(crate) fn workspace_source_mounts(
    repository_root: &Path,
) -> Result<Vec<WorkspaceSourceMount>, String> {
    let metadata = workspace_metadata(repository_root)?;
    let workspace_members: BTreeSet<_> = required_array(&metadata, "workspace_members", "root")?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "cargo metadata workspace member is not a string".to_owned())
        })
        .collect::<Result<_, _>>()?;
    let scopes = package_audit_scopes(&metadata, &workspace_members)?;
    let mut result = Vec::new();
    for scope in scopes {
        let (mounts, _) = audit_package_source_mounts(&scope.root, &scope.production_roots)
            .map_err(|error| {
                format!(
                    "invalid production source graph for {}: {error}",
                    scope.name
                )
            })?;
        let spliced_contexts = spliced_child_contexts(&mounts)?;
        for (source_path, contexts) in mounts {
            // Contexts reached through a `#[path]` arrive inside their parent's
            // spliced source, so analysing them here too would double-count
            // them. Any other context for the same file -- `mod foo;
            // #[path = "foo.rs"] mod alias;` is valid Rust and mounts one file
            // twice -- still needs analysing, so the contexts are filtered
            // rather than the file skipped.
            let remaining: BTreeSet<SourceMountContext> = contexts
                .into_iter()
                .filter(|context| {
                    !spliced_contexts.contains(&(
                        source_path.clone(),
                        context.logical_module_path.clone(),
                        context.cfg.clone(),
                    ))
                })
                .collect();
            if remaining.is_empty() {
                continue;
            }
            let source = read_spliced_source(&source_path, &scope.root)?;
            result.push(WorkspaceSourceMount {
                package: scope.name.clone(),
                source_path,
                contexts: remaining,
                source,
            });
        }
    }
    result.sort_by(|left, right| {
        (&left.package, &left.source_path).cmp(&(&right.package, &right.source_path))
    });
    Ok(result)
}

fn is_owned_handler_mount(
    package_name: &str,
    logical_paths: &BTreeSet<String>,
    owner: &CapabilityOwner,
) -> bool {
    package_name == owner.package
        && logical_paths.len() == 1
        && logical_paths
            .iter()
            .all(|path| owner.owns_module(package_name, path))
}

#[cfg(test)]
pub(crate) fn audit_package_registration_sources(
    package_name: &str,
    sources: &BTreeMap<PathBuf, BTreeSet<String>>,
    production_lib_roots: &BTreeSet<PathBuf>,
) -> Result<(), String> {
    let test_owner = CapabilityOwner {
        capability: "handler_registration".to_owned(),
        package: "wow-world".to_owned(),
        module: "crate::handlers".to_owned(),
        allow_descendants: true,
        tracking_issue: 153,
    };
    audit_package_registration_sources_with_owner(
        package_name,
        sources,
        production_lib_roots,
        &test_owner,
    )
}

pub(crate) fn audit_package_registration_sources_with_owner(
    package_name: &str,
    sources: &BTreeMap<PathBuf, BTreeSet<String>>,
    production_lib_roots: &BTreeSet<PathBuf>,
    owner: &CapabilityOwner,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let mut exact_collectors = 0usize;
    for (source_path, logical_paths) in sources {
        if is_owned_handler_mount(package_name, logical_paths, owner) {
            continue;
        }
        let source = fs::read_to_string(source_path)
            .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
        let collector_owner = package_name == HANDLER_PACKAGE_NAME
            && production_lib_roots.contains(source_path)
            && logical_paths == &BTreeSet::from(["crate".to_owned()]);
        match analyze_registration_syntax_outside_handlers(source_path, &source, collector_owner) {
            Ok(report) => exact_collectors += report.exact_packet_handler_collectors,
            Err(error) => errors.push(format!("package {package_name}: {error}")),
        }
    }
    if package_name == HANDLER_PACKAGE_NAME && exact_collectors != 1 {
        errors.push(format!(
            "package {HANDLER_PACKAGE_NAME} must define exactly one unconditional module-level \
             inventory::collect!(PacketHandlerEntry), found {exact_collectors}"
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

pub(crate) fn audit_registration_ownership(
    repository_root: &Path,
    owner: &CapabilityOwner,
) -> Result<RegistrationOwnershipReport, String> {
    let metadata = workspace_metadata(repository_root)?;
    let registry_capable = registry_capable_package_ids(&metadata)?;
    let workspace_members: BTreeSet<_> = required_array(&metadata, "workspace_members", "root")?
        .iter()
        .map(|member| {
            member
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "cargo metadata workspace member is not a string".to_owned())
        })
        .collect::<Result<_, _>>()?;
    let scopes = package_audit_scopes(&metadata, &workspace_members)?;

    let mut scanned_files = BTreeSet::new();
    let mut macro_scan_files = BTreeSet::new();
    let mut explicit_path_modules = 0usize;
    let mut errors = Vec::new();
    let mut package_names = Vec::new();
    for scope in &scopes {
        let (sources, explicit_paths) =
            audit_package_source_graph(&scope.root, &scope.production_roots).map_err(|error| {
                format!(
                    "invalid production source graph for {}: {error}",
                    scope.name
                )
            })?;
        for source_path in sources.keys() {
            macro_scan_files.insert((scope.name.clone(), source_path.clone()));
            let source = fs::read_to_string(source_path)
                .map_err(|error| format!("cannot read {}: {error}", source_path.display()))?;
            let exported_macros = exported_macro_names(source_path, &source)?;
            if is_pinned_wow_logging_source(scope, source_path)? {
                let expected: Vec<_> = WOW_LOGGING_EXPORTED_MACROS
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect();
                if exported_macros != expected {
                    errors.push(format!(
                        "package {} source {} changed the exact exported-macro surface: expected \
                         {expected:?}, found {exported_macros:?}",
                        scope.name,
                        source_path.display()
                    ));
                }
            } else if !exported_macros.is_empty() {
                errors.push(format!(
                    "package {} source {} exports declarative macros outside the exact pinned \
                     wow-logging surface: {}",
                    scope.name,
                    source_path.display(),
                    exported_macros.join(", ")
                ));
            }
            let includes = include_macro_bodies(source_path, &source)?;
            if !includes.is_empty()
                && !is_pinned_wow_proto_include_surface(scope, source_path, &includes)?
            {
                errors.push(format!(
                    "package {} source {} uses include! outside the exact pinned wow-proto \
                     generated-source surface; included source is outside the workspace handler \
                     ownership grammar",
                    scope.name,
                    source_path.display()
                ));
            }
            if registry_capable.contains(&scope.id) {
                continue;
            }
            let inventory_calls = inventory_registration_macro_fingerprints(source_path, &source)?;
            if is_pinned_wow_script_inventory_source(scope, source_path)? {
                let expected = pinned_wow_script_inventory_fingerprints()?;
                if inventory_calls != expected {
                    errors.push(format!(
                        "package {} source {} changed the exact non-handler inventory macro \
                         surface: expected {expected:?}, found {inventory_calls:?}",
                        scope.name,
                        source_path.display()
                    ));
                }
            } else if !inventory_calls.is_empty() {
                errors.push(format!(
                    "package {} source {} invokes inventory registration macros outside the \
                     exact pinned wow-script non-handler surface: {}",
                    scope.name,
                    source_path.display(),
                    inventory_calls.join(", ")
                ));
            }
            let alias_violations = registration_alias_violations(&source)?;
            if !alias_violations.is_empty() {
                errors.push(format!(
                    "package {} source {} exposes registration alias capability: {}",
                    scope.name,
                    source_path.display(),
                    alias_violations.join("; ")
                ));
            }
            let definitions = handler_capable_macro_definitions(source_path, &source)?;
            if !definitions.is_empty() {
                errors.push(format!(
                    "package {} source {} defines handler-capable macro_rules! outside the \
                     declared handler-registration owner: {}",
                    scope.name,
                    source_path.display(),
                    definitions.join(", ")
                ));
            }
            let invocations = handler_capable_macro_invocations(source_path, &source)?;
            if !invocations.is_empty() {
                errors.push(format!(
                    "package {} source {} invokes macros with handler-capable paths or source \
                     tokens outside the declared handler-registration owner: {}",
                    scope.name,
                    source_path.display(),
                    invocations.join(", ")
                ));
            }
        }
        if registry_capable.contains(&scope.id) {
            package_names.push(scope.name.clone());
            explicit_path_modules += explicit_paths;
            for source_path in sources.keys() {
                scanned_files.insert((scope.name.clone(), source_path.clone()));
            }
            if let Err(error) = audit_package_registration_sources_with_owner(
                &scope.name,
                &sources,
                &scope.production_lib_roots,
                owner,
            ) {
                errors.push(error);
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }

    Ok(RegistrationOwnershipReport {
        scanned_packages: registry_capable.len(),
        scanned_files: scanned_files.len(),
        macro_scan_packages: scopes.len(),
        macro_scan_files: macro_scan_files.len(),
        explicit_path_modules,
        package_names,
    })
}
