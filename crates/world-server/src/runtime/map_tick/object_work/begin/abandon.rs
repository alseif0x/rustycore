//! Explicit abandonment is available only before the APP respawn prefix ran.
use super::*;

impl ObjectWorkBeginFailure {
    /// Map Idle alone does not account for an Objects-phase admission or effects.
    pub(crate) fn try_abandon_before_prefix(
        self,
        manager: &mut wow_map::MapManager,
    ) -> Result<(), Self> {
        match self {
            Self::BeforePrefix { plan } => match manager.try_abandon_tick(plan) {
                Ok(()) => Ok(()),
                Err((_status, plan)) => Err(Self::BeforePrefix { plan }),
            },
            failure @ Self::AfterPrefix { .. } => Err(failure),
        }
    }
}
