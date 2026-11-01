//! Adapter for the canonical map's deferred player visibility obligation.
//!
//! Readiness only marks the Player. The existing map relocation phase selects
//! the recipient; its Session publishes against the real client GUID ledger.

#[cfg(test)]
#[path = "../../unit_tests/session/deferred_visibility/tests.rs"]
mod tests;
