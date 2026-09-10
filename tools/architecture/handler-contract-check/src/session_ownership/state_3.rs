//! Session ownership inventory state definitions, part 3 of 3.
//!
//! Separated from the session_ownership.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) fn collect_units(
    units: Vec<SourceUnit>,
    persistence_accesses: PersistenceAccessBaseline,
) -> Result<SessionSyntaxBaseline, String> {
    let registry_sources: Vec<_> = units
        .iter()
        .filter(|unit| unit.availability.production)
        .map(|unit| ProductionRegistrySource {
            package: unit.role.package_name(),
            module: &unit.logical_module_path,
            source_path: &unit.repository_relative_path,
            inherited_cfg: &unit.cfg,
            source: &unit.source,
        })
        .collect();
    let registry_accesses = inventory_registry_accesses(&registry_sources)
        .map_err(|error| format!("cannot inventory direct registry accesses:\n{error}"))?;
    let bridge_sources: Vec<_> = units
        .iter()
        .filter(|unit| {
            unit.availability.source_class().is_some()
                && matches!(unit.role, PackageRole::World | PackageRole::Server)
        })
        .map(|unit| BridgeSource {
            package: unit.role.package_name(),
            module: &unit.logical_module_path,
            source_path: &unit.repository_relative_path,
            inherited_cfg: &unit.cfg,
            source: &unit.source,
        })
        .collect();
    let bridge_accesses = inventory_bridge_accesses(&bridge_sources)
        .map_err(|error| format!("cannot inventory legacy/canonical bridges:\n{error}"))?;
    let mut builder = BaselineBuilder::default();
    for unit in units {
        if unit.availability.source_class().is_none() {
            continue;
        }
        let syntax = syn::parse_file(&unit.source)
            .map_err(|error| format!("cannot parse {}: {error}", unit.source_path.display()))?;
        let mut include_guard = IncludeMacroGuard::default();
        include_guard.visit_file(&syntax);
        if include_guard.count > 0 {
            builder.errors.push(format!(
                "{} contains {} include! macro invocation(s); generated source inputs are outside \
                 the closed ownership grammar",
                unit.source_path.display(),
                include_guard.count,
            ));
        }
        collect_items(
            unit.role,
            &syntax.items,
            &unit.logical_module_path,
            &unit.cfg,
            unit.availability,
            &mut builder,
        );
    }
    builder.finish(registry_accesses, persistence_accesses, bridge_accesses)
}

