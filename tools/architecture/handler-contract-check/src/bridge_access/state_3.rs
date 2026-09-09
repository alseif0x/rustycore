//! Bridge access scan state definitions, part 3 of 3.
//!
//! Separated from the bridge_access.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) fn validate_record(label: &str, record: &BridgeAccessRecord) -> Result<(), String> {
    if record.multiplicity == 0 {
        return Err(format!(
            "{label} bridge baseline contains a zero-multiplicity row at {}::{}",
            record.module, record.enclosing
        ));
    }
    if record.package.is_empty()
        || record.module.is_empty()
        || record.path.is_empty()
        || record.enclosing.is_empty()
        || record.fingerprint.is_empty()
    {
        return Err(format!(
            "{label} bridge baseline contains an incomplete row at {}::{}",
            record.module, record.enclosing
        ));
    }
    if record.direction_markers.is_empty()
        || record
            .direction_markers
            .windows(2)
            .any(|window| window[0] >= window[1])
    {
        return Err(format!(
            "{label} bridge direction markers are empty or noncanonical at {}::{}",
            record.module, record.enclosing
        ));
    }
    if record.evidence.is_empty() {
        return Err(format!(
            "{label} bridge row has no AST evidence at {}::{}",
            record.module, record.enclosing
        ));
    }
    let mut previous: Option<EvidenceIdentity> = None;
    for evidence in &record.evidence {
        if evidence.multiplicity == 0 {
            return Err(format!(
                "{label} bridge evidence has zero multiplicity at {}::{}",
                record.module, record.enclosing
            ));
        }
        let identity = EvidenceIdentity {
            side: evidence.side,
            kind: evidence.kind,
            symbol: evidence.symbol.clone(),
            fingerprint: evidence.fingerprint.clone(),
        };
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} bridge evidence is not in strict canonical order at {}::{}",
                record.module, record.enclosing
            ));
        }
        previous = Some(identity);
    }
    let sides: BTreeSet<_> = record
        .evidence
        .iter()
        .map(|evidence| evidence.side)
        .collect();
    if record
        .direction_markers
        .contains(&BridgeDirection::UnresolvedDualSide)
        && !(sides.contains(&BridgeSide::Canonical) && sides.contains(&BridgeSide::Legacy))
    {
        return Err(format!(
            "{label} unresolved bridge lacks evidence from both sides at {}::{}",
            record.module, record.enclosing
        ));
    }
    Ok(())
}

pub(super) fn validated_baseline_map(
    label: &str,
    baseline: &BridgeAccessBaseline,
) -> Result<BTreeMap<BridgeIdentity, usize>, String> {
    if baseline.schema_version != BRIDGE_SCHEMA_VERSION {
        return Err(format!(
            "{label} bridge baseline schema version is {}, expected {BRIDGE_SCHEMA_VERSION}",
            baseline.schema_version
        ));
    }
    let mut map = BTreeMap::new();
    let mut previous: Option<BridgeIdentity> = None;
    for record in &baseline.bridges {
        validate_record(label, record)?;
        let identity = record.identity();
        if previous
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
        {
            return Err(format!(
                "{label} bridge rows are not in strict canonical order near {}::{}",
                record.module, record.enclosing
            ));
        }
        previous = Some(identity.clone());
        if map.insert(identity, record.multiplicity).is_some() {
            return Err(format!(
                "{label} bridge baseline contains a duplicate row at {}::{}",
                record.module, record.enclosing
            ));
        }
    }
    Ok(map)
}

pub(super) fn describe_bridge(identity: &BridgeIdentity) -> String {
    format!(
        "{} {} {}::{} directions={:?} cfg=[{}] fingerprint={}",
        identity.package,
        identity.path,
        identity.module,
        identity.enclosing,
        identity.direction_markers,
        identity.cfg.join(", "),
        identity.fingerprint
    )
}

/// Exact comparison of additions, removals, syntax/direction swaps, cfg drift,
/// and multiplicity. Debt reduction must delete the obsolete baseline row in
/// the same reviewed change, so it cannot be silently reintroduced later.
pub(crate) fn compare_bridge_access_baseline(
    expected: &BridgeAccessBaseline,
    actual: &BridgeAccessBaseline,
) -> Result<(), String> {
    let expected = validated_baseline_map("expected", expected)?;
    let actual = validated_baseline_map("actual", actual)?;
    let mut errors = Vec::new();
    for (identity, actual_count) in &actual {
        match expected.get(identity) {
            None => errors.push(format!(
                "untracked legacy/canonical bridge: {} (multiplicity {actual_count})",
                describe_bridge(identity)
            )),
            Some(expected_count) if expected_count != actual_count => errors.push(format!(
                "legacy/canonical bridge multiplicity changed: {} expected {expected_count}, actual {actual_count}",
                describe_bridge(identity)
            )),
            Some(_) => {}
        }
    }
    for (identity, expected_count) in &expected {
        if !actual.contains_key(identity) {
            errors.push(format!(
                "obsolete legacy/canonical bridge baseline row: {} (expected multiplicity {expected_count})",
                describe_bridge(identity)
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Check that every direction-bearing anchor copied from the runtime ledger is
/// present exactly once as a definition in its expected package/module.
pub(crate) fn validate_curated_bridge_anchors(
    baseline: &BridgeAccessBaseline,
) -> Result<(), String> {
    validated_baseline_map("curated-anchor", baseline)?;
    let mut errors = Vec::new();
    for anchor in CURATED_ANCHORS {
        let count: usize = baseline
            .bridges
            .iter()
            .filter(|record| {
                record.package == anchor.package
                    && record.module == anchor.module
                    && record.evidence.iter().any(|evidence| {
                        evidence.kind == BridgeEvidenceKind::CuratedAnchorDefinition
                            && evidence.symbol == anchor.name
                    })
            })
            .map(|record| record.multiplicity)
            .sum();
        if count != 1 {
            errors.push(format!(
                "curated bridge anchor {} {}::{} expected exactly one definition, found {count}",
                anchor.package, anchor.module, anchor.name
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
