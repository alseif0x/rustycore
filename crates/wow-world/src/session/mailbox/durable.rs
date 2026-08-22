// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable Session mailbox rails.
//!
//! The creature-runtime rail keeps authoritative combat transitions in
//! committed FIFO order under bounded memory, and the loot-money rail fences
//! durable persistence against admission, logout and unknown-commit outcomes.
//! Both survive general-queue backpressure, so they never silently drop work.

use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::protocol::{
    ApplyCreatureMeleeDamageLikeCppCommand, CreatureAttackStartLikeCppCommand,
    CreatureAttackStopLikeCppCommand, ReconcilePvpCombatExpiryLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand, SessionCommand,
};

/// Durable FIFO handoff for map-owned creature transitions that have
/// already committed authoritative state.
///
/// The bounded general-purpose session queue may legitimately reject visual
/// fanout under backpressure. These commands cannot be dropped, but the global
/// map tick also cannot wait for a stalled session. C++ publishes every melee
/// swing and attack transition in order, so this rail retains each committed
/// event until the owning session drains it. A session that cannot drain a
/// bounded backlog is marked desynchronized and disconnected instead of
/// silently losing authoritative events or growing memory without limit.
pub const MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP: usize = 4_096;

#[derive(Default)]
pub struct DurableCreatureRuntimeCommandsLikeCpp {
    commands: VecDeque<SessionCommand>,
    overflowed: bool,
}

impl DurableCreatureRuntimeCommandsLikeCpp {
    fn publish_like_cpp(&mut self, command: SessionCommand) -> bool {
        if self.commands.len() >= MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            self.overflowed = true;
            return false;
        }
        self.commands.push_back(command);
        true
    }

    pub fn publish_attack_start_like_cpp(
        &mut self,
        command: CreatureAttackStartLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::CreatureAttackStartLikeCpp(command))
    }

    pub fn publish_attack_stop_like_cpp(
        &mut self,
        command: CreatureAttackStopLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::CreatureAttackStopLikeCpp(command))
    }

    pub fn publish_pvp_combat_expiry_like_cpp(
        &mut self,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ReconcilePvpCombatExpiryLikeCpp(command))
    }

    pub fn publish_melee_damage_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
    }

    pub fn publish_send_if_visible_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendIfVisibleLikeCpp(command))
    }

    /// Publish START+GO as one queue element so capacity checks and session
    /// drains cannot observe only one half of a committed spell cast.
    pub fn publish_creature_spell_cast_if_visible_like_cpp(
        &mut self,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
    }

    pub fn drain_like_cpp(&mut self) -> Vec<SessionCommand> {
        self.commands.drain(..).collect()
    }

    pub fn take_overflowed_and_discard_like_cpp(&mut self) -> bool {
        let overflowed = std::mem::take(&mut self.overflowed);
        if overflowed {
            self.commands.clear();
        }
        overflowed
    }
}

/// Durable result of one character-row money mutation.
///
/// The detached SQL worker records this before publishing its session command.
/// Both paths share [`Self::applied`], so logout reconciliation and normal
/// command delivery cannot apply the same durable delta twice.
#[derive(Clone, Debug)]
pub struct DurableLootMoneyCompletionLikeCpp {
    pub durable_money_before: u64,
    pub durable_money_after: u64,
    pub durable_applied_amount: u64,
    pub applied: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct DurableLootMoneyPersistenceStateLikeCpp {
    in_flight: usize,
    completions: Vec<DurableLootMoneyCompletionLikeCpp>,
    indeterminate: bool,
    admission_closed: bool,
    permanently_closed: bool,
    active_save_fences: usize,
}

/// Per-character fence for detached loot-money transactions.
///
/// A tracker is published in [`PlayerBroadcastInfo`] and registered directly
/// by a source session before it opens a transaction for every recipient. This
/// avoids command acknowledgements (and their A↔B deadlocks), while allowing a
/// target session to wait for and reconcile durable completions before an
/// absolute `Player::SaveToDB` money write.
#[derive(Debug)]
pub struct DurableLootMoneyPersistenceTrackerLikeCpp {
    state: Mutex<DurableLootMoneyPersistenceStateLikeCpp>,
    changed: tokio::sync::watch::Sender<u64>,
    money_mutation_serial: Arc<tokio::sync::Mutex<()>>,
}

impl Default for DurableLootMoneyPersistenceTrackerLikeCpp {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            state: Mutex::new(DurableLootMoneyPersistenceStateLikeCpp::default()),
            changed,
            money_mutation_serial: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}

impl DurableLootMoneyPersistenceTrackerLikeCpp {
    /// Serialize DB money mutations for this character across stored-item and
    /// group payouts. Multi-recipient workers acquire these locks in sorted
    /// GUID order before taking the matching character-row locks.
    pub async fn lock_money_mutation_like_cpp(&self) -> tokio::sync::OwnedMutexGuard<()> {
        Arc::clone(&self.money_mutation_serial).lock_owned().await
    }

