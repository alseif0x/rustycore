//! Canonical Player ownership for native death-operation transport.

use std::num::NonZeroU64;

use super::Player;
use wow_core::ObjectGuid;

/// Immutable inputs captured for the native `Unit::Kill` operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDeathOperationContext {
    attacker_guid: ObjectGuid,
    durability_loss: bool,
}

impl PlayerDeathOperationContext {
    pub const fn new(attacker_guid: ObjectGuid, durability_loss: bool) -> Self {
        Self {
            attacker_guid,
            durability_loss,
        }
    }

    pub const fn attacker_guid(self) -> ObjectGuid {
        self.attacker_guid
    }

    pub const fn durability_loss(self) -> bool {
        self.durability_loss
    }
}

/// Opaque identity scoped to one Player incarnation.
///
/// A value alone is not globally unique. Any transport beyond this Player must bind it to the
/// stable Player handle and GUID that own the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerDeathOperationId(NonZeroU64);

/// Read-only phase fact for a native death operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDeathOperationPhase {
    Ready,
    InFlight,
    Complete,
}

/// Read-only snapshot of the current native death operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDeathOperationSnapshot {
    id: PlayerDeathOperationId,
    context: PlayerDeathOperationContext,
    phase: PlayerDeathOperationPhase,
}

impl PlayerDeathOperationSnapshot {
    pub const fn id(self) -> PlayerDeathOperationId {
        self.id
    }

    pub const fn context(self) -> PlayerDeathOperationContext {
        self.context
    }

    pub const fn phase(self) -> PlayerDeathOperationPhase {
        self.phase
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlayerDeathOperationState {
    next_id: Option<NonZeroU64>,
    current: Option<PlayerDeathOperationSnapshot>,
}

impl Default for PlayerDeathOperationState {
    fn default() -> Self {
        Self {
            next_id: NonZeroU64::new(1),
            current: None,
        }
    }
}

impl Player {
    /// Admit a death operation only for a living Player with nonzero native-width health.
    ///
    /// Source `Unit::Kill` is synchronous (`Unit.cpp:10457-10652`). Its victim health check,
    /// Player alive check, and JUST_DIED health mutation are in `Unit.cpp:10457-10652`,
    /// `Player.cpp:1158-1200`, and `Unit.cpp:8527-8585`. Ready/InFlight fence the future
    /// async Rust transport; they are not C++ phases and perform none of those mutations.
    pub fn begin_native_death_operation(
        &mut self,
        context: PlayerDeathOperationContext,
    ) -> Option<PlayerDeathOperationId> {
        if self.death_operation.current.is_some()
            || !self.unit.is_alive()
            || self.unit.data().health == 0
        {
            return None;
        }

        let state = &mut self.death_operation;
        let id = PlayerDeathOperationId(state.next_id?);
        state.next_id = id
            .0
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new);
        state.current = Some(PlayerDeathOperationSnapshot {
            id,
            context,
            phase: PlayerDeathOperationPhase::Ready,
        });
        Some(id)
    }

    /// Claim the matching ready operation once, retaining its context in Player-owned state.
    ///
    /// If the Player is no longer alive or has zero health, the ready operation remains owned
    /// here so its caller can resolve the admitted work without silently dropping it.
    pub fn claim_native_death_operation(
        &mut self,
        id: PlayerDeathOperationId,
    ) -> Option<PlayerDeathOperationContext> {
        if !self.unit.is_alive() || self.unit.data().health == 0 {
            return None;
        }

        let operation = self.death_operation.current.as_mut()?;
        if operation.id != id || operation.phase != PlayerDeathOperationPhase::Ready {
            return None;
        }

        operation.phase = PlayerDeathOperationPhase::InFlight;
        Some(operation.context)
    }

    /// Mark the matching claimed operation complete.
    ///
    /// Completion does not require a dead Player: a native callback may legitimately resurrect
    /// the Player before the async transport returns.
    pub fn complete_native_death_operation(&mut self, id: PlayerDeathOperationId) -> bool {
        let Some(operation) = self.death_operation.current.as_mut() else {
            return false;
        };
        if operation.id != id || operation.phase != PlayerDeathOperationPhase::InFlight {
            return false;
        }

        operation.phase = PlayerDeathOperationPhase::Complete;
        true
    }

    /// Retire only the matching completed operation.
    pub fn retire_completed_native_death_operation(
        &mut self,
        id: PlayerDeathOperationId,
    ) -> bool {
        if !self
            .death_operation
            .current
            .is_some_and(|operation| {
                operation.id == id && operation.phase == PlayerDeathOperationPhase::Complete
            })
        {
            return false;
        }

        self.death_operation.current = None;
        true
    }

    /// Return the current immutable operation facts, if any.
    pub fn native_death_operation_snapshot(&self) -> Option<PlayerDeathOperationSnapshot> {
        self.death_operation.current
    }

    /// Whether a ready or in-flight native death operation remains unresolved.
    pub fn has_unresolved_native_death_operation(&self) -> bool {
        self.death_operation.current.is_some_and(|operation| {
            matches!(
                operation.phase,
                PlayerDeathOperationPhase::Ready | PlayerDeathOperationPhase::InFlight
            )
        })
    }
}

#[cfg(test)]
#[path = "../player_tests/death_operation.rs"]
mod tests;
