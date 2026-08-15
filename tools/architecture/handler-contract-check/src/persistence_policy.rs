// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Semantic ownership policy for the exact persistence syntax inventory.
//!
//! The AST snapshot answers “what concrete persistence syntax exists?”. This
//! policy answers “who currently owns it, what connection/ordering semantics
//! constrain it, and which open slice removes or decides the exception?”. A
//! selector is deliberately small and exact: every snapshot row must match
//! one group, no row may match two, and a group that stops matching is stale.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::persistence_access::{
    PersistenceAccessBaseline, PersistenceOperation, PersistenceTarget,
};

const POLICY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u32,
    generated_code: GeneratedCodePolicy,
    groups: Vec<Group>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct GeneratedCodePolicy {
    expected_rows: usize,
    discovery_rule: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Group {
    id: String,
    source_class: String,
    packages: Vec<String>,
    #[serde(default)]
    sources: Vec<String>,
    #[serde(default)]
    source_prefixes: Vec<String>,
    #[serde(default)]
    modules: Vec<String>,
    #[serde(default)]
    enclosings: Vec<String>,
    #[serde(default)]
    enclosing_prefixes: Vec<String>,
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    operations: Vec<String>,
    logical_databases: Vec<String>,
    capability_owner: String,
    connection_affinity: String,
    current_order: String,
    failure_and_unknown_commit: String,
    target_issues: Vec<u64>,
    retirement_condition: String,
    stable_boundary: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowAnnotations {
    schema_version: u32,
    workflows: Vec<WorkflowAnnotation>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowAnnotation {
    package: String,
    // The canonical module belongs to the identity: one physical file can
    // hold same-named persistence functions in different inline modules, and
    // dropping it would merge their rows under a single annotation while
    // assigning potentially different owners, ordering, and retirement
    // conditions to the merged workflow.
    module: String,
    source: String,
    enclosing: String,
    logical_databases: Vec<String>,
    boundary: String,
    target_issues: Vec<u64>,
    stable_boundary: bool,
}

type WorkflowKey = (String, String, String, String);

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn load_annotations(path: &Path) -> Result<WorkflowAnnotations, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let annotations: WorkflowAnnotations = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid persistence workflow annotations {}: {error}",
            path.display()
        )
    })?;
    if annotations.schema_version != 1 {
        return Err(format!(
            "persistence workflow annotations schema_version must be 1, got {}",
            annotations.schema_version
        ));
    }
    Ok(annotations)
}

fn workflow_key(package: &str, module: &str, source: &str, enclosing: &str) -> WorkflowKey {
    (
        package.to_owned(),
        module.to_owned(),
        source.to_owned(),
        enclosing.to_owned(),
    )
}

fn target_logical_database(target: PersistenceTarget) -> Option<&'static str> {
    match target {
        PersistenceTarget::LoginDatabase | PersistenceTarget::LoginStatements => Some("login"),
        PersistenceTarget::WorldDatabase | PersistenceTarget::WorldStatements => Some("world"),
        PersistenceTarget::CharacterDatabase
        | PersistenceTarget::CharStatements
        | PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp => Some("characters"),
        PersistenceTarget::HotfixDatabase | PersistenceTarget::HotfixStatements => Some("hotfix"),
        _ => None,
    }
}