    #[must_use]
    pub fn begin_like_cpp(
        self: &Arc<Self>,
    ) -> Result<DurableLootMoneyPersistenceGuardLikeCpp, DurableLootMoneyAdmissionClosedLikeCpp>
    {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.admission_closed {
            return Err(DurableLootMoneyAdmissionClosedLikeCpp);
        }
        state.in_flight += 1;
        drop(state);
        let _ = self.changed.send_modify(|generation| {
            *generation = generation.wrapping_add(1);
        });
        Ok(DurableLootMoneyPersistenceGuardLikeCpp {
            tracker: Arc::clone(self),
            resolved: false,
        })
    }

    /// Close admission before observing `in_flight` and keep it closed across
    /// snapshot plus SQL commit. Either a source registers first and the save
    /// waits, or the save closes first and the source fails before BEGIN.
    #[must_use]
    pub fn close_admission_for_save_like_cpp(self: &Arc<Self>) -> DurableLootMoneySaveFenceLikeCpp {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.active_save_fences = state.active_save_fences.saturating_add(1);
        state.admission_closed = true;
        DurableLootMoneySaveFenceLikeCpp {
            tracker: Arc::clone(self),
        }
    }

    /// Logout closes the old registry-published tracker permanently. A source
    /// that cloned it before unregister cannot mutate the character after its
    /// final save.
    pub fn close_admission_permanently_like_cpp(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.admission_closed = true;
        state.permanently_closed = true;
    }

    pub async fn wait_until_idle_like_cpp(&self) {
        let mut changed = self.changed.subscribe();
        loop {
            if self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .in_flight
                == 0
            {
                return;
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }

    /// Returns every completion whose shared exact-once gate is still open.
    /// Applied entries are pruned only after their CAS is observable.
    #[must_use]
    pub fn pending_completions_like_cpp(&self) -> Vec<DurableLootMoneyCompletionLikeCpp> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.completions.retain(|completion| {
            !completion
                .applied
                .load(std::sync::atomic::Ordering::Acquire)
        });
        state.completions.clone()
    }

    #[must_use]
    pub fn is_indeterminate_like_cpp(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .indeterminate
    }

    pub fn mark_indeterminate_like_cpp(&self) {
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.indeterminate = true;
            state.admission_closed = true;
            state.permanently_closed = true;
        }
        let _ = self.changed.send_modify(|generation| {
            *generation = generation.wrapping_add(1);
        });
    }

    fn finish_like_cpp(
        &self,
        completion: Option<DurableLootMoneyCompletionLikeCpp>,
        indeterminate: bool,
    ) {
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            debug_assert!(state.in_flight != 0);
            state.in_flight = state.in_flight.saturating_sub(1);
            if let Some(completion) = completion {
                state.completions.push(completion);
            }
            state.indeterminate |= indeterminate;
            if indeterminate {
                state.admission_closed = true;
                state.permanently_closed = true;
            }
        }
        let _ = self.changed.send_modify(|generation| {
            *generation = generation.wrapping_add(1);
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurableLootMoneyAdmissionClosedLikeCpp;

impl std::fmt::Display for DurableLootMoneyAdmissionClosedLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("character money persistence admission is closed")
    }
}

impl std::error::Error for DurableLootMoneyAdmissionClosedLikeCpp {}

#[derive(Debug)]
pub struct DurableLootMoneySaveFenceLikeCpp {
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
}

impl Drop for DurableLootMoneySaveFenceLikeCpp {
    fn drop(&mut self) {
        let mut state = self
            .tracker
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        debug_assert!(state.active_save_fences != 0);
        state.active_save_fences = state.active_save_fences.saturating_sub(1);
        if state.active_save_fences == 0 && !state.permanently_closed {
            state.admission_closed = false;
        }
    }
}

/// RAII registration for one recipient's durable money mutation.
#[derive(Debug)]
pub struct DurableLootMoneyPersistenceGuardLikeCpp {
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    resolved: bool,
}

impl DurableLootMoneyPersistenceGuardLikeCpp {
    pub fn commit_like_cpp(&mut self, completion: DurableLootMoneyCompletionLikeCpp) {
        if self.resolved {
            return;
        }
        self.resolved = true;
        self.tracker.finish_like_cpp(Some(completion), false);
    }

    /// A COMMIT was attempted but its outcome could not be reconciled. The
    /// target must skip absolute money saves until it disconnects/reloads.
    pub fn mark_indeterminate_like_cpp(&mut self) {
        if self.resolved {
            return;
        }
        self.resolved = true;
        self.tracker.finish_like_cpp(None, true);
    }
}

impl Drop for DurableLootMoneyPersistenceGuardLikeCpp {
    fn drop(&mut self) {
        if !self.resolved {
            self.tracker.finish_like_cpp(None, false);
            self.resolved = true;
        }
    }
}
