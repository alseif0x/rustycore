//! #1263 F6-8E: the recorded expectations of the F6-8D3a regressions.
//!
//! The F6-8D3a-1/1b/2/3 tests used to drive one fixture twice, once through
//! the legacy bridge and once through the admitted canonical executor, and
//! compare the two. F6-8E deleted the legacy bridge. Its observables were
//! recorded on the last base that still had it (`origin/3.4.3` at
//! `6c1a7de2f`, where every non-perturbed comparison was equal) into
//! `f6_8d3a_recorded_expectations.tsv`: one line per comparison the parity
//! check made, `<test>\t<label>\t<legacy-bridge value>`, in the order the test
//! made them. The fields F6-8E deleted from the outcome types (the
//! legacy-owner skip flag, mirror/incarnation rejection and sync counters,
//! legacy victim syncs, the failed respawn publication count) were removed
//! from the recorded values, and trailing whitespace is trimmed on both sides;
//! nothing else was edited.
//!
//! How the file was produced (review follow-up of PR #1355): on `6c1a7de2f`
//! the shared parity comparison of these four modules was temporarily
//! instrumented to print one `GOLDEN\t<test path>\t<label>\t<equal>\t<legacy
//! bridge value>\t<canonical value>` line per comparison, and
//! `cargo test -p wow-world --lib f6_8d3a -- --nocapture --test-threads=1`
//! was captured. Each `GOLDEN` line was reduced to `<test fn>\t<label>\t<legacy
//! bridge value>` (module path dropped), with the edits listed above. The
//! instrumentation itself was never committed: the bridge it observed no
//! longer exists, so the recording cannot be regenerated on a later base. It
//! is reproducible only by re-applying that print on `6c1a7de2f`. #1263
//! F6-8E-2 renamed the canonical runtime's `Legacy*` outcome types, and the
//! recorded `Debug` values carry those type names, so the same rename was
//! applied to the values (no other change).
//!
//! A test now drives the executor alone and checks each observable against
//! the next recorded value with the same test and label. The perturbed
//! negative controls still drive an unperturbed recording, so their expected
//! differences are what the perturbation changed.

use std::collections::HashMap;

const RECORDED_LIKE_CPP: &str = include_str!("f6_8d3a_recorded_expectations.tsv");

/// The recorded values of one test, consumed in recording order per label.
pub(super) struct RecordedExpectationsLikeCpp {
    test: &'static str,
    values: HashMap<&'static str, Vec<&'static str>>,
    cursors: HashMap<String, usize>,
}

impl RecordedExpectationsLikeCpp {
    pub(super) fn for_test_like_cpp(test: &'static str) -> Self {
        let mut values: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        for line in RECORDED_LIKE_CPP.lines() {
            let mut fields = line.splitn(3, '\t');
            let (Some(name), Some(label), Some(value)) =
                (fields.next(), fields.next(), fields.next())
            else {
                panic!("malformed recorded expectation line: {line}");
            };
            if name == test {
                values.entry(label).or_default().push(value);
            }
        }
        assert!(!values.is_empty(), "no recorded expectations for {test}");
        Self {
            test,
            values,
            cursors: HashMap::new(),
        }
    }

    /// The next recorded value of `label`.
    pub(super) fn next_like_cpp(&mut self, label: &str) -> &'static str {
        let cursor = self.cursors.entry(label.to_string()).or_default();
        let value = self
            .values
            .get(label)
            .and_then(|values| values.get(*cursor))
            .unwrap_or_else(|| {
                panic!(
                    "{}: no recorded value #{} for `{label}`",
                    self.test, *cursor
                )
            });
        *cursor += 1;
        value
    }

    /// Start one comparison pass.
    pub(super) fn pass_like_cpp(&mut self) -> RecordedPassLikeCpp<'_> {
        RecordedPassLikeCpp {
            expectations: self,
            mismatches: Vec::new(),
        }
    }
}

/// One comparison pass: every observable checked against its next recorded
/// value. `finish_like_cpp` names every observable that differs.
pub(super) struct RecordedPassLikeCpp<'a> {
    expectations: &'a mut RecordedExpectationsLikeCpp,
    mismatches: Vec<String>,
}

impl RecordedPassLikeCpp<'_> {
    pub(super) fn check_like_cpp(&mut self, label: &str, actual: String) {
        let recorded = self.expectations.next_like_cpp(label);
        // The recording is stored without trailing whitespace (a counter
        // rendering cut before its `plan:` field ends in a space).
        let actual = actual.trim_end();
        if recorded != actual {
            self.mismatches.push(format!(
                "{label}:\n  recorded: {recorded}\n  executor: {actual}"
            ));
        }
    }

    pub(super) fn finish_like_cpp(self) -> Result<(), Vec<String>> {
        if self.mismatches.is_empty() {
            Ok(())
        } else {
            Err(self.mismatches)
        }
    }
}

/// Panic with every mismatch of a pass.
pub(super) fn assert_recorded_like_cpp(context: &str, result: Result<(), Vec<String>>) {
    if let Err(mismatches) = result {
        panic!(
            "{context}: the executor departs from the recorded expectations:\n{}",
            mismatches.join("\n")
        );
    }
}
