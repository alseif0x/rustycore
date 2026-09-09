//! Session ownership inventory regression scenarios.
//!
//! Separated from the session_ownership.rs root under #660.

use super::*;

// The checker regenerates the semantic policy from the annotations and the
// freshly scanned inventory and demands exact equality. Both checked-in
// files must therefore already agree with each other: a snapshot updated
// without its policy fails CI only after a full scan, which is the one
// feedback loop this tool cannot afford to leave to CI.
// A missing or non-JSON snapshot must name the offending path instead of
// rendering a policy from nothing: this command exists to install a CI
// artifact, so the plausible mistake is pointing it at the wrong download.
mod scenarios_1;