fn generate_policy(
    annotations: &WorkflowAnnotations,
    baseline: &PersistenceAccessBaseline,
) -> Result<Policy, String> {
    let mut production_rows: BTreeMap<WorkflowKey, Vec<_>> = BTreeMap::new();
    let mut test_sources: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for access in &baseline.accesses {
        if access.source_class == "production" {
            production_rows
                .entry(workflow_key(
                    &access.package,
                    &access.module,
                    &access.source,
                    &access.enclosing,
                ))
                .or_default()
                .push(access);
        } else if access.source_class == "test_fixture" {
            test_sources
                .entry(access.package.clone())
                .or_default()
                .insert(access.source.clone());
        }
    }

    let mut annotation_map = BTreeMap::new();
    let mut errors = Vec::new();
    for annotation in &annotations.workflows {
        let key = workflow_key(
            &annotation.package,
            &annotation.module,
            &annotation.source,
            &annotation.enclosing,
        );
        if annotation.logical_databases.is_empty()
            || !non_empty(&annotation.boundary)
            || annotation
                .logical_databases
                .iter()
                .any(|value| !non_empty(value))
        {
            errors.push(format!(
                "persistence workflow annotation {:?} is missing semantic data",
                key
            ));
        }
        if annotation.logical_databases.iter().any(|database| {
            !matches!(
                database.as_str(),
                "login" | "world" | "characters" | "hotfix" | "updater-control"
            )
        }) {
            errors.push(format!(
                "persistence workflow annotation {:?} names an unknown logical database",
                key
            ));
        }
        if !annotation
            .logical_databases
            .windows(2)
            .all(|pair| pair[0] < pair[1])
            || !annotation
                .target_issues
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        {
            errors.push(format!(
                "persistence workflow annotation {:?} databases/issues must be sorted and unique",
                key
            ));
        }
        if annotation.stable_boundary && !annotation.target_issues.is_empty() {
            errors.push(format!(
                "stable workflow annotation {:?} names retirement issues",
                key
            ));
        }
        if !annotation.stable_boundary && annotation.target_issues.is_empty() {
            errors.push(format!(
                "non-stable workflow annotation {:?} names no retirement issue",
                key
            ));
        }
        if annotation_map.insert(key.clone(), annotation).is_some() {
            errors.push(format!(
                "duplicate persistence workflow annotation {:?}",
                key
            ));
        }
    }
    for key in production_rows.keys() {
        if !annotation_map.contains_key(key) {
            errors.push(format!(
                "production persistence workflow {:?} has no annotation",
                key
            ));
        }
    }
    for key in annotation_map.keys() {
        if !production_rows.contains_key(key) {
            errors.push(format!(
                "obsolete persistence workflow annotation {:?}",
                key
            ));
        }
    }
    for (key, rows) in &production_rows {
        let Some(annotation) = annotation_map.get(key) else {
            continue;
        };
        let exact_databases = rows
            .iter()
            .filter_map(|row| target_logical_database(row.target))
            .collect::<BTreeSet<_>>();
        for database in exact_databases {
            if !annotation
                .logical_databases
                .iter()
                .any(|annotated| annotated == database)
            {
                errors.push(format!(
                    "persistence workflow annotation {:?} omits exact typed logical database {database}",
                    key
                ));
            }
        }
    }
    if !errors.is_empty() {
        errors.sort();
        return Err(errors.join("\n"));
    }

    let mut groups = Vec::new();
    for (key, rows) in production_rows {
        let annotation = annotation_map[&key];
        let (package, module, source, enclosing) = key;
        let dependency_surface = matches!(enclosing.as_str(), "module")
            || enclosing.starts_with("struct ")
            || enclosing.starts_with("enum ")
            || enclosing.starts_with("trait ");
        let explicit_transaction = rows.iter().any(|row| {
            matches!(
                row.operation,
                PersistenceOperation::Begin
                    | PersistenceOperation::Commit
                    | PersistenceOperation::Rollback
                    | PersistenceOperation::TransactionConstruct
                    | PersistenceOperation::TransactionAppend
            ) || matches!(
                row.target,
                PersistenceTarget::SqlTransaction | PersistenceTarget::SqlxTransaction
            )
        });
        let unknown_commit = rows.iter().any(|row| {
            row.target == PersistenceTarget::SqlTransactionCommitError
                || (row.operation == PersistenceOperation::Commit
                    && row.symbol == "commit_with_outcome_like_cpp")
        });
        let logical = annotation.logical_databases.join(", ");
        let affinity = if annotation.stable_boundary {
            format!(
                "The typed adapter is instantiated against exactly one of {logical} per value/transaction; the list denotes supported logical databases, not a simultaneous distributed workflow."
            )
        } else if dependency_surface {
            format!(
                "This is a shared dependency/type surface for {logical}; it does not itself imply a runtime transaction."
            )
        } else if annotation.logical_databases.len() > 1 {
            format!(
                "The workflow uses independent {logical} connections/pools in source order; no distributed ACID boundary is inferred."
            )
        } else if explicit_transaction {
            format!(
                "The workflow preserves one explicit {logical} connection/transaction across its current statement sequence."
            )
        } else {
            format!(
                "The workflow acquires or uses the {logical} adapter as currently coded; no wider transaction is inferred."
            )
        };
        let order = if dependency_surface {
            "No runtime order is inferred from this shared dependency/type surface; consumers retain their own traced order.".to_owned()
        } else if explicit_transaction {
            "Preserve the traced load/plan/append/commit/publication order for the whole workflow; helpers and concrete types are not independent units.".to_owned()
        } else {
            "Preserve the workflow's current source and await order; the inventory does not invent an atomic boundary.".to_owned()
        };
        let failure = if unknown_commit {
            "Preserve the explicit distinction between definite rollback and unknown commit outcome, including quarantine/reconciliation behavior.".to_owned()
        } else if explicit_transaction {
            "Preserve current rollback/commit error propagation and publication suppression; no stronger retry or unknown-outcome guarantee is inferred.".to_owned()
        } else {
            "Preserve the workflow's current returned, logged, mapped, or ignored error path; syntax alone adds no unknown-commit guarantee.".to_owned()
        };
        groups.push(Group {
            id: format!("workflow:{package}:{module}:{source}::{enclosing}"),
            source_class: "production".to_owned(),
            packages: vec![package.clone()],
            sources: vec![source.clone()],
            source_prefixes: Vec::new(),
            modules: vec![module.clone()],
            enclosings: vec![enclosing.clone()],
            enclosing_prefixes: Vec::new(),
            targets: Vec::new(),
            operations: Vec::new(),
            logical_databases: annotation.logical_databases.clone(),
            capability_owner: format!(
                "{} currently owns the complete {package} workflow {enclosing} in {source}.",
                annotation.boundary
            ),
            connection_affinity: affinity,
            current_order: order,
            failure_and_unknown_commit: failure,
            target_issues: annotation.target_issues.clone(),
            retirement_condition: if annotation.stable_boundary {
                "This adapter workflow is an intended stable boundary; future callers must use its typed contract without leaking the underlying pool.".to_owned()
            } else {
                format!(
                    "The annotated issues {:?} must migrate or explicitly decide the entire workflow before this exception can disappear.",
                    annotation.target_issues
                )
            },
            stable_boundary: annotation.stable_boundary,
        });
    }

    for (package, sources) in test_sources {
        groups.push(Group {
            id: format!("test-fixtures:{package}"),
            source_class: "test_fixture".to_owned(),
            packages: vec![package.clone()],
            sources: sources.into_iter().collect(),
            source_prefixes: Vec::new(),
            modules: Vec::new(),
            enclosings: Vec::new(),
            enclosing_prefixes: Vec::new(),
            targets: Vec::new(),
            operations: Vec::new(),
            logical_databases: vec!["test-fixture".to_owned()],
            capability_owner: format!("{package} test fixtures own their exact persistence syntax."),
            connection_affinity: "Fixture-only syntax preserves the production contract it exercises; it owns no production pool.".to_owned(),
            current_order: "Fixture source order remains pinned by the exact syntax snapshot.".to_owned(),
            failure_and_unknown_commit: "Fixture assertions preserve the error/outcome behavior under test.".to_owned(),
            target_issues: Vec::new(),
            retirement_condition: "Remove the fixture group when the exact snapshot contains no test-only persistence row for this package.".to_owned(),
            stable_boundary: false,
        });
    }
    Ok(Policy {
        schema_version: POLICY_SCHEMA_VERSION,
        generated_code: GeneratedCodePolicy {
            expected_rows: baseline
                .accesses
                .iter()
                .filter(|access| access.generated_input)
                .count(),
            discovery_rule: "Exact attribute, query-macro, and item-macro inputs which can generate concrete persistence syntax; this is an orthogonal subset of production/test_fixture rows.".to_owned(),
        },
        groups,
    })
}

