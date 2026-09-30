//! Public movement failures do not expose the raw actor callback or witness.

use super::super::ObjectMapTickError;
use super::*;
use crate::MapKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorMovementError {
    Tick(ObjectMapTickError),
    StaleParticipant {
        key: MapKey,
        admitted_incarnation: u64,
        current_incarnation: Option<u64>,
    },
    OutsideSelection {
        guid: ObjectGuid,
    },
    ActorUnavailable {
        guid: ObjectGuid,
    },
    WitnessMismatch {
        guid: ObjectGuid,
    },
    NoActorOperation,
    OperationMismatch {
        guid: ObjectGuid,
    },
}

impl From<ActorTickAccessError> for ActorMovementError {
    fn from(error: ActorTickAccessError) -> Self {
        match error {
            ActorTickAccessError::Tick(error) => Self::Tick(error),
            ActorTickAccessError::StaleParticipant {
                key,
                admitted_incarnation,
                current_incarnation,
            } => Self::StaleParticipant {
                key,
                admitted_incarnation,
                current_incarnation,
            },
            ActorTickAccessError::OutsideSelection { guid } => Self::OutsideSelection { guid },
            ActorTickAccessError::ActorUnavailable { guid } => Self::ActorUnavailable { guid },
            ActorTickAccessError::WitnessMismatch { guid } => Self::WitnessMismatch { guid },
            ActorTickAccessError::NoActorOperation => Self::NoActorOperation,
            ActorTickAccessError::OperationMismatch { guid } => Self::OperationMismatch { guid },
        }
    }
}
