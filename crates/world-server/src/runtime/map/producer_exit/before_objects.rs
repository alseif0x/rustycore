//! Causes at the producer's existing terminal boundaries before Objects.
pub(crate) enum BeforeObjectsExitCause {
    UnresolvedMapPassRejected(wow_map::MapTickResumeLikeCpp),
    ShutdownAbandonRejected(wow_map::MapTickResumeLikeCpp),
    UnresolvedMapPassManagerPoisoned,
    ShutdownAbandonManagerPoisoned,
}