pub(crate) fn render_persistence_policy(
    annotation_path: &Path,
    baseline: &PersistenceAccessBaseline,
) -> Result<String, String> {
    let policy = generate_policy(&load_annotations(annotation_path)?, baseline)?;
    serde_json::to_string_pretty(&policy)
        .map_err(|error| format!("cannot serialize persistence policy: {error}"))
}

fn group_matches(
    group: &Group,
    source_class: &str,
    package: &str,
    module: &str,
    source: &str,
    enclosing: &str,
    target: &str,
    operation: &str,
) -> bool {
    group.source_class == source_class
        && group.packages.iter().any(|candidate| candidate == package)
        && (group.modules.is_empty() || group.modules.iter().any(|candidate| candidate == module))
        && (group.sources.iter().any(|candidate| candidate == source)
            || group
                .source_prefixes
                .iter()
                .any(|prefix| source.starts_with(prefix)))
        && (group.enclosings.is_empty() && group.enclosing_prefixes.is_empty()
            || group
                .enclosings
                .iter()
                .any(|candidate| candidate == enclosing)
            || group
                .enclosing_prefixes
                .iter()
                .any(|prefix| enclosing.starts_with(prefix)))
        && (group.targets.is_empty() || group.targets.iter().any(|candidate| candidate == target))
        && (group.operations.is_empty()
            || group
                .operations
                .iter()
                .any(|candidate| candidate == operation))
}

