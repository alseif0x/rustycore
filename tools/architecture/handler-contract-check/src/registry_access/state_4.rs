//! Registry access scan state definitions, part 4 of 4.
//!
//! Separated from the registry_access.rs root under #660. Behaviour is preserved.

use super::*;

/// Parse and inventory an already-classified set of production source mounts.
/// Input order is irrelevant. Duplicate mounts intentionally increase record
/// multiplicity, so callers should pass each `(package, logical module, file)`
/// context once.
pub(crate) fn inventory_registry_accesses(
    sources: &[ProductionRegistrySource<'_>],
) -> Result<RegistryAccessBaseline, String> {
    let mut ordered: Vec<_> = sources.iter().copied().collect();
    ordered.sort_by(|left, right| {
        (left.package, left.module, left.source_path).cmp(&(
            right.package,
            right.module,
            right.source_path,
        ))
    });
    let mut seen_mounts = BTreeSet::new();
    let mut errors = Vec::new();
    let mut parsed_sources = Vec::new();
    for source in ordered {
        if source.package.is_empty() || source.module.is_empty() || source.source_path.is_empty() {
            errors.push("registry source package/module/path must be non-empty".to_owned());
            continue;
        }
        if !seen_mounts.insert((source.package, source.module, source.source_path)) {
            errors.push(format!(
                "duplicate production registry source mount {} {} {}",
                source.package, source.module, source.source_path
            ));
            continue;
        }
        let syntax = match syn::parse_file(source.source) {
            Ok(syntax) => syntax,
            Err(error) => {
                errors.push(format!(
                    "cannot parse registry source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        };
        let cfg = extend_cfg_context(source.inherited_cfg, &syntax.attrs);
        match cfg_context_allows_production(source.inherited_cfg, &syntax.attrs) {
            Ok(true) => {}
            Ok(false) => {
                errors.push(format!(
                    "source {} was passed as production but its file attributes are test-only",
                    source.source_path
                ));
                continue;
            }
            Err(error) => {
                errors.push(format!(
                    "invalid file cfg in registry source {}: {error}",
                    source.source_path
                ));
                continue;
            }
        }
        parsed_sources.push(ParsedRegistrySource {
            mount: source,
            syntax,
            cfg,
        });
    }

    let global_aliases = build_global_alias_index(&parsed_sources, &mut errors);
    let mut accumulator = AccessAccumulator::default();
    for source in &parsed_sources {
        analyze_module_items(
            &source.syntax.items,
            RecordContext {
                package: source.mount.package,
                module: source.mount.module,
                source: source.mount.source_path,
            },
            None,
            &global_aliases,
            source.cfg.clone(),
            &mut accumulator,
            &mut errors,
        );
    }
    if errors.is_empty() {
        Ok(accumulator.finish())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors.join("\n"))
    }
}

pub(super) fn validated_baseline_map(
    label: &str,
    baseline: &RegistryAccessBaseline,
) -> Result<BTreeMap<AccessIdentity, usize>, String> {
    if baseline.schema_version != REGISTRY_SCHEMA_VERSION {
        return Err(format!(
            "{label} registry baseline schema version is {}, expected {REGISTRY_SCHEMA_VERSION}",
            baseline.schema_version
        ));
    }
    let mut map = BTreeMap::new();
    let mut previous: Option<AccessIdentity> = None;
    for record in &baseline.accesses {
        if record.count == 0 {
            return Err(format!(
                "{label} registry baseline contains zero-count row for {:?} {}",
                record.registry, record.symbol
            ));
        }
        let identity = record.identity();
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} registry baseline rows are not in strict canonical order near {:?} {}",
                record.registry, record.symbol
            ));
        }
        previous = Some(identity.clone());
        if map.insert(identity, record.count).is_some() {
            return Err(format!(
                "{label} registry baseline contains a duplicate row for {:?} {}",
                record.registry, record.symbol
            ));
        }
    }
    Ok(map)
}

pub(super) fn describe_identity(identity: &AccessIdentity) -> String {
    format!(
        "{} {} {}::{} {} {:?} {:?} {} [{}]",
        identity.package,
        identity.source,
        identity.module,
        identity.enclosing,
        identity.symbol,
        identity.registry,
        identity.operation,
        identity.fingerprint,
        identity.cfg.join(", ")
    )
}

/// Exact comparison: additions, removals, same-count swaps, and multiplicity
/// changes all fail. A debt reduction therefore requires deleting its stale
/// baseline row in the same reviewed change, preventing later reintroduction.
pub(crate) fn compare_registry_access_baseline(
    expected: &RegistryAccessBaseline,
    actual: &RegistryAccessBaseline,
) -> Result<(), String> {
    let expected = validated_baseline_map("expected", expected)?;
    let actual = validated_baseline_map("actual", actual)?;
    let mut errors = Vec::new();
    for (identity, actual_count) in &actual {
        match expected.get(identity) {
            None => errors.push(format!(
                "untracked direct registry access: {} (count {actual_count})",
                describe_identity(identity)
            )),
            Some(expected_count) if expected_count != actual_count => errors.push(format!(
                "direct registry access multiplicity changed: {} expected {expected_count}, actual {actual_count}",
                describe_identity(identity)
            )),
            Some(_) => {}
        }
    }
    for (identity, expected_count) in &expected {
        if !actual.contains_key(identity) {
            errors.push(format!(
                "obsolete direct registry baseline row: {} (expected count {expected_count})",
                describe_identity(identity)
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