pub(crate) fn repository_relative_path(
    repository_root: &Path,
    source_path: &Path,
) -> Result<String, String> {
    let relative = source_path.strip_prefix(repository_root).map_err(|_| {
        format!(
            "audited source {} is outside repository root {}",
            source_path.display(),
            repository_root.display()
        )
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let part = component.as_os_str().to_str().ok_or_else(|| {
            format!(
                "audited source path is not valid UTF-8: {}",
                source_path.display()
            )
        })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

pub(super) fn repository_units(
    repository_root: &Path,
    role: PackageRole,
    package_root: &str,
    crate_root: &str,
) -> Result<Vec<SourceUnit>, String> {
    let package_root = repository_root.join(package_root);
    let crate_root = repository_root.join(crate_root);
    let (mounts, _) = audit_package_source_mounts(&package_root, &[crate_root])?;
    let spliced_contexts = crate::ownership::spliced_child_contexts(&mounts)?;
    let mut units = Vec::new();
    for (source_path, contexts) in mounts {
        // Filtered by context, not by file: see the workspace collector for why
        // a file reached both through `#[path]` and ordinarily keeps the
        // ordinary mount.
        let contexts: std::collections::BTreeSet<_> = contexts
            .into_iter()
            .filter(|context| {
                !spliced_contexts.contains(&(
                    source_path.clone(),
                    context.logical_module_path.clone(),
                    context.cfg.clone(),
                ))
            })
            .collect();
        if contexts.is_empty() {
            continue;
        }
        let source = read_spliced_source(&source_path, &package_root)?;
        for SourceMountContext {
            logical_module_path,
            cfg,
            production_possible,
            test_possible,
        } in contexts
        {
            let repository_relative_path = repository_relative_path(repository_root, &source_path)?;
            units.push(SourceUnit {
                role,
                source_path: source_path.clone(),
                repository_relative_path,
                logical_module_path,
                cfg,
                availability: Availability {
                    production: production_possible,
                    test: test_possible,
                },
                source: source.clone(),
            });
        }
    }
    Ok(units)
}

pub(super) fn persistence_classification(package: &str) -> &'static str {
    match package {
        "wow-database" => "reviewed_adapter",
        "world-server" | "bnet-server" => "composition",
        _ => "direct_application_or_domain_access",
    }
}

pub(super) fn collect_workspace_persistence_baseline(
    repository_root: &Path,
) -> Result<PersistenceAccessBaseline, String> {
    let mounts = workspace_source_mounts(repository_root)?;
    let dependencies = workspace_dependency_aliases(repository_root)?;
    let relative_paths = mounts
        .iter()
        .map(|mount| repository_relative_path(repository_root, &mount.source_path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut sources = Vec::new();
    for (mount, relative_path) in mounts.iter().zip(relative_paths.iter()) {
        for context in &mount.contexts {
            if !context.production_possible && !context.test_possible {
                continue;
            }
            sources.push(ClassifiedPersistenceSource {
                classification: persistence_classification(&mount.package),
                package: &mount.package,
                module: &context.logical_module_path,
                source_path: relative_path,
                inherited_cfg: &context.cfg,
                source: &mount.source,
            });
        }
    }
    inventory_persistence_accesses_with_dependencies(&sources, &dependencies)
        .map_err(|error| format!("cannot inventory persistence accesses:\n{error}"))
}

/// Collect the session syntax surface, optionally without the persistence scan.
///
/// The persistence inventory costs a full workspace scan — minutes, and the
/// dominant cost of the whole gate. `print-baseline` renders an envelope whose
/// persistence field is `#[serde(skip)]`, so paying for that scan there bought
/// a value that was then discarded unread.
pub(super) fn collect_repository_baseline_with_persistence(
    repository_root: &Path,
    with_persistence: bool,
) -> Result<SessionSyntaxBaseline, String> {
    let mut units = repository_units(
        repository_root,
        PackageRole::World,
        WORLD_PACKAGE_ROOT,
        WORLD_CRATE_ROOT,
    )?;
    units.extend(repository_units(
        repository_root,
        PackageRole::Server,
        SERVER_PACKAGE_ROOT,
        SERVER_CRATE_ROOT,
    )?);
    units.extend(repository_units(
        repository_root,
        PackageRole::Network,
        NETWORK_PACKAGE_ROOT,
        NETWORK_CRATE_ROOT,
    )?);
    units.extend(repository_units(
        repository_root,
        PackageRole::Social,
        SOCIAL_PACKAGE_ROOT,
        SOCIAL_CRATE_ROOT,
    )?);
    let persistence_accesses = if with_persistence {
        collect_workspace_persistence_baseline(repository_root)?
    } else {
        PersistenceAccessBaseline::default()
    };
    let baseline = collect_units(units, persistence_accesses)?;
    validate_curated_bridge_anchors(&baseline.bridge_accesses)
        .map_err(|error| format!("invalid curated bridge inventory:\n{error}"))?;
    Ok(baseline)
}

pub(super) fn set_drift<T>(label: &str, expected: &[T], actual: &[T], errors: &mut Vec<String>)
where
    T: Clone + Ord + Serialize,
{
    let expected: BTreeSet<_> = expected.iter().cloned().collect();
    let actual: BTreeSet<_> = actual.iter().cloned().collect();
    for removed in expected.difference(&actual) {
        errors.push(format!(
            "obsolete {label} baseline entry: {}",
            serde_json::to_string(removed).expect("surface serializes")
        ));
    }
    for added in actual.difference(&expected) {
        errors.push(format!(
            "unreviewed {label} surface: {}",
            serde_json::to_string(added).expect("surface serializes")
        ));
    }
}

pub(super) fn compare_baseline(
    expected: &SessionSyntaxBaseline,
    actual: &SessionSyntaxBaseline,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if expected.world_session.definition != actual.world_session.definition {
        errors.push(format!(
            "WorldSession definition changed: expected {:?}, actual {:?}",
            expected.world_session.definition, actual.world_session.definition
        ));
    }
    set_drift(
        "WorldSession field",
        &expected.world_session.fields,
        &actual.world_session.fields,
        &mut errors,
    );
    set_drift(
        "WorldSession impl",
        &expected.world_session.impls,
        &actual.world_session.impls,
        &mut errors,
    );
    set_drift(
        "WorldSession impl item",
        &expected.world_session.impl_items,
        &actual.world_session.impl_items,
        &mut errors,
    );
    if expected.session_resources.definition != actual.session_resources.definition {
        errors.push(format!(
            "SessionResources definition changed: expected {:?}, actual {:?}",
            expected.session_resources.definition, actual.session_resources.definition
        ));
    }
    set_drift(
        "SessionResources field",
        &expected.session_resources.fields,
        &actual.session_resources.fields,
        &mut errors,
    );
    set_drift(
        "SessionResources construction site",
        &expected.session_resources.construction_sites,
        &actual.session_resources.construction_sites,
        &mut errors,
    );
    if expected.session_factory.definition != actual.session_factory.definition {
        errors.push(format!(
            "create_session definition changed: expected {:?}, actual {:?}",
            expected.session_factory.definition, actual.session_factory.definition
        ));
    }
    if expected.session_factory.signature != actual.session_factory.signature {
        errors.push(format!(
            "create_session signature changed: expected {:?}, actual {:?}",
            expected.session_factory.signature, actual.session_factory.signature
        ));
    }
    if expected.session_factory.body_fingerprint != actual.session_factory.body_fingerprint {
        errors.push(format!(
            "create_session body fingerprint changed: expected {:?}, actual {:?}",
            expected.session_factory.body_fingerprint, actual.session_factory.body_fingerprint
        ));
    }
    set_drift(
        "create_session session-bearing helper body",
        &expected.session_factory.session_helper_bodies,
        &actual.session_factory.session_helper_bodies,
        &mut errors,
    );
    set_drift(
        "create_session call site",
        &expected.session_factory.call_sites,
        &actual.session_factory.call_sites,
        &mut errors,
    );
    set_drift(
        "WorldSession::new call site",
        &expected.session_factory.world_session_new_sites,
        &actual.session_factory.world_session_new_sites,
        &mut errors,
    );
    set_drift(
        "create_session setter call",
        &expected.session_factory.setter_call_sites,
        &actual.session_factory.setter_call_sites,
        &mut errors,
    );
    if expected.session_command != actual.session_command {
        errors.push(format!(
            "SessionCommand variants changed: expected {:?}, actual {:?}",
            expected.session_command, actual.session_command
        ));
    }
    set_drift(
        "SessionCommand transitive payload type",
        &expected.session_command_payload_types,
        &actual.session_command_payload_types,
        &mut errors,
    );
    set_drift(
        "generated ownership input",
        &expected.generated_surface_inputs,
        &actual.generated_surface_inputs,
        &mut errors,
    );
    if let Err(error) =
        compare_registry_access_baseline(&expected.registry_accesses, &actual.registry_accesses)
    {
        errors.push(error);
    }
    if let Err(error) =
        compare_bridge_access_baseline(&expected.bridge_accesses, &actual.bridge_accesses)
    {
        errors.push(error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

pub(super) fn load_policy(path: &Path) -> Result<PolicyEnvelope, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let policy: PolicyEnvelope = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid session ownership policy {}: {error}",
            path.display()
        )
    })?;
    if policy.schema_version != 1 {
        return Err(format!(
            "session ownership policy schema_version must be 1, got {}",
            policy.schema_version
        ));
    }
    Ok(policy)
}

/// Check the repository against the exact AST surface checked into the policy.
pub fn check_repository(policy_path: Option<&Path>) -> Result<String, String> {
    check_repository_scoped(policy_path, true)
}

/// Check the session syntax surface without the exact persistence comparison.
///
/// The persistence scan is minutes; the session surface is seconds. Pull
/// requests run this so the ownership ratchet still guards every change,
/// while the exact inventory is compared on push and on the weekly cron —
/// detected at merge rather than at review time, not detected less.
pub fn check_repository_syntax_only(policy_path: Option<&Path>) -> Result<String, String> {
    check_repository_scoped(policy_path, false)
}

pub(super) fn check_repository_scoped(
    policy_path: Option<&Path>,
    with_persistence: bool,
) -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let policy_path = policy_path
        .map(Path::to_owned)
        .unwrap_or_else(|| repository_root.join(POLICY_RELATIVE_PATH));
    let policy = load_policy(&policy_path)?;
    let actual = collect_repository_baseline_with_persistence(&repository_root, with_persistence)?;
    compare_baseline(&policy.syntax_baseline, &actual)?;
    if !with_persistence {
        return Ok(format!(
            "session ownership: PASS (syntax only; {} production + {} test-fixture WorldSession \
             fields; {} impl owners / {} exact associated items; {} SessionResources fields; {} \
             SessionCommand variants; {} exact direct-registry rows; exact persistence inventory \
             deferred to push and cron)",
            actual
                .world_session
                .fields
                .iter()
                .filter(|field| field.cfg.is_empty())
                .count(),
            actual
                .world_session
                .fields
                .iter()
                .filter(|field| !field.cfg.is_empty())
                .count(),
            actual.world_session.impls.len(),
            actual.world_session.impl_items.len(),
            actual.session_resources.fields.len(),
            actual.session_command.variants.len(),
            actual.registry_accesses.accesses.len(),
        ));
    }
    let persistence_snapshot_path = repository_root.join(&policy.persistence_access_snapshot);
    let persistence_snapshot_source =
        fs::read_to_string(&persistence_snapshot_path).map_err(|error| {
            format!(
                "cannot read {}: {error}",
                persistence_snapshot_path.display()
            )
        })?;
    let expected_persistence: PersistenceAccessBaseline =
        serde_json::from_str(&persistence_snapshot_source).map_err(|error| {
            format!(
                "invalid persistence access snapshot {}: {error}",
                persistence_snapshot_path.display()
            )
        })?;
    compare_persistence_access_baseline(&expected_persistence, &actual.persistence_accesses)?;
    let persistence_policy_path = repository_root.join(PERSISTENCE_POLICY_RELATIVE_PATH);
    let persistence_annotations_path = repository_root.join(PERSISTENCE_ANNOTATIONS_RELATIVE_PATH);
    let issue_ledger_path = repository_root.join(ISSUE_LEDGER_RELATIVE_PATH);
    let (semantic_production_rows, semantic_test_rows, generated_persistence_rows, semantic_groups) =
        crate::persistence_policy::validate_persistence_policy(
            &persistence_policy_path,
            &persistence_annotations_path,
            &issue_ledger_path,
            &actual.persistence_accesses,
        )
        .map_err(|error| format!("invalid persistence semantic ownership:\n{error}"))?;
    let production_session_fields = actual
        .world_session
        .fields
        .iter()
        .filter(|field| field.source_class == "production")
        .count();
    let test_session_fields = actual
        .world_session
        .fields
        .iter()
        .filter(|field| field.source_class == "test_fixture")
        .count();
    let production_persistence_rows = actual
        .persistence_accesses
        .accesses
        .iter()
        .filter(|access| access.source_class == "production")
        .count();
    let test_persistence_rows = actual
        .persistence_accesses
        .accesses
        .iter()
        .filter(|access| access.source_class == "test_fixture")
        .count();
    debug_assert_eq!(production_persistence_rows, semantic_production_rows);
    debug_assert_eq!(test_persistence_rows, semantic_test_rows);
    Ok(format!(
        "session ownership: PASS ({production_session_fields} production + {test_session_fields} \
         test-fixture WorldSession fields; {} impl owners / {} exact associated \
         items; {} SessionResources fields; {} factory setter/install calls; {} SessionCommand \
         variants / {} transitive payload types; {} exact generated \
         inputs; {} exact direct-registry rows; {production_persistence_rows} production + \
         {test_persistence_rows} test-fixture persistence rows \
         ({generated_persistence_rows} generated-input rows, subset; {semantic_groups} exact semantic groups); {} exact bridge rows; \
         include/target-macro surfaces fail closed)",
        actual.world_session.impls.len(),
        actual.world_session.impl_items.len(),
        actual.session_resources.fields.len(),
        actual.session_factory.setter_call_sites.len(),
        actual.session_command.variants.len(),
        actual.session_command_payload_types.len(),
        actual.generated_surface_inputs.len(),
        actual.registry_accesses.accesses.len(),
        actual.bridge_accesses.bridges.len(),
    ))
}

/// Render the current syntax surface as a minimal schema-v1 policy envelope.
///
/// The command only returns text; callers must review and merge the
/// `syntax_baseline` object deliberately into the semantic policy.
/// Print every `#[path]` mount in the workspace, resolved by `syn`.
///
/// The Python guard charges a `#[path]` child's lines to the parent that mounts
/// it, and finding those mounts by scanning text meant reimplementing a Rust
/// lexer: comments, escapes, raw strings, char literals, macro bodies, trivia
/// between attribute tokens. Each gap was a way to move a ceiling, and the list
/// does not end -- an invoked macro can generate a real mount that no scanner
/// can see without expanding it. There is one parser in this repository that
/// already gets this right, so the guard asks it instead of growing a second.
pub fn print_path_module_mounts() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let mounts = crate::ownership::workspace_path_module_mounts(&repository_root)?;
    serde_json::to_string_pretty(&mounts)
        .map_err(|error| format!("cannot serialize path module mounts: {error}"))
}

pub fn print_repository_baseline() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    // Without persistence: the envelope skips that field when serializing, so
    // scanning for it would discard the result.
    let baseline = collect_repository_baseline_with_persistence(&repository_root, false)?;
    serde_json::to_string_pretty(&BaselineEnvelope {
        schema_version: 1,
        persistence_access_snapshot: PERSISTENCE_ACCESS_SNAPSHOT_RELATIVE_PATH,
        syntax_baseline: &baseline,
    })
    .map_err(|error| format!("cannot serialize session ownership baseline: {error}"))
}