fn open_issue_states(issue_ledger_path: &Path) -> Result<BTreeMap<u64, String>, String> {
    let source = fs::read_to_string(issue_ledger_path)
        .map_err(|error| format!("cannot read {}: {error}", issue_ledger_path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid issue ledger {}: {error}",
            issue_ledger_path.display()
        )
    })?;
    let mut states = BTreeMap::new();
    let issues = value
        .get("issues")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "architecture issue ledger must contain an issues array".to_owned())?;
    for issue in issues {
        let number = issue
            .get("number")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "architecture issue entry has no numeric number".to_owned())?;
        let state = issue
            .get("state")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("architecture issue #{number} has no state"))?;
        states.insert(number, state.to_owned());
    }
    Ok(states)
}

/// Validate semantic coverage of every exact inventory row.
pub(crate) fn validate_persistence_policy(
    policy_path: &Path,
    annotation_path: &Path,
    issue_ledger_path: &Path,
    baseline: &PersistenceAccessBaseline,
) -> Result<(usize, usize, usize, usize), String> {
    let source = fs::read_to_string(policy_path)
        .map_err(|error| format!("cannot read {}: {error}", policy_path.display()))?;
    let policy: Policy = serde_json::from_str(&source).map_err(|error| {
        format!(
            "invalid persistence policy {}: {error}",
            policy_path.display()
        )
    })?;
    let generated = generate_policy(&load_annotations(annotation_path)?, baseline)?;
    if policy != generated {
        return Err(format!(
            "checked persistence policy is stale; regenerate it from {} with print-persistence-policy",
            annotation_path.display()
        ));
    }
    validate_policy(policy, issue_ledger_path, baseline)
}

