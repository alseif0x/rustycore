/// Transaction classification is distinct from local acknowledgement or client readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSaveOutcomeLikeCpp {
    /// Transaction confirmed; local acknowledgement may lose an incarnation race.
    Applied,
    /// Transfer retained native save intent; no full-save transaction submitted.
    Deferred,
    /// No coherent projection/owner or lifecycle port; not a classified rollback.
    Unavailable,
    /// Submitted full-save transaction returned a known failure.
    Failed,
    /// Existing durable-work reconciliation/fence forbids save or replay.
    /// Includes uncertain COMMIT; does not assert that this call submitted one.
    Quarantined,
}
