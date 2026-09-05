//! Finite contract checks. A failed case stays a failed structured result.
//! These fixtures do not establish production scheduling or real DB durability.

mod common;
mod native;
mod oracle;

use conformance_contract::{Fault, Result};
use serde::Serialize;
use serde_json::{Value, json};

use crate::composition::Mode;

pub const COMMON_CASES: &[&str] = &[
    "registration_order",
    "zero_optional_neutrality",
    "policy_and_state_isolation",
    "summon_reentry_order",
    "mixed_reverse_reentry",
    "callback_result_oracle",
    "nullable_summon_partial_effects",
    "stale_outer_write",
    "fallible_action_partial_effects",
    "active_detached_transfer",
    "failed_attach_retains_state",
    "stale_and_forged_handles",
    "reset_scoped_contribution",
    "unload_preserves_other_module",
    "versioned_snapshot_replay",
    "stale_snapshot_rejected",
    "unsupported_executor_switch",
    "cumulative_host_calls",
    "bounded_reentry_depth",
    "output_limit_partial_effects",
];

pub const NATIVE_ONLY_CASES: &[&str] = &[
    "invalid_registration_identity",
    "invalid_registration_versions",
    "invalid_registration_capabilities",
    "registration_conflicts",
    "module_count_limit",
    "registration_state_limit",
    "malformed_state_write",
    "oversized_state_write",
    "shared_state_type_isolation",
];

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: Value,
}

pub type CaseResult<T = Value> = std::result::Result<T, String>;

fn checked(name: &'static str, case: impl FnOnce() -> CaseResult) -> Check {
    match case() {
        Ok(detail) => Check {
            name,
            passed: true,
            detail,
        },
        Err(error) => Check {
            name,
            passed: false,
            detail: json!({ "error": error }),
        },
    }
}

fn ok<T>(result: Result<T>) -> CaseResult<T> {
    result.map_err(|fault| format!("unexpected contract fault: {fault:?}"))
}

fn require(condition: bool, message: &str) -> CaseResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn fault<T: std::fmt::Debug>(result: Result<T>, expected: Fault) -> CaseResult<()> {
    match result {
        Err(actual) if actual == expected => Ok(()),
        other => Err(format!("expected {expected:?}, observed {other:?}")),
    }
}

pub fn run(mode: Mode) -> Vec<Check> {
    let mut checks = common::run(mode);
    if mode == Mode::Native {
        checks.extend(native::run());
    }
    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn failed_oracle_is_not_reported_as_a_passing_check() {
        let result = checked("deliberate_failure", || Err("oracle disagrees".to_owned()));
        assert!(!result.passed);
        assert_eq!(result.detail["error"], "oracle disagrees");
    }

    #[test]
    fn required_case_names_are_unique() {
        let names: BTreeSet<_> = COMMON_CASES.iter().chain(NATIVE_ONLY_CASES).collect();
        assert_eq!(names.len(), COMMON_CASES.len() + NATIVE_ONLY_CASES.len());
    }

    #[test]
    fn native_matrix_covers_every_required_case_and_passes() {
        let checks = run(Mode::Native);
        let expected: BTreeSet<_> = COMMON_CASES
            .iter()
            .chain(NATIVE_ONLY_CASES)
            .copied()
            .collect();
        let actual: BTreeSet<_> = checks.iter().map(|check| check.name).collect();
        assert_eq!(actual, expected);
        assert_eq!(checks.len(), expected.len(), "duplicate check execution");
        let failures: Vec<_> = checks.into_iter().filter(|check| !check.passed).collect();
        assert!(
            failures.is_empty(),
            "failed native contract checks: {failures:#?}"
        );
    }
}