fn validate_policy(
    policy: Policy,
    issue_ledger_path: &Path,
    baseline: &PersistenceAccessBaseline,
) -> Result<(usize, usize, usize, usize), String> {
    let mut errors = Vec::new();
    if policy.schema_version != POLICY_SCHEMA_VERSION {
        errors.push(format!(
            "persistence policy schema_version must be {POLICY_SCHEMA_VERSION}, got {}",
            policy.schema_version
        ));
    }
    if !non_empty(&policy.generated_code.discovery_rule) {
        errors.push("generated-code discovery rule must be non-empty".to_owned());
    }
    let issue_states = open_issue_states(issue_ledger_path)?;
    let mut ids = BTreeSet::new();
    let mut group_hits = vec![0usize; policy.groups.len()];
    let mut source_hits = policy
        .groups
        .iter()
        .map(|group| vec![0usize; group.sources.len()])
        .collect::<Vec<_>>();
    for group in &policy.groups {
        if !ids.insert(group.id.clone()) {
            errors.push(format!("duplicate persistence semantic group {}", group.id));
        }
        if !matches!(group.source_class.as_str(), "production" | "test_fixture") {
            errors.push(format!(
                "persistence group {} has invalid source_class {}",
                group.id, group.source_class
            ));
        }
        if group.packages.is_empty()
            || (group.sources.is_empty() && group.source_prefixes.is_empty())
            || group.logical_databases.is_empty()
            || !non_empty(&group.capability_owner)
            || !non_empty(&group.connection_affinity)
            || !non_empty(&group.current_order)
            || !non_empty(&group.failure_and_unknown_commit)
            || !non_empty(&group.retirement_condition)
        {
            errors.push(format!(
                "persistence group {} is missing required semantic ownership fields",
                group.id
            ));
        }
        if group.source_class == "production"
            && !group.stable_boundary
            && group.target_issues.is_empty()
        {
            errors.push(format!(
                "production persistence group {} must name an open decision/removal issue",
                group.id
            ));
        }
        if group.stable_boundary && !group.target_issues.is_empty() {
            errors.push(format!(
                "stable persistence boundary {} must not depend on an indefinitely open issue",
                group.id
            ));
        }
        for issue in &group.target_issues {
            match issue_states.get(issue).map(String::as_str) {
                Some("open") | Some("OPEN") => {}
                Some(state) => errors.push(format!(
                    "persistence group {} targets stale issue #{} ({state})",
                    group.id, issue
                )),
                None => errors.push(format!(
                    "persistence group {} targets issue #{} absent from the architecture ledger",
                    group.id, issue
                )),
            }
        }
        if group.stable_boundary && group.source_class != "production" {
            errors.push(format!(
                "only a production semantic group may declare a stable boundary: {}",
                group.id
            ));
        }
    }

    let mut production = 0;
    let mut test_fixture = 0;
    let mut generated = 0;
    for access in &baseline.accesses {
        match access.source_class.as_str() {
            "production" => production += 1,
            "test_fixture" => test_fixture += 1,
            other => errors.push(format!("unknown persistence source class {other}")),
        }
        if access.generated_input {
            generated += 1;
        }
        let target = serde_json::to_value(access.target)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .expect("persistence target serializes as a string");
        let operation = serde_json::to_value(access.operation)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .expect("persistence operation serializes as a string");
        let matches = policy
            .groups
            .iter()
            .enumerate()
            .filter(|(_, group)| {
                group_matches(
                    group,
                    &access.source_class,
                    &access.package,
                    &access.module,
                    &access.source,
                    &access.enclosing,
                    &target,
                    &operation,
                )
            })
            .map(|(index, group)| (index, group.id.as_str()))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            errors.push(format!(
                "persistence row {} {} {}::{:?}/{:?} matches {} semantic groups: {:?}",
                access.source_class,
                access.package,
                access.source,
                access.target,
                access.operation,
                matches.len(),
                matches.iter().map(|(_, id)| id).collect::<Vec<_>>()
            ));
        } else {
            group_hits[matches[0].0] += 1;
            for (source_index, source) in policy.groups[matches[0].0].sources.iter().enumerate() {
                if source == &access.source {
                    source_hits[matches[0].0][source_index] += 1;
                }
            }
        }
    }
    for (group, hits) in policy.groups.iter().zip(group_hits) {
        if hits == 0 {
            errors.push(format!(
                "obsolete persistence semantic group {} matches no exact inventory row",
                group.id
            ));
        }
    }
    for (group_index, group) in policy.groups.iter().enumerate() {
        for (source, hits) in group.sources.iter().zip(&source_hits[group_index]) {
            if *hits == 0 {
                errors.push(format!(
                    "obsolete persistence semantic source selector {} in group {} matches no exact inventory row",
                    source, group.id
                ));
            }
        }
    }
    if generated != policy.generated_code.expected_rows {
        errors.push(format!(
            "generated persistence row count changed: expected {}, actual {generated}",
            policy.generated_code.expected_rows
        ));
    }
    if errors.is_empty() {
        Ok((production, test_fixture, generated, policy.groups.len()))
    } else {
        errors.sort();
        errors.dedup();
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence_access::{
        PersistenceAccessRecord, PersistenceOperation, PersistenceTarget,
    };

    fn row(source: &str) -> PersistenceAccessRecord {
        PersistenceAccessRecord {
            classification: "direct_application_or_domain_access".to_owned(),
            source_class: "production".to_owned(),
            package: "wow-world".to_owned(),
            module: "crate::handlers::character".to_owned(),
            source: source.to_owned(),
            enclosing: "fn save".to_owned(),
            target: PersistenceTarget::CharacterDatabase,
            operation: PersistenceOperation::PoolAccess,
            symbol: "pool".to_owned(),
            visibility: String::new(),
            cfg: Vec::new(),
            fingerprint: "db.pool()".to_owned(),
            generated_input: false,
            count: 1,
        }
    }

    fn policy(groups: &str) -> String {
        format!(
            r#"{{
              "schema_version": 1,
              "generated_code": {{"expected_rows": 0, "discovery_rule": "macro-expanded persistence syntax"}},
              "groups": [{groups}]
            }}"#
        )
    }

    fn group(id: &str, source: &str, issue: u64) -> String {
        format!(
            r#"{{
              "id": "{id}", "source_class": "production", "packages": ["wow-world"],
              "sources": ["{source}"], "source_prefixes": [], "enclosings": [],
              "enclosing_prefixes": [], "targets": [], "operations": [],
              "logical_databases": ["characters"], "capability_owner": "player persistence",
              "connection_affinity": "one characters pool connection", "current_order": "source order",
              "failure_and_unknown_commit": "current DatabaseError propagation",
              "target_issues": [{issue}], "retirement_condition": "typed port owns the workflow",
              "stable_boundary": false
            }}"#
        )
    }

    fn validate(policy_source: &str, rows: Vec<PersistenceAccessRecord>) -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "rustycore-persistence-policy-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir_all(&root).unwrap();
        let policy_path = root.join("policy.json");
        let ledger_path = root.join("ledger.json");
        fs::write(&policy_path, policy_source).unwrap();
        fs::write(
            &ledger_path,
            r#"{"issues":[{"number":153,"state":"open"},{"number":999,"state":"closed"}]}"#,
        )
        .unwrap();
        let parsed: Policy = serde_json::from_str(policy_source).unwrap();
        let result = validate_policy(
            parsed,
            &ledger_path,
            &PersistenceAccessBaseline {
                schema_version: 3,
                accesses: rows,
            },
        )
        .map(|_| ());
        let _ = fs::remove_dir_all(root);
        result
    }

    fn annotation(source: &str) -> WorkflowAnnotation {
        WorkflowAnnotation {
            package: "wow-world".to_owned(),
            module: "crate::handlers::character".to_owned(),
            source: source.to_owned(),
            enclosing: "fn save".to_owned(),
            logical_databases: vec!["characters".to_owned()],
            boundary: "Player lifecycle persistence capability".to_owned(),
            target_issues: vec![153],
            stable_boundary: false,
        }
    }

    #[test]
    fn generated_policy_requires_exact_reviewed_workflow_annotations() {
        let source = "crates/wow-world/src/handlers/character.rs";
        let baseline = PersistenceAccessBaseline {
            schema_version: 3,
            accesses: vec![row(source)],
        };
        let annotations = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(source)],
        };
        let first = generate_policy(&annotations, &baseline).expect("exact annotation generates");
        let second = generate_policy(&annotations, &baseline).expect("generation is deterministic");
        assert_eq!(first, second);

        let other = "crates/wow-world/src/handlers/quest.rs";
        let two_rows = PersistenceAccessBaseline {
            schema_version: 3,
            accesses: vec![row(other), row(source)],
        };
        let forward = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(source), annotation(other)],
        };
        let reverse = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(other), annotation(source)],
        };
        assert_eq!(
            generate_policy(&forward, &two_rows).unwrap(),
            generate_policy(&reverse, &two_rows).unwrap(),
            "annotation and snapshot ordering cannot affect the generated policy"
        );

        let missing = WorkflowAnnotations {
            schema_version: 1,
            workflows: Vec::new(),
        };
        assert!(
            generate_policy(&missing, &baseline)
                .expect_err("new workflows fail closed")
                .contains("has no annotation")
        );

        let stale_source = "crates/wow-world/src/removed.rs";
        let stale = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(stale_source)],
        };
        let error = generate_policy(&stale, &baseline).expect_err("dead annotations fail");
        assert!(error.contains("obsolete persistence workflow annotation"));
        assert!(error.contains("has no annotation"));

        let duplicate = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(source), annotation(source)],
        };
        assert!(
            generate_policy(&duplicate, &baseline)
                .expect_err("duplicate annotations fail")
                .contains("duplicate persistence workflow annotation")
        );

        let mut login_row = row(source);
        login_row.target = PersistenceTarget::LoginDatabase;
        let mixed = PersistenceAccessBaseline {
            schema_version: 3,
            accesses: vec![row(source), login_row],
        };
        assert!(
            generate_policy(&annotations, &mixed)
                .expect_err("a newly typed logical database cannot be auto-authorized")
                .contains("omits exact typed logical database login")
        );
    }

    #[test]
    fn generated_policy_distinguishes_same_named_workflows_across_modules() {
        let source = "crates/wow-world/src/handlers/character.rs";
        // One physical file with same-named persistence functions in two
        // inline modules: the rows stay distinct by module, so the
        // annotations must too, or owners/ordering/retirement would merge.
        let mut inner_row = row(source);
        inner_row.module = "crate::handlers::character::inner".to_owned();
        let baseline = PersistenceAccessBaseline {
            schema_version: 3,
            accesses: vec![row(source), inner_row],
        };
        let mut inner_annotation = annotation(source);
        inner_annotation.module = "crate::handlers::character::inner".to_owned();
        inner_annotation.boundary = "Inner module persistence capability".to_owned();
        let both = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(source), inner_annotation.clone()],
        };
        let policy = generate_policy(&both, &baseline).expect("both modules annotated");
        assert!(policy
            .groups
            .iter()
            .any(|group| group.id
                == "workflow:wow-world:crate::handlers::character:crates/wow-world/src/handlers/character.rs::fn save"));
        assert!(policy.groups.iter().any(|group| group.id
            == "workflow:wow-world:crate::handlers::character::inner:crates/wow-world/src/handlers/character.rs::fn save"));
        validate(
            &serde_json::to_string(&policy).unwrap(),
            baseline.accesses.clone(),
        )
        .expect("module selectors make the generated groups disjoint");

        // An annotation missing the second module's identity fails closed.
        let outer_only = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(source)],
        };
        assert!(
            generate_policy(&outer_only, &baseline)
                .expect_err("the inner-module workflow needs its own annotation")
                .contains("has no annotation")
        );

        // And a stale module annotation does not merge into the outer one.
        let mut stale_module = annotation(source);
        stale_module.module = "crate::handlers::character::gone".to_owned();
        let stale = WorkflowAnnotations {
            schema_version: 1,
            workflows: vec![annotation(source), stale_module],
        };
        assert!(
            generate_policy(&stale, &baseline)
                .expect_err("dead module annotations fail")
                .contains("obsolete persistence workflow annotation")
        );
    }

    #[test]
    fn generated_policy_separates_explicit_transactions_from_unknown_commit_outcomes() {
        let source = "crates/wow-world/src/handlers/character.rs";

        let mut outcome_commit = row(source);
        outcome_commit.operation = PersistenceOperation::Commit;
        outcome_commit.symbol = "commit_with_outcome_like_cpp".to_owned();
        outcome_commit.fingerprint = "tx.commit_with_outcome_like_cpp()".to_owned();
        let generated = generate_policy(
            &WorkflowAnnotations {
                schema_version: 1,
                workflows: vec![annotation(source)],
            },
            &PersistenceAccessBaseline {
                schema_version: 3,
                accesses: vec![outcome_commit],
            },
        )
        .expect("an outcome-aware commit generates a semantic policy");
        assert!(
            generated.groups[0]
                .failure_and_unknown_commit
                .contains("unknown commit outcome")
        );

        let mut error_type = row(source);
        error_type.target = PersistenceTarget::SqlTransactionCommitError;
        error_type.operation = PersistenceOperation::TypeReference;
        error_type.symbol = "SqlTransactionCommitError".to_owned();
        error_type.fingerprint = "SqlTransactionCommitError".to_owned();
        let generated = generate_policy(
            &WorkflowAnnotations {
                schema_version: 1,
                workflows: vec![annotation(source)],
            },
            &PersistenceAccessBaseline {
                schema_version: 3,
                accesses: vec![error_type],
            },
        )
        .expect("an outcome error type generates a semantic policy");
        let group = &generated.groups[0];
        assert!(group.connection_affinity.contains("no wider transaction"));
        assert!(group.current_order.contains("source and await order"));
        assert!(
            group
                .failure_and_unknown_commit
                .contains("unknown commit outcome")
        );

        let mut unrelated_unknown = row(source);
        unrelated_unknown.target = PersistenceTarget::MySqlPool;
        unrelated_unknown.operation = PersistenceOperation::ArgumentEscape;
        unrelated_unknown.symbol = "is_unknown_database_error_like_cpp".to_owned();
        let generated = generate_policy(
            &WorkflowAnnotations {
                schema_version: 1,
                workflows: vec![annotation(source)],
            },
            &PersistenceAccessBaseline {
                schema_version: 3,
                accesses: vec![unrelated_unknown],
            },
        )
        .expect("unrelated unknown vocabulary generates an ordinary policy");
        assert!(
            generated.groups[0]
                .failure_and_unknown_commit
                .contains("syntax alone adds no unknown-commit guarantee")
        );
    }

    #[test]
    fn semantic_policy_rejects_unowned_overlapping_stale_and_obsolete_entries() {
        let source = "crates/wow-world/src/handlers/character.rs";
        validate(&policy(&group("player", source, 153)), vec![row(source)])
            .expect("one exact open-owned group is valid");

        let error = validate(
            &policy(&group("player", source, 153)),
            vec![row("crates/wow-world/src/new_leak.rs")],
        )
        .expect_err("new production access without a semantic owner fails");
        assert!(error.contains("matches 0 semantic groups"), "{error}");
        assert!(
            error.contains("obsolete persistence semantic group"),
            "{error}"
        );

        let overlap = format!(
            "{},{}",
            group("player-a", source, 153),
            group("player-b", source, 153)
        );
        let error = validate(&policy(&overlap), vec![row(source)])
            .expect_err("overlapping semantic groups fail");
        assert!(error.contains("matches 2 semantic groups"), "{error}");

        let error = validate(&policy(&group("stale", source, 999)), vec![row(source)])
            .expect_err("closed decision/removal issue fails");
        assert!(error.contains("stale issue #999"), "{error}");
    }
}