/// Render the dedicated exact persistence snapshot without editing it.
pub fn print_repository_persistence_baseline() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let baseline = collect_workspace_persistence_baseline(&repository_root)?;
    render_persistence_access_baseline(&baseline)
}

/// Render the canonical semantic policy from an already computed snapshot.
///
/// The policy is a pure function of the reviewed annotations and the exact
/// inventory, but recomputing that inventory costs a full workspace scan. CI
/// publishes the recomputed snapshot as an artifact when the ratchet moves;
/// deriving the policy from that same artifact keeps both checked-in files
/// consistent without paying for a second scan, and without letting a
/// separately scanned policy disagree with the snapshot beside it.
pub fn print_repository_persistence_policy_from_snapshot(
    snapshot_path: &Path,
) -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let source = fs::read_to_string(snapshot_path)
        .map_err(|error| format!("cannot read {}: {error}", snapshot_path.display()))?;
    let baseline: PersistenceAccessBaseline = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid persistence access snapshot {}: {error}",
            snapshot_path.display()
        )
    })?;
    crate::persistence_policy::render_persistence_policy(
        &repository_root.join(PERSISTENCE_ANNOTATIONS_RELATIVE_PATH),
        &baseline,
    )
}

/// Render the canonical semantic policy derived from reviewed workflow annotations.
pub fn print_repository_persistence_policy() -> Result<String, String> {
    let repository_root = crate::repository_root()?;
    let baseline = collect_workspace_persistence_baseline(&repository_root)?;
    crate::persistence_policy::render_persistence_policy(
        &repository_root.join(PERSISTENCE_ANNOTATIONS_RELATIVE_PATH),
        &baseline,
    )
}
