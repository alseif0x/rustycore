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

use serde_json::Value;
use syn::visit::Visit;
use syn::{Item, ItemMod};

use crate::registrations::{
    analyze_registration_syntax_outside_handlers, exported_macro_names,
    handler_capable_macro_definitions, handler_capable_macro_invocations, include_macro_bodies,
    inventory_registration_macro_fingerprints, registration_alias_violations,
};

const HANDLER_PACKAGE_NAME: &str = "wow-handler";
const WORLD_PACKAGE_NAME: &str = "wow-world";
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
    visited_mounts: BTreeSet<(PathBuf, String)>,
    active_sources: Vec<PathBuf>,
}

#[derive(Debug)]
struct SourceMount {
    module_directory: PathBuf,
    logical_paths: BTreeSet<String>,
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
    inside_inline_module: bool,
    package_root: &Path,
    graph: &mut SourceGraph,
) -> Result<(), String> {
    let child_logical_path = format!("{logical_module_path}::{}", module.ident);
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
        package_root,
        graph,
    )
}

fn walk_items(
    items: &[Item],
    source_path: &Path,
    module_directory: &Path,
    logical_module_path: &str,
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
    package_root: &Path,
    graph: &mut SourceGraph,
) -> Result<(), String> {
    let resolved = validate_source_file(source_path, package_root, "production module source")?;
    if let Some(previous_mount) = graph.mounts.get_mut(&resolved) {
        if previous_mount.module_directory != module_directory {
            return Err(format!(
                "source {} is mounted with conflicting module directories {} and {}",
                resolved.display(),
                previous_mount.module_directory.display(),
                module_directory.display()
            ));
        }
        previous_mount
            .logical_paths
            .insert(logical_module_path.to_owned());
    } else {
        graph.mounts.insert(
            resolved.clone(),
            SourceMount {
                module_directory: module_directory.to_owned(),
                logical_paths: BTreeSet::from([logical_module_path.to_owned()]),
            },
        );
    }
    if graph.active_sources.contains(&resolved) {
        return Err(format!(
            "recursive Rust module source while mounting {} as {logical_module_path}",
            resolved.display()
        ));
    }
    if !graph
        .visited_mounts
        .insert((resolved.clone(), logical_module_path.to_owned()))
    {
        return Ok(());
    }

    graph.active_sources.push(resolved.clone());
    let source = fs::read_to_string(&resolved)
        .map_err(|error| format!("cannot read {}: {error}", resolved.display()))?;
    let syntax = syn::parse_file(&source)
        .map_err(|error| format!("cannot parse {}: {error}", resolved.display()))?;
    let result = walk_items(
        &syntax.items,
        &resolved,
        module_directory,
        logical_module_path,
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

pub(crate) fn audit_package_source_graph(
    package_root: &Path,
    production_roots: &[PathBuf],
) -> Result<(BTreeMap<PathBuf, BTreeSet<String>>, usize), String> {
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
            &package_root,
            &mut graph,
        )?;
    }
    Ok((
        graph
            .mounts
            .into_iter()
            .map(|(source, mount)| (source, mount.logical_paths))
            .collect(),
        graph.explicit_path_declarations.len(),
    ))
}

fn is_owned_handler_mount(package_name: &str, logical_paths: &BTreeSet<String>) -> bool {
    package_name == WORLD_PACKAGE_NAME
        && !logical_paths.is_empty()
        && logical_paths
            .iter()
            .all(|path| path == "crate::handlers" || path.starts_with("crate::handlers::"))
}

pub(crate) fn audit_package_registration_sources(
    package_name: &str,
    sources: &BTreeMap<PathBuf, BTreeSet<String>>,
    production_lib_roots: &BTreeSet<PathBuf>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let mut exact_collectors = 0usize;
    for (source_path, logical_paths) in sources {
        if is_owned_handler_mount(package_name, logical_paths) {
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
                     audited crate::handlers owner: {}",
                    scope.name,
                    source_path.display(),
                    definitions.join(", ")
                ));
            }
            let invocations = handler_capable_macro_invocations(source_path, &source)?;
            if !invocations.is_empty() {
                errors.push(format!(
                    "package {} source {} invokes macros with handler-capable paths or source \
                     tokens outside the audited crate::handlers owner: {}",
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
            if let Err(error) = audit_package_registration_sources(
                &scope.name,
                &sources,
                &scope.production_lib_roots,
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
